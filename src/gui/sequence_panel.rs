//! Sequence selector panel. In v1 the combo box does not yet swap the
//! running sequence; see TODO on `AppState::active_sequence`.

use eframe::egui::{self, Ui};
use crate::gui::app_state::AppState;

pub fn show(ui: &mut Ui, state: &mut AppState) {
	ui.heading("Sequence");
	ui.add_space(4.0);

	ui.horizontal(|ui| {
		ui.label("Active:");
		ui.monospace(&state.active_sequence);
	});

	ui.horizontal(|ui| {
		ui.label("Select:");
		egui::ComboBox::from_id_salt("sequence_combo")
			.selected_text(state.selected_sequence.clone())
			.show_ui(ui, |ui| {
				for name in &state.available_sequences {
					ui.selectable_value(
						&mut state.selected_sequence,
						name.clone(),
						name,
					);
				}
			});
	});

	if state.selected_sequence != state.active_sequence {
		ui.colored_label(
			ui.visuals().warn_fg_color,
			"(runtime sequence swap not yet implemented)",
		);
	}
}
