//! GUI app state for st-click.

use crossbeam_channel::Sender;
use crate::sequencer::SequencerCommand;

pub struct AppState {
	/// Name of the sequence currently being played by the audio thread.
	/// Updated when the sequencer confirms a switch has landed (so the
	/// UI displays the truth, not the requested-but-not-yet-applied
	/// selection).
	pub active_sequence: String,
	/// Names of all sequences in the loaded YAML, for the combo box.
	pub available_sequences: Vec<String>,
	/// Selection currently shown in the combo box. Differs from
	/// `active_sequence` only briefly between a user click and the
	/// sequencer's bar-boundary swap.
	pub selected_sequence: String,
	/// Most recent beat counter value received from the audio thread.
	pub beat_count: u64,
	/// Wall-clock instant of the most recent beat boundary, for the
	/// indicator-lamp fade.
	pub last_beat_at: std::time::Instant,
	/// Channel into the sequencer for runtime commands. `None` only
	/// in tests / headless setups.
	pub command_tx: Option<Sender<SequencerCommand>>,
}

impl AppState {
	pub fn new(active_sequence: String, available_sequences: Vec<String>) -> Self {
		let selected_sequence = active_sequence.clone();
		Self {
			active_sequence,
			available_sequences,
			selected_sequence,
			beat_count: 0,
			last_beat_at: std::time::Instant::now() - std::time::Duration::from_secs(10),
			command_tx: None,
		}
	}
}
