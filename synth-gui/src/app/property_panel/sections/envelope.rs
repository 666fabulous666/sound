use super::super::{promote_button, OverrideBinding, ParameterImpact};
use crate::app::property_panel::helpers::{rescale_envelope, ParameterBehavior, SliderParam};
use crate::engine::score::{default_params::*, node_params::EnvelopeParams};
use egui::Ui;

pub fn show_envelope_section(
    ui: &mut Ui,
    binding: &mut OverrideBinding<EnvelopeParams>,
    impact: &mut ParameterImpact,
    is_drum: bool,
    depth: usize,
    promote: &mut Option<EnvelopeParams>,
) -> bool {
    let mut changed = false;
    ui.collapsing("Envelope", |ui| {
        if binding.is_locked() {
            let mut preview = binding.resolved().clone();
            ui.add_enabled_ui(false, |ui| {
                draw_envelope_controls(ui, &mut preview, impact, is_drum, false);
            });
        } else if let Some(value) = binding.value_mut() {
            changed |= draw_envelope_controls(ui, value, impact, is_drum, true);
        }
        if let Some(value) = promote_button(ui, binding, depth) {
            *promote = Some(value);
        }
    });
    changed
}

pub fn draw_envelope_controls(
    ui: &mut Ui,
    value: &mut EnvelopeParams,
    impact: &mut ParameterImpact,
    is_drum: bool,
    editable: bool,
) -> bool {
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

    let attack_resp = attack_param.draw(ui, &mut value.attack, impact);
    let decay_resp = decay_param.draw(ui, &mut value.decay, impact);

    if attack_resp.changed() || decay_resp.changed() {
        rescale_envelope(value);
        editable
    } else {
        false
    }
}
