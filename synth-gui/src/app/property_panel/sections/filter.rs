use super::super::{promote_button, OverrideBinding, ParameterImpact};
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam};
use crate::engine::score::{default_params::*, node_params::LowpassParams};
use crate::engine::waves::FilterType;
use crate::time_freq::Freq;
use egui::Ui;

pub fn show_filter_section(
    ui: &mut Ui,
    binding: &mut OverrideBinding<LowpassParams>,
    impact: &mut ParameterImpact,
    _is_drum: bool,
    depth: usize,
    promote: &mut Option<LowpassParams>,
) -> bool {
    let mut changed = false;
    ui.heading("Filter");
    if binding.is_locked() {
        let mut preview = binding.resolved().clone();
        ui.add_enabled_ui(false, |ui| {
            draw_filter_controls(ui, &mut preview, impact, false);
        });
    } else if let Some(value) = binding.value_mut() {
        changed |= draw_filter_controls(ui, value, impact, true);
    }
    if let Some(value) = promote_button(ui, binding, depth) {
        *promote = Some(value);
    }
    changed
}

pub fn draw_filter_controls(
    ui: &mut Ui,
    value: &mut LowpassParams,
    impact: &mut ParameterImpact,
    editable: bool,
) -> bool {
    let mut changed = false;

    if editable {
        let enable_resp = ui.checkbox(&mut value.enabled, "Enable filter");
        if enable_resp.changed() {
            impact.register_behavior(ParameterBehavior::AestheticImmediate);
            changed = true;
        }
    } else {
        let mut preview_enabled = value.enabled;
        ui.add_enabled_ui(false, |ui| {
            ui.checkbox(&mut preview_enabled, "Enable filter");
        });
    }

    ui.add_enabled_ui(value.enabled && editable, |ui| {
        ui.horizontal(|ui| {
            ui.label("Type:");
            if ui
                .selectable_value(&mut value.filter_type, FilterType::Lowpass, "Lowpass")
                .clicked()
            {
                impact.register_behavior(ParameterBehavior::AestheticImmediate);
                changed = true;
            }
            if ui
                .selectable_value(&mut value.filter_type, FilterType::Highpass, "Highpass")
                .clicked()
            {
                impact.register_behavior(ParameterBehavior::AestheticImmediate);
                changed = true;
            }
            if ui
                .selectable_value(&mut value.filter_type, FilterType::Bandpass, "Bandpass")
                .clicked()
            {
                impact.register_behavior(ParameterBehavior::AestheticImmediate);
                changed = true;
            }
        });

        let order_param = SliderParam::new("Order", 1..=10)
            .default(default_lp_order())
            .behavior(ParameterBehavior::AestheticImmediate);
        let cutoff_param = SliderParam::new("Base", 0.01..=100.0)
            .default(default_cutoff_multiplier())
            .behavior(ParameterBehavior::AestheticImmediate)
            .logarithmic(true);

        let order_resp = order_param.draw(ui, &mut value.order, impact);
        let cutoff_resp = cutoff_param.draw(ui, &mut value.cutoff.base, impact);
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

        let start_resp = relax_start.draw(ui, &mut value.cutoff.relaxation.start, impact);
        let end_resp = relax_end.draw(ui, &mut value.cutoff.relaxation.end, impact);
        let rate_resp = relax_rate.draw(ui, &mut value.cutoff.relaxation.rate, impact);
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
        let magnitude_resp = magnitude_param.draw(ui, &mut value.cutoff.lfo.magnitude, impact);
        let freq_resp = freq_param.draw(ui, &mut value.cutoff.lfo.frequency, impact);
        changed |= magnitude_resp.changed() || freq_resp.changed();

        let sync_resp = ui.checkbox(
            &mut value.cutoff.lfo.sync_with_clock,
            "Sync with global clock",
        );
        if sync_resp.changed() {
            impact.register_behavior(ParameterBehavior::AestheticImmediate);
            changed = true;
        }
    });

    changed && editable
}
