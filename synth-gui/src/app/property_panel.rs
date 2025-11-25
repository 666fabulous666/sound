pub mod helpers;
pub mod hover_texts;
mod navigation;
mod preset_section;
mod rhythm;
mod sections;

use super::colormap::color_from_value;
use egui::{Color32, ColorImage, RichText, ScrollArea, TextEdit, TextureOptions, Vec2};
use helpers::{ParameterBehavior, SliderParam};
use preset_section::preset_section;
use rustfft::{num_complex::Complex32, FftPlanner};
use sections::{
    accents, bend, chorus, envelope, harmonics, harmony, lowpass, mix, noise, power,
    rhythm as rhythm_section, vibrato,
};

use crate::{
    app::{
        property_panel::navigation::navigation, GuiApp, SpectrogramPreview, ALL_WAVES, DRUM_WAVES,
    },
    engine::score::{
        node_params::{
            BendParams, EnvelopeParams, HarmonyParams, LowpassParams, ParamResolution, PowerParams,
            ResolvedTrackParams, RhythmParams, VibratoParams, WaveParams,
        },
        note::NoteVariant,
        sequence::Sequence,
        track_node::{GroupMode, NodeKind},
        ChorusParams, Interval, NotesGroup,
    },
    engine::waves::generate_wave,
    layout_left,
    shortcuts::*,
    time_freq::{Freq, Time},
    Token, F0,
};
use std::{collections::BTreeMap, f32::consts::TAU, sync::Arc};

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
pub(super) struct ParameterImpact {
    needs_regeneration: bool,
    needs_mix_update: bool,
}

const SPECTROGRAM_WINDOW: usize = 1024;
const SPECTROGRAM_HOP: usize = 256;
const SPECTROGRAM_MIN_DB: f32 = -80.0;

struct SpectrogramRequest {
    token: Token,
    path: Vec<usize>,
    sequence: Sequence,
    params: ResolvedTrackParams,
}

pub struct OverrideBinding<'a, T: Clone> {
    resolved: T,
    slot: &'a mut Option<T>,
    locked_by_parent: bool,
    active_here: bool,
}

#[derive(Clone)]
enum PromotionKind {
    Envelope(EnvelopeParams),
    Lowpass(LowpassParams),
    Bend(BendParams),
    Vibrato(VibratoParams),
    Chorus(ChorusParams),
    Power(PowerParams),
    Noise(crate::engine::score::node_params::NoiseParams),
    Harmonics(crate::engine::score::HarmonicsParams),
    Wave(WaveParams),
    Rhythm(RhythmParams),
    Harmony(HarmonyParams),
}

fn promote_button<T: Clone>(
    ui: &mut egui::Ui,
    binding: &OverrideBinding<T>,
    depth: usize,
) -> Option<T> {
    if depth == 0 || binding.is_locked() || !binding.is_active_here() {
        return None;
    }
    let response = ui.small_button("Promote to parent override").on_hover_text(
        "Copy this value to the parent override, clear it locally, and apply it to siblings.",
    );
    if response.clicked() {
        Some(binding.resolved().clone())
    } else {
        None
    }
}

fn group_override_section<T: Clone>(
    ui: &mut egui::Ui,
    title: &str,
    tooltip: &str,
    binding: &mut OverrideBinding<T>,
    impact: &mut ParameterImpact,
    render_controls: impl Fn(&mut egui::Ui, &mut T, &mut ParameterImpact, bool) -> bool,
) -> (bool, Option<T>) {
    let mut changed = false;
    let mut distribute_value = None;
    ui.collapsing(title, |ui| {
        let mut active = binding.is_active_here();
        ui.vertical(|ui| {
            let response = ui
                .add_enabled_ui(!binding.is_locked(), |ui| {
                    ui.checkbox(&mut active, "Override for children")
                })
                .inner;
            let response = response.on_hover_text(tooltip);
            if response.changed() {
                changed |= binding.set_override(active);
            }
            if binding.is_active_here() && !binding.is_locked() {
                if ui
                    .small_button("Distribute to children")
                    .on_hover_text("Copy this override value into every child, enable the override on those children, and remove it here.")
                    .clicked()
                {
                    distribute_value = Some(binding.resolved().clone());
                }
            }
        });

        if binding.is_locked() || !binding.is_active_here() {
            let mut preview = binding.resolved().clone();
            ui.add_enabled_ui(false, |ui| {
                render_controls(ui, &mut preview, impact, false);
            });
        } else if let Some(value) = binding.value_mut() {
            changed |= render_controls(ui, value, impact, true);
        }
    });
    (changed, distribute_value)
}

fn draw_rhythm_override_controls(
    ui: &mut egui::Ui,
    params: &mut RhythmParams,
    impact: &mut ParameterImpact,
    editable: bool,
    edit_vec_fn: &dyn Fn(&mut egui::Ui, &mut Vec<usize>, usize),
) -> bool {
    let mut temp_seq = Sequence::new(Token(0));
    params.apply_to_sequence(&mut temp_seq);
    let before = params.clone();
    ui.add_enabled_ui(editable, |ui| {
        rhythm_section::show_rhythm_section(ui, &mut temp_seq, impact, edit_vec_fn, None);
    });
    let after = RhythmParams::from_sequence(&temp_seq);
    if editable && after != before {
        *params = after;
        true
    } else {
        false
    }
}

fn draw_harmony_override_controls(
    ui: &mut egui::Ui,
    params: &mut HarmonyParams,
    impact: &mut ParameterImpact,
    editable: bool,
) -> bool {
    let mut temp_seq = Sequence::new(Token(0));
    params.apply_to_sequence(&mut temp_seq);
    let before = params.clone();
    let mut dummy_notes: BTreeMap<Token, NotesGroup> = BTreeMap::new();
    ui.add_enabled_ui(editable, |ui| {
        harmony::show_harmony_section(ui, &mut temp_seq, &mut dummy_notes, impact, None);
    });
    let after = HarmonyParams::from_sequence(&temp_seq);
    if editable && after != before {
        *params = after;
        true
    } else {
        false
    }
}

