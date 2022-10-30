use crate::beat_values::*;
use jack::jack_sys as j;
use crossbeam_channel::*;
use std::mem::MaybeUninit;
use std::{thread, time};
use crate::note_map;

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
	let mut beat = 0;
	for i in 0..self.length {
	    let v = &mut self.seq.get_mut(i as usize).unwrap();
	    if i as f32 % frames < 1.0 {
		beat = beat + 1;
		println!("frame matches beat value");
		//frame matches beat value

		if n == 0 && (beat - skip_n - 1) % every_n == 0 {
		    println!("n beats have been skipped. go time.");
		    //n beats have been skipped. go time.
		    v.push(signal.to_owned());
		    n = 0;
		} else if n >= 1{
		    n = n - 1;
		}
	    }
	}
    }

    fn process_position(&mut self,
			    pos_frame: u64,
			    next_beat_frame: u64
    ) -> Vec<OwnedMidi> {
	let mut ret = Vec::new();
	let mut beat_this_cycle = false;
	if ((self.last_frame < next_beat_frame) &&
	    (next_beat_frame <= pos_frame)) ||
	    self.last_frame == 0 {
			beat_this_cycle = true;
	}
	let final_beat = self.beat_counter == self.n_beats;
	let mut beat_frame = 1;
	let nframes = pos_frame - self.last_frame;

	if beat_this_cycle {
	    println!("{:?}, {:?}, {:?}", self.last_frame, next_beat_frame, pos_frame);
	    if self.last_frame == 0 {
		beat_frame = 1;
	    } else {
		beat_frame = next_beat_frame - self.last_frame;
	    }
	    println!("{:?}", beat_frame);
	    println!("{:?}", nframes);
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
//	    println!("{:?} {:?} {:?} {:?}", beat_this_cycle, i, beat_frame, nframes);
	    if beat_this_cycle && i == beat_frame {
		println!("final beat {:?}", final_beat);
		println!("beat_counter {:?}", self.beat_counter);
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
    jack_client_addr: usize
}
impl Sequencer {
    pub fn new(midi_tx: Sender<OwnedMidi>,
	       ps_rx: Receiver<()>,
	       jack_client_addr: usize
    ) -> Sequencer {
	let sync = st_sync::client::Client::new();
	Sequencer { midi_tx, sync, ps_rx, jack_client_addr }
    }
    pub fn start(self) {
	let client_pointer: *const j::jack_client_t = std::ptr::from_exposed_addr(self.jack_client_addr);

	let mut suppress_err: bool = false;

	let mut next_beat_frame = 0;

	loop {
	    match self.sync.recv_next_beat_frame() {
		Ok(val) => {
		    next_beat_frame = val;
		    break;
		}
		Err(message) => {
		    if !suppress_err {
			println!("{}", message);
		    }
		    suppress_err = true;
		}
	    }
	}
	let mut first = true;

	unsafe {
	    let mut pos = MaybeUninit::uninit().as_mut_ptr();
    	    j::jack_transport_query(client_pointer, pos);

	    println!("next beat frame at this point: {:?}", next_beat_frame);
	    
	    let mut seq = Sequence::new((*pos).beats_per_bar, next_beat_frame, 1);



	    let n_0 = note_map::cminus1_on();
	    seq.add_notes(n_0, 1, 0, Crotchet);
	    // let n_1 = note_map::dflat1_on();
	    // seq.add_notes(n_1, 4, 1, Crotchet);
	    // let n_2 = note_map::dminus1_on();
	    // seq.add_notes(n_2, 4, 2, Crotchet);
	    // let n_3 = note_map::eflat1_on();
	    // seq.add_notes(n_3, 4, 3, Crotchet);
	    // let n_4 = note_map::dflat1_on();
	    // seq.add_notes(n_4, 1, 0, tuplet(HalfNote, 3));
	    let n_5 = note_map::dflat1_on();
	    seq.add_notes(n_5, 1, 0, tuplet(HalfNote, 7));
	    
	    loop {
	    	let state = j::jack_transport_query(client_pointer, pos);
		match self.ps_rx.try_recv(){
		    Ok(()) => (),
		    Err(_) => continue
		}

		match self.sync.recv_next_beat_frame() {
		    Ok(val) => next_beat_frame = val,
		    Err(_) => ()
		}
		
		let midi_vec = &seq.process_position((*pos).frame as u64, next_beat_frame);

		for signal in midi_vec {
		    println!("{:?}", signal);
		    let om = OwnedMidi { time: signal.time, bytes: signal.bytes.to_owned() };
		    self.midi_tx.send(om);
		}
	    }
	}
    }
}
