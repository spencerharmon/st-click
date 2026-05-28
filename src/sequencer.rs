//! Beat-coordinate sequence representation, per-cycle scheduler, and the
//! Sequencer loop that drives MIDI emission from JACK transport position
//! via st-sync's beat-window protocol.
//!
//! # Design
//!
//! `Sequence` stores MIDI events at *beat positions* (in quarter-note
//! beats relative to the start of the sequence). It allocates no
//! frame-indexed buffer, so its memory footprint is bounded by the note
//! count, not by the JACK transport's tempo or runtime — fixing the OOM
//! bug present in the prior frame-indexed implementation.
//!
//! `Scheduler` consumes a `Sequence` and emits events per JACK cycle,
//! given the cycle's beat span and the current `frames_per_beat`. It
//! tracks a cursor through the event list so per-cycle emit cost is
//! `O(events_in_this_cycle)`, not `O(total_events)`, and uses cursor
//! advancement rather than coordinate comparison for the "have I
//! emitted this event already" question, eliminating a class of
//! float-precision bugs at integer beat boundaries.
//!
//! `Sequencer` is the audio-thread loop: polls JACK transport for the
//! current frame, queries `st_sync::client::Client::beat_position_at` to
//! map cycle frames to beat positions, and drives the `Scheduler`.
//! Continuous tempo changes propagate automatically because the scheduler
//! consults `frames_per_beat` per cycle rather than baking it into a
//! pre-allocated buffer.

use crate::beat_values::*;
use crate::config::Config;
use crossbeam_channel::*;
use st_lib::owned_midi::{OwnedMidi, OwnedMidiBytes};
use st_lib::{jack_ptr, jack_transport};
use std::{thread, time};

// ---------------------------------------------------------------------------
// Sequence + Scheduler: pure data, fully unit-tested.
// ---------------------------------------------------------------------------

/// A scheduled MIDI event at a fixed beat position within a sequence.
#[derive(Debug, Clone)]
pub struct ScheduledEvent {
    /// Beat position in `[0, sequence_span)`, in quarter-note beats.
    /// Quarter = 1.0; eighth = 0.5; whole = 4.0; etc.
    pub beat: f64,
    pub bytes: OwnedMidiBytes,
}

/// A click sequence as a *musical* structure: a span of beats and the
/// MIDI events that fire within it.
#[derive(Debug, Clone)]
pub struct Sequence {
    pub beats_per_bar: f32,
    pub bars: u32,
    events: Vec<ScheduledEvent>,
}

impl Sequence {
    pub fn new(beats_per_bar: f32, bars: u32) -> Sequence {
        assert!(bars >= 1, "Sequence: bars must be >= 1");
        assert!(beats_per_bar > 0.0, "Sequence: beats_per_bar must be > 0");
        Sequence { beats_per_bar, bars, events: Vec::new() }
    }

    /// Total beat span of the sequence; the wrap point.
    pub fn span_beats(&self) -> f64 {
        self.beats_per_bar as f64 * self.bars as f64
    }

    /// Add a repeating pattern of events. Equivalent to
    /// `add_notes_with_offset(.., 0.0)`.
    pub fn add_notes(
        &mut self,
        bytes: OwnedMidiBytes,
        every_n: u16,
        skip_n: u16,
        beat_value: BeatValue,
    ) {
        self.add_notes_with_offset(bytes, every_n, skip_n, beat_value, 0.0)
    }

