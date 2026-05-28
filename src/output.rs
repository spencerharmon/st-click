use crossbeam_channel::*;
use crate::sequencer::{Sequencer, SequencerCommand};
use st_lib::owned_midi::OwnedMidi;
use st_lib::jack_ptr;
use jack::RawMidi;

pub struct Output;

impl Output {
    pub fn new() -> Output {
        Output { }
    }

    /// Bring up JACK, register the MIDI-out port, and spawn the sequencer
    /// loop on a background thread. Returns immediately so the caller
    /// (typically `main`) can hand the main thread to eframe.
    ///
    /// `beat_tx`, when supplied, receives the running beat counter each
    /// time the sequencer detects a beat boundary. Pass `None` for
    /// headless runs.
    ///
    /// `command_rx` / `active_tx` carry runtime control messages between
    /// the GUI (or NSM) and the sequencer:
    ///   - `command_rx`: `SwitchTo(name)` requests; sequencer applies at
    ///     the next bar boundary.
    ///   - `active_tx`: sequencer pushes the currently-playing sequence
    ///     name after each switch lands.
    ///
    /// The JACK active-client guard is intentionally leaked so JACK
    /// keeps running for the lifetime of the process.
    pub fn jack_output(
	&self,
	sequence_name: String,
	beat_tx: Option<Sender<u64>>,
	command_rx: Option<Receiver<SequencerCommand>>,
	active_tx: Option<Sender<String>>,
    ) {
	// carries midi signals
        let (midi_tx, midi_rx) = bounded::<OwnedMidi>(1000);
	// signals once per process cycle
        let (ps_tx, ps_rx) = bounded(1);

        let (client, _status) =
            jack::Client::new("st-click", jack::ClientOptions::NO_START_SERVER).unwrap();
        let mut midi_port = client
            .register_port("midi", jack::MidiOut::default())
            .unwrap();
	let client_pointer = client.raw();

	let process = jack::ClosureProcessHandler::new(
            move |_client: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
                let _ = ps_tx.try_send(());

		// Get output buffer
		let mut out = midi_port.writer(ps);

		if let Ok(msg) = midi_rx.try_recv() {
		    let rm = RawMidi { time: msg.time, bytes: &msg.bytes };
		    let _ = out.write(&rm);
		}

                jack::Control::Continue
            },
        );
        let active_client = client.activate_async((), process).unwrap();
	// Keep JACK alive for the rest of the process.
	std::mem::forget(active_client);

	let client_addr = jack_ptr::expose_client(client_pointer);

	// The sequencer constructs `st_sync::client::Client`, which
	// `tokio::task::spawn`s its reader task — that requires a tokio
	// runtime context. Grab the current runtime's handle here (we're
	// called from `main` after the runtime is built and entered) and
	// move it into the sequencer thread so the spawn succeeds.
	let rt_handle = tokio::runtime::Handle::current();

	std::thread::Builder::new()
	    .name("st-click-sequencer".into())
	    .spawn(move || {
		let _guard = rt_handle.enter();
		let sequencer = Sequencer::new(
		    midi_tx,
		    ps_rx,
		    client_addr,
		    sequence_name,
		    beat_tx,
		    command_rx,
		    active_tx,
		);
		sequencer.start();
	    })
	    .expect("failed to spawn sequencer thread");
    }
}
