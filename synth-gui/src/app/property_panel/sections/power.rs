use super::super::ParameterImpact;
use crate::app::property_panel::helpers::{
    update_notes_group, ParameterBehavior, SliderParam, ValueTransform,
};
use crate::app::property_panel::hover_texts::{POW_FACT_EVOL_TEXT, POW_FACT_TEXT};
use crate::engine::score::{default_params::*, sequence::Sequence, NotesGroup};
use crate::time_freq::Freq;
use crate::Token;
use egui::Ui;
use std::collections::BTreeMap;

pub fn show_power_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    notes: &mut BTreeMap<Token, NotesGroup>,
    impact: &mut ParameterImpact,
) {
    let header = ui.collapsing("Power factor", |ui| {
        let defaults = default_pow_fact();
        let initial_param = SliderParam::new("Initial value", 0.0..=1000.0)
            .default(defaults.0)
            .behavior(ParameterBehavior::AestheticImmediate)
            .logarithmic(true);

        let evol_transform = ValueTransform {
            to_exposed: |value: &Freq| {
                let hz = value.as_hz();
                Freq(hz.signum() * hz.abs().sqrt())
            },
            from_exposed: |value: Freq| {
                let hz = value.as_hz();
                Freq(hz.signum() * hz.powi(2))
            },
        };
        let evol_param =
            SliderParam::new_with_transform("Evolution", Freq(-10.0)..=Freq(10.0), evol_transform)
                .default(defaults.1)
                .behavior(ParameterBehavior::AestheticImmediate);

        let initial_resp = initial_param.draw(ui, &mut seq.pow_fact.0, impact);
        let evol_resp = evol_param.draw(ui, &mut seq.pow_fact.1, impact);
        let _ = evol_resp.clone().on_hover_text(POW_FACT_EVOL_TEXT);

        if initial_resp.changed() {
            update_notes_group(notes, seq.token, |ng| ng.pow_fact.0 = seq.pow_fact.0);
        }
        if evol_resp.changed() {
            update_notes_group(notes, seq.token, |ng| ng.pow_fact.1 = seq.pow_fact.1);
        }
    });
    header.header_response.on_hover_text(POW_FACT_TEXT);
}
