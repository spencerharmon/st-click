
mod output;
mod sequencer;
mod beat_values;
mod note_utils;
mod config;
mod gui;

use clap::Parser;
use crossbeam_channel::unbounded;

#[derive(Parser)]
struct Cli {
	sequence_name: String,
	/// Run headless (no GUI window).
	#[clap(long)]
	no_gui: bool,
}

fn main() {
	let cli = Cli::parse();

	// Build the tokio runtime and keep it alive for the rest of the
	// process. `Output::jack_output` grabs the current runtime handle
	// to forward into the sequencer thread (st_sync::Client spawns
	// tokio tasks).
	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.expect("failed to build tokio runtime");
	let _rt_guard = rt.enter();

	let o = output::Output::new();

	if cli.no_gui {
		o.jack_output(cli.sequence_name, None);
		loop {
			std::thread::park();
		}
	}

	// Load the YAML once on the GUI side just to populate the sequence
	// combo box. The audio thread loads it again inside `Sequencer::start`.
	let cfg = config::Config::new();
	let names = cfg.sequence_names();
	let state = gui::app_state::AppState::new(cli.sequence_name.clone(), names);

	let (beat_tx, beat_rx) = unbounded::<u64>();
	o.jack_output(cli.sequence_name, Some(beat_tx));

	if let Err(e) = gui::run(state, beat_rx) {
		eprintln!("eframe exited with error: {e}");
		std::process::exit(1);
	}
}
