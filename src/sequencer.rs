use crate::beat_values::*;
use crossbeam_channel::*;
use std::{thread, time};
//use crate::note_map;
use crate::config::Config;
use st_lib::owned_midi::{OwnedMidi, OwnedMidiBytes};
use st_lib::{jack_ptr, jack_transport};

fn get_beat_value(base_beat_value: BeatValue, n_dots: u16, n_tuplet: u16) -> f32 {
    ((base_beat_value * 2.0) / n_tuplet as f32) * f32::powf(1.5, n_dots.into())
}

pub struct Sequence {
    beats_per_bar: f32,
    frames_per_beat: u64,
    length: u64,
    seq: Vec<Vec<OwnedMidiBytes>>,
    playhead: u64,
    last_frame: u64,
    n_beats: u16,
    beat_counter: u16
}
impl Sequence {
    pub fn new (beats_per_bar: f32, frames_per_beat: u64, bars: u32) -> Sequence {
	let length = beats_per_bar as u64 * frames_per_beat * bars as u64 / 2;
	let mut seq: Vec<Vec<OwnedMidiBytes>> = Vec::new();
	for i in 0..length as usize {
	    let v = Vec::new();
	    seq.push(v);
	}

	let playhead = 0;
	let last_frame = 0;

	let n_beats = (beats_per_bar as u32 * bars) as u16;
	let beat_counter = 0;
	Sequence {
	    beats_per_bar,
	    frames_per_beat,
	    length,
	    seq,
	    playhead,
	    last_frame,
	    n_beats,
	    beat_counter
	}
    }
    pub fn add_notes(&mut self, signal: OwnedMidiBytes, every_n: u16, skip_n: u16, beat_value: BeatValue){
	self.add_notes_with_offset(signal, every_n, skip_n, beat_value, 0.0)
    }

    /// Like `add_notes`, but additionally shifts every emitted slot by
    /// `offset_beats` quarter-note beats (signed). Positive = later,
    /// negative = earlier. Wraps modulo `self.length`.
    ///
    /// `offset_beats` is independent of `beat_value`, so it composes
    /// naturally with `tuplet` constructions — e.g. a quarter triplet
    /// with `offset_beats = 1.0` stays a triplet, just shifted by a
    /// quarter against the bar.
    pub fn add_notes_with_offset(
	&mut self,
	signal: OwnedMidiBytes,
	every_n: u16,
	skip_n: u16,
	beat_value: BeatValue,
	offset_beats: f32,
    ){
	let frames = beat_value * self.frames_per_beat as f32 / 2.0;
	// Same `/2` convention as `frames` above and `Sequence::new`'s
	// `length` (half-frame grid).
	let offset_frames =
	    (offset_beats * self.frames_per_beat as f32 / 2.0).round() as i64;
	let length = self.length as i64;
	let mut n = skip_n;
	let mut beat: i32 = 0;
	for i in 0..self.length {
	    if (i as f32) % frames < 1.0 {
		//frame matches beat value
		if n == 0 && (beat - skip_n as i32) % every_n as i32 == 0 {
		    //n beats have been skipped. go time.
		    let shifted = (i as i64 + offset_frames).rem_euclid(length) as usize;
		    self.seq[shifted].push(signal.to_owned());
		    n = 0;
		} else if n > 0 {
		    n = n - 1;
		}
		beat = beat + 1;
	    }
	}
    }
    fn set_frame(&mut self, frame: u64){
	self.last_frame = frame;
    }

    fn process_position(&mut self,
			pos_frame: u64,
			next_beat_frame: u64,
			beat_this_cycle: bool
			
    ) -> Vec<OwnedMidi> {
	let mut ret = Vec::new();

	let mut beat_frame = 1;
	let nframes = pos_frame - self.last_frame;

	if beat_this_cycle {
	    if self.last_frame == 0 {
		beat_frame = 1;
	    } else {
		beat_frame = next_beat_frame - self.last_frame;
	    }
	}


	
	for i in 1..nframes + 1 {
	    let v = &mut self.seq.get_mut((self.playhead) as usize);
	    for iv in v {
		for m in &mut **iv {
    		    let mut om = OwnedMidi { time: (i/2) as u32, bytes: Vec::new() };
		    for b in m.to_owned() {
			om.bytes.push(b);
		    }
		    ret.push(om);
		}
	    }
	    if beat_this_cycle && i == beat_frame {
		if self.beat_counter == self.n_beats {
		    self.beat_counter = 1;
		    self.playhead = 0;
		} else {
		    self.beat_counter = self.beat_counter + 1;
		}
	    } else {
		self.playhead = self.playhead + 1;
	    }
	}
	self.last_frame = pos_frame;
	ret
    }
    
}

