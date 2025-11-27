use super::super::{promote_button, OverrideBinding, ParameterImpact};
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam};
use crate::app::property_panel::hover_texts::{
    ASYM_DETUNE_TEXT, DETUNE_SHIFT_TEXT, DETUNE_TEXT, DETUNE_TIME_DEP_TEXT, DETUNE_WEIGHTING_TEXT,
    SYM_DETUNE_TEXT, UNISSON_DETUNE_TEXT, VOICE_LAYERS_TEXT,
};
use crate::engine::score::ChorusParams;
use crate::time_freq::Freq;
use egui::Ui;

pub fn show_chorus_section(
    ui: &mut Ui,
    binding: &mut OverrideBinding<ChorusParams>,
    impact: &mut ParameterImpact,
    depth: usize,
    promote: &mut Option<ChorusParams>,
) -> bool {
    let mut changed = false;
    let defaults = ChorusParams::default();
    ui.heading("Chorus (Unison Detune)")
        .on_hover_text(UNISSON_DETUNE_TEXT);
    if binding.is_locked() {
        let mut preview = binding.resolved().clone();
        ui.add_enabled_ui(false, |ui| {
            draw_chorus_controls(ui, &mut preview, impact, &defaults, false);
        });
    } else if let Some(value) = binding.value_mut() {
        changed |= draw_chorus_controls(ui, value, impact, &defaults, true);
    }
    if let Some(value) = promote_button(ui, binding, depth) {
        *promote = Some(value);
    }
    changed
}

pub fn draw_chorus_controls(
    ui: &mut Ui,
    value: &mut ChorusParams,
    impact: &mut ParameterImpact,
    defaults: &ChorusParams,
    editable: bool,
) -> bool {
    let voices_param = SliderParam::new("Voice layers", 1..=10)
        .default(defaults.voices)
        .behavior(ParameterBehavior::AestheticImmediate);
    let delta_param = SliderParam::new("Detune (Δf)", 0.0..=1.0)
        .default(defaults.delta)
        .behavior(ParameterBehavior::AestheticImmediate)
        .logarithmic(true);
    let delta_shift_param = SliderParam::new("Detune shift", -1.0..=1.0)
        .default(defaults.delta_shift)
        .behavior(ParameterBehavior::AestheticImmediate);
    let time_dep_param = SliderParam::new("Detune over time", Freq(-5.0)..=Freq(5.0))
        .default(defaults.time_dependency)
        .behavior(ParameterBehavior::AestheticImmediate);
    let sym_param = SliderParam::new("Even", -2.0..=2.0)
        .default(defaults.sym)
        .behavior(ParameterBehavior::AestheticImmediate);
    let asym_param = SliderParam::new("Odd", -2.0..=2.0)
        .default(defaults.asym)
        .behavior(ParameterBehavior::AestheticImmediate);

    let voices_resp = voices_param.draw(ui, &mut value.voices, impact);
    let _ = voices_resp.clone().on_hover_text(VOICE_LAYERS_TEXT);
    let delta_resp = delta_param.draw(ui, &mut value.delta, impact);
    let _ = delta_resp.clone().on_hover_text(DETUNE_TEXT);
    let delta_shift_resp = delta_shift_param.draw(ui, &mut value.delta_shift, impact);
    let _ = delta_shift_resp.clone().on_hover_text(DETUNE_SHIFT_TEXT);
    let time_dep_resp = time_dep_param.draw(ui, &mut value.time_dependency, impact);
    let _ = time_dep_resp.clone().on_hover_text(DETUNE_TIME_DEP_TEXT);

    ui.label("Weighting (around f₀)")
        .on_hover_text(DETUNE_WEIGHTING_TEXT);

    let sym_resp = sym_param.draw(ui, &mut value.sym, impact);
    let _ = sym_resp.clone().on_hover_text(SYM_DETUNE_TEXT);
    let asym_resp = asym_param.draw(ui, &mut value.asym, impact);
    let _ = asym_resp.clone().on_hover_text(ASYM_DETUNE_TEXT);

    editable
        && (voices_resp.changed()
            || delta_resp.changed()
            || delta_shift_resp.changed()
            || time_dep_resp.changed()
            || sym_resp.changed()
            || asym_resp.changed())
}
