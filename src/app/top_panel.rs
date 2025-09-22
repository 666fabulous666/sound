use arc_swap::ArcSwap;
use std::sync::{Arc, Mutex};

use cpal::traits::DeviceTrait;

use crate::{
    app::GuiApp,
    engine::{notes::Sequence, reverb::Reverb},
    stream::stream,
};

impl GuiApp {
    pub fn top_panel(
        &mut self,
        ctx: &egui::Context,
        save: &mut bool,
        load: &mut bool,
        exit: &mut bool,
    ) {
        let now = self.now();
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New score").clicked() {
                    self.new_score();
                    self.selected = None;
                }
                if ui.button("Add track").clicked() {
                    let seq = Sequence::new(self.last_token);
                    self.last_token += 1; // TODO: handle it internally
                    self.new_seq(seq);
                    self.selected = Some(self.last_token);
                    if self.stream.is_none() {
                        self.start_stream(now.clone());
                    }
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    *save = ui.button("Save…").clicked();
                    *load = if ui.button("Load…").clicked() {
                        self.stream = None;
                        true
                    } else {
                        false
                    };
                    *exit = ui.button("Exit").clicked();
                }
                if ui
                    .add(egui::Button::new(if self.stream.is_none() {
                        "▶"
                    } else {
                        "⏸"
                    }))
                    .clicked()
                {
                    if self.stream.is_some() {
                        self.stream = None
                    } else {
                        self.start_stream(now.clone());
                    }
                }

                if self.stream.is_some() {
                    ui.label("stream");
                }
            });
            ui.separator();
            ui.columns(2, |cols| {
                Self::edit_vec(
                    &mut cols[0],
                    &mut self.score_params.delays.0,
                    Some("Left Delays (ms)"),
                    0.0,
                );
                Self::edit_vec(
                    &mut cols[1],
                    &mut self.score_params.delays.1,
                    Some("Right Delays (ms)"),
                    0.0,
                );
            });
        });
    }

    fn start_stream(&mut self, clock: Arc<Mutex<f64>>) {
        let sample_rate = self
            .device
            .default_output_config()
            .expect("Can't find default output config")
            .sample_rate()
            .0 as f64;
        self.stream = Some(stream(
            440.0,
            &self.device,
            clock,
            // self.notes.clone(),
            self.shared_notes.clone(),
            (
                Reverb::new(0.5, 0.5, self.score_params.delays.0.clone(), sample_rate),
                Reverb::new(0.5, 0.5, self.score_params.delays.1.clone(), sample_rate),
            ),
        ))
    }
}
