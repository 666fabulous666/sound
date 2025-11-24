use super::super::{OverrideBinding, ParameterImpact};
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam};
use crate::engine::score::node_params::NoiseParams;
use crate::time_freq::Freq;
use egui::Ui;

pub fn show_noise_section(
    ui: &mut Ui,
    binding: &mut OverrideBinding<NoiseParams>,
    impact: &mut ParameterImpact,
) -> bool {
    let mut changed = false;
    let header = ui.collapsing("Noise", |ui| {
        if binding.is_locked() {
            let mut preview = binding.resolved().clone();
            draw_noise_controls(ui, &mut preview, impact, false);
        } else if let Some(value) = binding.value_mut() {
            changed |= draw_noise_controls(ui, value, impact, true);
        }
    });
    header.header_response.on_hover_text(
        "Multiplies signal by (1.0 - alpha * random) where random ∈ [0, 1]. \
         Alpha is time-varying with relaxation and LFO modulation."
    );
    changed
}

pub fn draw_noise_controls(
    ui: &mut Ui,
    value: &mut NoiseParams,
    impact: &mut ParameterImpact,
    editable: bool,
) -> bool {
    let base_param = SliderParam::new("Alpha (base)", 0.0..=1.0)
        .default(0.0)
        .behavior(ParameterBehavior::AestheticImmediate);

    let base_resp = base_param.draw(ui, &mut value.noise.base, impact);
    let mut changed = base_resp.changed();

    ui.separator();
    ui.label("Relaxation");
    let relax_start = SliderParam::new("Start", 0.0..=2.0)
        .default(1.0)
        .behavior(ParameterBehavior::AestheticImmediate);
    let relax_end = SliderParam::new("End", 0.0..=2.0)
        .default(1.0)
        .behavior(ParameterBehavior::AestheticImmediate);
    let relax_rate = SliderParam::new("Rate", 0.0..=20.0)
        .default(0.0)
        .behavior(ParameterBehavior::AestheticImmediate);

    let start_resp = relax_start.draw(ui, &mut value.noise.relaxation.start, impact);
    let end_resp = relax_end.draw(ui, &mut value.noise.relaxation.end, impact);
    let rate_resp = relax_rate.draw(ui, &mut value.noise.relaxation.rate, impact);
    changed |= start_resp.changed() || end_resp.changed() || rate_resp.changed();

    ui.separator();
    ui.label("Sinusoidal modulation");
    let magnitude_param = SliderParam::new("Magnitude", 0.0..=1.0)
        .default(0.0)
        .behavior(ParameterBehavior::AestheticImmediate);
    let freq_param = SliderParam::new("Frequency", Freq(0.001)..=Freq(20.0))
        .default(Freq(0.5))
        .behavior(ParameterBehavior::AestheticImmediate)
        .logarithmic(true);
    let magnitude_resp = magnitude_param.draw(ui, &mut value.noise.lfo.magnitude, impact);
    let freq_resp = freq_param.draw(ui, &mut value.noise.lfo.frequency, impact);
    changed |= magnitude_resp.changed() || freq_resp.changed();

    let sync_resp = ui.checkbox(
        &mut value.noise.lfo.sync_with_clock,
        "Sync with global clock",
    );
    if sync_resp.changed() {
        impact.register_behavior(ParameterBehavior::AestheticImmediate);
        changed = true;
    }

    editable && changed
}
