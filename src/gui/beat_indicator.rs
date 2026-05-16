//! Beat indicator lamp for st-click. Identical pattern to the conductor's.

use eframe::egui::{self, Color32, Sense, Ui};
use std::time::Duration;
use crate::gui::app_state::AppState;

const LAMP_SIZE: f32 = 28.0;
const LAMP_DECAY: Duration = Duration::from_millis(120);
const LAMP_ON: Color32 = Color32::from_rgb(0x66, 0xff, 0x88);
const LAMP_OFF: Color32 = Color32::from_rgb(0x20, 0x40, 0x28);

pub fn show(ui: &mut Ui, state: &AppState) {
	let elapsed = state.last_beat_at.elapsed();
	let intensity = if elapsed >= LAMP_DECAY {
		0.0
	} else {
		1.0 - (elapsed.as_secs_f32() / LAMP_DECAY.as_secs_f32())
	};
	let color = lerp_color(LAMP_OFF, LAMP_ON, intensity);

	let (rect, _) = ui.allocate_exact_size(
		egui::vec2(LAMP_SIZE, LAMP_SIZE),
		Sense::hover(),
	);
	let center = rect.center();
	let radius = LAMP_SIZE * 0.5 - 2.0;
	ui.painter().circle_filled(center, radius, color);
	ui.painter().circle_stroke(
		center,
		radius,
		egui::Stroke::new(1.0, ui.visuals().widgets.inactive.fg_stroke.color),
	);
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
	let t = t.clamp(0.0, 1.0);
	let lerp = |x: u8, y: u8| -> u8 {
		(x as f32 + (y as f32 - x as f32) * t).round() as u8
	};
	Color32::from_rgb(
		lerp(a.r(), b.r()),
		lerp(a.g(), b.g()),
		lerp(a.b(), b.b()),
	)
}
