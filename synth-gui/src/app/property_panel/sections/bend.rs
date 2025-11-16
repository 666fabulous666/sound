use super::super::ParameterImpact;
use crate::app::property_panel::helpers::{
    update_notes_group, ParameterBehavior, SliderParam, ValueTransform,
};
use crate::engine::score::{default_params::*, sequence::Sequence, NotesGroup};
use crate::Token;
use egui::Ui;
use std::collections::BTreeMap;

pub fn show_bend_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    notes: &mut BTreeMap<Token, NotesGroup>,
    impact: &mut ParameterImpact,
) {
    ui.collapsing("Bend", |ui| {
        let magnitude_transform = ValueTransform {
            to_exposed: |value: &f64| value * 1e4,
            from_exposed: |value: f64| value * 1e-4,
        };
        let mag_param =
            SliderParam::new_with_transform("Magnitude", -200.0..=200.0, magnitude_transform)
                .default(default_bend().0)
                .behavior(ParameterBehavior::AestheticImmediate);

        let speed_param = SliderParam::new("Speed", 1.0..=1000.0)
            .default(default_bend().1)
            .behavior(ParameterBehavior::AestheticImmediate)
            .logarithmic(true);

        let mag_resp = mag_param.draw(ui, &mut seq.bend.0, impact);
        let speed_resp = speed_param.draw(ui, &mut seq.bend.1, impact);

        if mag_resp.changed() {
            update_notes_group(notes, seq.token, |ng| ng.bend.0 = seq.bend.0);
        }
        if speed_resp.changed() {
            update_notes_group(notes, seq.token, |ng| ng.bend.1 = seq.bend.1);
        }
    });
}
