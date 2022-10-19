use std::{thread, time};
use tokio::task;
use tokio::time::{sleep, Duration};
use crossbeam_channel::*;

pub struct Output;

impl Output {
    pub fn new() -> Output {
        Output { }
    }
    pub async fn jack_output(self)  {
        let (tx, rx) = bounded(1000);
	
        let (client, _status) =
            jack::Client::new("st-click", jack::ClientOptions::NO_START_SERVER).unwrap();
        let mut midi_port = client
            .register_port("midi", jack::MidiOut::default())
            .unwrap();

	let process = jack::ClosureProcessHandler::new(
            move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
                
                // Get output buffer
                let mut out = midi_port.writer(ps);
		match rx.try_recv() {
		    Ok(msg) => {
			let (frame, rawmidi) = msg;
			out.write(&rawmidi);
			()
		    }
		    Err(_) => ()
		}
                // Continue as normal
                jack::Control::Continue
            },
        );
        let active_client = client.activate_async((), process).unwrap();
	loop {
	    let dur = time::Duration::from_millis(1000);
	    thread::sleep(dur);
	    let zero: &[u8]  = &[0; 0];
	    let rm = jack::RawMidi { time: 0, bytes: zero };
	    tx.send((1234, rm));
	}
    }
}

