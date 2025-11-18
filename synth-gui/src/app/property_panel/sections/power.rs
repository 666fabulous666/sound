use super::super::{OverrideBinding, ParameterImpact};
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam, ValueTransform};
use crate::app::property_panel::hover_texts::{POW_FACT_EVOL_TEXT, POW_FACT_TEXT};
use crate::engine::score::{default_params::*, node_params::PowerParams};
use crate::time_freq::Freq;
use egui::Ui;

pub fn show_power_section(
    ui: &mut Ui,
    binding: &mut OverrideBinding<PowerParams>,
    impact: &mut ParameterImpact,
) -> bool {
    let mut changed = false;
    let header = ui.collapsing("Power factor", |ui| {
        if binding.is_locked() {
            let mut preview = binding.resolved().clone();
            draw_power_controls(ui, &mut preview, impact, false);
        } else if let Some(value) = binding.value_mut() {
            changed |= draw_power_controls(ui, value, impact, true);
        }
    });
    header.header_response.on_hover_text(POW_FACT_TEXT);
    changed
}

pub fn draw_power_controls(
    ui: &mut Ui,
    value: &mut PowerParams,
    impact: &mut ParameterImpact,
    editable: bool,
) -> bool {
    let defaults = default_pow_fact();
    let initial_param = SliderParam::new("Initial value", 0.0..=1000.0)
        .default(defaults.0)
        .behavior(ParameterBehavior::AestheticImmediate)
        .logarithmic(true);

    let evol_transform = ValueTransform {
        to_exposed: |val: &Freq| {
            let hz = val.as_hz();
            Freq(hz.signum() * hz.abs().sqrt())
        },
        from_exposed: |val: Freq| {
            let hz = val.as_hz();
            Freq(hz.signum() * hz.powi(2))
        },
    };
    let evol_param =
        SliderParam::new_with_transform("Evolution", Freq(-10.0)..=Freq(10.0), evol_transform)
            .default(defaults.1)
            .behavior(ParameterBehavior::AestheticImmediate);

    let initial_resp = initial_param.draw(ui, &mut value.initial, impact);
    let evol_resp = evol_param.draw(ui, &mut value.evolution, impact);
    let _ = evol_resp.clone().on_hover_text(POW_FACT_EVOL_TEXT);

    editable && (initial_resp.changed() || evol_resp.changed())
}