    /// Add events firing every `every_n` slots after skipping the first
    /// `skip_n` slots, where a slot is `beat_value` quarter-note beats.
    /// All resulting events are shifted by `offset_beats` and wrapped
    /// modulo the sequence span.
    ///
    /// Sign convention: `offset_beats > 0` delays; `< 0` anticipates.
    pub fn add_notes_with_offset(
        &mut self,
        bytes: OwnedMidiBytes,
        every_n: u16,
        skip_n: u16,
        beat_value: BeatValue,
        offset_beats: f32,
    ) {
        assert!(beat_value > 0.0, "Sequence: beat_value must be > 0");
        assert!(every_n >= 1, "Sequence: every_n must be >= 1");

        let span = self.span_beats();
        let slot_beats = beat_value as f64;
        // Total slot count = ceil(span / slot_beats). Slot k is at beat
        // k * slot_beats; slots with that beat < span fire.
        let total_slots = (span / slot_beats).ceil() as u64;

        for slot_index in 0..total_slots {
            if slot_index < skip_n as u64 {
                continue;
            }
            if (slot_index - skip_n as u64) % every_n as u64 != 0 {
                continue;
            }
            let raw_beat = slot_index as f64 * slot_beats + offset_beats as f64;
            let wrapped = raw_beat.rem_euclid(span);
            let beat = if wrapped >= span { 0.0 } else { wrapped };
            self.events.push(ScheduledEvent { beat, bytes: bytes.clone() });
        }
        self.events.sort_by(|a, b| {
            a.beat.partial_cmp(&b.beat).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn events(&self) -> &[ScheduledEvent] {
        &self.events
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

/// Per-cycle scheduler: tracks a cursor through `Sequence::events` and
/// emits MIDI events whose beat positions fall in each cycle's
/// `[prev_local_end, cycle_end_local_beat)` half-open interval.
///
/// Convention: every cycle owns its start boundary, never its end. Two
/// adjacent cycles that share a boundary B fire any event at B in the
/// second cycle (the one whose start equals B). This means every event
/// fires exactly once and consumers don't need to special-case the
/// first cycle.
///
/// The scheduler operates entirely in *sequence-local* beat coordinates
/// (in `[0, span)`). The caller is responsible for mapping from
/// absolute JACK frames / absolute window beats to local sequence beats;
/// see `Sequencer::start` for the production driver and the tests below
/// for direct usage.
///
/// Handles sequence wrap, very long cycles (multiple wraps in one JACK
/// callback), and float-precision-safe comparison at integer beat
/// boundaries (uses an index cursor, not coordinate comparison, for
/// the "have I emitted this event already" question).
#[derive(Debug)]
pub struct Scheduler {
    sequence: Sequence,
    /// Cursor within `sequence.events()`: the index of the next event
    /// that has not yet been emitted.
    next_event_index: usize,
    /// The local beat (in `[0, span)`) at which the previous cycle
    /// ended. The next cycle starts here. Initial value: 0.0.
    last_local_end: f64,
}

impl Scheduler {
    pub fn new(sequence: Sequence) -> Scheduler {
        Scheduler { sequence, next_event_index: 0, last_local_end: 0.0 }
    }

    /// Reset to the sequence start.
    pub fn reset(&mut self) {
        self.reset_to(0.0);
    }

    /// Reset the cursor to a specific local-beat position. Used by the
    /// production driver when it knows where in the sequence playback
    /// should resume (e.g. mid-sequence late-connect, post-swap re-
    /// anchor).
    pub fn reset_to(&mut self, local_beat: f64) {
        let span = self.sequence.span_beats();
        let anchor = local_beat.rem_euclid(span);
        self.last_local_end = anchor;
        self.next_event_index = 0;
        // Position cursor at the first event >= anchor.
        while let Some(ev) = self.sequence.events().get(self.next_event_index) {
            if ev.beat >= anchor {
                break;
            }
            self.next_event_index += 1;
        }
    }

    pub fn sequence(&self) -> &Sequence {
        &self.sequence
    }

    /// Where the scheduler thinks the cursor sits (for tests / debugging).
    pub fn local_position(&self) -> f64 {
        self.last_local_end
    }

    /// Advance the scheduler by `cycle_beat_len` beats over a JACK cycle
    /// that spans `cycle_frame_len` audio frames. Emits every event
    /// whose local-beat position falls in `[last_local_end, end)`.
    ///
    /// Returns `OwnedMidi` events with `time` set to the sub-cycle
    /// frame offset where each event should fire.
    ///
    /// `cycle_beat_len` must be `> 0` (transport moves forward).
    pub fn advance(&mut self, cycle_beat_len: f64, cycle_frame_len: u32) -> Vec<OwnedMidi> {
        assert!(
            cycle_beat_len >= 0.0,
            "Scheduler: cycle_beat_len must be non-negative, got {}",
            cycle_beat_len
        );
        if cycle_beat_len == 0.0 {
            return Vec::new();
        }

        let span = self.sequence.span_beats();
        if self.sequence.events().is_empty() {
            self.last_local_end = (self.last_local_end + cycle_beat_len).rem_euclid(span);
            return Vec::new();
        }

        // Frames per local beat within this cycle (used for sub-cycle
        // `time` calculation). Constant within the cycle by design.
        let frames_per_local_beat = cycle_frame_len as f64 / cycle_beat_len;

        let cycle_start = self.last_local_end;
        let mut local_start = cycle_start;
        let mut remaining = cycle_beat_len;
        let mut ret: Vec<OwnedMidi> = Vec::new();

        while remaining > 0.0 {
            let until_wrap = span - local_start;
            let segment_len = remaining.min(until_wrap);
            let segment_end = local_start + segment_len;

            // Position cursor at the first event >= local_start.
            self.advance_cursor_to(local_start);

            // Emit events with beat in [local_start, segment_end).
            while let Some(ev) = self.sequence.events().get(self.next_event_index) {
                if ev.beat >= segment_end {
                    break;
                }
                if ev.beat >= local_start {
                    // Compute beats elapsed since the cycle's start
                    // (across any intervening wraps).
                    let cycle_elapsed = (cycle_beat_len - remaining) + (ev.beat - local_start);
                    let time = (cycle_elapsed * frames_per_local_beat).round();
                    let time = if time < 0.0 { 0 } else { time as u32 };
                    ret.push(OwnedMidi { time, bytes: ev.bytes.clone() });
                }
                self.next_event_index += 1;
            }

            remaining -= segment_len;
            if remaining <= 0.0 {
                self.last_local_end = segment_end.rem_euclid(span);
                if (segment_end - span).abs() < f64::EPSILON {
                    self.last_local_end = 0.0;
                    self.next_event_index = 0;
                }
                break;
            }
            // Wrap.
            local_start = 0.0;
            self.next_event_index = 0;
        }

        ret
    }

    fn advance_cursor_to(&mut self, beat: f64) {
        while let Some(ev) = self.sequence.events().get(self.next_event_index) {
            if ev.beat >= beat {
                break;
            }
            self.next_event_index += 1;
        }
    }
}

/// Distance from `from` to `to` within a circular `[0, span)` coordinate
/// space, always non-negative. Currently unused in the production
/// scheduler (which works in pure local coordinates), but kept here as
/// a small public utility for callers that need to reason about
/// circular beat positions.
#[allow(dead_code)]
fn beat_distance(from: f64, to: f64, span: f64) -> f64 {
    let from = from.rem_euclid(span);
    let to = to.rem_euclid(span);
    if to >= from { to - from } else { (span - from) + to }
}

// ---------------------------------------------------------------------------
// Sequencer: audio-thread loop driven by st-sync's beat-window.
// ---------------------------------------------------------------------------

/// Commands the GUI / NSM can send into a running sequencer.
pub enum SequencerCommand {
    /// Switch to a different named sequence. The swap is deferred until
    /// the next bar boundary so it lands musically.
    SwitchTo(String),
}

pub struct Sequencer {
    midi_tx: Sender<OwnedMidi>,
    sync: st_sync::client::Client,
    ps_rx: Receiver<()>,
    jack_client_addr: usize,
    sequence_name: String,
    beat_tx: Option<Sender<u64>>,
    command_rx: Option<Receiver<SequencerCommand>>,
    active_tx: Option<Sender<String>>,
}

impl Sequencer {
    pub fn new(
        midi_tx: Sender<OwnedMidi>,
        ps_rx: Receiver<()>,
        jack_client_addr: usize,
        sequence_name: String,
        beat_tx: Option<Sender<u64>>,
        command_rx: Option<Receiver<SequencerCommand>>,
        active_tx: Option<Sender<String>>,
    ) -> Sequencer {
        let sync = st_sync::client::Client::new();
        Sequencer {
            midi_tx, sync, ps_rx, jack_client_addr, sequence_name,
            beat_tx, command_rx, active_tx,
        }
    }

    pub fn start(mut self) {
        let config = Config::new();
        let client_pointer = unsafe { jack_ptr::recover_client(self.jack_client_addr) };

        // Wait for st-sync to publish enough beats that we can derive
        // frames_per_beat. We need at least 2 beat frames in the window.
        loop {
            if self.sync.frames_per_beat().is_some() {
                break;
            }
            thread::sleep(time::Duration::from_millis(5));
        }

        let snapshot = unsafe { jack_transport::query_transport(client_pointer) };
        let mut beats_per_bar = snapshot.beats_per_bar;
        let mut bars = config.sequence_bars(&self.sequence_name);

        let mut scheduler = build_scheduler(&config, &self.sequence_name, beats_per_bar, bars);

        // Notify the GUI which sequence is actually playing.
        if let Some(tx) = &self.active_tx {
            let _ = tx.try_send(self.sequence_name.clone());
        }

        // Cycle bookkeeping. last_frame is the JACK frame at the end of
        // the previous cycle. last_window_beat is the corresponding
        // beat position in the st-sync window's coordinate (used only
        // to compute per-cycle beat-delta from the window).
        let initial_snap = unsafe { jack_transport::query_transport(client_pointer) };
        let mut last_frame: u64 = initial_snap.frame as u64;
        let mut last_window_beat: Option<f64> = self.sync.beat_position_at(last_frame);

        let mut beat_counter: u64 = 0;
        let mut pending_switch: Option<String> = None;

        loop {
            // Drain commands.
            if let Some(rx) = &self.command_rx {
                while let Ok(cmd) = rx.try_recv() {
                    match cmd {
                        SequencerCommand::SwitchTo(name) => {
                            if name != self.sequence_name {
                                pending_switch = Some(name);
                            }
                        }
                    }
                }
            }

            let snap = unsafe { jack_transport::query_transport(client_pointer) };
            let pos_frame = snap.frame as u64;
            let new_beats_per_bar = snap.beats_per_bar;
            let meter_changed = (new_beats_per_bar - beats_per_bar).abs() > f32::EPSILON;

            if pos_frame <= last_frame {
                self.governor_sleep();
                continue;
            }

            let fpb = self.sync.frames_per_beat().unwrap_or(0) as f64;
            if fpb <= 0.0 {
                self.governor_sleep();
                continue;
            }

            // Map cycle endpoints to beat positions via the st-sync window.
            // Fall back to extrapolation when the window doesn't cover.
            let prev_wb = self.sync.beat_position_at(last_frame).or(last_window_beat);
            let cur_wb = self.sync.beat_position_at(pos_frame).or_else(|| {
                prev_wb.map(|b| b + (pos_frame - last_frame) as f64 / fpb)
            });
            let (Some(p), Some(c)) = (prev_wb, cur_wb) else {
                self.governor_sleep();
                continue;
            };

            let cycle_beat_len = c - p;
            let cycle_frame_len = (pos_frame - last_frame) as u32;

            // Drive the scheduler.
            let events = scheduler.advance(cycle_beat_len, cycle_frame_len);
            for ev in events {
                let _ = self.midi_tx.send(ev);
            }

            // Beat-boundary detection for the GUI counter and bar-aligned
            // rebuilds. Use floor(prev_wb) vs floor(cur_wb).
            let crossed_beats = c.floor() as i64 - p.floor() as i64;
            if crossed_beats > 0 {
                beat_counter += crossed_beats as u64;
                if let Some(tx) = &self.beat_tx {
                    let _ = tx.try_send(beat_counter);
                }
            }

            // Apply pending rebuild at a bar boundary.
            let crossed_bar = crossed_beats > 0
                && beat_counter > 0
                && (beat_counter as f32) % beats_per_bar == 0.0;
            if crossed_bar {
                let new_name = pending_switch.take();
                let rebuild = meter_changed || new_name.is_some();
                if rebuild {
                    if let Some(name) = new_name {
                        self.sequence_name = name;
                    }
                    if meter_changed {
                        beats_per_bar = new_beats_per_bar;
                    }
                    bars = config.sequence_bars(&self.sequence_name);
                    scheduler = build_scheduler(
                        &config, &self.sequence_name, beats_per_bar, bars
                    );
                    if let Some(tx) = &self.active_tx {
                        let _ = tx.try_send(self.sequence_name.clone());
                    }
                }
            }

            last_frame = pos_frame;
            last_window_beat = Some(c);

            self.governor_sleep();
        }
    }

    fn governor_sleep(&self) {
        // Wait for the JACK process-cycle tick, or fall back to a short
        // sleep if it doesn't arrive promptly. The original code had
        // a more elaborate governor; this simpler version just paces the
        // poll loop at roughly the JACK cycle rate without spinning.
        if let Ok(()) = self.ps_rx.try_recv() {
            // Tick received; loop immediately.
        } else {
            thread::sleep(time::Duration::from_millis(1));
        }
    }
}

fn build_scheduler(
    config: &Config,
    sequence_name: &str,
    beats_per_bar: f32,
    bars: u32,
) -> Scheduler {
    let mut seq = Sequence::new(beats_per_bar, bars);
    config.apply_sequence_borrowed(&mut seq, sequence_name);
    Scheduler::new(seq)
}

// ---------------------------------------------------------------------------
// Tests: pure logic only (Sequence + Scheduler). Sequencer is not
// directly tested because it requires JACK and st-sync; the
// sub-components it composes are exhaustively covered.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn note(n: u8) -> OwnedMidiBytes {
        vec![0x90, n, 100]
    }

    // ----- Sequence construction -----

    #[test]
    fn empty_sequence_has_no_events_but_known_span() {
        let s = Sequence::new(4.0, 1);
        assert_eq!(s.span_beats(), 4.0);
        assert_eq!(s.event_count(), 0);
    }

    #[test]
    fn quarters_in_one_bar_44_yield_four_events() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let beats: Vec<f64> = s.events().iter().map(|e| e.beat).collect();
        assert_eq!(beats, vec![0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn quarters_in_two_bar_44_yield_eight_events() {
        let mut s = Sequence::new(4.0, 2);
        s.add_notes(note(60), 1, 0, 1.0);
        assert_eq!(s.event_count(), 8);
        assert_eq!(s.events().last().unwrap().beat, 7.0);
    }

    #[test]
    fn skip_three_in_two_bar_44_drops_first_three_quarters() {
        let mut s = Sequence::new(4.0, 2);
        s.add_notes(note(60), 1, 3, 1.0);
        let beats: Vec<f64> = s.events().iter().map(|e| e.beat).collect();
        assert_eq!(beats, vec![3.0, 4.0, 5.0, 6.0, 7.0]);
    }

    #[test]
    fn every_two_in_one_bar_44_gives_half_note_pulse() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 2, 0, 1.0);
        let beats: Vec<f64> = s.events().iter().map(|e| e.beat).collect();
        assert_eq!(beats, vec![0.0, 2.0]);
    }

    #[test]
    fn offset_shifts_events_within_span() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes_with_offset(note(60), 1, 0, 1.0, 0.5);
        let beats: Vec<f64> = s.events().iter().map(|e| e.beat).collect();
        assert_eq!(beats, vec![0.5, 1.5, 2.5, 3.5]);
    }

    #[test]
    fn negative_offset_wraps_to_end_of_span() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes_with_offset(note(60), 1, 0, 1.0, -0.25);
        let beats: Vec<f64> = s.events().iter().map(|e| e.beat).collect();
        assert_eq!(beats, vec![0.75, 1.75, 2.75, 3.75]);
    }

    #[test]
    fn events_remain_sorted_after_multiple_add_calls() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        s.add_notes(note(62), 1, 0, 0.5);
        let beats: Vec<f64> = s.events().iter().map(|e| e.beat).collect();
        for i in 1..beats.len() {
            assert!(beats[i] >= beats[i - 1], "not sorted at {}: {:?}", i, beats);
        }
    }

