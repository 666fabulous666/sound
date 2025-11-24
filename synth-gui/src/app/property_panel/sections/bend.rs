use super::super::{OverrideBinding, ParameterImpact};
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam, ValueTransform};
use crate::engine::score::{default_params::*, node_params::BendParams};
use egui::Ui;

pub fn show_bend_section(
    ui: &mut Ui,
    binding: &mut OverrideBinding<BendParams>,
    impact: &mut ParameterImpact,
) -> bool {
    let mut changed = false;
    ui.collapsing("Bend", |ui| {
        if binding.is_locked() {
            let mut preview = binding.resolved().clone();
            ui.add_enabled_ui(false, |ui| {
                draw_bend_controls(ui, &mut preview, impact);
            });
        } else if let Some(value) = binding.value_mut() {
            changed |= draw_bend_controls(ui, value, impact);
        }
    });
    changed
}

pub fn draw_bend_controls(
    ui: &mut Ui,
    value: &mut BendParams,
    impact: &mut ParameterImpact,
) -> bool {
    let magnitude_transform = ValueTransform {
        to_exposed: |val: &f64| val * 1e4,
        from_exposed: |val: f64| val * 1e-4,
    };
    let mag_param =
        SliderParam::new_with_transform("Magnitude", -200.0..=200.0, magnitude_transform)
            .default(default_bend().0)
            .behavior(ParameterBehavior::AestheticImmediate);

    let speed_param = SliderParam::new("Speed", 1.0..=1000.0)
        .default(default_bend().1)
        .behavior(ParameterBehavior::AestheticImmediate)
        .logarithmic(true);

    let mag_resp = mag_param.draw(ui, &mut value.magnitude, impact);
    let speed_resp = speed_param.draw(ui, &mut value.speed, impact);
    mag_resp.changed() || speed_resp.changed()
}
