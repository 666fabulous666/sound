use super::super::{OverrideBinding, ParameterImpact};
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam, ValueTransform};
use crate::engine::score::{default_params::*, node_params::VibratoParams};
use crate::time_freq::Freq;
use egui::Ui;

pub fn show_vibrato_section(
    ui: &mut Ui,
    binding: &mut OverrideBinding<VibratoParams>,
    impact: &mut ParameterImpact,
) -> bool {
    let mut changed = false;
    ui.collapsing("Vibrato", |ui| {
        if binding.is_locked() {
            let mut preview = binding.resolved().clone();
            changed |= draw_vibrato_controls(ui, &mut preview, impact, false);
        } else if let Some(value) = binding.value_mut() {
            changed |= draw_vibrato_controls(ui, value, impact, true);
        }
    });
    changed
}

pub fn draw_vibrato_controls(
    ui: &mut Ui,
    value: &mut VibratoParams,
    impact: &mut ParameterImpact,
    editable: bool,
) -> bool {
    let magnitude_transform = ValueTransform {
        to_exposed: |val: &f64| val * 1e6,
        from_exposed: |val: f64| val * 1e-6,
    };
    let magnitude_param =
        SliderParam::new_with_transform("Magnitude", 0.0..=1000.0, magnitude_transform)
            .default(default_vibrato().0)
            .behavior(ParameterBehavior::AestheticImmediate);

    let freq_param = SliderParam::new("Frequency", Freq(0.01)..=Freq(100.0))
        .default(default_vibrato().1)
        .behavior(ParameterBehavior::AestheticImmediate)
        .logarithmic(true);

    let mag_resp = magnitude_param.draw(ui, &mut value.magnitude, impact);
    let freq_resp = freq_param.draw(ui, &mut value.frequency, impact);
    editable && (mag_resp.changed() || freq_resp.changed())
}
