pub mod helpers;
pub mod hover_texts;
mod navigation;
mod rhythm;
mod sections;

use egui::{RichText, ScrollArea, TextEdit};
use helpers::{ParameterBehavior, SliderParam};
use sections::{
    accents, bend, chorus, envelope, harmony, lowpass, mix, power, rhythm as rhythm_section,
    vibrato,
};

use crate::{
    app::{property_panel::navigation::navigation, GuiApp, ALL_WAVES, DRUM_WAVES},
    engine::score::{
        node_params::ParamResolution,
        sequence::Sequence,
        track_node::{GroupMode, NodeKind},
        ChorusParams, Interval, NotesGroup,
    },
    layout_left,
    shortcuts::*,
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
pub(super) struct ParameterImpact {
    needs_regeneration: bool,
    needs_mix_update: bool,
}

pub struct OverrideBinding<'a, T: Clone> {
    resolved: T,
    slot: &'a mut Option<T>,
    locked_by_parent: bool,
    active_here: bool,
}

fn group_override_section<T: Clone>(
    ui: &mut egui::Ui,
    title: &str,
    tooltip: &str,
    binding: &mut OverrideBinding<T>,
    impact: &mut ParameterImpact,
    render_controls: impl Fn(&mut egui::Ui, &mut T, &mut ParameterImpact, bool) -> bool,
) -> bool {
    let mut changed = false;
    ui.collapsing(title, |ui| {
        let mut active = binding.is_active_here();
        let response = ui
            .add_enabled_ui(!binding.is_locked(), |ui| {
                ui.checkbox(&mut active, "Override for children")
            })
            .inner;
        let response = response.on_hover_text(tooltip);
        if response.changed() {
            changed |= binding.set_override(active);
        }

        if binding.is_locked() || !binding.is_active_here() {
            let mut preview = binding.resolved().clone();
            ui.add_enabled_ui(false, |ui| {
                render_controls(ui, &mut preview, impact, false);
            });
        } else if let Some(value) = binding.value_mut() {
            changed |= render_controls(ui, value, impact, true);
        }
    });
    changed
}

