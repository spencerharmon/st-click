use jack::jack_sys as j;
use tokio::task;
use crossbeam_channel::*;
use std::mem::MaybeUninit;
use crate::beat_values::*;
use crate::sequencer::{Sequencer, OwnedMidi};
use std::{thread, time};
use jack::RawMidi;
pub struct Output;

impl Output {
    pub fn new() -> Output {
        Output { }
    }
    pub async fn jack_output(self, sequence_name: String)  {
	//carries midi signals
        let (midi_tx, midi_rx) = bounded::<OwnedMidi>(1000);
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
			let rm = RawMidi { time: msg.time, bytes: &msg.bytes };
			out.write(&rm);
			()
		    }
		    Err(e) => ()
		}

                jack::Control::Continue
            },
        );
        let active_client = client.activate_async((), process).unwrap();

	let mut sequencer = Sequencer::new(midi_tx, ps_rx, client_pointer.expose_addr(), sequence_name);

	sequencer.start();
    }
}