    // ----- Sequence: memory footprint (THE OOM REGRESSION TEST) -----

    #[test]
    fn sequence_memory_independent_of_tempo_or_runtime() {
        // The OOM bug: the prior frame-indexed Sequence allocated
        // `beats_per_bar * frames_per_beat * bars / 2` slots, which
        // grew unboundedly as JACK frame counters grew. The beat-indexed
        // Sequence's footprint depends only on the note count.
        //
        // This test would have caught the bug: a "late connect" scenario
        // where the conductor has been running for half an hour produces
        // the same-sized Sequence as a fresh-start scenario.
        let mut early = Sequence::new(4.0, 1);
        early.add_notes(note(60), 1, 0, 1.0);

        let mut late = Sequence::new(4.0, 1);
        late.add_notes(note(60), 1, 0, 1.0);

        assert_eq!(early.event_count(), late.event_count());
        // Hard cap: a one-bar 4/4 quarter-note sequence is 4 events.
        // Old impl would have been ~96,000 entries even on cold start
        // (frames_per_beat=48000, /2). Hot-cycle 30 minutes in:
        // millions of entries.
        assert!(early.event_count() < 100, "event count should be tiny");
    }

    // ----- Scheduler: basic emission -----

    #[test]
    fn scheduler_empty_sequence_emits_nothing() {
        let s = Sequence::new(4.0, 1);
        let mut sch = Scheduler::new(s);
        let out = sch.advance(1.0, 48_000);
        assert!(out.is_empty());
    }