impl<'a, T: Clone> OverrideBinding<'a, T> {
    pub fn new(
        resolution: ParamResolution<T>,
        slot: &'a mut Option<T>,
        depth: usize,
    ) -> Self {
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
                    if let Some(sel) = self.selected.clone() {
                        let depth = sel.len();
                        let bend_resolution = self.score.track_root.resolve_bend(&sel);
                        let vibrato_resolution = self.score.track_root.resolve_vibrato(&sel);
                        let chorus_resolution = self.score.track_root.resolve_chorus(&sel);
                        let envelope_resolution = self.score.track_root.resolve_envelope(&sel);
                        let lowpass_resolution = self.score.track_root.resolve_lowpass(&sel);
                        let power_resolution = self.score.track_root.resolve_power(&sel);
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
                            mix::show_mix_section(ui, track_node_mut, &mut impact);
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

                                let is_drum = DRUM_WAVES.contains(&seq_mut.wave_type);
                                let mut overrides_dirty = false;

                                let mut envelope_binding = OverrideBinding::new(
                                    envelope_resolution.clone(),
                                    &mut overrides.envelope,
                                    depth,
                                );
                                overrides_dirty |= envelope::show_envelope_section(
                                    ui,
                                    &mut envelope_binding,
                                    &mut impact,
                                    is_drum,
                                );

                                let mut lowpass_binding = OverrideBinding::new(
                                    lowpass_resolution.clone(),
                                    &mut overrides.lowpass,
                                    depth,
                                );
                                overrides_dirty |= lowpass::show_lowpass_section(
                                    ui,
                                    &mut lowpass_binding,
                                    &mut impact,
                                    is_drum,
                                );

                                let mut bend_binding = OverrideBinding::new(
                                    bend_resolution.clone(),
                                    &mut overrides.bend,
                                    depth,
                                );
                                overrides_dirty |= bend::show_bend_section(
                                    ui,
                                    &mut bend_binding,
                                    &mut impact,
                                );

                                let mut vibrato_binding = OverrideBinding::new(
                                    vibrato_resolution.clone(),
                                    &mut overrides.vibrato,
                                    depth,
                                );
                                overrides_dirty |= vibrato::show_vibrato_section(
                                    ui,
                                    &mut vibrato_binding,
                                    &mut impact,
                                );

                                if !DRUM_WAVES.contains(&seq_mut.wave_type) {
                                    let mut chorus_binding = OverrideBinding::new(
                                        chorus_resolution.clone(),
                                        &mut overrides.chorus,
                                        depth,
                                    );
                                    overrides_dirty |= chorus::show_chorus_section(
                                        ui,
                                        &mut chorus_binding,
                                        &mut impact,
                                    );
                                }

                                let mut power_binding = OverrideBinding::new(
                                    power_resolution.clone(),
                                    &mut overrides.power,
                                    depth,
                                );
                                overrides_dirty |= power::show_power_section(
                                    ui,
                                    &mut power_binding,
                                    &mut impact,
                                );

                                let edit_vec_generators =
                                    |ui: &mut egui::Ui, gens: &mut Vec<usize>, default_val| {
                                        GuiApp::edit_vec(ui, gens, default_val, layout_left());
                                    };
                                rhythm_section::show_rhythm_section(
                                    ui,
                                    seq_mut,
                                    &mut impact,
                                    &edit_vec_generators,
                                );
                                harmony::show_harmony_section(
                                    ui,
                                    seq_mut,
                                    &mut self.score.notes,
                                    &mut impact,
                                );
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

                                let mut envelope_binding = OverrideBinding::new(
                                    envelope_resolution.clone(),
                                    &mut overrides.envelope,
                                    depth,
                                );
                                group_overrides_dirty |= group_override_section(
                                    ui,
                                    "Envelope",
                                    "Force envelope parameters on descendants",
                                    &mut envelope_binding,
                                    &mut impact,
                                    |ui, value, impact, editable| {
                                        envelope::draw_envelope_controls(
                                            ui,
                                            value,
                                            impact,
                                            false,
                                            editable,
                                        )
                                    },
                                );

                                let mut lowpass_binding = OverrideBinding::new(
                                    lowpass_resolution.clone(),
                                    &mut overrides.lowpass,
                                    depth,
                                );
                                group_overrides_dirty |= group_override_section(
                                    ui,
                                    "Lowpass",
                                    "Apply lowpass filter settings to all children",
                                    &mut lowpass_binding,
                                    &mut impact,
                                    |ui, value, impact, editable| {
                                        lowpass::draw_lowpass_controls(ui, value, impact, editable)
                                    },
                                );

                                let mut bend_binding = OverrideBinding::new(
                                    bend_resolution.clone(),
                                    &mut overrides.bend,
                                    depth,
                                );
                                group_overrides_dirty |= group_override_section(
                                    ui,
                                    "Bend",
                                    "Override pitch bend for descendants",
                                    &mut bend_binding,
                                    &mut impact,
                                    |ui, value, impact, editable| {
                                        let changed = bend::draw_bend_controls(ui, value, impact);
                                        editable && changed
                                    },
                                );

                                let mut vibrato_binding = OverrideBinding::new(
                                    vibrato_resolution.clone(),
                                    &mut overrides.vibrato,
                                    depth,
                                );
                                group_overrides_dirty |= group_override_section(
                                    ui,
                                    "Vibrato",
                                    "Share vibrato settings with children",
                                    &mut vibrato_binding,
                                    &mut impact,
                                    |ui, value, impact, editable| {
                                        vibrato::draw_vibrato_controls(ui, value, impact, editable)
                                    },
                                );

                                let mut chorus_binding = OverrideBinding::new(
                                    chorus_resolution.clone(),
                                    &mut overrides.chorus,
                                    depth,
                                );
                                group_overrides_dirty |= group_override_section(
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
                                );

                                let mut power_binding = OverrideBinding::new(
                                    power_resolution.clone(),
                                    &mut overrides.power,
                                    depth,
                                );
                                group_overrides_dirty |= group_override_section(
                                    ui,
                                    "Power factor",
                                    "Drive power-factor distortion for child sequences",
                                    &mut power_binding,
                                    &mut impact,
                                    |ui, value, impact, editable| {
                                        power::draw_power_controls(ui, value, impact, editable)
                                    },
                                );

                                if group_overrides_dirty {
                                    needs_override_refresh = true;
                                }
                            }
                        }

                        if needs_override_refresh {
                            self.score.refresh_notes_for_path(&sel);
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
