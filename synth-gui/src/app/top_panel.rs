use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::{
    app::{GuiApp, PropertySection},
    engine::{
        reverb::Reverb,
        score::{
            sequence::Sequence,
            track_node::{NodeKind, TrackNode},
        },
        waves::WaveType,
    },
    F0, TOOLBAR_ICON_SIZE,
};
use cpal::traits::DeviceTrait;
use egui::{Align2, Area, Frame, Id, Order, Pos2, Rect, RichText, Vec2};
#[cfg(not(target_arch = "wasm32"))]
use log::{error, info};
use synth_core::stream::stream;

impl GuiApp {
    pub fn top_panel(
        &mut self,
        ctx: &egui::Context,
        save: &mut bool,
        load: &mut bool,
        exit: &mut bool,
    ) {
        #[cfg(target_arch = "wasm32")]
        let _ = exit;
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                if let Some(logo) = &self.logo {
                    let size = egui::Vec2::new(self.property_panel_width - 3.5, 125.0);
                    ui.image((logo.id(), size));
                }
                ui.separator();

                ui.vertical(|ui| {
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if toolbar_icon_button(ui, "🎼", "New score").clicked() {
                            self.show_new_score_confirm = true;
                        }
                        if !self.show_start {
                            if toolbar_icon_button(ui, "➕", "Add track").clicked() {
                                let seq = TrackNode::from_sequence(Sequence::new(
                                    self.score.last_token.next(),
                                ));
                                self.new_node(seq);
                            }
                        }

                        if !self.show_doc {
                            if toolbar_icon_button(ui, "🎶", "Examples").clicked() {
                                self.try_load_default(ctx);
                            }
                        }
                        *load = false;
                        if toolbar_icon_button(ui, "📂", "Load").clicked() {
                            self.stream = None;
                            self.show_start = false;
                            *load = true;
                        }

                        if !self.show_start {
                            *save = toolbar_icon_button(ui, "💾", "Save").clicked();
                            let play_label = if self.stream.is_none() { "▶" } else { "⏸" };
                            let play_tip = if self.stream.is_none() {
                                "Play\nShortcut: Space bar"
                            } else {
                                "Pause\nShortcut: Space bar"
                            };
                            if toolbar_icon_button(ui, play_label, play_tip).clicked()
                                || ui.input(|i| i.key_pressed(egui::Key::Space))
                            {
                                if self.stream.is_some() {
                                    self.stream = None
                                } else {
                                    self.start_stream(self.clock.clone());
                                }
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                self.draw_record_controls(ui);
                            }
                        }
                        if !self.show_doc {
                            if toolbar_icon_button(ui, "📖", "README").clicked() {
                                self.show_doc = true;
                            }
                        }
                        let elapsed = self.now().as_secs();
                        if toolbar_icon_button(ui, "🕛", "Reset elapsed time").clicked() {
                            self.clock.store(0, Ordering::Relaxed);
                            self.score.reset_playback();
                            self.score.generate_notes(self.now(), &mut self.rng);
                            self.score
                                .shared_notes
                                .store(Arc::new(self.score.notes.clone()));
                        }
                        ui.label(format!("{elapsed:.1}s"));
                        if !self.show_start {
                            self.draw_tempo_control(ui);
                            self.draw_spectrogram_toggle(ui);
                            self.draw_zoom_controls(ui);
                            self.draw_track_height_controls(ui);
                        }

                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            if toolbar_icon_button(ui, "❌", "Exit\nShortcut: Escape").clicked() {
                                self.show_exit_confirm = true;
                            }
                        }
                    });

                    ui.separator();

