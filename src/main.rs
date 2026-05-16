
mod output;
mod sequencer;
mod beat_values;
mod note_utils;
mod config;
mod session;

use std::env;
use std::sync::{Arc, Mutex};

use clap::Parser;
use st_lib::nsm;

#[derive(Parser)]
struct Cli {
	/// Default sequence name (overridden by NSM session if loaded).
	sequence_name: String,
}

#[tokio::main]
async fn main() {
	let cli = Cli::parse();
	let live = Arc::new(Mutex::new(session::Session {
		sequence_name: cli.sequence_name.clone(),
	}));

	// If NSM_URL is set, wait for /nsm/client/open before starting JACK
	// so a saved sequence name overrides the CLI arg.
	if env::var("NSM_URL").is_ok() {
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
							return;
						}
					}
					*session_path.lock().unwrap() = path;
					ack.ok("opened");
					break;
				}
				nsm::Event::AnnounceError { code, message } => {
					eprintln!("[st-click] NSM rejected announce ({code}): {message}");
					return;
				}
				nsm::Event::AnnounceOk { manager_name, .. } => {
					println!("[st-click] NSM connected to {manager_name}");
				}
				_ => {}
			}
		}

		spawn_nsm_followup(client, session_path, live.clone());
	}

	let sequence_name = live.lock().unwrap().sequence_name.clone();
	let o = output::Output::new();
	o.jack_output(sequence_name).await;
}

fn spawn_nsm_followup(
	mut client: nsm::Client,
	session_path: Arc<Mutex<String>>,
	live: Arc<Mutex<session::Session>>,
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
					// `:switch:` — load the new session's sequence name.
					// Cannot swap a playing sequence yet (no live control
					// channel into the sequencer); update LiveConfig so
					// the next Save reflects it.
					match session::load(&path) {
						Ok(Some(s)) => *live.lock().unwrap() = s,
						Ok(None)    => {}
						Err(e) => {
							ack.err(-1, format!("load failed: {e}"));
							continue;
						}
					}
					*session_path.lock().unwrap() = path;
					ack.ok("switched (live sequence change requires restart)");
				}
				nsm::Event::ShowGui | nsm::Event::HideGui => {
					// No GUI yet.
				}
				_ => {}
			}
		}
	});
}
