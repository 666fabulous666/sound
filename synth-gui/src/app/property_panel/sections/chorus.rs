use super::super::ParameterImpact;
use crate::app::property_panel::helpers::{update_notes_group, ParameterBehavior, SliderParam};
use crate::app::property_panel::hover_texts::{
    ASYM_DETUNE_TEXT, DETUNE_SHIFT_TEXT, DETUNE_TEXT, DETUNE_TIME_DEP_TEXT, DETUNE_WEIGHTING_TEXT,
    SYM_DETUNE_TEXT, UNISSON_DETUNE_TEXT, VOICE_LAYERS_TEXT,
};
use crate::engine::score::{sequence::Sequence, ChorusParams, NotesGroup};
use crate::time_freq::Freq;
use crate::Token;
use egui::Ui;
use std::collections::BTreeMap;

pub fn show_chorus_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    notes: &mut BTreeMap<Token, NotesGroup>,
    impact: &mut ParameterImpact,
) {
    let defaults = ChorusParams::default();
    let header = ui.collapsing("Chorus (Unison Detune)", |ui| {
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

        let voices_resp = voices_param.draw(ui, &mut seq.chorus.voices, impact);
        let _ = voices_resp.clone().on_hover_text(VOICE_LAYERS_TEXT);
        let delta_resp = delta_param.draw(ui, &mut seq.chorus.delta, impact);
        let _ = delta_resp.clone().on_hover_text(DETUNE_TEXT);
        let delta_shift_resp = delta_shift_param.draw(ui, &mut seq.chorus.delta_shift, impact);
        let _ = delta_shift_resp.clone().on_hover_text(DETUNE_SHIFT_TEXT);
        let time_dep_resp = time_dep_param.draw(ui, &mut seq.chorus.time_dependency, impact);
        let _ = time_dep_resp.clone().on_hover_text(DETUNE_TIME_DEP_TEXT);

        ui.label("Weighting (around f₀)")
            .on_hover_text(DETUNE_WEIGHTING_TEXT);

        let sym_resp = sym_param.draw(ui, &mut seq.chorus.sym, impact);
        let _ = sym_resp.clone().on_hover_text(SYM_DETUNE_TEXT);
        let asym_resp = asym_param.draw(ui, &mut seq.chorus.asym, impact);
        let _ = asym_resp.clone().on_hover_text(ASYM_DETUNE_TEXT);

        if voices_resp.changed()
            || delta_resp.changed()
            || delta_shift_resp.changed()
            || time_dep_resp.changed()
            || sym_resp.changed()
            || asym_resp.changed()
        {
            update_notes_group(notes, seq.token, |ng| {
                ng.chorus = seq.chorus.clone();
            });
        }
    });
    header.header_response.on_hover_text(UNISSON_DETUNE_TEXT);
}
