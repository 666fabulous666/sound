use egui::Ui;

use crate::{
    app::property_panel::{
        helpers::{ParameterBehavior, SliderParam, ValueTransform},
        promote_button, OverrideBinding, ParameterImpact,
    },
    engine::score::{default_params::default_harmonics_attenuation, HarmonicsParams},
};

pub fn show_harmonics_section(
    ui: &mut Ui,
    binding: &mut OverrideBinding<HarmonicsParams>,
    impact: &mut ParameterImpact,
    depth: usize,
    promote: &mut Option<HarmonicsParams>,
) -> bool {
    let mut changed = false;

    ui.collapsing("Harmonics", |ui| {
        if binding.is_locked() {
            let mut preview = binding.resolved().clone();
            ui.add_enabled_ui(false, |ui| {
                draw_harmonics_controls(ui, &mut preview, impact, false);
            });
        } else if let Some(value) = binding.value_mut() {
            changed |= draw_harmonics_controls(ui, value, impact, true);
        }
        if let Some(value) = promote_button(ui, binding, depth) {
            *promote = Some(value);
        }
    });

    changed
}

pub fn draw_harmonics_controls(
    ui: &mut Ui,
    params: &mut HarmonicsParams,
    impact: &mut ParameterImpact,
    editable: bool,
) -> bool {
    let mut local_changed = false;
    ui.add_enabled_ui(editable, |ui| {
        let usize_transform = ValueTransform {
            to_exposed: |v: &usize| *v as u32,
            from_exposed: |value: u32| value as usize,
        };

        let resp = SliderParam::new_with_transform("Harmonics", 0u32..=16u32, usize_transform)
            .behavior(ParameterBehavior::Mix)
            .tooltip("Number of upper partials (multiples)")
            .draw(ui, &mut params.harmonics, impact);
        local_changed |= resp.changed();

        let resp = SliderParam::new_with_transform("Subharmonics", 0u32..=16u32, usize_transform)
            .behavior(ParameterBehavior::Mix)
            .tooltip("Number of lower partials (divisors)")
            .draw(ui, &mut params.subharmonics, impact);
        local_changed |= resp.changed();

        let resp = SliderParam::new("Attenuation", 0.0..=1.0)
            .default(default_harmonics_attenuation())
            .behavior(ParameterBehavior::Mix)
            .tooltip("Geometric falloff applied to partials (1 = flat, 0 = only fundamental)")
            .draw(ui, &mut params.attenuation, impact);
        local_changed |= resp.changed();
    });
    local_changed
}
