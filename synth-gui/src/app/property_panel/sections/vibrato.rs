use super::super::ParameterImpact;
use crate::app::property_panel::helpers::{
    update_notes_group, ParameterBehavior, SliderParam, ValueTransform,
};
use crate::engine::score::{default_params::*, sequence::Sequence, NotesGroup};
use crate::time_freq::Freq;
use crate::Token;
use egui::Ui;
use std::collections::BTreeMap;

pub fn show_vibrato_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    notes: &mut BTreeMap<Token, NotesGroup>,
    impact: &mut ParameterImpact,
) {
    ui.collapsing("Vibrato", |ui| {
        let magnitude_transform = ValueTransform {
            to_exposed: |value: &f64| value * 1e6,
            from_exposed: |value: f64| value * 1e-6,
        };
        let magnitude_param =
            SliderParam::new_with_transform("Magnitude", 0.0..=1000.0, magnitude_transform)
                .default(default_vibrato().0)
                .behavior(ParameterBehavior::AestheticImmediate);

        let freq_param = SliderParam::new("Frequency", Freq(0.01)..=Freq(100.0))
            .default(default_vibrato().1)
            .behavior(ParameterBehavior::AestheticImmediate)
            .logarithmic(true);

        let magnitude_resp = magnitude_param.draw(ui, &mut seq.vibrato.0, impact);
        let freq_resp = freq_param.draw(ui, &mut seq.vibrato.1, impact);

        if magnitude_resp.changed() {
            update_notes_group(notes, seq.token, |ng| ng.vibrato.0 = seq.vibrato.0);
        }
        if freq_resp.changed() {
            update_notes_group(notes, seq.token, |ng| ng.vibrato.1 = seq.vibrato.1);
        }
    });
}
