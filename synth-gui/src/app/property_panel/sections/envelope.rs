use super::super::ParameterImpact;
use crate::app::property_panel::helpers::{
    rescale_envelope, update_notes_group, ParameterBehavior, SliderParam,
};
use crate::engine::score::{default_params::*, sequence::Sequence, NotesGroup};
use crate::Token;
use egui::Ui;
use std::collections::BTreeMap;

pub fn show_envelope_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    notes: &mut BTreeMap<Token, NotesGroup>,
    impact: &mut ParameterImpact,
    is_drum: bool,
) {
    ui.collapsing("Envelope", |ui| {
        let defaults = if is_drum {
            default_drum_attack_decay()
        } else {
            default_attack_decay()
        };

        let attack_param = SliderParam::new("Attack", 0.01..=100.0)
            .default(defaults.0)
            .behavior(ParameterBehavior::AestheticImmediate)
            .logarithmic(true);
        let decay_param = SliderParam::new("Decay", 0.01..=100.0)
            .default(defaults.1)
            .behavior(ParameterBehavior::AestheticImmediate)
            .logarithmic(true);

        let attack_resp = attack_param.draw(ui, &mut seq.attack_decay.0, impact);
        let decay_resp = decay_param.draw(ui, &mut seq.attack_decay.1, impact);

        if attack_resp.changed() || decay_resp.changed() {
            rescale_envelope(seq);
            update_notes_group(notes, seq.token, |ng| {
                ng.attack_decay = seq.attack_decay;
            });
        }
    });
}
