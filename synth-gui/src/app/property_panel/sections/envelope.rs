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

        let mut envelope_changed = attack_resp.changed() || decay_resp.changed();

        let lowpass_resp = ui.checkbox(&mut seq.lowpass_enabled, "Enable Lowpass Filter");
        if lowpass_resp.changed() {
            impact.register_behavior(ParameterBehavior::AestheticImmediate);
            envelope_changed = true;
        }

        let mut lp_changed = false;
        if seq.lowpass_enabled {
            let order_param = SliderParam::new("Lowpass order", 1..=5)
                .default(default_lp_order())
                .behavior(ParameterBehavior::AestheticImmediate);
            let lp_attack_param = SliderParam::new("Lowpass Attack", 0.01..=100.0)
                .default(defaults.0)
                .behavior(ParameterBehavior::AestheticImmediate)
                .logarithmic(true);
            let lp_decay_param = SliderParam::new("Lowpass Decay", 0.01..=100.0)
                .default(defaults.1)
                .behavior(ParameterBehavior::AestheticImmediate)
                .logarithmic(true);
            let cutoff_param = SliderParam::new("Lowpass cutoff multiplier", 0.01..=100.0)
                .default(default_cutoff_multiplier())
                .behavior(ParameterBehavior::AestheticImmediate)
                .logarithmic(true);

            let order_resp = order_param.draw(ui, &mut seq.lp_order, impact);
            let lp_attack_resp = lp_attack_param.draw(ui, &mut seq.lp_attack_decay.0, impact);
            let lp_decay_resp = lp_decay_param.draw(ui, &mut seq.lp_attack_decay.1, impact);
            let cutoff_resp = cutoff_param.draw(ui, &mut seq.cutoff_multiplier, impact);

            lp_changed = order_resp.changed()
                || lp_attack_resp.changed()
                || lp_decay_resp.changed()
                || cutoff_resp.changed();
        }

        if envelope_changed || lp_changed {
            rescale_envelope(seq);
            update_notes_group(notes, seq.token, |ng| {
                ng.attack_decay = seq.attack_decay;
                ng.lp_attack_decay = seq.lp_attack_decay;
                ng.cutoff_multiplier = seq.cutoff_multiplier;
                ng.lowpass_enabled = seq.lowpass_enabled;
                ng.lp_order = seq.lp_order;
            });
        }
    });
}
