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

        let hue_param = SliderParam::new("Hue", 0.0..=360.0)
            .default(0.0)
            .behavior(ParameterBehavior::AestheticImmediate)
            .tooltip("Color hue in HSL space (0-360)");
        hue_param.draw(ui, &mut track_node.hue, impact);

        let h = track_node.hue / 60.0;
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
        let color =
            egui::Color32::from_rgb((r1 * 255.0) as u8, (g1 * 255.0) as u8, (b1 * 255.0) as u8);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 2.0, color);
    });
}
