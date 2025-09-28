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
    F0,
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
                    self.show_start = false;
                }
                if !self.show_start {
                    if ui.button("Add track").clicked() {
                        let seq = Sequence::new(self.last_token.next());
                        self.new_seq(seq);
                        self.selected = Some(self.sequences.len() - 1);
                        // if self.stream.is_none() {
                        //     self.start_stream(self.clock.clone());
                        // }
                    }
                }

                *load = if ui.button("Load…").clicked() {
                    self.stream = None;
                    self.show_start = false;
                    true
                } else {
                    false
                };
                if !self.show_start {
                    *save = ui.button("Save…").clicked();
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
                };
                if !self.show_doc {
                    if ui.button("Examples").clicked() {
                        self.try_load_default(ctx);
                    }
                }
                if !self.show_doc {
                    if ui.button("README").clicked() {
                        self.show_doc = true;
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    *exit = ui.button("Exit").clicked();
                }

                // if self.stream.is_some() {
                //     ui.label("stream");
                // }
                // ui.label(format!("#notes: {}", {
                //     let tmp = self.notes.iter().map(|ng| ng.notes.len()).sum::<usize>();
                //     tmp
                // }));
                // #[cfg(target_arch = "wasm32")]
                // {
                //     self.fps = 0.9 * self.fps + 1e8 / self.instant.elapsed().as_nanos() as f64;
                //     self.instant = instant::Instant::now();
                //     ui.label(format!("fps: {:.0}", self.fps));
                //     ui.add(Slider::new(&mut self.min_fps, 12.0..=MAX_FPS).show_value(false))
                //         .on_hover_text(format!("Minimum fps: {}", self.min_fps));
                // }
            });
            if !self.show_start {
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
            F0,
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
