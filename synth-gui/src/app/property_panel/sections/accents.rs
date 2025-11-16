use super::super::ParameterImpact;
use crate::app::property_panel::helpers::{ParameterBehavior, SliderParam, ValueTransform};
use crate::engine::score::default_params::default_accents;
use crate::engine::score::sequence::Sequence;
use egui::Ui;

pub fn show_accents_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    impact: &mut ParameterImpact,
    edit_vec_fn: impl Fn(&mut Ui, &mut Vec<f64>, f64),
) {
    ui.collapsing("Accents", |ui| {
        let defaults = default_accents();
        let magnitude_transform = ValueTransform {
            to_exposed: |value: &f64| 1.0 / value.max(f64::MIN_POSITIVE),
            from_exposed: |display: f64| 1.0 / display.max(f64::MIN_POSITIVE),
        };
        let magnitude_param =
            SliderParam::new_with_transform("Magnitude", 0.01..=100.0, magnitude_transform)
                .default(defaults.0)
                .behavior(ParameterBehavior::AestheticImmediate)
                .logarithmic(true);

        magnitude_param.draw(ui, &mut seq.accents.0, impact);

        let mut gens_disp: Vec<f64> = seq
            .accents
            .1
            .iter()
            .map(|&x| 1.0 / x.max(f64::MIN_POSITIVE))
            .collect();

        let previous = gens_disp.clone();
        ui.label("Generators");
        edit_vec_fn(ui, &mut gens_disp, 1.0);

        if gens_disp != previous && !gens_disp.iter().any(|&v| v == 0.0) {
            seq.accents.1 = gens_disp
                .into_iter()
                .map(|x| 1.0 / x.max(f64::MIN_POSITIVE))
                .collect();
            impact.register_behavior(ParameterBehavior::AestheticImmediate);
        }
    });
}