impl<'a, T: Clone> OverrideBinding<'a, T> {
    pub fn new(resolution: ParamResolution<T>, slot: &'a mut Option<T>, depth: usize) -> Self {
        let locked = resolution.locked_for_depth(depth);
        let active_here = resolution.source_depth == Some(depth);
        let resolved_value = resolution.value;
        Self {
            resolved: resolved_value,
            slot,
            locked_by_parent: locked,
            active_here,
        }
    }

    pub fn is_locked(&self) -> bool {
        self.locked_by_parent
    }

    pub fn is_active_here(&self) -> bool {
        self.active_here
    }

    pub fn resolved(&self) -> &T {
        &self.resolved
    }

    pub fn value_mut(&mut self) -> Option<&mut T> {
        if self.locked_by_parent {
            return None;
        }
        if self.slot.is_none() {
            *self.slot = Some(self.resolved.clone());
        }
        self.active_here = true;
        self.slot.as_mut()
    }

    pub fn set_override(&mut self, enabled: bool) -> bool {
        if enabled == self.active_here || self.locked_by_parent {
            return false;
        }
        if enabled {
            if self.slot.is_none() {
                *self.slot = Some(self.resolved.clone());
            }
            self.active_here = true;
        } else {
            *self.slot = None;
            self.active_here = false;
        }
        true
    }
}

impl ParameterImpact {
    fn require_regeneration(&mut self) {
        self.needs_regeneration = true;
    }

    fn require_mix_update(&mut self) {
        self.needs_mix_update = true;
    }

    fn needs_regeneration(&self) -> bool {
        self.needs_regeneration
    }

    fn needs_mix_update(&self) -> bool {
        self.needs_mix_update
    }

    fn register_behavior(&mut self, behavior: ParameterBehavior) {
        match behavior {
            ParameterBehavior::AestheticImmediate => {}
            ParameterBehavior::StructuralImmediate => self.require_regeneration(),
            ParameterBehavior::StructuralFutureOnly => {}
            ParameterBehavior::Mix => self.require_mix_update(),
        }
    }
}

