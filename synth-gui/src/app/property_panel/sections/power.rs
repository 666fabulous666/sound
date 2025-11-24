use super::super::{OverrideBinding, ParameterImpact};
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam};
use crate::app::property_panel::hover_texts::POW_FACT_TEXT;
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
            ui.add_enabled_ui(false, |ui| {
                draw_power_controls(ui, &mut preview, impact, false);
            });
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
    let base_param = SliderParam::new("Base", 0.01..=10.0)
        .default(defaults.0)
        .behavior(ParameterBehavior::AestheticImmediate)
        .logarithmic(true);

    let base_resp = base_param.draw(ui, &mut value.power.base, impact);
    let mut changed = base_resp.changed();

    ui.separator();
    ui.label("Relaxation");
    let relax_start = SliderParam::new("Start", 0.01..=10.0)
        .default(1.0)
        .behavior(ParameterBehavior::AestheticImmediate)
        .logarithmic(true);
    let relax_end = SliderParam::new("End", 0.01..=10.0)
        .default(1.0)
        .behavior(ParameterBehavior::AestheticImmediate)
        .logarithmic(true);
    let relax_rate = SliderParam::new("Rate", 0.0..=20.0)
        .default(0.0)
        .behavior(ParameterBehavior::AestheticImmediate);

    let start_resp = relax_start.draw(ui, &mut value.power.relaxation.start, impact);
    let end_resp = relax_end.draw(ui, &mut value.power.relaxation.end, impact);
    let rate_resp = relax_rate.draw(ui, &mut value.power.relaxation.rate, impact);
    changed |= start_resp.changed() || end_resp.changed() || rate_resp.changed();

    ui.separator();
    ui.label("Sinusoidal modulation");
    let magnitude_param = SliderParam::new("Magnitude", 0.0..=10.0)
        .default(0.0)
        .behavior(ParameterBehavior::AestheticImmediate);
    let freq_param = SliderParam::new("Frequency", Freq(0.001)..=Freq(20.0))
        .default(Freq(0.5))
        .behavior(ParameterBehavior::AestheticImmediate)
        .logarithmic(true);
    let magnitude_resp = magnitude_param.draw(ui, &mut value.power.lfo.magnitude, impact);
    let freq_resp = freq_param.draw(ui, &mut value.power.lfo.frequency, impact);
    changed |= magnitude_resp.changed() || freq_resp.changed();

    let sync_resp = ui.checkbox(
        &mut value.power.lfo.sync_with_clock,
        "Sync with global clock",
    );
    if sync_resp.changed() {
        impact.register_behavior(ParameterBehavior::AestheticImmediate);
        changed = true;
    }

    editable && changed
}
