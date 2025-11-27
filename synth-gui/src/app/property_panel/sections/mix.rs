use super::super::ParameterImpact;
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam, ValueTransform};
use crate::engine::score::{
    default_params::*,
    probability::Probability,
    track_node::{NodeKind, TrackNode},
};
use egui::{Key, Ui};

pub fn show_mix_section(
    ui: &mut Ui,
    track_node: &mut TrackNode,
    impact: &mut ParameterImpact,
    pan_locked_by_parent: bool,
    resolved_pan: f64,
) {
    ui.heading("Mix");
    let kb_changed = ui.ctx().input(|i| {
        let step = if i.modifiers.shift { 0.5 } else { 0.05 };
        let mut changed = false;
        if i.key_down(Key::Plus) {
            let new_value = (track_node.volume() + step).min(32.0);
            *track_node.volume_mut() = new_value;
            changed = true;
        }
        if i.key_down(Key::Minus) {
            let new_value = (track_node.volume() - step).max(0.0);
            *track_node.volume_mut() = new_value;
            changed = true;
        }
        changed
    });
    if kb_changed {
        impact.register_behavior(ParameterBehavior::Mix);
    }

    let volume_param = SliderParam::new("Volume", 0.0..=5.0)
        .default(default_volume())
        .behavior(ParameterBehavior::Mix)
        .shortcut_hint("+ / - (Shift×10)")
        .tooltip("Hold + / - to change");
    volume_param.draw(ui, track_node.volume_mut(), impact);

    let pan_param = SliderParam::new("Pan", 0.0..=1.0)
        .default(default_pan())
        .behavior(ParameterBehavior::Mix)
        .tooltip("0.5 is centered, 0.0 is left, 1.0 is right");
    if pan_locked_by_parent {
        let mut preview = resolved_pan;
        ui.add_enabled_ui(false, |ui| {
            pan_param.draw(ui, &mut preview, impact);
        });
        ui.label("Pan locked by ancestor override");
    } else {
        pan_param.draw(ui, track_node.pan_mut(), impact);
    }

    if let NodeKind::Group { aesthetic, .. } = &mut track_node.kind {
        let mut lock_pan = aesthetic.lock_pan;
        let response = ui
            .add_enabled_ui(!pan_locked_by_parent, |ui| {
                ui.checkbox(&mut lock_pan, "Override pan for children")
            })
            .inner
            .on_hover_text(
                "Use this pan for all descendants unless another ancestor overrides it",
            );
        if response.changed() {
            aesthetic.lock_pan = lock_pan;
            impact.register_behavior(ParameterBehavior::Mix);
        }
    }

    let proba_transform = ValueTransform {
        to_exposed: |p: &Probability| p.as_f64(),
        from_exposed: |value: f64| Probability::new(value),
    };
    let proba_param = SliderParam::new_with_transform("Proba", 0.0..=1.0, proba_transform)
        .default(default_proba())
        .behavior(ParameterBehavior::StructuralFutureOnly)
        .tooltip("Probability for this sequence to generate notes");
    proba_param.draw(ui, track_node.proba_mut(), impact);

    ui.separator();
    ui.label("Delays (beats)");
    let mut changed = false;
    ui.columns(2, |cols| {
        cols[0].vertical(|ui_left| {
            ui_left.label("Left");
            changed |= edit_delay_channel(ui_left, &mut track_node.delays.left);
        });
        cols[1].vertical(|ui_right| {
            ui_right.label("Right");
            changed |= edit_delay_channel(ui_right, &mut track_node.delays.right);
        });
    });
    if changed {
        impact.register_behavior(ParameterBehavior::Mix);
    }
}

fn edit_delay_channel(ui: &mut Ui, channel: &mut crate::engine::score::DelayChannel) -> bool {
    let mut changed = false;
    {
        let mut dry = channel.dry.unwrap_or(0.0);
        let dry_resp = ui
            .add(
                egui::Slider::new(&mut dry, 0.0..=1.0)
                    .text("Dry")
                    .clamp_to_range(true),
            )
            .on_hover_text("Dry factor for this channel");
        if dry_resp.changed() {
            channel.dry = Some(dry.clamp(0.0, 1.0));
            changed = true;
        }
    }
    ui.separator();

    let taps = &mut channel.taps;
    let initial_len = taps.len();
    taps.retain_mut(|tap| {
        let mut keep = true;
        ui.horizontal(|ui| {
            let beat_response = ui
                .add(
                    egui::DragValue::new(&mut tap.beat)
                        .range(0.0..=32.0)
                        .speed(0.01),
                )
                .on_hover_text("Delay in beats");
            let mut weight = tap.weight.max(0.0);
            let weight_response = ui
                .add(
                    egui::DragValue::new(&mut weight)
                        .range(0.0..=10.0)
                        .speed(0.05),
                )
                .on_hover_text("Relative wet weight");
            changed |= beat_response.changed() || weight_response.changed();
            if weight_response.changed() {
                tap.weight = weight.max(0.0);
            }
            if ui.small_button("✖").on_hover_text("Remove delay").clicked() {
                keep = false;
                changed = true;
            }
        });
        keep
    });
    if taps.len() != initial_len {
        changed = true;
    }
    if ui
        .button("+")
        .on_hover_text("Right click a value to remove it")
        .clicked()
    {
        taps.push(crate::engine::score::DelayTap {
            beat: 0.0,
            weight: 1.0,
        });
        changed = true;
    }
    changed
}
