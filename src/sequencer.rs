use crate::beat_values::*;
use jack::jack_sys as j;
use crossbeam_channel::*;
use std::mem::MaybeUninit;
use std::{thread, time};
use jack::RawMidi;

fn get_beat_value(base_beat_value: BeatValue, n_dots: u16, n_tuplet: u16) -> f32 {
    ((base_beat_value * 2.0) / n_tuplet as f32) * f32::powf(1.5, n_dots.into())
}

pub struct Sequence <'a> {
    beats_per_bar: f32,
    frames_per_beat: u64,
    length: u64,
    seq: Vec<Vec<RawMidi<'a>>>,
    pos: u64,
    test_time: u32
}
impl <'a> Sequence <'a> {
    pub fn new (beats_per_bar: f32, frames_per_beat: u64, bars: u32) -> Sequence <'a> {
	let length = beats_per_bar as u64 * frames_per_beat * bars as u64 / 2;
	let mut seq: Vec<Vec<RawMidi>> = Vec::new();
	for i in 0..length as usize {
	    let v = Vec::new();
	    seq.push(v);
	}
	
	let pos = 0;

	let test_time = 0;
	Sequence { beats_per_bar, frames_per_beat, length, seq, pos, test_time }
    }
    pub fn add_notes(&mut self, signal: RawMidi<'a>, every_n: u16, skip_n: u16, beat_value: BeatValue){
	let frames = beat_value * self.frames_per_beat as f32 / 2.0;
	let mut n = skip_n;
	let mut beat = 0;
	for i in 0..self.length {
	    let v = &mut self.seq.get_mut(i as usize).unwrap();
	    if i as f32 % frames < 0.001 {
		beat = beat + 1;
		println!("frame matches beat value");
		//frame matches beat value

		if n == 0 && (beat - skip_n - 1) % every_n == 0 {
		    println!("n beats have been skipped. go time.");
		    //n beats have been skipped. go time.
		    v.push(signal);
		    n = 0;
		} else if n >= 1{
		    n = n - 1;
		}
	    }
	}
    }
    fn process_position(&mut self,
			pos_frame: u64
    ) -> Vec<RawMidi<'a>> {
	let mut ret = Vec::new();

	let new_pos = pos_frame % self.length;

    	let zero: &[u8] = &[0; 1];
	let  mut rm = jack::RawMidi { time: 0, bytes: zero };		    
	if new_pos > self.pos{
//	    let t = new_pos - self.pos;
	    for i in self.pos..new_pos {
		let v = &mut self.seq.get_mut(i as usize);
		for iv in v {
		    for m in &mut **iv {
			rm.time = self.test_time as u32;
			rm.time = ((i - self.pos)/2) as u32;
//			println!("time {:?}", (i - self.pos));
			rm.bytes = m.bytes;
			ret.push(rm);
		    }
		}
	    }
	}
	else if self.pos > new_pos {
	    let t = self.length - self.pos + new_pos;
	    for i in self.pos..self.length {
		let v = &mut self.seq.get_mut(i as usize);
		for iv in v {
		    for m in &mut **iv {
//			rm.time = t as u32;
			rm.time = ((i - self.pos) / 2) as u32;

//			println!("calculated time {:?}", (i - self.pos));
			rm.bytes = m.bytes;
   			ret.push(rm);
		    }
		}
	    }
	    for i in 0..new_pos{
		let v = &mut self.seq.get_mut(i as usize);
		for iv in v {
		    for m in &mut **iv {
//			rm.time = t as u32;
//			println!("calculated time {:?}", (self.length - self.pos + i));
			rm.time = ((self.length - self.pos + i) / 2) as u32;
			rm.bytes = m.bytes;
   			ret.push(rm);
		    }
		}
	    }
	}
	
	self.pos = new_pos;
	ret
    }
}

pub struct Sequencer<'a>{
    sync: st_sync::client::Client,
    midi_tx: Sender<RawMidi<'a>>,
    ps_rx: Receiver<()>,
    jack_client_addr: usize
}
impl <'a> Sequencer<'_> {
    pub fn new(midi_tx: Sender<RawMidi>,
	       ps_rx: Receiver<()>,
	       jack_client_addr: usize
    ) -> Sequencer {
	let sync = st_sync::client::Client::new();
	Sequencer { sync, midi_tx, ps_rx, jack_client_addr }
    }
    pub async fn start(self) {
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
	unsafe {
	    let mut pos = MaybeUninit::uninit().as_mut_ptr();
    	    j::jack_transport_query(client_pointer, pos);

	    let mut seq = Sequence::new((*pos).beats_per_bar, next_beat_frame, 1);

	    let zero: &[u8] = &[0; 1];
	    let one: &[u8] = &[1; 1];
	    let two: &[u8] = &[2; 1];
	    let three: &[u8] = &[3; 1];
    	    let rm0 = jack::RawMidi { time: 0, bytes: zero };
    	    let rm1 = jack::RawMidi { time: 0, bytes: one };
    	    let rm2 = jack::RawMidi { time: 0, bytes: two };
    	    let rm3 = jack::RawMidi { time: 0, bytes: three };
	    seq.add_notes(rm0, 4, 0, Crotchet);
	    seq.add_notes(rm1, 4, 1, Crotchet);
	    seq.add_notes(rm2, 4, 2, Crotchet);
	    seq.add_notes(rm3, 4, 3, Crotchet);
	    loop {
	    	let state = j::jack_transport_query(client_pointer, pos);
		match self.ps_rx.try_recv(){
		    Ok(()) => (),
		    Err(_) => continue
		}

		let midi_vec = &seq.process_position((*pos).frame as u64);

		for signal in midi_vec {
		    println!("{:?}", signal);
		    self.midi_tx.send(*signal);
		}
	    }
	}
    }
}