pub struct Sequencer{
    midi_tx: Sender<OwnedMidi>,
    sync: st_sync::client::Client,
    ps_rx: Receiver<()>,
    jack_client_addr: usize,
    sequence_name: String,
    /// Optional GUI sink: each detected beat boundary is pushed here as
    /// the running 1-based beat counter. None when running headless.
    beat_tx: Option<Sender<u64>>,
}
impl Sequencer {
    pub fn new(midi_tx: Sender<OwnedMidi>,
	       ps_rx: Receiver<()>,
	       jack_client_addr: usize,
	       sequence_name: String,
	       beat_tx: Option<Sender<u64>>,
    ) -> Sequencer {
	let sync = st_sync::client::Client::new();
	Sequencer { midi_tx, sync, ps_rx, jack_client_addr, sequence_name, beat_tx }
    }
    pub fn start(self) {
	let config = Config::new();
	let client_pointer = unsafe { jack_ptr::recover_client(self.jack_client_addr) };

	let mut suppress_err: bool = false;

	let mut next_beat_frame = 0;

	let snapshot = unsafe { jack_transport::query_transport(client_pointer) };
	let beats_per_bar = snapshot.beats_per_bar;
	let mut pos_frame = snapshot.frame;

	// use first beat frame for sequence calculations
	if let Ok(first_beat_frame) = self.sync.recv_next_beat_frame() {
	    next_beat_frame = first_beat_frame;
	}

	let mut seq = Sequence::new(beats_per_bar, next_beat_frame, config.sequence_bars(&self.sequence_name));
	let mut i = 1;
	config.apply_sequence(&mut seq, self.sequence_name);
	
	let mut governor_on = true;
	let mut last_frame = 0;
	let mut first = true;
	let mut beat_counter = 0;
	let mut check_for_beat_frame = false;
	loop {
	    let snap = unsafe { jack_transport::query_transport(client_pointer) };
	    pos_frame = snap.frame;
	    if check_for_beat_frame {
	        if let Ok(val) = self.sync.try_recv_next_beat_frame() {
		    next_beat_frame = val;
		    check_for_beat_frame = false;
		}
	    }
	    let beat_this_cycle = next_beat_frame > last_frame as u64 && next_beat_frame <= pos_frame as u64;

	    if beat_this_cycle {
		beat_counter = beat_counter + 1;
		println!("{}", beat_counter);
		check_for_beat_frame = true;
		if let Some(tx) = &self.beat_tx {
		    // Non-blocking; if the GUI is gone the audio thread
		    // keeps running.
		    let _ = tx.try_send(beat_counter as u64);
		}
	    }
	    if first {
		if beat_this_cycle &&
		    beat_counter % beats_per_bar as usize == 0 &&
		    beat_counter >= beats_per_bar as usize {
			first = false;
			seq.set_frame(last_frame);
		    } else {
			last_frame = pos_frame;
			continue
		    }
	    }
	    last_frame = pos_frame;
	    if let Ok(()) = self.ps_rx.try_recv(){
		governor_on = false;
	    } else {
		thread::sleep(time::Duration::from_millis(1));
	    }
	    if governor_on && !beat_this_cycle {
		continue
	    }

	    let midi_vec = &seq.process_position(pos_frame as u64, next_beat_frame, beat_this_cycle);

	    for signal in midi_vec {
		println!("{:?}", signal);
		let om = OwnedMidi { time: signal.time, bytes: signal.bytes.to_owned() };
		self.midi_tx.send(om);
	    }

	    governor_on = true;
	}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: count how many slots in `seq.seq` contain at least one signal.
    fn populated_slot_count(seq: &Sequence) -> usize {
        seq.seq.iter().filter(|v| !v.is_empty()).count()
    }

    /// Helper: collect indices of slots that contain at least one signal.
    fn populated_indices(seq: &Sequence) -> Vec<usize> {
        seq.seq
            .iter()
            .enumerate()
            .filter_map(|(i, v)| if !v.is_empty() { Some(i) } else { None })
            .collect()
    }

    #[test]
    fn new_sequence_has_expected_length() {
        // beats_per_bar = 4, frames_per_beat = 100, bars = 1
        // length = 4 * 100 * 1 / 2 = 200
        let s = Sequence::new(4.0, 100, 1);
        assert_eq!(s.seq.len(), 200);
        assert_eq!(populated_slot_count(&s), 0);
    }

    #[test]
    fn add_notes_on_every_quarter_in_one_bar_44() {
        // 4/4 bar, 100 frames per beat, 1 bar -> length 200
        // QuarterNote = 1.0; frames per "slot" = 1.0 * 100 / 2 = 50
        // every_n=1, skip_n=0 -> 200 / 50 = 4 hits
        let mut s = Sequence::new(4.0, 100, 1);
        let signal: OwnedMidiBytes = vec![0x90, 60, 127];
        s.add_notes(signal, 1, 0, QuarterNote);

        let hits = populated_indices(&s);
        assert_eq!(hits.len(), 4, "should hit on each quarter beat in the bar");
        assert_eq!(hits, vec![0, 50, 100, 150]);
    }

    #[test]
    fn add_notes_skip_first_n_quarters() {
        // skip_n=2 -> first 2 quarter positions are skipped, then plays every quarter
        let mut s = Sequence::new(4.0, 100, 1);
        let signal: OwnedMidiBytes = vec![0x90, 60, 127];
        s.add_notes(signal, 1, 2, QuarterNote);

        let hits = populated_indices(&s);
        assert_eq!(hits, vec![100, 150]);
    }

    #[test]
    fn add_notes_every_2_quarters() {
        // every_n=2, skip_n=0 -> hit on quarters 0, 2 (i.e. half-notes)
        let mut s = Sequence::new(4.0, 100, 1);
        let signal: OwnedMidiBytes = vec![0x90, 60, 127];
        s.add_notes(signal, 2, 0, QuarterNote);

        let hits = populated_indices(&s);
        assert_eq!(hits, vec![0, 100]);
    }

    #[test]
    fn add_notes_eighth_notes_in_one_bar_44() {
        // EighthNote = 0.5; frames per slot = 0.5 * 100 / 2 = 25
        // length 200 / 25 = 8 hits
        let mut s = Sequence::new(4.0, 100, 1);
        let signal: OwnedMidiBytes = vec![0x90, 60, 127];
        s.add_notes(signal, 1, 0, EighthNote);

        let hits = populated_indices(&s);
        assert_eq!(hits.len(), 8);
        assert_eq!(hits, vec![0, 25, 50, 75, 100, 125, 150, 175]);
    }

    #[test]
    fn get_beat_value_no_dots_no_tuplet() {
        // base * 2 / 1 * 1 = base * 2 -- matches existing impl
        assert_eq!(get_beat_value(QuarterNote, 0, 1), 2.0);
    }

    #[test]
    fn get_beat_value_triplet() {
        // quarter triplet: 1.0 * 2 / 3 = 0.666...
        let v = get_beat_value(QuarterNote, 0, 3);
        assert!((v - (2.0 / 3.0)).abs() < 1e-6);
    }

    #[test]
    fn get_beat_value_dotted_quarter() {
        // 1 dot = *1.5; quarter dotted: 1.0 * 2 / 1 * 1.5 = 3.0
        assert_eq!(get_beat_value(QuarterNote, 1, 1), 3.0);
    }

    // ----------------------------------------------------------------------
    // `offset` primitive — generic behavioral tests.
    //
    // These assert the *contract* of `offset`, not specific frame indices:
    //
    //   1. Zero offset is a no-op vs `add_notes`.
    //   2. Positive offset shifts every hit later by the same amount; the
    //      number of hits is preserved (modulo wraparound).
    //   3. Negative offset shifts earlier; equivalent to a positive offset
    //      of `length - |offset|` after wrapping.
    //   4. An offset equal to one full sequence span produces the same
    //      hit set as zero offset.
    //   5. Offset composes additively across the bar boundary.
    // ----------------------------------------------------------------------

    /// Convenience: build a fresh 4/4, 1-bar sequence and lay down quarters
    /// with the given offset.
    fn quarters_with_offset(offset_beats: f32) -> Sequence {
        let mut s = Sequence::new(4.0, 100, 1);
        let signal: OwnedMidiBytes = vec![0x90, 60, 127];
        s.add_notes_with_offset(signal, 1, 0, QuarterNote, offset_beats);
        s
    }

    #[test]
    fn offset_zero_matches_add_notes() {
        let baseline = quarters_with_offset(0.0);
        let mut control = Sequence::new(4.0, 100, 1);
        control.add_notes(vec![0x90, 60, 127], 1, 0, QuarterNote);
        assert_eq!(populated_indices(&baseline), populated_indices(&control));
    }

    #[test]
    fn offset_positive_shifts_every_hit_by_constant() {
        // Quarters in 4/4@100 land at [0, 50, 100, 150]. A half-beat
        // offset should push each one forward by 25 half-frames.
        let baseline = populated_indices(&quarters_with_offset(0.0));
        let shifted  = populated_indices(&quarters_with_offset(0.5));

        assert_eq!(baseline.len(), shifted.len(),
            "offset should preserve hit count when no wraparound occurs");
        let length = 200i64;
        for (b, s) in baseline.iter().zip(shifted.iter()) {
            let delta = (*s as i64 - *b as i64).rem_euclid(length);
            assert_eq!(delta, 25,
                "every hit should shift by the same amount (got {} → {})", b, s);
        }
    }

    #[test]
    fn offset_negative_wraps_to_equivalent_positive() {
        // `-0.5` quarters should equal `+3.5` quarters in 4/4 — both
        // resolve to the same hit set after modulo. (Use whole half-beats
        // so the rounding tie-break of `offset * frames_per_beat / 2`
        // doesn't bite — that's a separate property to test.)
        let neg = populated_indices(&quarters_with_offset(-0.5));
        let pos = populated_indices(&quarters_with_offset(3.5));
        let mut a = neg.clone(); a.sort();
        let mut b = pos.clone(); b.sort();
        assert_eq!(a, b);
    }

    #[test]
    fn offset_full_span_is_identity() {
        // Offsetting by exactly the sequence span (4 beats in 4/4) should
        // wrap back to the same hit set.
        let baseline = populated_indices(&quarters_with_offset(0.0));
        let wrapped  = populated_indices(&quarters_with_offset(4.0));
        let mut a = baseline.clone(); a.sort();
        let mut b = wrapped.clone();  b.sort();
        assert_eq!(a, b);
    }

    #[test]
    fn offset_preserves_hit_count_under_wrap() {
        // A hit that gets pushed past the end of the buffer should reappear
        // at the start, not vanish. 3 beats offset on a quarter pattern in
        // 4/4 means the original hit at beat 0 moves to beat 3 (still in
        // buffer), beat 1 → beat 4 = wraps to 0, etc. Total hits stays 4.
        let baseline = populated_indices(&quarters_with_offset(0.0));
        let shifted  = populated_indices(&quarters_with_offset(3.0));
        assert_eq!(baseline.len(), shifted.len());
    }

    #[test]
    fn offset_composes_with_tuplet() {
        // A quarter triplet (3 in the space of 2 quarters) plus an offset
        // should remain a triplet — the spacing between successive hits
        // should be unchanged. Sort the indices, take diffs, and verify
        // they match the no-offset baseline's diffs (modulo wraparound).
        let mut base_seq = Sequence::new(4.0, 100, 1);
        base_seq.add_notes_with_offset(
            vec![0x90, 60, 127], 1, 0, tuplet(QuarterNote, 3), 0.0);
        let mut shifted_seq = Sequence::new(4.0, 100, 1);
        shifted_seq.add_notes_with_offset(
            vec![0x90, 60, 127], 1, 0, tuplet(QuarterNote, 3), 1.0);

        let base = populated_indices(&base_seq);
        let shifted = populated_indices(&shifted_seq);

        assert_eq!(base.len(), shifted.len(),
            "tuplet hit count should be preserved under offset");

        // Pairwise diffs around the cycle should match as a *multiset* —
        // an offset can rotate which hit ends up first when sorted, which
        // permutes the diff sequence but not its multiset.
        let length = 200i64;
        let mut diffs = |v: &Vec<usize>| -> Vec<i64> {
            let mut sorted = v.clone();
            sorted.sort();
            // Wrap diff included so the cycle is fully described.
            let mut d: Vec<i64> = sorted.windows(2)
                .map(|w| (w[1] as i64 - w[0] as i64).rem_euclid(length))
                .collect();
            if let (Some(&first), Some(&last)) = (sorted.first(), sorted.last()) {
                d.push((first as i64 + length - last as i64).rem_euclid(length));
            }
            d.sort();
            d
        };
        assert_eq!(diffs(&base), diffs(&shifted),
            "offset must not alter the spacing between successive tuplet hits");
    }

    #[test]
    fn offset_two_entries_at_complementary_offsets_interleave() {
        // Two quarter patterns, one at offset 0 and one at offset 0.5,
        // should jointly fire 8 distinct hits in 4/4 — i.e. an effective
        // eighth-note rate without changing `beat_value`.
        let mut s = Sequence::new(4.0, 100, 1);
        s.add_notes_with_offset(vec![0x90, 60, 127], 1, 0, QuarterNote, 0.0);
        s.add_notes_with_offset(vec![0x90, 62, 127], 1, 0, QuarterNote, 0.5);

        assert_eq!(populated_slot_count(&s), 8,
            "two interleaved quarter parts should yield 8 populated slots");
    }

    // ----------------------------------------------------------------------
    // Multi-bar sequences. The `bars` parameter to `Sequence::new` scales
    // the buffer length, which means a single `add_notes` call lays out
    // hits across the full multi-bar span.
    // ----------------------------------------------------------------------

    #[test]
    fn multi_bar_buffer_length_scales_linearly() {
        let one = Sequence::new(4.0, 100, 1);
        let two = Sequence::new(4.0, 100, 2);
        let four = Sequence::new(4.0, 100, 4);
        assert_eq!(two.seq.len(), 2 * one.seq.len());
        assert_eq!(four.seq.len(), 4 * one.seq.len());
    }

    #[test]
    fn quarters_in_two_bar_44_yield_eight_hits() {
        let mut s = Sequence::new(4.0, 100, 2);
        s.add_notes(vec![0x90, 60, 127], 1, 0, QuarterNote);
        let hits = populated_indices(&s);
        assert_eq!(hits.len(), 8,
            "2 bars of 4/4 quarters should produce 8 hits, not 4");
    }

    #[test]
    fn skip_three_quarters_works_across_bar_boundary() {
        // In a 2-bar 4/4 buffer, skip_n=3 every_n=1 should fire on
        // quarters 4, 5, 6, 7 (counting from 0 across both bars) — i.e.
        // the entire second bar. Verifies that `skip` operates over the
        // sequence span, not the bar.
        let mut s = Sequence::new(4.0, 100, 2);
        s.add_notes(vec![0x90, 60, 127], 1, 3, QuarterNote);
        let hits = populated_indices(&s);
        assert_eq!(hits.len(), 5,
            "skip 3 of 8 quarters should leave 5 hits");
    }

    #[test]
    fn offset_wraps_within_multi_bar_span() {
        // Offset of 5 beats in a 2-bar (8-beat) 4/4 sequence wraps to
        // an effective offset of 5 inside the span. The hit set must
        // equal that of offset 5.0 directly (trivially) and also of
        // offset -3.0 (5 - 8 = -3, wrap-equivalent).
        let mut a = Sequence::new(4.0, 100, 2);
        a.add_notes_with_offset(vec![0x90, 60, 127], 1, 0, QuarterNote, 5.0);
        let mut b = Sequence::new(4.0, 100, 2);
        b.add_notes_with_offset(vec![0x90, 60, 127], 1, 0, QuarterNote, -3.0);
        let mut hits_a = populated_indices(&a); hits_a.sort();
        let mut hits_b = populated_indices(&b); hits_b.sort();
        assert_eq!(hits_a, hits_b);
    }
}
