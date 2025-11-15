pub mod helpers;
pub mod hover_texts;
mod navigation;
mod rhythm;

use egui::{Grid, RichText, ScrollArea, TextEdit};
use helpers::{slider_with_reset, u32_cell};

use crate::{
    app::{
        property_panel::{
            hover_texts::{
                GROOVE_OFFSET_TEXT, HARMONISE_TEXT, LOOP_LENGTH_TEXT, OCTAVE_TEXT, POW_FACT_TEXT,
                REPEAT_TEXT, SHUFFLE_TEXT, TIME_QUANTUM_TEXT, TOLERENCE_TEXT, UNISSON_DETUNE_TEXT,
                VARIATION_INTERVALS_TEXT, VARIATION_STEPS_TEXT,
            },
            navigation::navigation,
        },
        GuiApp, ALL_WAVES, DRUM_WAVES,
    },
    engine::score::{default_params::*, sequence::Sequence, track_node::NodeKind, Interval},
    layout_left,
    shortcuts::*,
    time_freq::Time,
    Token,
};

#[derive(Clone)]
pub enum Action {
    None,
    Mute,
    Delete,
    Clone,
    Parent,
    FirstChild,
    SelectUp,
    SelectDown,
    MoveUp,
    MoveDown,
    Wrap,
    Promote,
    Dissolve,
    GroupAbove,
    GroupBelow,
}
impl Action {
    pub fn label_and_action(self) -> (&'static str, Self) {
        match self {
            Action::None => ("None", self),
            Action::Delete => ("Delete", self),
            Action::Clone => ("Clone", self),
            Action::Parent => ("Parent", self),
            Action::FirstChild => ("First child", self),
            Action::SelectUp => ("Select up", self),
            Action::SelectDown => ("Select down", self),
            Action::MoveUp => ("Move up", self),
            Action::MoveDown => ("Move down", self),
            Action::Wrap => ("Wrap", self),
            Action::Promote => ("Promote", self),
            Action::Dissolve => ("Dissolve", self),
            Action::GroupAbove => ("Group above", self),
            Action::GroupBelow => ("Group below", self),
            Action::Mute => ("Mute", self),
        }
    }
}

#[derive(Default, Clone)]
struct ParameterImpact {
    needs_regeneration: bool,
    needs_mix_update: bool,
    octave_shifts: Vec<(Token, i32)>,
}

impl ParameterImpact {
    fn require_regeneration(&mut self) {
        self.needs_regeneration = true;
    }

    fn require_mix_update(&mut self) {
        self.needs_mix_update = true;
    }

    fn queue_octave_shift(&mut self, token: Token, shift: i32) {
        if shift != 0 {
            self.octave_shifts.push((token, shift));
        }
    }

    fn needs_regeneration(&self) -> bool {
        self.needs_regeneration
    }

    fn needs_mix_update(&self) -> bool {
        self.needs_mix_update
    }

    fn drain_octave_shifts(&mut self) -> Vec<(Token, i32)> {
        std::mem::take(&mut self.octave_shifts)
    }
}

