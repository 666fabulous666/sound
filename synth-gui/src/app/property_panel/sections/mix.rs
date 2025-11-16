use super::super::ParameterImpact;
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam, ValueTransform};
use crate::engine::score::{default_params::*, probability::Probability, track_node::TrackNode};
use egui::{Key, Ui};

pub fn show_mix_section(ui: &mut Ui, track_node: &mut TrackNode, impact: &mut ParameterImpact) {
    ui.collapsing("Mix", |ui| {
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
        pan_param.draw(ui, track_node.pan_mut(), impact);

        let proba_transform = ValueTransform {
            to_exposed: |p: &Probability| p.as_f64(),
            from_exposed: |value: f64| Probability::new(value),
        };
        let proba_param = SliderParam::new_with_transform("Proba", 0.0..=1.0, proba_transform)
            .default(default_proba())
            .behavior(ParameterBehavior::StructuralFutureOnly)
            .tooltip("Probability for this sequence to generate notes");
        proba_param.draw(ui, track_node.proba_mut(), impact);
    });
}
