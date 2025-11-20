use std::sync::{atomic::AtomicU64, Arc};

use crate::{
    app::GuiApp,
    engine::{
        reverb::Reverb,
        score::{sequence::Sequence, track_node::TrackNode},
    },
    layout_left, F0,
};
use cpal::traits::DeviceTrait;
use egui::{
    Align2, ColorImage, ImageButton, Layout, RichText, TextureHandle, TextureOptions, Vec2,
};
#[cfg(not(target_arch = "wasm32"))]
use log::{error, info};
use synth_core::stream::stream;

const METRONOME_BRIGHT: &[u8] = include_bytes!("../../../assets/metronome_bright.gif");
const METRONOME_DARK: &[u8] = include_bytes!("../../../assets/metronome_dark.gif");
const CHRONO_BRIGHT: &[u8] = include_bytes!("../../../assets/chronometer_bright.gif");
const CHRONO_DARK: &[u8] = include_bytes!("../../../assets/chronometer_dark.gif");
const NEW_SCORE_BRIGHT: &[u8] = include_bytes!("../../../assets/new_score_bright.gif");
const NEW_SCORE_DARK: &[u8] = include_bytes!("../../../assets/new_score_dark.gif");
const ADD_TRACK_BRIGHT: &[u8] = include_bytes!("../../../assets/add_track_bright.gif");
const ADD_TRACK_DARK: &[u8] = include_bytes!("../../../assets/add_track_dark.gif");

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
        self.ensure_toolbar_icons(ctx);
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
                        if toolbar_icon_button(ui, self.new_score_icon.as_ref(), "New score", "New")
                            .clicked()
                        {
                            self.show_new_score_confirm = true;
                        }
                        if !self.show_start {
                            if toolbar_icon_button(
                                ui,
                                self.add_track_icon.as_ref(),
                                "Add track",
                                "Add",
                            )
                            .clicked()
                            {
                                let seq = TrackNode::from_sequence(Sequence::new(
                                    self.score.last_token.next(),
                                ));
                                self.new_node(seq);
                                // TODO: reactivate
                                // self.selected = Some(self.score.sequences.len() - 1);
                            }
                        }

                        *load = false;
                        if toolbar_icon_button(ui, self.load_icon.as_ref(), "Load", "📂").clicked()
                        {
                            self.stream = None;
                            self.show_start = false;
                            *load = true;
                        }

                        // NOTE: already tested
                        if !self.show_start {
                            *save = false;
                            if toolbar_icon_button(ui, self.save_icon.as_ref(), "Save", "💾")
                                .clicked()
                            {
                                *save = true;
                            }
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
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                ui.add_space(8.0);
                                self.draw_record_controls(ui);
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
                            if ui
                                .button("Exit")
                                .on_hover_ui(|ui| {
                                    ui.label(RichText::new("Shortcut: Escape").weak());
                                })
                                .clicked()
                            {
                                self.show_exit_confirm = true;
                            }
                        }
                        let elapsed = self.now().as_secs();
                        ui.horizontal(|ui| {
                            if let Some(icon) = self.chrono_icon.as_ref() {
                                ui.image((icon.id(), Vec2::splat(32.0)))
                                    .on_hover_text("Elapsed playback time");
                            }
                            ui.label(format!("{elapsed:.1}s"));
                        });
                        if !self.show_start {
                            ui.separator();
                            self.draw_tempo_control(ui);
                            ui.separator();
                            self.draw_spectrogram_toggle(ui);
                        }
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
        self.show_confirmation_dialogs(ctx, save, exit);
    }

    fn draw_tempo_control(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(icon) = self.tempo_icon.as_ref() {
                ui.image((icon.id(), Vec2::splat(32.0)))
                    .on_hover_text("Metronome tempo");
            }
            ui.label("Tempo (BPM)");
            let mut tempo_bpm = self.score.tempo().beats_per_minute();
            let changed = ui
                .add(
                    egui::DragValue::new(&mut tempo_bpm)
                        .range(20.0..=240.0)
                        .speed(0.5),
                )
                .changed();
            if changed {
                self.set_tempo_bpm(tempo_bpm);
            }
        });
    }

    fn draw_spectrogram_toggle(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let prev = self.show_spectrogram_panel;
            if ui
                .checkbox(&mut self.show_spectrogram_panel, "Show spectrogram preview")
                .on_hover_text("Toggle the docked spectrogram panel at the bottom")
                .changed()
            {
                if self.show_spectrogram_panel && !prev {
                    self.spectrogram_render_requested = true;
                }
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn draw_record_controls(&mut self, ui: &mut egui::Ui) {
        if self.recorder.is_recording() {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                RichText::new("● Recording")
                    .strong()
                    .color(egui::Color32::LIGHT_RED),
            );
            if ui.button("Stop").on_hover_text("Stop recording").clicked() {
                if let Err(err) = self.recorder.stop() {
                    error!("Failed to stop recording: {err:?}");
                    self.record_error = Some(err.to_string());
                } else {
                    self.record_error = None;
                }
            }
        } else if ui
            .button("Record WAV…")
            .on_hover_text("Record master output to a WAV file")
            .clicked()
        {
            self.start_recording_dialog();
        }
        if let Some(err) = &self.record_error {
            ui.colored_label(egui::Color32::LIGHT_RED, err);
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
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_new_score_confirm = false;
                        }
                        if ui.button("Save before closing").clicked() {
                            *save = true;
                        }
                        if ui.button("Discard & start new").clicked() {
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
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_exit_confirm = false;
                        }
                        if ui.button("Save before exit").clicked() {
                            *save = true;
                        }
                        if ui.button("Exit anyway").clicked() {
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

    fn ensure_toolbar_icons(&mut self, ctx: &egui::Context) {
        let dark_mode = ctx.style().visuals.dark_mode;
        let needs_refresh = self.toolbar_icon_dark_mode != Some(dark_mode)
            || self.tempo_icon.is_none()
            || self.chrono_icon.is_none()
            || self.new_score_icon.is_none()
            || self.add_track_icon.is_none();
        if !needs_refresh {
            return;
        }
        self.tempo_icon = Some(load_toolbar_image(
            ctx,
            "toolbar_metronome",
            themed_bytes(METRONOME_BRIGHT, METRONOME_DARK, dark_mode),
        ));
        self.chrono_icon = Some(load_toolbar_image(
            ctx,
            "toolbar_chrono",
            themed_bytes(CHRONO_BRIGHT, CHRONO_DARK, dark_mode),
        ));
        self.new_score_icon = Some(load_toolbar_image(
            ctx,
            "toolbar_new_score",
            themed_bytes(NEW_SCORE_BRIGHT, NEW_SCORE_DARK, dark_mode),
        ));
        self.add_track_icon = Some(load_toolbar_image(
            ctx,
            "toolbar_add_track",
            themed_bytes(ADD_TRACK_BRIGHT, ADD_TRACK_DARK, dark_mode),
        ));
        self.toolbar_icon_dark_mode = Some(dark_mode);
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

fn toolbar_icon_button(
    ui: &mut egui::Ui,
    icon: Option<&TextureHandle>,
    tooltip: &str,
    fallback_label: &str,
) -> egui::Response {
    if let Some(texture) = icon {
        ui.add(ImageButton::new((texture.id(), Vec2::splat(32.0))).frame(false))
            .on_hover_text(tooltip)
    } else {
        ui.button(fallback_label).on_hover_text(tooltip)
    }
}

fn themed_bytes<'a>(bright: &'a [u8], dark: &'a [u8], dark_mode: bool) -> &'a [u8] {
    if dark_mode {
        dark
    } else {
        bright
    }
}

fn load_toolbar_image(ctx: &egui::Context, name: &str, bytes: &'static [u8]) -> TextureHandle {
    let rgba = image::load_from_memory(bytes)
        .expect("invalid toolbar icon")
        .to_rgba8();
    let (w, h) = rgba.dimensions();
    let pixels = rgba.into_vec();
    let image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
    ctx.load_texture(name, image, TextureOptions::LINEAR)
}
