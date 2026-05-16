//! Sequence selector panel. Changing the selection sends a
//! `SwitchTo` command to the sequencer; the sequencer applies it at
//! the next bar boundary and reports back, at which point
//! `active_sequence` updates.

use eframe::egui::{self, Ui};
use crate::gui::app_state::AppState;
use crate::sequencer::SequencerCommand;

pub fn show(ui: &mut Ui, state: &mut AppState) {
	ui.heading("Sequence");
	ui.add_space(4.0);

	ui.horizontal(|ui| {
		ui.label("Active:");
		ui.monospace(&state.active_sequence);
	});

	let mut requested: Option<String> = None;
	ui.horizontal(|ui| {
		ui.label("Select:");
		egui::ComboBox::from_id_salt("sequence_combo")
			.selected_text(state.selected_sequence.clone())
			.show_ui(ui, |ui| {
				for name in &state.available_sequences {
					let resp = ui.selectable_value(
						&mut state.selected_sequence,
						name.clone(),
						name,
					);
					if resp.clicked() {
						requested = Some(name.clone());
					}
				}
			});
	});

	if let Some(name) = requested {
		if name != state.active_sequence {
			if let Some(tx) = &state.command_tx {
				let _ = tx.try_send(SequencerCommand::SwitchTo(name));
			}
		}
	}

	if state.selected_sequence != state.active_sequence {
		ui.colored_label(
			ui.visuals().warn_fg_color,
			"(switching at next bar boundary…)",
		);
	}
}
