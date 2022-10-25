use jack::jack_sys as j;
use tokio::task;
use crossbeam_channel::*;
use std::mem::MaybeUninit;
use crate::sequencer::Sequencer;
use std::{thread, time};
pub struct Output;

impl Output {
    pub fn new() -> Output {
        Output { }
    }
    pub async fn jack_output(self)  {
	//carries midi signals
        let (midi_tx, midi_rx) = bounded(1000);
	//signals once per process cycle
        let (ps_tx, ps_rx) = bounded(1);
	
	
        let (client, _status) =
            jack::Client::new("st-click", jack::ClientOptions::NO_START_SERVER).unwrap();
        let mut midi_port = client
            .register_port("midi", jack::MidiOut::default())
            .unwrap();
	let client_pointer = client.raw();

	let process = jack::ClosureProcessHandler::new(
            move |client: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
                match ps_tx.try_send(()) {
		    Ok(()) => (),
		    Err(_) => ()
		}

		// Get output buffer
		let mut out = midi_port.writer(ps);

		match midi_rx.try_recv() {
		    Ok(msg) => {
			out.write(&msg);
			()
		    }
		    Err(e) => ()
		}

                jack::Control::Continue
            },
        );
        let active_client = client.activate_async((), process).unwrap();

	let sequencer = Sequencer::new(midi_tx, ps_rx, client_pointer.expose_addr());

	tokio::task::spawn(sequencer.start());
	loop {
	    continue
	}
    }
}