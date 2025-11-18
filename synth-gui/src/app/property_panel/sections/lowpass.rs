use super::super::{OverrideBinding, ParameterImpact};
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam};
use crate::engine::score::{default_params::*, node_params::LowpassParams};
use crate::time_freq::Freq;
use egui::Ui;

pub fn show_lowpass_section(
    ui: &mut Ui,
    binding: &mut OverrideBinding<LowpassParams>,
    impact: &mut ParameterImpact,
    _is_drum: bool,
) -> bool {
    let mut changed = false;
    ui.collapsing("Lowpass", |ui| {
        if binding.is_locked() {
            let mut preview = binding.resolved().clone();
            draw_lowpass_controls(ui, &mut preview, impact, false);
        } else if let Some(value) = binding.value_mut() {
            changed |= draw_lowpass_controls(ui, value, impact, true);
        }
    });
    changed
}

pub fn draw_lowpass_controls(
    ui: &mut Ui,
    value: &mut LowpassParams,
    impact: &mut ParameterImpact,
    editable: bool,
) -> bool {
    let mut changed = false;

    if editable {
        let enable_resp = ui.checkbox(&mut value.enabled, "Enable lowpass");
        if enable_resp.changed() {
            impact.register_behavior(ParameterBehavior::AestheticImmediate);
            changed = true;
        }
    } else {
        let mut preview_enabled = value.enabled;
        ui.add_enabled_ui(false, |ui| {
            ui.checkbox(&mut preview_enabled, "Enable lowpass");
        });
    }

    ui.add_enabled_ui(value.enabled && editable, |ui| {
        let order_param = SliderParam::new("Order", 1..=5)
            .default(default_lp_order())
            .behavior(ParameterBehavior::AestheticImmediate);
        let cutoff_param = SliderParam::new("Cutoff multiplier", 0.01..=100.0)
            .default(default_cutoff_multiplier())
            .behavior(ParameterBehavior::AestheticImmediate)
            .logarithmic(true);

        let order_resp = order_param.draw(ui, &mut value.order, impact);
        let cutoff_resp = cutoff_param.draw(ui, &mut value.cutoff_multiplier, impact);
        changed |= order_resp.changed() || cutoff_resp.changed();

        ui.separator();
        ui.label("Relaxation");
        let relax_defaults = default_lp_relaxation();
        let relax_start = SliderParam::new("Start", 0.01..=64.0)
            .default(relax_defaults.start)
            .behavior(ParameterBehavior::AestheticImmediate)
            .logarithmic(true);
        let relax_end = SliderParam::new("End", 0.01..=64.0)
            .default(relax_defaults.end)
            .behavior(ParameterBehavior::AestheticImmediate)
            .logarithmic(true);
        let relax_rate = SliderParam::new("Rate", 0.0..=20.0)
            .default(relax_defaults.rate)
            .behavior(ParameterBehavior::AestheticImmediate);

        let start_resp = relax_start.draw(ui, &mut value.relaxation.start, impact);
        let end_resp = relax_end.draw(ui, &mut value.relaxation.end, impact);
        let rate_resp = relax_rate.draw(ui, &mut value.relaxation.rate, impact);
        changed |= start_resp.changed() || end_resp.changed() || rate_resp.changed();

        ui.separator();
        ui.label("Sinusoidal modulation");
        let lfo_defaults = default_lp_lfo();
        let magnitude_param = SliderParam::new("Magnitude", 0.0..=10.0)
            .default(lfo_defaults.magnitude)
            .behavior(ParameterBehavior::AestheticImmediate);
        let freq_param = SliderParam::new("Frequency", Freq(0.001)..=Freq(20.0))
            .default(lfo_defaults.frequency)
            .behavior(ParameterBehavior::AestheticImmediate)
            .logarithmic(true);
        let magnitude_resp = magnitude_param.draw(ui, &mut value.lfo.magnitude, impact);
        let freq_resp = freq_param.draw(ui, &mut value.lfo.frequency, impact);
        changed |= magnitude_resp.changed() || freq_resp.changed();

        let sync_resp = ui.checkbox(&mut value.lfo.sync_with_clock, "Sync with global clock");
        if sync_resp.changed() {
            impact.register_behavior(ParameterBehavior::AestheticImmediate);
            changed = true;
        }
    });

    changed && editable
}
