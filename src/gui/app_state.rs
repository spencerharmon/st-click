//! GUI app state for st-click.

pub struct AppState {
	/// Name of the sequence currently being played by the audio thread.
	/// In v1 this is read-only after launch; runtime-switching the
	/// sequence requires a deeper refactor of `Sequencer::start`.
	/// TODO: wire a control channel so the combo box can swap sequences.
	pub active_sequence: String,
	/// Names of all sequences in the loaded YAML, for the combo box.
	pub available_sequences: Vec<String>,
	/// Selection currently shown in the combo box. May differ from
	/// `active_sequence` until runtime swap is implemented.
	pub selected_sequence: String,
	/// Most recent beat counter value received from the audio thread.
	pub beat_count: u64,
	/// Wall-clock instant of the most recent beat boundary, for the
	/// indicator-lamp fade.
	pub last_beat_at: std::time::Instant,
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
		}
	}
}
