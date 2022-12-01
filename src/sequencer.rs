use crate::beat_values::*;
use jack::jack_sys as j;
use crossbeam_channel::*;
use std::mem::MaybeUninit;
use std::{thread, time};
//use crate::note_map;
use crate::config::Config;

type OwnedMidiBytes = Vec<u8>;

#[derive(Debug)]
pub struct OwnedMidi {
    pub time: u32,
    pub bytes: OwnedMidiBytes
}

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
	let beat_counter = 1;
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
	let frames = beat_value * self.frames_per_beat as f32 / 2.0;
	let mut n = skip_n;
	let mut beat: i32 = 0;
	for i in 0..self.length {
	    let v = &mut self.seq.get_mut(i as usize).unwrap();
	    if i as f32 % frames < 1.0 {
		//frame matches beat value
		if n == 0 && (beat - skip_n as i32) % every_n as i32 == 0 {
		    //n beats have been skipped. go time.
		    v.push(signal.to_owned());
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

	let final_beat = self.beat_counter == self.n_beats;
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
		} else {
		    self.beat_counter = self.beat_counter + 1;
		}
		if final_beat {
		    self.playhead = 0;
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
    sequence_name: String
}
impl Sequencer {
    pub fn new(midi_tx: Sender<OwnedMidi>,
	       ps_rx: Receiver<()>,
	       jack_client_addr: usize,
	       sequence_name: String
    ) -> Sequencer {
	let sync = st_sync::client::Client::new();
	Sequencer { midi_tx, sync, ps_rx, jack_client_addr, sequence_name }
    }
    pub fn start(self) {
	let config = Config::new();
	let client_pointer: *const j::jack_client_t = std::ptr::from_exposed_addr(self.jack_client_addr);

	let mut suppress_err: bool = false;

	let mut next_beat_frame = 0;

	let mut pos = MaybeUninit::uninit().as_mut_ptr();

	let mut pos_frame = 0;
	let mut beats_per_bar = 0.0;
	unsafe {
    	    j::jack_transport_query(client_pointer, pos);
	    beats_per_bar = (*pos).beats_per_bar;
	    pos_frame = (*pos).frame as u64;
	}

	// use first beat frame for sequence calculations
	loop {
	    if let Ok(first_beat_frame) = self.sync.recv_next_beat_frame() {
		next_beat_frame = first_beat_frame;
		break
	    }
	}
	let mut seq = Sequence::new(beats_per_bar, next_beat_frame, 1);
	let mut i = 1;
	config.apply_sequence(&mut seq, self.sequence_name);
	
	let mut governor_on = true;
	let mut last_frame = 0;
	let mut first = true;
	let mut beat_counter = 1;
	loop {
	    unsafe {
		let state = j::jack_transport_query(client_pointer, pos);
		pos_frame = (*pos).frame as u64;
	    }
	    if let Ok(val) = self.sync.recv_next_beat_frame() {
		next_beat_frame = val;
	    }
	    let beat_this_cycle = next_beat_frame > last_frame as u64 && next_beat_frame <= pos_frame as u64;
	    last_frame = pos_frame;

	    if beat_this_cycle {
		beat_counter = beat_counter + 1;
		println!("{}", beat_counter);
	    }
	    if first {
		if beat_this_cycle && beat_counter % beats_per_bar as usize == 1 {
		    first = false;
		    seq.set_frame(pos_frame);
		} else {
		    continue
		}
	    }
	    if let Ok(()) = self.ps_rx.try_recv(){
		governor_on = false;
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
