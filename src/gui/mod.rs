//! GUI glue for st-click.

pub mod app_state;
pub mod beat_indicator;
pub mod sequence_panel;

use crossbeam_channel::Receiver;
use eframe::egui;
use std::time::Duration;

use app_state::AppState;

pub struct App {
	state: AppState,
	beat_rx: Receiver<u64>,
}

impl App {
	pub fn new(state: AppState, beat_rx: Receiver<u64>) -> Self {
		Self { state, beat_rx }
	}

	fn drain_beats(&mut self) {
		while let Ok(n) = self.beat_rx.try_recv() {
			self.state.beat_count = n;
			self.state.last_beat_at = std::time::Instant::now();
		}
	}
}

impl eframe::App for App {
	fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
		self.drain_beats();
		let ctx = ui.ctx().clone();

		egui::Panel::bottom("status_bar").show_inside(ui, |ui| {
			ui.horizontal(|ui| {
				ui.label("st-click");
				ui.separator();
				ui.label(format!("beats: {}", self.state.beat_count));
			});
		});

		ui.horizontal(|ui| {
			beat_indicator::show(ui, &self.state);
			ui.add_space(8.0);
			ui.heading("Click");
		});
		ui.separator();
		sequence_panel::show(ui, &mut self.state);

		ctx.request_repaint_after(Duration::from_millis(30));
	}
}

pub fn run(state: AppState, beat_rx: Receiver<u64>) -> Result<(), eframe::Error> {
	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([480.0, 280.0])
			.with_title("st-click"),
		..Default::default()
	};

	eframe::run_native(
		"st-click",
		options,
		Box::new(move |cc| {
			cc.egui_ctx.set_visuals(egui::Visuals::dark());
			Ok(Box::new(App::new(state, beat_rx)))
		}),
	)
}