impl GuiApp {
    pub fn property_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("props")
            .min_width(self.property_panel_width.max(240.0))
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    let mut action = Action::None;
                    let mut impact = ParameterImpact::default();
                    if let Some(sel) = self.selected.clone() {
                        if let Some(track_node_mut) = self.score.track_root.get_mut(&sel) {
                            navigation(ui, &mut action, track_node_mut);

                            // Show heading based on variant
                            if track_node_mut.is_group() {
                                ui.heading(format!("Group {:?}", sel));
                            } else {
                                ui.heading(format!("Sequence {:?}", sel));
                            }

                            // === COMMON SECTION (shown for both Groups and Sequences) ===

                            // Name field (common)
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                let is_group = track_node_mut.is_group();
                                let name = track_node_mut.name_mut();
                                let resp = ui.add(
                                    TextEdit::singleline(name)
                                        .hint_text(if is_group {
                                            "Group name…"
                                        } else {
                                            "Sequence name…"
                                        })
                                        .desired_width(200.0),
                                );
                                if resp.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    ui.memory_mut(|m| m.surrender_focus(resp.id));
                                }
                            });

                            // Volume control (common) - unified for both sequences and groups
                            ui.horizontal(|ui| {
                                let volume = track_node_mut.volume_mut();

                                // Keyboard shortcuts for volume adjustment
                                let kb_changed = ui.ctx().input(|i| {
                                    let step = if i.modifiers.shift { 0.5 } else { 0.05 };
                                    let mut changed = false;
                                    if i.key_down(egui::Key::Plus) {
                                        *volume = (*volume + step).min(32.0);
                                        changed = true;
                                    }
                                    if i.key_down(egui::Key::Minus) {
                                        *volume = (*volume - step).max(0.0);
                                        changed = true;
                                    }
                                    changed
                                });

                                let vol_resp = slider_with_reset(
                                    ui,
                                    volume,
                                    0.0..=5.0,
                                    "Volume",
                                    Some("+ / - (Shift×10)"),
                                    default_volume(),
                                    false,
                                )
                                .on_hover_ui(|ui| {
                                    ui.label(egui::RichText::new("Hold + / - to change").weak());
                                });

                                if vol_resp.changed() || vol_resp.secondary_clicked() || kb_changed
                                {
                                    impact.require_mix_update();
                                }
                            });

                            // Pan control (common) - unified for both sequences and groups
                            ui.horizontal(|ui| {
                                let pan = track_node_mut.pan_mut();

                                let pan_resp = slider_with_reset(
                                    ui,
                                    pan,
                                    0.0..=1.0,
                                    "Pan",
                                    None,
                                    default_pan(),
                                    false,
                                )
                                .on_hover_ui(|ui| {
                                    ui.label("0.5 is centered, 0.0 is left, 1.0 is right");
                                    ui.label(egui::RichText::new("Right-click to reset").weak());
                                });

                                if pan_resp.changed() || pan_resp.secondary_clicked() {
                                    *pan = pan.clamp(0.0, 1.0);
                                    impact.require_mix_update();
                                }
                            });

                            // Proba control (common)
                            ui.horizontal(|ui| {
                                let mut proba_value = track_node_mut.proba().as_f64();
                                let proba_resp = slider_with_reset(
                                    ui,
                                    &mut proba_value,
                                    0.0..=1.0,
                                    "Proba",
                                    None,
                                    default_proba().as_f64(),
                                    false,
                                )
                                .on_hover_ui(|ui| {
                                    ui.label(egui::RichText::new("Right-click to reset").weak());
                                });

                                if proba_resp.changed() || proba_resp.secondary_clicked() {
                                    track_node_mut.set_proba(proba_value);
                                    impact.require_regeneration();
                                }
                            });

                            // Hue control (common)
                            ui.horizontal(|ui| {
                                let hue = &mut track_node_mut.hue;
                                let hue_resp = slider_with_reset(
                                    ui,
                                    hue,
                                    0.0..=360.0,
                                    "Hue",
                                    None,
                                    0.0,
                                    false,
                                )
                                .on_hover_ui(|ui| {
                                    ui.label("Color hue in HSL space (0-360)");
                                    ui.label(egui::RichText::new("Right-click to reset").weak());
                                });

                                if hue_resp.changed() || hue_resp.secondary_clicked() {
                                    *hue = hue.rem_euclid(360.0);
                                    impact.require_regeneration();
                                }

                                // Color preview square - Convert HSL to RGB
                                let h = *hue / 60.0;
                                let c = 1.0; // chroma at full saturation
                                let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
                                let (r1, g1, b1) = match h as i32 {
                                    0 => (c, x, 0.0),
                                    1 => (x, c, 0.0),
                                    2 => (0.0, c, x),
                                    3 => (0.0, x, c),
                                    4 => (x, 0.0, c),
                                    _ => (c, 0.0, x),
                                };
                                let color = egui::Color32::from_rgb(
                                    (r1 * 255.0) as u8,
                                    (g1 * 255.0) as u8,
                                    (b1 * 255.0) as u8,
                                );

                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(20.0, 20.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 2.0, color);
                            });

                            ui.separator();

                            // === TYPE-SPECIFIC SECTION ===
                            if let Some(seq_mut) = track_node_mut.as_seq_mut() {
                                // Sequence-specific controls

                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("Wave:");
                                    let mut w_choice = seq_mut.wave_type;

                                    egui::ComboBox::from_id_salt("wave_type_combo")
                                        .selected_text((&w_choice).to_string())
                                        .show_ui(ui, |ui| {
                                            for var in ALL_WAVES.iter() {
                                                ui.selectable_value(
                                                    &mut w_choice,
                                                    *var,
                                                    (&var).to_string(),
                                                );
                                            }
                                        });
                                    if w_choice != seq_mut.wave_type {
                                        seq_mut.wave_type = w_choice;
                                        if let Some(ng) = self.score.notes.get_mut(&seq_mut.token) {
                                            ng.wave_type = seq_mut.wave_type;
                                        }
                                    }
                                });

                                ui.collapsing("Envelope", |ui| {
                                    let is_drum = DRUM_WAVES.contains(&seq_mut.wave_type);
                                    helpers::envelope_section(
                                        ui,
                                        seq_mut,
                                        &mut self.score.notes,
                                        is_drum,
                                    );
                                });

                                ui.collapsing("Bend", |ui| {
                                    helpers::bend_section(ui, seq_mut, &mut self.score.notes);
                                });
                                ui.collapsing("Vibrato", |ui| {
                                    helpers::vibrato_section(ui, seq_mut, &mut self.score.notes);
                                });
                                if !DRUM_WAVES.contains(&seq_mut.wave_type) {
                                    let header = ui.collapsing("Chorus (Unison Detune)", |ui| {
                                        helpers::chorus_section(ui, seq_mut, &mut self.score.notes);
                                    });
                                    header.header_response.on_hover_text(UNISSON_DETUNE_TEXT);
                                };

                                let header = ui.collapsing("Power factor", |ui| {
                                    helpers::power_factor_section(
                                        ui,
                                        seq_mut,
                                        &mut self.score.notes,
                                    );
                                });
                                header.header_response.on_hover_text(POW_FACT_TEXT);
                                ui.collapsing("Rythm", |ui| {
                                    {
                                        ui.horizontal(|ui| {
                                            let mut numerator = seq_mut.time_quantum.numerator();
                                            let mut denominator =
                                                seq_mut.time_quantum.denominator();
                                            ui.label("Time quantum:")
                                                .on_hover_text(TIME_QUANTUM_TEXT);
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut numerator)
                                                        .range(1..=128),
                                                )
                                                .changed()
                                            {
                                                seq_mut
                                                    .time_quantum
                                                    .set_numerator(numerator)
                                                    .expect(
                                                        "UI enforces valid time-quantum numerator",
                                                    );
                                                impact.require_regeneration();
                                            };
                                            ui.label("/");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut denominator)
                                                        .range(1..=128),
                                                )
                                                .changed()
                                            {
                                                seq_mut
                                                    .time_quantum
                                                    .set_denominator(denominator)
                                                    .expect(
                                                    "UI enforces valid time-quantum denominator",
                                                );
                                                impact.require_regeneration();
                                            };
                                        });
                                    }
                                    ui.separator();
                                    {
                                        let step = seq_mut.time_quantum.step_duration();

                                        let mut t_min = seq_mut.t_min.as_secs();
                                        let mut t_max = seq_mut.t_max.as_secs();

                                        egui::CollapsingHeader::new("Sequence position").show(
                                            ui,
                                            |ui| {
                                                ui.add(
                                                    egui::Slider::new(&mut t_min, 0.0..=t_max)
                                                        .text("t_min"),
                                                );
                                                ui.add(
                                                    egui::Slider::new(
                                                        &mut t_max,
                                                        t_min..=seq_mut.loop_len.as_secs(),
                                                    )
                                                    .text("t_max"),
                                                );

                                                t_max =
                                                    t_max.clamp(t_min, seq_mut.loop_len.as_secs());

                                                if (t_min - seq_mut.t_min.as_secs()).abs()
                                                    > f64::EPSILON
                                                {
                                                    seq_mut.t_min =
                                                        step * (Time(t_min) / step).round();
                                                    impact.require_regeneration();
                                                }
                                                if (Time(t_max) - seq_mut.t_max).as_secs().abs()
                                                    > f64::EPSILON
                                                {
                                                    seq_mut.t_max =
                                                        step * (Time(t_max) / step).round();
                                                    impact.require_regeneration();
                                                }
                                            },
                                        );

                                        if !ui.ctx().wants_keyboard_input() {
                                            let (left, right, mods) = ui.ctx().input(|i| {
                                                (
                                                    i.key_pressed(egui::Key::ArrowLeft),
                                                    i.key_pressed(egui::Key::ArrowRight),
                                                    i.modifiers,
                                                )
                                            });

                                            if left || right {
                                                let step = seq_mut.time_quantum.step_duration();
                                                let dir = if left { -1.0 } else { 1.0 };

                                                let cmd = mods.command;
                                                let alt = mods.alt;

                                                let mut new_min = seq_mut.t_min.as_secs();
                                                let mut new_max = seq_mut.t_max.as_secs();
                                                let s = step.as_secs();

                                                match (cmd, alt) {
                                                    (true, false) => {
                                                        new_min =
                                                            (new_min + dir * s).clamp(0.0, new_max);
                                                    }
                                                    (false, true) => {
                                                        new_max = (new_max + dir * s).clamp(
                                                            new_min,
                                                            seq_mut.loop_len.as_secs(),
                                                        );
                                                    }
                                                    _ => {
                                                        let span = new_max - new_min;
                                                        new_min = (new_min + dir * s).clamp(
                                                            0.0,
                                                            (seq_mut.loop_len.as_secs() - span)
                                                                .max(0.0),
                                                        );
                                                        new_max = (new_min + span)
                                                            .min(seq_mut.loop_len.as_secs());
                                                    }
                                                }

                                                seq_mut.t_min = Time(new_min);
                                                seq_mut.t_max = Time(new_max);
                                                impact.require_regeneration();
                                            }
                                        }
                                    }
                                    ui.separator();
                                    {
                                        if rhythm::inclusion_section(
                                            ui,
                                            seq_mut,
                                            |ui, gens, default_val| {
                                                Self::edit_vec(
                                                    ui,
                                                    gens,
                                                    default_val,
                                                    layout_left(),
                                                );
                                            },
                                        ) {
                                            impact.require_regeneration();
                                        }
                                    }
                                    ui.separator();
                                    {
                                        if rhythm::exclusion_section(
                                            ui,
                                            seq_mut,
                                            |ui, gens, default_val| {
                                                Self::edit_vec(
                                                    ui,
                                                    gens,
                                                    default_val,
                                                    layout_left(),
                                                );
                                            },
                                        ) {
                                            impact.require_regeneration();
                                        }
                                        ui.separator();
                                        ui.horizontal(|ui| {
                                            let mut tmp_beat_offset = seq_mut.beat_offset.clone();
                                            ui.label("Groove offset:")
                                                .on_hover_text(GROOVE_OFFSET_TEXT);
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut tmp_beat_offset)
                                                        .range(0..=256),
                                                )
                                                .changed()
                                            {
                                                seq_mut.beat_offset = tmp_beat_offset;
                                                impact.require_regeneration();
                                            };
                                        });
                                        ui.horizontal(|ui| {
                                            let mut loop_len = seq_mut.loop_len.clone();
                                            ui.label("Loop length:")
                                                .on_hover_text(LOOP_LENGTH_TEXT);
                                            let slider = ui.add(
                                                egui::DragValue::new(&mut loop_len)
                                                    .range(0.0..=512.0),
                                            );
                                            if slider.changed() {
                                                loop_len = loop_len.max(Time(0.0));
                                                seq_mut.loop_len = loop_len.max(Time(0.0));
                                                seq_mut.t_max = seq_mut.t_max.min(loop_len);
                                                impact.require_regeneration();
                                            };
                                            let mut repeat = seq_mut.repeat.clone();
                                            ui.label("Repeat:").on_hover_text(REPEAT_TEXT);
                                            let slider = ui.add(
                                                egui::DragValue::new(&mut repeat).range(1..=64),
                                            );
                                            if slider.changed() {
                                                seq_mut.repeat = repeat;
                                                impact.require_regeneration();
                                            };
                                        });
                                    }
                                });
                                ui.collapsing("Harmony", |ui| {
                                    ui.checkbox(&mut seq_mut.glide, "Glide");
                                    let mut harmonise = seq_mut.harmonise;
                                    if ui
                                        .checkbox(&mut harmonise, "Harmonise")
                                        .on_hover_text(HARMONISE_TEXT)
                                        .changed()
                                    {
                                        seq_mut.harmonise = harmonise;
                                        impact.require_regeneration();
                                    }
                                    ui.separator();
                                    {
                                        ui.horizontal(|ui| {
                                            let mut tmp_tolerance = seq_mut.tolerance.clone();
                                            ui.label("Tolerance:").on_hover_text(TOLERENCE_TEXT);
                                            ui.label("<-");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut tmp_tolerance.0)
                                                        .range(-4.0..=16.0),
                                                )
                                                .changed()
                                            {
                                                seq_mut.tolerance.0 = tmp_tolerance.0;
                                                impact.require_regeneration();
                                            };
                                            ui.label(",");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut tmp_tolerance.1)
                                                        .range(-4.0..=16.0),
                                                )
                                                .changed()
                                            {
                                                seq_mut.tolerance.1 = tmp_tolerance.1;
                                                impact.require_regeneration();
                                            };
                                            ui.label("->");
                                        });
                                    }

                                    {
                                        let mut changed = false;
                                        let mut octave_changed = false;
                                        let mut interval = seq_mut.interval.clone();
                                        let mut shuffle = seq_mut.shuffle;
                                        if let Interval::RDTempered(
                                            ref mut nb_rd_steps,
                                            ref mut tones,
                                            ref mut octave,
                                        ) = interval
                                        {
                                            // octave (aesthetic - update existing notes immediately)
                                            let old_octave = *octave;
                                            ui.horizontal(|ui| {
                                                ui.label("Octave:").on_hover_text(OCTAVE_TEXT);
                                                if ui
                                                    .add(egui::Slider::new(octave, -4..=4))
                                                    .changed()
                                                {
                                                    octave_changed = true;
                                                };
                                            });
                                            if octave_changed {
                                                let octave_shift = *octave - old_octave;
                                                // Store for deferred update (after mutable borrow is dropped)
                                                impact.queue_octave_shift(
                                                    seq_mut.token,
                                                    octave_shift,
                                                );
                                                // Also update the sequence's interval
                                                if let Interval::RDTempered(
                                                    _,
                                                    _,
                                                    ref mut seq_octave,
                                                ) = seq_mut.interval
                                                {
                                                    *seq_octave = *octave;
                                                }
                                            }
                                            if !harmonise {
                                                // nb_rd_steps
                                                ui.horizontal(|ui| {
                                                    ui.label("Variation steps:")
                                                        .on_hover_text(VARIATION_STEPS_TEXT);
                                                    if ui
                                                        .add(egui::Slider::new(nb_rd_steps, 0..=16))
                                                        .changed()
                                                    {
                                                        changed = true;
                                                    };
                                                });
                                                ui.label("Variation intervals:")
                                                    .on_hover_text(VARIATION_INTERVALS_TEXT);
                                                ui.horizontal_wrapped(|ui| {
                                                    for tone in -11..=11 {
                                                        let mut selected = tones.contains(&tone);

                                                        if ui
                                                            .checkbox(
                                                                &mut selected,
                                                                tone.to_string(),
                                                            )
                                                            .changed()
                                                        {
                                                            if selected {
                                                                if !tones.contains(&tone) {
                                                                    tones.push(tone);
                                                                    tones.sort_unstable();
                                                                }
                                                            } else {
                                                                if let Some(pos) = tones
                                                                    .iter()
                                                                    .position(|&v| v == tone)
                                                                {
                                                                    tones.remove(pos);
                                                                }
                                                            }
                                                            changed = !tones.is_empty();
                                                        }
                                                    }
                                                });
                                            } else {
                                                ui.collapsing("Harmoniser", |ui| {
                                                    let mut harmoniser = seq_mut.harmoniser;
                                                    let mut changed = false;

                                                    ui.label(
                                                        RichText::new(
                                                            "Weights per interval (0..=6)",
                                                        )
                                                        .weak(),
                                                    );
                                                    ui.add_space(4.0);

                                                    Grid::new("harmoniser_grid_inverted")
                                                        .striped(true)
                                                        .num_columns(3) // interval label + 2 modes
                                                        .show(ui, |ui| {
                                                            // Header
                                                            ui.label(
                                                                RichText::new("Interval").weak(),
                                                            );
                                                            ui.label(
                                                                RichText::new("Tension").weak(),
                                                            );
                                                            ui.end_row();

                                                            // Rows: one per interval
                                                            for i in 0..=6 {
                                                                ui.label(format!("{i}"));
                                                                let r = u32_cell(
                                                                    ui,
                                                                    &mut harmoniser[i],
                                                                    0..=32,
                                                                    0,
                                                                );
                                                                if r.changed() {
                                                                    changed = true;
                                                                }
                                                                ui.end_row();
                                                            }
                                                        });

                                                    ui.horizontal_wrapped(|ui| {
                                                        if ui.button("Reset all to 16").clicked() {
                                                            harmoniser = [16; 7];
                                                            changed = true;
                                                        }
                                                        if ui.button("Reset defaults").clicked() {
                                                            harmoniser = default_harmoniser();
                                                            changed = true;
                                                        }
                                                    });

                                                    if changed {
                                                        seq_mut.harmoniser = harmoniser;
                                                    }
                                                });
                                            }
                                            ui.separator();
                                            changed |= ui
                                                .checkbox(&mut shuffle, "Shuffle")
                                                .on_hover_text(SHUFFLE_TEXT)
                                                .changed();
                                        }
                                        if changed {
                                            seq_mut.interval = interval;
                                            seq_mut.shuffle = shuffle;
                                            impact.require_regeneration();
                                        }
                                        if ui
                                            .add(
                                                egui::Slider::new(&mut seq_mut.chord, 1..=12)
                                                    .text("Chord"),
                                            )
                                            .changed()
                                        {
                                            impact.require_regeneration();
                                        }
                                        if ui
                                            .add(
                                                egui::Slider::new(&mut seq_mut.arpegio, -4.0..=4.0)
                                                    .text("Arpegio"),
                                            )
                                            .changed()
                                        {
                                            impact.require_regeneration();
                                        }
                                        if ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut seq_mut.reverse_prob,
                                                    0.0..=1.0,
                                                )
                                                .text("Reverse prob"),
                                            )
                                            .changed()
                                        {
                                            impact.require_regeneration();
                                        }
                                        if ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut seq_mut.shuffle_prob,
                                                    0.0..=1.0,
                                                )
                                                .text("Shuffle prob"),
                                            )
                                            .changed()
                                        {
                                            impact.require_regeneration();
                                        }
                                        if ui
                                            .add(egui::Checkbox::new(
                                                &mut seq_mut.random_chord,
                                                "Random skip chord notes",
                                            ))
                                            .changed()
                                        {
                                            impact.require_regeneration();
                                        }
                                    }
                                });
                                ui.collapsing("Accents", |ui| {
                                    let def = default_accents();
                                    let mut mag_disp =
                                        1.0 / seq_mut.accents.0.max(f64::MIN_POSITIVE);
                                    let mag_resp = slider_with_reset(
                                        ui,
                                        &mut mag_disp,
                                        0.01..=100.0,
                                        "Magnitude",
                                        None,
                                        1.0 / def.0.max(f64::MIN_POSITIVE),
                                        true,
                                    );
                                    if mag_resp.changed() {
                                        seq_mut.accents.0 = 1.0 / mag_disp.max(f64::MIN_POSITIVE);
                                    }

                                    let mut gens_disp: Vec<f64> = seq_mut
                                        .accents
                                        .1
                                        .iter()
                                        .map(|&x| 1.0 / x.max(f64::MIN_POSITIVE))
                                        .collect();

                                    let old = gens_disp.clone();
                                    ui.label("Generators");
                                    Self::edit_vec(ui, &mut gens_disp, 1.0, layout_left());

                                    if gens_disp != old && !gens_disp.iter().any(|&v| v == 0.0) {
                                        let restored: Vec<f64> = gens_disp
                                            .into_iter()
                                            .map(|x| 1.0 / x.max(f64::MIN_POSITIVE))
                                            .collect();
                                        seq_mut.accents.1 = restored;
                                    }
                                });
                            } else {
                                // Group-specific controls
                                match &mut track_node_mut.kind {
                                    NodeKind::Group { collapsed, .. } => {
                                        // Collapse / expand
                                        if ui
                                            .button(if *collapsed {
                                                "Uncollapse"
                                            } else {
                                                "Collapse"
                                            })
                                            .on_hover_ui(|ui| {
                                                ui.label(RichText::new(shortcut(COLLAPSE)).weak());
                                            })
                                            .clicked()
                                            || (!ui.ctx().wants_keyboard_input()
                                                && ui.input(|i| i.key_pressed(COLLAPSE)))
                                        {
                                            *collapsed = !*collapsed;
                                        }
                                    }
                                    NodeKind::Seq(_) => unreachable!(),
                                }
                            }
                        }
                    } else {
                        ui.label("Click a block to edit");
                    }

                    if let Some(mut sel) = self.selected.clone() {
                        match action {
                            Action::None => {}
                            Action::Delete => {
                                // Compute selection target BEFORE deletion
                                let (idx, parent_path, siblings) = {
                                    let idx = *sel.last().unwrap();
                                    let parent_path = &sel[..sel.len().saturating_sub(1)];
                                    let siblings = self
                                        .score
                                        .track_root
                                        .get(parent_path)
                                        .map(|p| p.child_count())
                                        .unwrap_or(0);
                                    (idx, parent_path.to_vec(), siblings)
                                };

                                // Perform deletion
                                self.del_node(&sel);

                                // Decide new selection
                                self.selected = if siblings > 1 {
                                    // There will be at least one sibling left after deletion
                                    let mut p = parent_path.clone();
                                    // Prefer next sibling at the same index (which now points to what was "next")
                                    let new_idx = if idx < siblings - 1 {
                                        idx
                                    } else {
                                        idx.saturating_sub(1)
                                    };
                                    p.push(new_idx);
                                    Some(p)
                                } else {
                                    // No siblings left: select the parent (or None if we deleted the only root child)
                                    if parent_path.is_empty() {
                                        None
                                    } else {
                                        Some(parent_path)
                                    }
                                };
                            }
                            Action::Clone => {
                                // Clone the selected node (implementation assumed: inserts right after original)
                                self.clone_node(&sel);

                                // Move selection to the new clone (original index + 1)
                                if let Some(last) = sel.last_mut() {
                                    *last += 1;
                                }
                                self.selected = Some(sel);
                            }
                            Action::MoveUp => {
                                if let Some(new_path) = self.score.swap_with_prev(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::MoveDown => {
                                if let Some(new_path) = self.score.swap_with_next(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::Wrap => {
                                if let Some(new_path) =
                                    self.score.wrap_into_group_at(&sel, "Group".into())
                                {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::Promote => {
                                if let Some(new_path) = self.score.promote_one_rank(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::Dissolve => {
                                if let Some(new_path) = self.score.dissolve_group_at(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::GroupAbove => {
                                if let Some(new_path) = self.score.move_into_prev_group(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::GroupBelow => {
                                if let Some(new_path) = self.score.move_into_next_group(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::SelectUp => {
                                if let Some(ref path) = self.selected {
                                    self.selected = self.score.prev_sibling(path, true);
                                }
                            }
                            Action::SelectDown => {
                                if let Some(ref path) = self.selected {
                                    self.selected = self.score.next_sibling(path, true);
                                }
                            }
                            Action::Parent => {
                                self.selected = self.score.parent_of(&sel);
                            }
                            Action::FirstChild => {
                                self.selected = self.score.first_child_of(&sel);
                            }
                            Action::Mute => {
                                self.score
                                    .track_root
                                    .get_mut(&sel)
                                    .as_mut()
                                    .map(|track_node| {
                                        track_node.toggle_mute();
                                        impact.require_regeneration();
                                    });
                            }
                        }
                    }
                    // Apply deferred volume change (after mutable borrow is dropped)
                    // This updates NotesGroup volumes for ALL sequences using chain products from root
                    if impact.needs_mix_update() {
                        self.update_all_volumes_from_tree();
                    }

                    // Apply deferred octave change (after mutable borrow is dropped)
                    for (token, octave_shift) in impact.drain_octave_shifts() {
                        self.update_sequence_octaves(token, octave_shift);
                    }

                    if impact.needs_regeneration() {
                        if let Some(sel) = self.selected.clone() {
                            if ui.input(|i| !i.pointer.button_down(egui::PointerButton::Primary)) {
                                // Clone the node and clear not_generate_until on all sequences within it
                                let mut node = self.score.track_root.get_mut(&sel).unwrap().clone();
                                Self::visit_sequences_mut(&mut node, &mut |seq: &mut Sequence| {
                                    seq.not_generate_until = None;
                                });
                                // Regenerate this node and all following sequences in preorder
                                self.edit_node_at(node, &sel);
                            }
                        }
                    }
                });
                self.property_panel_width = ui.available_width();
            });
    }
}