pub(super) fn apply_octave_shift(
    notes: &mut std::collections::BTreeMap<Token, NotesGroup>,
    token: Token,
    shift: i32,
) {
    if shift == 0 {
        return;
    }
    if let Some(ng) = notes.get_mut(&token) {
        for note in ng.notes.iter_mut() {
            match &mut note.interval {
                Interval::Tempered(_, octave) => *octave += shift,
                Interval::RDTempered(_, _, octave) => *octave += shift,
            }
            if let Some(glide) = &mut note.glide {
                match glide {
                    Interval::Tempered(_, octave) => *octave += shift,
                    Interval::RDTempered(_, _, octave) => *octave += shift,
                }
            }
        }
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
                    let mut promotion_request: Option<PromotionKind> = None;

                    // Show preset section early and store the action
                    let preset_action = preset_section(ui, &mut self.preset_name_input);

                    if let Some(sel) = self.selected.clone() {
                        let depth = sel.len();
                        let bend_resolution = self.score.track_root.resolve_bend(&sel);
                        let vibrato_resolution = self.score.track_root.resolve_vibrato(&sel);
                        let chorus_resolution = self.score.track_root.resolve_chorus(&sel);
                        let envelope_resolution = self.score.track_root.resolve_envelope(&sel);
                        let lowpass_resolution = self.score.track_root.resolve_lowpass(&sel);
                        let power_resolution = self.score.track_root.resolve_power(&sel);
                        let noise_resolution = self.score.track_root.resolve_noise(&sel);
                        let harmonics_resolution = self.score.track_root.resolve_harmonics(&sel);
                        let wave_resolution = self.score.track_root.resolve_wave(&sel);
                        let harmony_resolution = self.score.track_root.resolve_harmony(&sel);
                        let rhythm_resolution = self.score.track_root.resolve_rhythm(&sel);
                        let pan_resolution = self.score.track_root.resolve_pan(&sel);
                        let mut needs_override_refresh = false;

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
                                        .desired_width(100.0),
                                );
                                if resp.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    ui.memory_mut(|m| m.surrender_focus(resp.id));
                                }

                                let h = track_node_mut.hue / 60.0;
                                let c = 1.0;
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
                                ui.menu_button(
                                    egui::RichText::new("   ").background_color(color),
                                    |ui| {
                                        let hue_param = SliderParam::new("Hue", 0.0..=360.0)
                                            .default(0.0)
                                            .behavior(ParameterBehavior::AestheticImmediate)
                                            .tooltip("Adjust track color");
                                        hue_param.draw(ui, &mut track_node_mut.hue, &mut impact);
                                    },
                                );
                            });
                            let pan_locked_by_parent = pan_resolution.locked_for_depth(depth);
                            let resolved_pan = pan_resolution.value;
                            mix::show_mix_section(
                                ui,
                                track_node_mut,
                                &mut impact,
                                pan_locked_by_parent,
                                resolved_pan,
                            );
                            let weight_param = SliderParam::new("Selection weight", 0.0..=32.0)
                                .default(1.0)
                                .behavior(ParameterBehavior::StructuralImmediate)
                                .tooltip("Used when the parent group operates in OR mode");
                            weight_param.draw(ui, &mut track_node_mut.or_weight, &mut impact);

                            ui.separator();

                            // === TYPE-SPECIFIC SECTION ===
                            if let (NodeKind::Seq(seq_mut), overrides) =
                                (&mut track_node_mut.kind, &mut track_node_mut.overrides)
                            {
                                ui.separator();
                                let wave_locked = wave_resolution.locked_for_depth(depth);
                                let mut effective_wave = if wave_locked {
                                    wave_resolution.value.wave
                                } else {
                                    seq_mut.wave_type
                                };
                                ui.horizontal(|ui| {
                                    ui.label("Wave:");
                                    ui.add_enabled_ui(!wave_locked, |ui| {
                                        let mut w_choice = effective_wave;
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
                                        if w_choice != effective_wave {
                                            effective_wave = w_choice;
                                        }
                                    });
                                });
                                if !wave_locked && effective_wave != seq_mut.wave_type {
                                    seq_mut.wave_type = effective_wave;
                                    if let Some(ng) = self.score.notes.get_mut(&seq_mut.token) {
                                        ng.wave_type = seq_mut.wave_type;
                                    }
                                }
                                if !wave_locked && depth > 0 {
                                    if ui
                                        .small_button("Promote wave to parent override")
                                        .on_hover_text(
                                            "Copy this wave to the parent override and clear it here",
                                        )
                                        .clicked()
                                    {
                                        promotion_request = Some(PromotionKind::Wave(WaveParams {
                                            wave: effective_wave,
                                        }));
                                    }
                                }

                                ui.horizontal(|ui| {
                                    ui.label("Phase mode:");
                                    let mut variant = seq_mut.note_variant;
                                    egui::ComboBox::from_id_salt("note_variant_combo")
                                        .selected_text(match variant {
                                            NoteVariant::PureTime => "Pure time",
                                            NoteVariant::PhaseTracked => "Phase tracked",
                                        })
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut variant,
                                                NoteVariant::PureTime,
                                                "Pure time",
                                            );
                                            ui.selectable_value(
                                                &mut variant,
                                                NoteVariant::PhaseTracked,
                                                "Phase tracked",
                                            );
                                        });
                                    if variant != seq_mut.note_variant {
                                        seq_mut.note_variant = variant;
                                        if let Some(ng) = self.score.notes.get_mut(&seq_mut.token) {
                                            for n in ng.notes.iter_mut() {
                                                n.variant = variant;
                                            }
                                        }
                                    }
                                });

                                let is_drum = DRUM_WAVES.contains(&effective_wave);
                                let mut overrides_dirty = false;

                                let mut envelope_binding = OverrideBinding::new(
                                    envelope_resolution.clone(),
                                    &mut overrides.envelope,
                                    depth,
                                );
                                let mut env_promote = None;
                                overrides_dirty |= envelope::show_envelope_section(
                                    ui,
                                    &mut envelope_binding,
                                    &mut impact,
                                    is_drum,
                                    depth,
                                    &mut env_promote,
                                );
                                if let Some(value) = env_promote {
                                    promotion_request = Some(PromotionKind::Envelope(value));
                                }

                                let mut lowpass_binding = OverrideBinding::new(
                                    lowpass_resolution.clone(),
                                    &mut overrides.lowpass,
                                    depth,
                                );
                                let mut lp_promote = None;
                                overrides_dirty |= lowpass::show_lowpass_section(
                                    ui,
                                    &mut lowpass_binding,
                                    &mut impact,
                                    is_drum,
                                    depth,
                                    &mut lp_promote,
                                );
                                if let Some(value) = lp_promote {
                                    promotion_request = Some(PromotionKind::Lowpass(value));
                                }

                                let mut bend_binding = OverrideBinding::new(
                                    bend_resolution.clone(),
                                    &mut overrides.bend,
                                    depth,
                                );
                                let mut bend_promote = None;
                                overrides_dirty |= bend::show_bend_section(
                                    ui,
                                    &mut bend_binding,
                                    &mut impact,
                                    depth,
                                    &mut bend_promote,
                                );
                                if let Some(value) = bend_promote {
                                    promotion_request = Some(PromotionKind::Bend(value));
                                }

                                let mut vibrato_binding = OverrideBinding::new(
                                    vibrato_resolution.clone(),
                                    &mut overrides.vibrato,
                                    depth,
                                );
                                let mut vibrato_promote = None;
                                overrides_dirty |= vibrato::show_vibrato_section(
                                    ui,
                                    &mut vibrato_binding,
                                    &mut impact,
                                    depth,
                                    &mut vibrato_promote,
                                );
                                if let Some(value) = vibrato_promote {
                                    promotion_request = Some(PromotionKind::Vibrato(value));
                                }

                                let mut harmonics_binding = OverrideBinding::new(
                                    harmonics_resolution.clone(),
                                    &mut overrides.harmonics,
                                    depth,
                                );
                                let mut harmonics_promote = None;
                                overrides_dirty |= harmonics::show_harmonics_section(
                                    ui,
                                    &mut harmonics_binding,
                                    &mut impact,
                                    depth,
                                    &mut harmonics_promote,
                                );
                                if let Some(value) = harmonics_promote {
                                    promotion_request = Some(PromotionKind::Harmonics(value));
                                }

                                if !DRUM_WAVES.contains(&effective_wave) {
                                    let mut chorus_binding = OverrideBinding::new(
                                        chorus_resolution.clone(),
                                        &mut overrides.chorus,
                                        depth,
                                    );
                                    let mut chorus_promote = None;
                                    overrides_dirty |= chorus::show_chorus_section(
                                        ui,
                                        &mut chorus_binding,
                                        &mut impact,
                                        depth,
                                        &mut chorus_promote,
                                    );
                                    if let Some(value) = chorus_promote {
                                        promotion_request = Some(PromotionKind::Chorus(value));
                                    }
                                }

                                let mut power_binding = OverrideBinding::new(
                                    power_resolution.clone(),
                                    &mut overrides.power,
                                    depth,
                                );
                                let mut power_promote = None;
                                overrides_dirty |= power::show_power_section(
                                    ui,
                                    &mut power_binding,
                                    &mut impact,
                                    depth,
                                    &mut power_promote,
                                );
                                if let Some(value) = power_promote {
                                    promotion_request = Some(PromotionKind::Power(value));
                                }

                                let mut noise_binding = OverrideBinding::new(
                                    noise_resolution.clone(),
                                    &mut overrides.noise,
                                    depth,
                                );
                                let mut noise_promote = None;
                                overrides_dirty |= noise::show_noise_section(
                                    ui,
                                    &mut noise_binding,
                                    &mut impact,
                                    depth,
                                    &mut noise_promote,
                                );
                                if let Some(value) = noise_promote {
                                    promotion_request = Some(PromotionKind::Noise(value));
                                }

                                let edit_vec_generators =
                                    |ui: &mut egui::Ui, gens: &mut Vec<usize>, default_val| {
                                        GuiApp::edit_vec(ui, gens, default_val, layout_left());
                                    };
                                let rhythm_locked = rhythm_resolution.locked_for_depth(depth);
                                if rhythm_locked {
                                    let mut preview = seq_mut.clone();
                                    rhythm_resolution
                                        .value
                                        .clone()
                                        .apply_to_sequence(&mut preview);
                                    ui.add_enabled_ui(false, |ui| {
                                        rhythm_section::show_rhythm_section(
                                            ui,
                                            &mut preview,
                                            &mut impact,
                                            &edit_vec_generators,
                                            None,
                                        );
                                    });
                                } else {
                                    let mut promote_clicked = false;
                                    rhythm_section::show_rhythm_section(
                                        ui,
                                        seq_mut,
                                        &mut impact,
                                        &edit_vec_generators,
                                        Some(&mut || {
                                            promote_clicked = true;
                                        }),
                                    );
                                    if promote_clicked && depth > 0 {
                                        promotion_request = Some(PromotionKind::Rhythm(
                                            RhythmParams::from_sequence(seq_mut),
                                        ));
                                    }
                                }

                                let harmony_locked = harmony_resolution.locked_for_depth(depth);
                                if harmony_locked {
                                    let mut preview = seq_mut.clone();
                                    harmony_resolution
                                        .value
                                        .clone()
                                        .apply_to_sequence(&mut preview);
                                    ui.add_enabled_ui(false, |ui| {
                                        harmony::show_harmony_section(
                                            ui,
                                            &mut preview,
                                            &mut self.score.notes,
                                            &mut impact,
                                            None,
                                        );
                                    });
                                } else {
                                    let mut promote_clicked = false;
                                    harmony::show_harmony_section(
                                        ui,
                                        seq_mut,
                                        &mut self.score.notes,
                                        &mut impact,
                                        Some(&mut || {
                                            promote_clicked = true;
                                        }),
                                    );
                                    if promote_clicked && depth > 0 {
                                        promotion_request = Some(PromotionKind::Harmony(
                                            HarmonyParams::from_sequence(seq_mut),
                                        ));
                                    }
                                }
                                accents::show_accents_section(
                                    ui,
                                    seq_mut,
                                    &mut impact,
                                    |ui, gens, default_val| {
                                        GuiApp::edit_vec(ui, gens, default_val, layout_left());
                                    },
                                );

                                if overrides_dirty {
                                    needs_override_refresh = true;
                                }

                                if impact.needs_regeneration() || impact.needs_mix_update() {
                                    self.spectrogram_render_requested = true;
                                }
                            } else if let (
                                NodeKind::Group {
                                    collapsed,
                                    mode,
                                    children,
                                    ..
                                },
                                overrides,
                            ) = (&mut track_node_mut.kind, &mut track_node_mut.overrides)
                            {
                                ui.horizontal(|ui| {
                                    ui.label("Group behavior:");
                                    let previous = *mode;
                                    egui::ComboBox::from_id_salt("group_mode_combo")
                                        .selected_text(match mode {
                                            GroupMode::And => "Play all children",
                                            GroupMode::Or => "Pick one child",
                                        })
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                mode,
                                                GroupMode::And,
                                                "Play all children",
                                            );
                                            ui.selectable_value(
                                                mode,
                                                GroupMode::Or,
                                                "Pick one child (OR mode)",
                                            );
                                        });
                                    if *mode != previous {
                                        impact.register_behavior(
                                            ParameterBehavior::StructuralImmediate,
                                        );
                                    }
                                });

                                if matches!(*mode, GroupMode::Or) {
                                    ui.separator();
                                    ui.label("Child weights (used in OR mode):");
                                    for (idx, child) in children.iter_mut().enumerate() {
                                        let label = if child.name().is_empty() {
                                            format!("Child {}", idx + 1)
                                        } else {
                                            child.name().to_owned()
                                        };
                                        let response = ui.add(
                                            egui::Slider::new(&mut child.or_weight, 0.0..=32.0)
                                                .text(label),
                                        );
                                        if response.changed() {
                                            impact.register_behavior(
                                                ParameterBehavior::StructuralImmediate,
                                            );
                                        }
                                    }
                                }

                                if ui
                                    .button(if *collapsed { "Uncollapse" } else { "Collapse" })
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new(shortcut(COLLAPSE)).weak());
                                    })
                                    .clicked()
                                    || (!ui.ctx().wants_keyboard_input()
                                        && ui.input(|i| i.key_pressed(COLLAPSE)))
                                {
                                    *collapsed = !*collapsed;
                                }

                                ui.separator();
                                ui.heading("Child overrides");
                                let mut group_overrides_dirty = false;

                                let (envelope_dirty, distribute_envelope) = {
                                    let mut envelope_binding = OverrideBinding::new(
                                        envelope_resolution.clone(),
                                        &mut overrides.envelope,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Envelope",
                                        "Force envelope parameters on descendants",
                                        &mut envelope_binding,
                                        &mut impact,
                                        |ui, value, impact, editable| {
                                            envelope::draw_envelope_controls(
                                                ui, value, impact, false, editable,
                                            )
                                        },
                                    )
                                };
                                let envelope_distributed = distribute_envelope.is_some();
                                if let Some(value) = distribute_envelope {
                                    for child in children.iter_mut() {
                                        child.overrides.envelope = Some(value.clone());
                                    }
                                    overrides.envelope = None;
                                }
                                group_overrides_dirty |= envelope_dirty || envelope_distributed;

                                let (lowpass_dirty, distribute_lowpass) = {
                                    let mut lowpass_binding = OverrideBinding::new(
                                        lowpass_resolution.clone(),
                                        &mut overrides.lowpass,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Lowpass",
                                        "Apply lowpass filter settings to all children",
                                        &mut lowpass_binding,
                                        &mut impact,
                                        |ui, value, impact, editable| {
                                            lowpass::draw_lowpass_controls(
                                                ui, value, impact, editable,
                                            )
                                        },
                                    )
                                };
                                let lowpass_distributed = distribute_lowpass.is_some();
                                if let Some(value) = distribute_lowpass {
                                    for child in children.iter_mut() {
                                        child.overrides.lowpass = Some(value.clone());
                                    }
                                    overrides.lowpass = None;
                                }
                                group_overrides_dirty |=
                                    lowpass_dirty || lowpass_distributed;

                                let (bend_dirty, distribute_bend) = {
                                    let mut bend_binding = OverrideBinding::new(
                                        bend_resolution.clone(),
                                        &mut overrides.bend,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Bend",
                                        "Override pitch bend for descendants",
                                        &mut bend_binding,
                                        &mut impact,
                                        |ui, value, impact, editable| {
                                            let changed =
                                                bend::draw_bend_controls(ui, value, impact);
                                            editable && changed
                                        },
                                    )
                                };
                                let bend_distributed = distribute_bend.is_some();
                                if let Some(value) = distribute_bend {
                                    for child in children.iter_mut() {
                                        child.overrides.bend = Some(value.clone());
                                    }
                                    overrides.bend = None;
                                }
                                group_overrides_dirty |= bend_dirty || bend_distributed;

                                let (vibrato_dirty, distribute_vibrato) = {
                                    let mut vibrato_binding = OverrideBinding::new(
                                        vibrato_resolution.clone(),
                                        &mut overrides.vibrato,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Vibrato",
                                        "Share vibrato settings with children",
                                        &mut vibrato_binding,
                                        &mut impact,
                                        |ui, value, impact, editable| {
                                            vibrato::draw_vibrato_controls(
                                                ui, value, impact, editable,
                                            )
                                        },
                                    )
                                };
                                let vibrato_distributed = distribute_vibrato.is_some();
                                if let Some(value) = distribute_vibrato {
                                    for child in children.iter_mut() {
                                        child.overrides.vibrato = Some(value.clone());
                                    }
                                    overrides.vibrato = None;
                                }
                                group_overrides_dirty |=
                                    vibrato_dirty || vibrato_distributed;

                                let (chorus_dirty, distribute_chorus) = {
                                    let mut chorus_binding = OverrideBinding::new(
                                        chorus_resolution.clone(),
                                        &mut overrides.chorus,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Chorus",
                                        "Override chorus detune for children",
                                        &mut chorus_binding,
                                        &mut impact,
                                        |ui, value, impact, editable| {
                                            chorus::draw_chorus_controls(
                                                ui,
                                                value,
                                                impact,
                                                &ChorusParams::default(),
                                                editable,
                                            )
                                        },
                                    )
                                };
                                let chorus_distributed = distribute_chorus.is_some();
                                if let Some(value) = distribute_chorus {
                                    for child in children.iter_mut() {
                                        child.overrides.chorus = Some(value.clone());
                                    }
                                    overrides.chorus = None;
                                }
                                group_overrides_dirty |=
                                    chorus_dirty || chorus_distributed;

                                let (power_dirty, distribute_power) = {
                                    let mut power_binding = OverrideBinding::new(
                                        power_resolution.clone(),
                                        &mut overrides.power,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Power factor",
                                        "Drive power-factor distortion for child sequences",
                                        &mut power_binding,
                                        &mut impact,
                                        |ui, value, impact, editable| {
                                            power::draw_power_controls(
                                                ui, value, impact, editable,
                                            )
                                        },
                                    )
                                };
                                let power_distributed = distribute_power.is_some();
                                if let Some(value) = distribute_power {
                                    for child in children.iter_mut() {
                                        child.overrides.power = Some(value.clone());
                                    }
                                    overrides.power = None;
                                }
                                group_overrides_dirty |= power_dirty || power_distributed;

                                let (noise_dirty, distribute_noise) = {
                                    let mut noise_binding = OverrideBinding::new(
                                        noise_resolution.clone(),
                                        &mut overrides.noise,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Noise",
                                        "Apply noise (random signal attenuation) to child sequences",
                                        &mut noise_binding,
                                        &mut impact,
                                        |ui, value, impact, editable| {
                                            noise::draw_noise_controls(
                                                ui, value, impact, editable,
                                            )
                                        },
                                    )
                                };
                                let noise_distributed = distribute_noise.is_some();
                                if let Some(value) = distribute_noise {
                                    for child in children.iter_mut() {
                                        child.overrides.noise = Some(value.clone());
                                    }
                                    overrides.noise = None;
                                }
                                group_overrides_dirty |= noise_dirty || noise_distributed;

                                let (harmonics_dirty, distribute_harmonics) = {
                                    let mut harmonics_binding = OverrideBinding::new(
                                        harmonics_resolution.clone(),
                                        &mut overrides.harmonics,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Harmonics",
                                        "Add harmonics/subharmonics for children",
                                        &mut harmonics_binding,
                                        &mut impact,
                                        |ui, value, impact, editable| {
                                            harmonics::draw_harmonics_controls(
                                                ui, value, impact, editable,
                                            )
                                        },
                                    )
                                };
                                let harmonics_distributed = distribute_harmonics.is_some();
                                if let Some(value) = distribute_harmonics {
                                    for child in children.iter_mut() {
                                        child.overrides.harmonics = Some(value.clone());
                                    }
                                    overrides.harmonics = None;
                                }
                                group_overrides_dirty |=
                                    harmonics_dirty || harmonics_distributed;

                                let (wave_dirty, distribute_wave) = {
                                    let mut wave_binding = OverrideBinding::new(
                                        wave_resolution.clone(),
                                        &mut overrides.wave,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Wave type",
                                        "Force all descendants to use this oscillator",
                                        &mut wave_binding,
                                        &mut impact,
                                        |ui, value, _impact, editable| {
                                            let mut changed = false;
                                            ui.add_enabled_ui(editable, |ui| {
                                                let mut choice = value.wave;
                                                egui::ComboBox::from_id_salt(
                                                    "group_wave_type_combo",
                                                )
                                                .selected_text((&choice).to_string())
                                                .show_ui(ui, |ui| {
                                                    for var in ALL_WAVES.iter() {
                                                        ui.selectable_value(
                                                            &mut choice,
                                                            *var,
                                                            (&var).to_string(),
                                                        );
                                                    }
                                                });
                                                if choice != value.wave {
                                                    value.wave = choice;
                                                    changed = true;
                                                }
                                            });
                                            changed
                                        },
                                    )
                                };
                                let wave_distributed = distribute_wave.is_some();
                                if let Some(value) = distribute_wave {
                                    for child in children.iter_mut() {
                                        match &mut child.kind {
                                            NodeKind::Seq(seq) => {
                                                seq.wave_type = value.wave;
                                            }
                                            NodeKind::Group { .. } => {
                                                child.overrides.wave = Some(value.clone());
                                            }
                                        }
                                    }
                                    overrides.wave = None;
                                }
                                group_overrides_dirty |= wave_dirty || wave_distributed;

                                let edit_vec_generators =
                                    |ui: &mut egui::Ui, gens: &mut Vec<usize>, default_val| {
                                        GuiApp::edit_vec(ui, gens, default_val, layout_left());
                                    };

                                let (rhythm_dirty, distribute_rhythm) = {
                                    let mut rhythm_binding = OverrideBinding::new(
                                        rhythm_resolution.clone(),
                                        &mut overrides.rhythm,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Rhythm",
                                        "Override timing (windows, repeats, offsets) for descendants",
                                        &mut rhythm_binding,
                                        &mut impact,
                                        |ui, value, impact, editable| {
                                            draw_rhythm_override_controls(
                                                ui,
                                                value,
                                                impact,
                                                editable,
                                                &edit_vec_generators,
                                            )
                                        },
                                    )
                                };
                                let rhythm_distributed = distribute_rhythm.is_some();
                                if let Some(value) = distribute_rhythm {
                                    for child in children.iter_mut() {
                                        child.overrides.rhythm = Some(value.clone());
                                    }
                                    overrides.rhythm = None;
                                }
                                group_overrides_dirty |=
                                    rhythm_dirty || rhythm_distributed;

                                let (harmony_dirty, distribute_harmony) = {
                                    let mut harmony_binding = OverrideBinding::new(
                                        harmony_resolution.clone(),
                                        &mut overrides.harmony,
                                        depth,
                                    );
                                    group_override_section(
                                        ui,
                                        "Harmony",
                                        "Share harmony/shuffle settings with children",
                                        &mut harmony_binding,
                                        &mut impact,
                                        |ui, value, impact, editable| {
                                            draw_harmony_override_controls(
                                                ui, value, impact, editable,
                                            )
                                        },
                                    )
                                };
                                let harmony_distributed = distribute_harmony.is_some();
                                if let Some(value) = distribute_harmony {
                                    for child in children.iter_mut() {
                                        child.overrides.harmony = Some(value.clone());
                                    }
                                    overrides.harmony = None;
                                }
                                group_overrides_dirty |=
                                    harmony_dirty || harmony_distributed;

                            if group_overrides_dirty {
                                needs_override_refresh = true;
                            }
                        }
                    }

                        if needs_override_refresh {
                            self.score.refresh_notes_for_path(&sel);
                            self.score
                                .shared_notes
                                .store(Arc::new(self.score.notes.clone()));
                        }
                    } else {
                        ui.label("Click a block to edit");
                    }

                    // Handle preset actions
                    if preset_action.save && !preset_action.preset_name.is_empty() {
                        self.save_preset(preset_action.preset_name.clone());
                    }
                    if preset_action.load {
                        self.load_preset(ctx);
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
                    if let Some(promotion) = promotion_request.clone() {
                        if let Some(sel) = self.selected.clone() {
                            if !sel.is_empty() {
                                let parent_path = &sel[..sel.len() - 1];
                                match promotion.clone() {
                                    PromotionKind::Envelope(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.envelope = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.envelope = None;
                                        }
                                    }
                                    PromotionKind::Lowpass(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.lowpass = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.lowpass = None;
                                        }
                                    }
                                    PromotionKind::Bend(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.bend = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.bend = None;
                                        }
                                    }
                                    PromotionKind::Vibrato(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.vibrato = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.vibrato = None;
                                        }
                                    }
                                    PromotionKind::Chorus(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.chorus = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.chorus = None;
                                        }
                                    }
                                    PromotionKind::Power(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.power = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.power = None;
                                        }
                                    }
                                    PromotionKind::Noise(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.noise = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.noise = None;
                                        }
                                    }
                                    PromotionKind::Harmonics(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.harmonics = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.harmonics = None;
                                        }
                                    }
                                    PromotionKind::Wave(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.wave = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.wave = None;
                                            if let NodeKind::Seq(seq) = &mut child.kind {
                                                seq.wave_type = v.wave;
                                            }
                                        }
                                    }
                                    PromotionKind::Rhythm(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.rhythm = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.rhythm = None;
                                        }
                                    }
                                    PromotionKind::Harmony(v) => {
                                        if let Some(parent) =
                                            self.score.track_root.get_mut(parent_path)
                                        {
                                            parent.overrides.harmony = Some(v.clone());
                                        }
                                        if let Some(child) = self.score.track_root.get_mut(&sel) {
                                            child.overrides.harmony = None;
                                        }
                                    }
                                }
                                self.score.refresh_notes_for_path(parent_path);
                                self.score
                                    .shared_notes
                                    .store(Arc::new(self.score.notes.clone()));
                                self.spectrogram_render_requested = true;
                                self.update_all_mix_from_tree();
                            }
                        }
                    }
                    // Apply deferred volume change (after mutable borrow is dropped)
                    // This updates NotesGroup mix for ALL sequences using chain products from root
                    if impact.needs_mix_update() {
                        self.update_all_mix_from_tree();
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

    pub fn update_spectrogram_preview_if_needed(
        &mut self,
        ctx: &egui::Context,
        background: Color32,
    ) {
        let Some(sel) = self.selected.clone() else {
            return;
        };
        let Some(node) = self.score.track_root.get(&sel) else {
            return;
        };
        let sequence = match &node.kind {
            NodeKind::Seq(seq) => seq.clone(),
            NodeKind::Group { .. } => return,
        };
        let params = self.score.track_root.resolved_params_for_path(&sel);
        let needs_render = self.spectrogram_render_requested
            || self
                .spectrogram_previews
                .get(&sequence.token)
                .map_or(true, |prev| {
                    prev.sequence != sequence
                        || prev.params != params
                        || prev.background != background
                        || prev.log_freq != self.spectrogram_log_freq
                });
        if !needs_render {
            return;
        }
        let request = SpectrogramRequest {
            token: sequence.token,
            path: sel,
            sequence,
            params,
        };
        self.generate_spectrogram_preview(ctx, request, background);
        self.spectrogram_render_requested = false;
    }

    fn generate_spectrogram_preview(
        &mut self,
        ctx: &egui::Context,
        request: SpectrogramRequest,
        background: Color32,
    ) {
        if let Some(image) = self.build_spectrogram_image(&request, background) {
            if let Some(existing) = self.spectrogram_previews.get_mut(&request.token) {
                existing.texture.set(image.clone(), TextureOptions::LINEAR);
                existing.size = image.size;
                existing.sequence = request.sequence.clone();
                existing.params = request.params.clone();
                existing.background = background;
                existing.log_freq = self.spectrogram_log_freq;
            } else {
                let size = image.size;
                let texture = ctx.load_texture(
                    format!("spectrogram_preview_{}", request.token.0),
                    image,
                    TextureOptions::LINEAR,
                );
                self.spectrogram_previews.insert(
                    request.token,
                    SpectrogramPreview {
                        texture,
                        size,
                        sequence: request.sequence.clone(),
                        params: request.params.clone(),
                        background,
                        log_freq: self.spectrogram_log_freq,
                    },
                );
            }
        }
    }

    pub fn spectrogram_panel(&mut self, ctx: &egui::Context) {
        if !self.show_spectrogram_panel {
            return;
        }
        egui::TopBottomPanel::bottom("spectrogram_panel")
            .resizable(true)
            .default_height(240.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Spectrogram").strong());
                    ui.add_space(8.0);
                    if ui
                        .checkbox(&mut self.spectrogram_log_freq, "Log freq scale")
                        .on_hover_text("Display spectrogram frequencies on a logarithmic axis")
                        .changed()
                    {
                        self.spectrogram_render_requested = true;
                    }
                });
                ui.separator();
                let preview = self
                    .selected
                    .as_ref()
                    .and_then(|sel| self.score.track_root.get(sel))
                    .and_then(|node| node.as_seq().map(|seq| seq.token))
                    .and_then(|token| self.spectrogram_previews.get(&token));
                if let Some(preview) = preview {
                    let width = ui.available_width().max(64.0);
                    let height = ui.available_height().max(120.0);
                    ui.image((preview.texture.id(), Vec2::new(width, height)));
                } else {
                    ui.label("Select a sequence to preview its spectrum.");
                }
            });
    }

    fn build_spectrogram_image(
        &self,
        request: &SpectrogramRequest,
        background: Color32,
    ) -> Option<ColorImage> {
        let samples = self.render_preview_samples(request)?;
        let (min_freq, max_freq) = self.spectrogram_freq_bounds();
        Some(samples_to_color_image(
            &samples,
            self.sample_rate as f32,
            min_freq,
            max_freq,
            background,
            self.spectrogram_log_freq,
        ))
    }

    fn render_preview_samples(&self, request: &SpectrogramRequest) -> Option<Vec<f32>> {
        let sample_rate = self.sample_rate.max(1.0);
        let sample_count = sample_rate.round() as usize;
        if sample_count == 0 {
            return None;
        }
        let harmony_params = if request.params.has_harmony_override() {
            request.params.harmony.clone()
        } else {
            HarmonyParams::from_sequence(&request.sequence)
        };
        let octave = interval_octave(&harmony_params.interval);
        let wave = if request.params.has_wave_override() {
            request.params.wave.wave
        } else {
            request.sequence.wave_type
        };
        let note_interval = Interval::Tempered(0, octave);
        let frequency = Freq(F0.as_hz() * note_interval.compute());
        let duration = Time(1.0);
        let mut memory = [0.0; 10];
        let pad_samples = sample_count;
        let mut samples = Vec::with_capacity(sample_count + 2 * pad_samples + SPECTROGRAM_WINDOW);
        samples.resize(pad_samples, 0.0);
        let track_volume = self.cumulative_volume_for_path(&request.path);
        let note_volume = preview_note_volume(&request.sequence, &request.params.envelope);
        let volume_scale = sanitize_volume(track_volume) * note_volume.abs() * 0.1;
        let sample_rate_freq = Freq(sample_rate);
        for i in 0..sample_count {
            let t = Time(i as f64 / sample_rate);
            let raw = generate_wave(
                &wave,
                frequency,
                None,
                t,
                duration,
                (
                    request.params.envelope.attack,
                    request.params.envelope.decay,
                ),
                &request.params.lowpass.cutoff,
                (request.params.bend.magnitude, request.params.bend.speed),
                (
                    request.params.vibrato.magnitude,
                    request.params.vibrato.frequency,
                ),
                &request.params.chorus,
                &request.params.harmonics,
                &request.params.power.power,
                &request.params.noise.noise,
                request.params.lowpass.enabled,
                request.params.lowpass.filter_type,
                request.params.lowpass.order,
                &mut memory,
                sample_rate_freq,
                t,
            );
            samples.push((raw * volume_scale) as f32);
        }
        samples.resize(samples.len().saturating_add(pad_samples), 0.0);
        let target_len = samples.len().saturating_add(SPECTROGRAM_WINDOW);
        samples.resize(target_len, 0.0);
        Some(samples)
    }

    fn spectrogram_freq_bounds(&self) -> (f32, f32) {
        (20.0, 20_000.0)
    }

    fn cumulative_volume_for_path(&self, path: &[usize]) -> f64 {
        let mut cumulative = sanitize_volume(self.score.track_root.volume());
        let mut current = &self.score.track_root;
        for &idx in path {
            match &current.kind {
                NodeKind::Group { children, .. } => {
                    if let Some(child) = children.get(idx) {
                        cumulative = sanitize_volume(cumulative * child.volume());
                        current = child;
                    } else {
                        break;
                    }
                }
                NodeKind::Seq(_) => {
                    cumulative = sanitize_volume(cumulative * current.volume());
                    break;
                }
            }
        }
        cumulative
    }
}

