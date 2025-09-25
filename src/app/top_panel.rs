use std::sync::{atomic::AtomicU64, Arc};

use cpal::traits::DeviceTrait;
#[cfg(target_arch = "wasm32")]
use egui::Slider;

#[cfg(target_arch = "wasm32")]
use crate::MAX_FPS;
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
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New score").clicked() {
                    self.new_score();
                    self.selected = None;
                }
                if ui.button("Add track").clicked() {
                    let seq = Sequence::new(self.last_token.next());
                    self.new_seq(seq);
                    self.selected = Some(self.sequences.len() - 1);
                    if self.stream.is_none() {
                        self.start_stream(self.clock.clone());
                    }
                }

                *load = if ui.button("Load…").clicked() {
                    self.stream = None;
                    true
                } else {
                    false
                };
                *save = ui.button("Save…").clicked();
                #[cfg(not(target_arch = "wasm32"))]
                {
                    *exit = ui.button("Exit").clicked();
                }
                if ui
                    .add(egui::Button::new(if self.stream.is_none() {
                        "▶"
                    } else {
                        "⏸"
                    }))
                    .clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Space))
                {
                    if self.stream.is_some() {
                        self.stream = None
                    } else {
                        self.start_stream(self.clock.clone());
                    }
                }

                if self.stream.is_some() {
                    ui.label("stream");
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.fps = 0.9 * self.fps + 1e8 / self.instant.elapsed().as_nanos() as f64;
                    self.instant = instant::Instant::now();
                    ui.label(format!("fps: {:.0}", self.fps));
                    ui.add(Slider::new(&mut self.min_fps, 12.0..=MAX_FPS).show_value(false))
                        .on_hover_text(format!("Minimum fps: {}", self.min_fps));
                }
            });
            ui.separator();
            {
                // let mut delays = self.delays.clone();
                ui.columns(2, |cols| {
                    Self::edit_vec(
                        &mut cols[0],
                        &mut self.delays.0,
                        Some("Left Delays (ms)"),
                        0.0,
                    );
                    Self::edit_vec(
                        &mut cols[1],
                        &mut self.delays.1,
                        Some("Right Delays (ms)"),
                        0.0,
                    );
                });
            }
        });
    }

    fn start_stream(&mut self, clock: Arc<AtomicU64>) {
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
            self.shared_notes.clone(),
            (
                Reverb::new(0.5, 0.5, sample_rate),
                Reverb::new(0.5, 0.5, sample_rate),
            ),
            self.shared_delays.clone(),
        ))
    }
}
