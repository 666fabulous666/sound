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
    engine::score::{sequence::Sequence, track_node::NodeKind, Interval, NotesGroup},
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

                                let is_drum = DRUM_WAVES.contains(&seq_mut.wave_type);
                                envelope::show_envelope_section(
                                    ui,
                                    seq_mut,
                                    &mut self.score.notes,
                                    &mut impact,
                                    is_drum,
                                );
                                lowpass::show_lowpass_section(
                                    ui,
                                    seq_mut,
                                    &mut self.score.notes,
                                    &mut impact,
                                    is_drum,
                                );
                                bend::show_bend_section(
                                    ui,
                                    seq_mut,
                                    &mut self.score.notes,
                                    &mut impact,
                                );
                                vibrato::show_vibrato_section(
                                    ui,
                                    seq_mut,
                                    &mut self.score.notes,
                                    &mut impact,
                                );
                                if !DRUM_WAVES.contains(&seq_mut.wave_type) {
                                    chorus::show_chorus_section(
                                        ui,
                                        seq_mut,
                                        &mut self.score.notes,
                                        &mut impact,
                                    );
                                }
                                power::show_power_section(
                                    ui,
                                    seq_mut,
                                    &mut self.score.notes,
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