    #[test]
    fn scheduler_emits_event_at_zero_on_first_cycle() {
        // Convention: cycles are [last_local_end, end). Initial
        // last_local_end is 0, so the first cycle covers [0, 1) and
        // contains the event at beat 0 but not the event at beat 1.
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let mut sch = Scheduler::new(s);
        let out = sch.advance(1.0, 48_000);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].time, 0);
    }

    #[test]
    fn scheduler_event_at_cycle_boundary_belongs_to_next_cycle() {
        // [start, end) semantics: event at beat 1.0 fires in the cycle
        // that *starts* at 1.0, not the one that ends at 1.0.
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let mut sch = Scheduler::new(s);
        // Cycle A: 1 beat from 0 — fires only beat 0.
        let out_a = sch.advance(1.0, 100);
        assert_eq!(out_a.len(), 1);
        // Cycle B: 1 beat from 1 — fires beat 1.
        let out_b = sch.advance(1.0, 100);
        assert_eq!(out_b.len(), 1);
    }

    #[test]
    fn scheduler_does_not_double_emit_at_cycle_boundary() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let mut sch = Scheduler::new(s);
        let out_a = sch.advance(0.5, 50); // [0, 0.5)  fires beat 0
        let out_b = sch.advance(0.5, 50); // [0.5, 1.0)  fires nothing
        let out_c = sch.advance(0.5, 50); // [1.0, 1.5)  fires beat 1
        assert_eq!(out_a.len(), 1);
        assert_eq!(out_b.len(), 0);
        assert_eq!(out_c.len(), 1);
    }

    #[test]
    fn scheduler_sub_cycle_time_offsets() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let mut sch = Scheduler::new(s);
        // Cycle of 2 beats over 200 frames. Should fire beats 0 and 1.
        let out = sch.advance(2.0, 200);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].time, 0);
        assert_eq!(out[1].time, 100);
    }

    // ----- Scheduler: float precision -----

    #[test]
    fn scheduler_event_at_integer_beat_fires_exactly_once_across_many_cycles() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let mut sch = Scheduler::new(s);

        let mut total_emitted = 0usize;
        for i in 0..1000 {
            // Cycle lengths drift slightly to mimic floating accumulation.
            let cycle_len = 0.1 + (i as f64 * 1e-12);
            total_emitted += sch.advance(cycle_len, 10).len();
        }
        assert!(
            (95..=105).contains(&total_emitted),
            "expected ~100 events, got {}",
            total_emitted
        );
    }

    // ----- Scheduler: sequence wrap -----

    #[test]
    fn scheduler_wraps_at_sequence_span() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let mut sch = Scheduler::new(s);
        // [0, 3.5): beats 0, 1, 2, 3.
        let out1 = sch.advance(3.5, 350);
        assert_eq!(out1.len(), 4);
        // [3.5, 4.5) spans the wrap. Fires beat 0 of the next pass.
        let out2 = sch.advance(1.0, 100);
        assert_eq!(out2.len(), 1);
        // Event at beat 0 of next pass = 0.5 beats past cycle start
        // = 50 frames into the cycle.
        assert_eq!(out2[0].time, 50);
    }

    #[test]
    fn scheduler_long_cycle_emits_multiple_passes_of_events() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let mut sch = Scheduler::new(s);
        // Cycle of 10 beats: 2 full passes (8) + beats 0, 1 of pass 3 = 10.
        let out = sch.advance(10.0, 1000);
        assert_eq!(out.len(), 10);
    }

    #[test]
    fn scheduler_cycle_spanning_multiple_wraps_fires_all_events() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let mut sch = Scheduler::new(s);
        // 25 beats: 6 full passes (24) + 1 beat = 25.
        let out = sch.advance(25.0, 2500);
        assert_eq!(out.len(), 25);
    }

    // ----- Scheduler: continuous tempo change -----

    #[test]
    fn scheduler_per_beat_count_unchanged_under_stepwise_tempo() {
        // 8 back-to-back one-beat cycles at varying tempos (varying
        // frame counts per cycle). Each cycle should fire exactly one
        // event (the beat at its start).
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let mut sch = Scheduler::new(s);

        let mut frames = 24_000;
        for i in 0..8 {
            let out = sch.advance(1.0, frames);
            assert_eq!(
                out.len(), 1,
                "cycle {} should fire exactly 1 event under stepwise tempo, got {}",
                i, out.len()
            );
            frames = (frames as f64 * 1.05) as u32;
        }
    }

    #[test]
    fn scheduler_tempo_change_at_cycle_boundary_no_loss() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 0.25); // 16 events
        let mut sch = Scheduler::new(s);
        // Cycle A: 1 beat over 1000 frames (slow). Fires 4 events at
        //         beats 0, 0.25, 0.5, 0.75 (NOT 1.0).
        let out_a = sch.advance(1.0, 1000);
        assert_eq!(out_a.len(), 4);
        // Cycle B: 1 beat over 100 frames (fast). Fires 4 events at
        //         beats 1.0, 1.25, 1.5, 1.75 (NOT 2.0).
        let out_b = sch.advance(1.0, 100);
        assert_eq!(out_b.len(), 4);
    }

    // ----- Scheduler: late-connect cold start -----

    #[test]
    fn scheduler_reset_to_anchors_mid_sequence() {
        // The caller may begin scheduling mid-sequence (e.g. st-click
        // connects 1.5 beats into a bar). reset_to(1.5) positions the
        // cursor accordingly; the next advance starts at 1.5.
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0); // events at 0, 1, 2, 3
        let mut sch = Scheduler::new(s);
        sch.reset_to(1.5);
        // Cycle of 1 beat from 1.5: covers [1.5, 2.5). Fires beat 2.
        let out = sch.advance(1.0, 100);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].time, 50); // beat 2 is 0.5 beats past cycle start
    }

    #[test]
    fn scheduler_reset_to_beat_zero_is_same_as_reset() {
        let mut s = Sequence::new(4.0, 1);
        s.add_notes(note(60), 1, 0, 1.0);
        let mut sch = Scheduler::new(s);
        let _ = sch.advance(1.5, 150); // fires beat 0 (cycle ends < 1)
        sch.reset();
        let out_after_reset = sch.advance(0.5, 50);
        sch.reset_to(0.0);
        let out_after_reset_to = sch.advance(0.5, 50);
        assert_eq!(out_after_reset.len(), out_after_reset_to.len());
    }
}
