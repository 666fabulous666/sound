use super::super::ParameterImpact;
use crate::app::property_panel::helpers::{update_notes_group, ParameterBehavior, SliderParam};
use crate::engine::score::{default_params::*, sequence::Sequence, NotesGroup};
use crate::time_freq::Freq;
use crate::Token;
use egui::Ui;
use std::collections::BTreeMap;

pub fn show_lowpass_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    notes: &mut BTreeMap<Token, NotesGroup>,
    impact: &mut ParameterImpact,
    _is_drum: bool,
) {
    ui.collapsing("Lowpass", |ui| {
        let mut changed = false;

        let enable_resp = ui.checkbox(&mut seq.lowpass_enabled, "Enable lowpass");
        if enable_resp.changed() {
            impact.register_behavior(ParameterBehavior::AestheticImmediate);
            changed = true;
        }

        ui.add_enabled_ui(seq.lowpass_enabled, |ui| {
            let order_param = SliderParam::new("Order", 1..=5)
                .default(default_lp_order())
                .behavior(ParameterBehavior::AestheticImmediate);
            let cutoff_param = SliderParam::new("Cutoff multiplier", 0.01..=100.0)
                .default(default_cutoff_multiplier())
                .behavior(ParameterBehavior::AestheticImmediate)
                .logarithmic(true);

            let order_resp = order_param.draw(ui, &mut seq.lp_order, impact);
            let cutoff_resp = cutoff_param.draw(ui, &mut seq.cutoff_multiplier, impact);
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

            let start_resp = relax_start.draw(ui, &mut seq.lp_relaxation.start, impact);
            let end_resp = relax_end.draw(ui, &mut seq.lp_relaxation.end, impact);
            let rate_resp = relax_rate.draw(ui, &mut seq.lp_relaxation.rate, impact);
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
            let magnitude_resp = magnitude_param.draw(ui, &mut seq.lp_lfo.magnitude, impact);
            let freq_resp = freq_param.draw(ui, &mut seq.lp_lfo.frequency, impact);
            changed |= magnitude_resp.changed() || freq_resp.changed();

            let sync_resp = ui.checkbox(&mut seq.lp_lfo.sync_with_clock, "Sync with global clock");
            if sync_resp.changed() {
                impact.register_behavior(ParameterBehavior::AestheticImmediate);
                changed = true;
            }
        });

        if changed {
            update_notes_group(notes, seq.token, |ng| {
                ng.lowpass_enabled = seq.lowpass_enabled;
                ng.lp_order = seq.lp_order;
                ng.cutoff_multiplier = seq.cutoff_multiplier;
                ng.lp_relaxation = seq.lp_relaxation;
                ng.lp_lfo = seq.lp_lfo;
            });
        }
    });
}
