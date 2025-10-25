use std::sync::{atomic::AtomicU64, Arc};

use cpal::traits::DeviceTrait;
#[cfg(target_arch = "wasm32")]
use egui::Slider;
use egui::{Layout, RichText};

#[cfg(target_arch = "wasm32")]
use crate::MAX_FPS;
use crate::{
    app::GuiApp,
    engine::{
        reverb::Reverb,
        score::{sequence::Sequence, track_node::TrackNode},
    },
    layout_left,
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
            ui.horizontal_centered(|ui| {
                if let Some(logo) = &self.logo {
                    let size = egui::Vec2::new(self.property_panel_width - 3.5, 125.0);
                    ui.image((logo.id(), size));
                }
                ui.separator();

                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("New score").clicked() {
                            self.new_score();
                            self.selected = None;
                            self.show_start = false;
                        }
                        if !self.show_start {
                            if ui.button("Add track").clicked() {
                                let seq = TrackNode::from_sequence(Sequence::new(
                                    self.score.last_token.next(),
                                ));
                                self.new_node(seq);
                                // TODO: reactivate
                                // self.selected = Some(self.score.sequences.len() - 1);
                            }
                        }

                        *load = if ui.button("Load…").clicked() {
                            self.stream = None;
                            self.show_start = false;
                            true
                        } else {
                            false
                        };

                        // NOTE: already tested
                        if !self.show_start {
                            *save = ui.button("Save…").clicked();
                            if ui
                                .add(egui::Button::new(if self.stream.is_none() {
                                    "▶"
                                } else {
                                    "⏸"
                                }))
                                .on_hover_ui(|ui| {
                                    ui.label(RichText::new("Shortcut: Space bar").weak());
                                })
                                .clicked()
                                || ui.input(|i| i.key_pressed(egui::Key::Space))
                            {
                                if self.stream.is_some() {
                                    self.stream = None
                                } else {
                                    self.start_stream(self.clock.clone());
                                }
                            }
                        }
                        if !self.show_doc {
                            if ui.button("Examples").clicked() {
                                self.try_load_default(ctx);
                            }
                            if ui.button("README").clicked() {
                                self.show_doc = true;
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            *exit = ui
                                .button("Exit")
                                .on_hover_ui(|ui| {
                                    ui.label(RichText::new("Shortcut: Escape").weak());
                                })
                                .clicked();
                        }
                        ui.label(format!("{:.1}s", self.now().as_secs()));
                    });
                    if !self.show_start {
                        ui.separator();
                        ui.columns(2, |cols| {
                            // Left Delays (normal)
                            cols[0].vertical(|mut ui_left| {
                                ui_left.label("Left Delays (ms)");
                                Self::edit_vec(
                                    &mut ui_left,
                                    &mut self.score.delays.0,
                                    0.0,
                                    layout_left(),
                                );
                            });

                            cols[1].vertical(|mut ui_right| {
                                // Right Delays (normal)
                                ui_right.horizontal(|ui_right| {
                                    ui_right.with_layout(
                                        Layout::right_to_left(egui::Align::Max),
                                        |ui_right| {
                                            ui_right.label("Right Delays (ms)");
                                        },
                                    )
                                });
                                Self::edit_vec(
                                    &mut ui_right,
                                    &mut self.score.delays.1,
                                    0.0,
                                    Layout::right_to_left(egui::Align::Max),
                                );
                            });
                        });
                    }
                });
            });
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
            self.score.shared_notes.clone(),
            (
                Reverb::new(0.5, 0.5, sample_rate),
                Reverb::new(0.5, 0.5, sample_rate),
            ),
            self.shared_delays.clone(),
        ))
    }
}