fn preview_note_volume(sequence: &Sequence, envelope: &EnvelopeParams) -> f64 {
    let base = sequence.accents.0;
    let extras: f64 = sequence.accents.1.iter().copied().sum();
    let denominator = if base <= 0.0 { 1.0 } else { base };
    let normalization = if envelope.normalization.is_finite() && envelope.normalization > 0.0 {
        envelope.normalization
    } else {
        1.0
    };
    ((base + 0.5 * extras) / denominator) / normalization
}

fn interval_octave(interval: &Interval) -> i32 {
    match interval {
        Interval::Tempered(_, octave) => *octave,
        Interval::RDTempered(_, _, octave) => *octave,
    }
}

fn samples_to_color_image(
    samples: &[f32],
    sample_rate: f32,
    min_freq: f32,
    max_freq: f32,
    background: Color32,
    log_freq: bool,
) -> ColorImage {
    let mut window = Vec::with_capacity(SPECTROGRAM_WINDOW);
    for i in 0..SPECTROGRAM_WINDOW {
        let phase = TAU * i as f32 / SPECTROGRAM_WINDOW as f32;
        window.push(0.5 - 0.5 * phase.cos());
    }
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(SPECTROGRAM_WINDOW);
    let height = SPECTROGRAM_WINDOW / 2;
    let mut columns: Vec<Vec<f32>> = Vec::new();
    let mut offset = 0usize;
    while offset + SPECTROGRAM_WINDOW <= samples.len() {
        let mut buffer: Vec<Complex32> = (0..SPECTROGRAM_WINDOW)
            .map(|i| Complex32::new(samples[offset + i] * window[i], 0.0))
            .collect();
        fft.process(&mut buffer);
        columns.push(
            buffer[..height]
                .iter()
                .map(|c| magnitude_to_value(c.norm() / SPECTROGRAM_WINDOW as f32))
                .collect(),
        );
        offset += SPECTROGRAM_HOP;
    }
    if columns.is_empty() {
        columns.push(vec![0.0; height]);
    }
    let width = columns.len();
    let bin_hz = (sample_rate / SPECTROGRAM_WINDOW as f32).max(1e-6);
    let min_bin = (min_freq / bin_hz).floor().clamp(0.0, (height - 1) as f32) as usize;
    let max_bin = (max_freq / bin_hz)
        .ceil()
        .clamp(min_bin as f32 + 1.0, height as f32) as usize;
    let visible_bins = (max_bin.saturating_sub(min_bin)).max(1);
    let mut image = ColorImage::new(
        [width, visible_bins],
        vec![background; width * visible_bins],
    );
    let denom = (visible_bins - 1).max(1) as f32;
    for (x, column) in columns.iter().enumerate() {
        for output_idx in 0..visible_bins {
            let value = if log_freq {
                let min_f = min_freq.max(1.0);
                let max_f = max_freq.max(min_f + 1.0);
                let ratio = max_f / min_f;
                let frac = output_idx as f32 / denom;
                let freq = min_f * ratio.powf(frac);
                let target = (freq / bin_hz).clamp(min_bin as f32, (max_bin - 1) as f32);
                let lower = target.floor() as usize;
                let upper = (lower + 1).min(column.len().saturating_sub(1));
                let t = target - lower as f32;
                let low_val = column.get(lower).copied().unwrap_or(0.0);
                let high_val = column.get(upper).copied().unwrap_or(low_val);
                low_val + (high_val - low_val) * t
            } else {
                column.get(min_bin + output_idx).copied().unwrap_or(0.0)
            };
            let y = visible_bins - 1 - output_idx.min(visible_bins - 1);
            image.pixels[y * width + x] = color_from_value(value, background);
        }
    }
    image
}

fn magnitude_to_value(magnitude: f32) -> f32 {
    let magnitude = magnitude.max(1e-8);
    let db = 20.0 * magnitude.log10();
    ((db - SPECTROGRAM_MIN_DB) / (0.0 - SPECTROGRAM_MIN_DB)).clamp(0.0, 1.0)
}

fn sanitize_volume(value: f64) -> f64 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}