                    // Second row: Property section selector (only when a track is selected)
                    if !self.show_start {
                        self.draw_section_selector(ui);
                    }
                });
            });
        });
        self.render_tempo_popup(ctx);
        self.show_confirmation_dialogs(ctx, save, exit);
    }

    fn draw_section_selector(&mut self, ui: &mut egui::Ui) {
        let Some(sel) = &self.selected else {
            return;
        };
        let Some(node) = self.score.track_root.get(sel) else {
            return;
        };

        let is_group = node.is_group();
        let is_drum = match &node.kind {
            NodeKind::Seq(seq) => matches!(
                seq.wave_type,
                WaveType::HiHat
                    | WaveType::Kick
                    | WaveType::Snare
                    | WaveType::Ride
                    | WaveType::Darbuka
            ),
            NodeKind::Group { .. } => false,
        };

        let sections = if is_group {
            PropertySection::group_sections()
        } else {
            PropertySection::sequence_sections()
        };

        ui.horizontal(|ui| {
            ui.add_space(4.0);
            for section in sections {
                // Skip Chorus and Harmony for drum waves (sequences only)
                if !is_group && is_drum {
                    if matches!(section, PropertySection::Chorus | PropertySection::Harmony) {
                        continue;
                    }
                }

                let is_active = self.active_property_section == *section;
                let button = egui::Button::new(RichText::new(section.label()).size(11.0).strong())
                    .fill(if is_active {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals().widgets.inactive.bg_fill
                    })
                    .min_size(Vec2::new(32.0, 20.0));

                if ui.add(button).on_hover_text(section.tooltip()).clicked() {
                    self.active_property_section = *section;
                }
            }
        });
    }

    fn draw_tempo_control(&mut self, ui: &mut egui::Ui) {
        let response = toolbar_icon_button(ui, "💓", "Tempo control");
        if response.clicked() {
            self.tempo_popup_open = !self.tempo_popup_open;
            self.tempo_popup_pos = Some(response.rect.left_bottom() + Vec2::new(0.0, 8.0));
        }
        let tempo = self.score.tempo().beats_per_minute();
        ui.label(format!("{tempo:.0} BPM"));
    }

    fn draw_spectrogram_toggle(&mut self, ui: &mut egui::Ui) {
        let prev = self.show_spectrogram_panel;
        if toolbar_icon_button(
            ui,
            "📡",
            if self.show_spectrogram_panel {
                "Hide spectrogram preview"
            } else {
                "Show spectrogram preview"
            },
        )
        .clicked()
        {
            self.show_spectrogram_panel = !self.show_spectrogram_panel;
            if self.show_spectrogram_panel && !prev {
                self.spectrogram_render_requested = true;
            }
        }
        let status = if self.show_spectrogram_panel {
            "Spectrogram on"
        } else {
            "Spectrogram off"
        };
        ui.label(status);
    }

    fn render_tempo_popup(&mut self, ctx: &egui::Context) {
        if !self.tempo_popup_open {
            return;
        }
        let pos = self
            .tempo_popup_pos
            .unwrap_or_else(|| ctx.available_rect().left_top());
        Area::new(Id::new("tempo_popup"))
            .order(Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(160.0);
                    ui.label("Tempo");
                    let mut tempo_bpm = self.score.tempo().beats_per_minute();
                    let changed = ui
                        .add(
                            egui::Slider::new(&mut tempo_bpm, 20.0..=240.0)
                                .logarithmic(false)
                                .text("BPM"),
                        )
                        .changed();
                    if changed {
                        self.set_tempo_bpm(tempo_bpm);
                    }
                    if ui.button("Close").clicked() {
                        self.tempo_popup_open = false;
                    }
                });
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_record_controls(&mut self, ui: &mut egui::Ui) {
        if self.recorder.is_recording() {
            if toolbar_icon_button(ui, "⏹", "Stop recording").clicked() {
                if let Err(err) = self.recorder.stop() {
                    error!("Failed to stop recording: {err:?}");
                    self.record_error = Some(err.to_string());
                } else {
                    self.record_error = None;
                }
            }
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                RichText::new("Recording")
                    .strong()
                    .color(egui::Color32::LIGHT_RED),
            );
        } else if toolbar_icon_button(ui, "⏺", "Record master output to a WAV file").clicked() {
            self.start_recording_dialog();
        }
        if let Some(err) = &self.record_error {
            ui.colored_label(egui::Color32::LIGHT_RED, err);
        }
    }

    fn draw_zoom_controls(&mut self, ui: &mut egui::Ui) {
        let (_, _, track_display_length) = self.timeline_lengths(self.score.tempo());
        let dummy_rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0));
        if toolbar_icon_button(ui, "🔎➖", "Zoom out timeline").clicked() {
            self.adjust_timeline_zoom(0.9, 0.5, track_display_length, dummy_rect);
        }
        if toolbar_icon_button(ui, "🔎➕", "Zoom in timeline").clicked() {
            self.adjust_timeline_zoom(1.1, 0.5, track_display_length, dummy_rect);
        }
        ui.label(format!("{:.0}%", (self.timeline_zoom * 100.0).round()));
    }

    fn draw_track_height_controls(&mut self, ui: &mut egui::Ui) {
        let response = toolbar_icon_button(ui, "📏", "Fixed track height");
        if response.clicked() {
            self.fixed_track_height = !self.fixed_track_height;
        }
        if self.fixed_track_height {
            ui.add(egui::Slider::new(&mut self.track_lane_height, 40.0..=240.0).text("px"));
        }
    }

    fn show_confirmation_dialogs(&mut self, ctx: &egui::Context, save: &mut bool, exit: &mut bool) {
        if self.show_new_score_confirm {
            let mut keep_open = true;
            egui::Window::new("Start a new score?")
                .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .collapsible(false)
                .resizable(false)
                .open(&mut keep_open)
                .show(ctx, |ui| {
                    ui.label("This discards the current score.");
                    ui.label("Consider saving first so no work is lost.");
                    ui.separator();

                    let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                    let esc_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));

                    if esc_pressed {
                        ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }
                    if enter_pressed {
                        ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() || esc_pressed {
                            self.show_new_score_confirm = false;
                        }
                        if ui.button("Save before closing").clicked() {
                            *save = true;
                        }
                        if ui.button("Discard & start new").clicked() || enter_pressed {
                            self.new_score();
                            self.selected = None;
                            self.show_start = false;
                            self.show_new_score_confirm = false;
                        }
                    });
                });
            if !keep_open {
                self.show_new_score_confirm = false;
            }
        }

        if self.show_exit_confirm {
            let mut keep_open = true;
            egui::Window::new("Exit Synth?")
                .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .collapsible(false)
                .resizable(false)
                .open(&mut keep_open)
                .show(ctx, |ui| {
                    ui.label("Exiting closes the app.");
                    ui.label("Save your work before leaving.");
                    ui.separator();

                    let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                    let esc_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));

                    if esc_pressed {
                        ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }
                    if enter_pressed {
                        ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() || esc_pressed {
                            self.show_exit_confirm = false;
                        }
                        if ui.button("Save before exit").clicked() {
                            *save = true;
                        }
                        if ui.button("Exit anyway").clicked() || enter_pressed {
                            self.show_exit_confirm = false;
                            *exit = true;
                        }
                    });
                });
            if !keep_open {
                self.show_exit_confirm = false;
            }
        }
    }

    fn start_stream(&mut self, clock: Arc<AtomicU64>) {
        self.shared_delays
            .store(Arc::new(self.score.track_root.delays.to_seconds(
                self.score.tempo(),
                0.5,
                0.5,
            )));
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
                Reverb::new(1.0, 1.0, sample_rate),
                Reverb::new(1.0, 1.0, sample_rate),
            ),
            self.shared_delays.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            Some(self.recorder.clone()),
            #[cfg(target_arch = "wasm32")]
            None,
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_recording_dialog(&mut self) {
        use rfd::FileDialog;
        if let Some(path) = FileDialog::new()
            .set_title("Record output to WAV")
            .add_filter("wav", &["wav"])
            .set_file_name("recording.wav")
            .save_file()
        {
            match self.recorder.start(&path, self.sample_rate as u32) {
                Ok(_) => {
                    self.record_error = None;
                    info!("Recording audio to {}", path.display());
                }
                Err(err) => {
                    error!("Failed to start recording: {err}");
                    self.record_error = Some(err.to_string());
                }
            }
        }
    }
}

fn toolbar_icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> egui::Response {
    let text = RichText::new(glyph).size(TOOLBAR_ICON_SIZE * 0.4);
    ui.add(
        egui::Button::new(text)
            .min_size(Vec2::new(TOOLBAR_ICON_SIZE * 0.6, TOOLBAR_ICON_SIZE * 0.6)),
    )
    .on_hover_text(tooltip)
}
