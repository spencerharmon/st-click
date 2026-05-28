
mod output;
mod sequencer;
mod beat_values;
mod note_utils;
mod config;
mod gui;
mod session;

use std::env;
use std::sync::{Arc, Mutex};

use clap::Parser;
use crossbeam_channel::unbounded;
use st_lib::nsm;

#[derive(Parser)]
struct Cli {
	/// Default sequence name (overridden by NSM session if loaded).
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
	// tokio tasks); the NSM follow-up task also lives on this runtime.
	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.expect("failed to build tokio runtime");
	let _rt_guard = rt.enter();

	let live = Arc::new(Mutex::new(session::Session {
		sequence_name: cli.sequence_name.clone(),
	}));

	// Sequencer control channels (created up front so NSM and GUI
	// both have a handle to `cmd_tx`).
	let (cmd_tx, cmd_rx) = unbounded::<sequencer::SequencerCommand>();
	let (active_tx, active_rx) = unbounded::<String>();
	let (beat_tx, beat_rx) = unbounded::<u64>();

	// If NSM_URL is set, block on the first /nsm/client/open before
	// starting JACK so a saved sequence name can replace the CLI arg.
	if env::var("NSM_URL").is_ok() {
		rt.block_on(async {
			let caps = nsm::Capabilities {
				switch: true,
				optional_gui: true,
				..Default::default()
			};
			let (mut client, _handle) = nsm::Builder::new("st-click")
				.capabilities(caps)
				.launch();

			println!("[st-click] NSM detected, waiting for /nsm/client/open ...");
			let session_path = Arc::new(Mutex::new(String::new()));

			while let Some(evt) = client.rx.recv().await {
				match evt {
					nsm::Event::Open { path, ack, .. } => {
						match session::load(&path) {
							Ok(Some(s)) => {
								println!("[st-click] loaded session: {s:?}");
								*live.lock().unwrap() = s;
							}
							Ok(None) => {
								println!("[st-click] no saved session at {path}, using CLI default");
								let snap = live.lock().unwrap().clone();
								if let Err(e) = session::save(&path, &snap) {
									eprintln!("[st-click] could not seed session: {e}");
								}
							}
							Err(e) => {
								ack.err(-1, format!("load failed: {e}"));
								eprintln!("[st-click] session load error: {e}");
								std::process::exit(1);
							}
						}
						*session_path.lock().unwrap() = path;
						ack.ok("opened");
						break;
					}
					nsm::Event::AnnounceError { code, message } => {
						eprintln!("[st-click] NSM rejected announce ({code}): {message}");
						std::process::exit(1);
					}
					nsm::Event::AnnounceOk { manager_name, .. } => {
						println!("[st-click] NSM connected to {manager_name}");
					}
					_ => {}
				}
			}

			spawn_nsm_followup(client, session_path, live.clone(), cmd_tx.clone());
		});
	}

	let sequence_name = live.lock().unwrap().sequence_name.clone();
	let o = output::Output::new();

	if cli.no_gui {
		o.jack_output(sequence_name, None, Some(cmd_rx), None);
		loop {
			std::thread::park();
		}
	}

	// Load the YAML once on the GUI side just to populate the sequence
	// combo box. The audio thread loads it again inside `Sequencer::start`.
	let cfg = config::Config::new();
	let names = cfg.sequence_names();
	let mut state = gui::app_state::AppState::new(sequence_name.clone(), names);
	state.command_tx = Some(cmd_tx.clone());

	// Bridge: whenever the sequencer confirms a sequence change, also
	// update `live` so the next NSM Save persists the right name.
	{
		let live = live.clone();
		let bridge_rx = active_rx.clone();
		std::thread::Builder::new()
			.name("st-click-active-bridge".into())
			.spawn(move || {
				while let Ok(name) = bridge_rx.recv() {
					live.lock().unwrap().sequence_name = name;
				}
			})
			.expect("failed to spawn active-bridge thread");
	}

	o.jack_output(sequence_name, Some(beat_tx), Some(cmd_rx), Some(active_tx));

	if let Err(e) = gui::run(state, beat_rx, Some(active_rx)) {
		eprintln!("eframe exited with error: {e}");
		std::process::exit(1);
	}
}

fn spawn_nsm_followup(
	mut client: nsm::Client,
	session_path: Arc<Mutex<String>>,
	live: Arc<Mutex<session::Session>>,
	cmd_tx: crossbeam_channel::Sender<sequencer::SequencerCommand>,
) {
	tokio::spawn(async move {
		while let Some(evt) = client.rx.recv().await {
			match evt {
				nsm::Event::Save { ack } => {
					let path = session_path.lock().unwrap().clone();
					if path.is_empty() {
						ack.err(-1, "no session path");
						continue;
					}
					let snap = live.lock().unwrap().clone();
					match session::save(&path, &snap) {
						Ok(()) => ack.ok("saved"),
						Err(e) => ack.err(-1, format!("save failed: {e}")),
					}
				}
				nsm::Event::Open { path, ack, .. } => {
					// `:switch:` — load the new session's sequence name
					// and request a live swap.
					match session::load(&path) {
						Ok(Some(s)) => {
							let name = s.sequence_name.clone();
							*live.lock().unwrap() = s;
							let _ = cmd_tx.try_send(
								sequencer::SequencerCommand::SwitchTo(name)
							);
						}
						Ok(None)    => {}
						Err(e) => {
							ack.err(-1, format!("load failed: {e}"));
							continue;
						}
					}
					*session_path.lock().unwrap() = path;
					ack.ok("switched");
				}
				nsm::Event::ShowGui | nsm::Event::HideGui => {
					// GUI show/hide not yet routed through the eframe
					// window. See st-conductor for the matching TODO.
				}
				_ => {}
			}
		}
	});
}
