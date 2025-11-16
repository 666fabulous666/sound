use egui::{Response, Ui};
use std::marker::PhantomData;

use crate::{
    engine::score::{sequence::Sequence, NotesGroup},
    rescale_factor, Token,
};

/// Helper for sliders with right-click reset functionality
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterBehavior {
    AestheticImmediate,
    StructuralImmediate,
    StructuralFutureOnly,
    Mix,
}

pub struct SliderParam<'a, T, U = T>
where
    U: egui::emath::Numeric + Copy,
{
    label: &'static str,
    range: std::ops::RangeInclusive<U>,
    tooltip: Option<&'a str>,
    default: Option<T>,
    shortcut_hint: Option<&'static str>,
    behavior: ParameterBehavior,
    transform: ValueTransform<T, U>,
    logarithmic: bool,
    _marker: PhantomData<T>,
}

#[derive(Clone, Copy)]
pub struct ValueTransform<T, U> {
    pub to_exposed: fn(&T) -> U,
    pub from_exposed: fn(U) -> T,
}

impl<T> ValueTransform<T, T>
where
    T: Copy,
{
    pub fn identity() -> Self {
        Self {
            to_exposed: |value| *value,
            from_exposed: |value| value,
        }
    }
}

impl<'a, T> SliderParam<'a, T, T>
where
    T: egui::emath::Numeric + Copy,
{
    pub fn new(label: &'static str, range: std::ops::RangeInclusive<T>) -> Self {
        Self {
            label,
            range,
            tooltip: None,
            default: None,
            shortcut_hint: None,
            behavior: ParameterBehavior::StructuralImmediate,
            transform: ValueTransform::identity(),
            logarithmic: false,
            _marker: PhantomData,
        }
    }
}

impl<'a, T, U> SliderParam<'a, T, U>
where
    U: egui::emath::Numeric + Copy,
    T: Copy,
{
    pub fn new_with_transform(
        label: &'static str,
        range: std::ops::RangeInclusive<U>,
        transform: ValueTransform<T, U>,
    ) -> Self {
        Self {
            label,
            range,
            tooltip: None,
            default: None,
            shortcut_hint: None,
            behavior: ParameterBehavior::StructuralImmediate,
            transform,
            logarithmic: false,
            _marker: PhantomData,
        }
    }

    pub fn tooltip(mut self, text: &'a str) -> Self {
        self.tooltip = Some(text);
        self
    }

    pub fn default(mut self, value: T) -> Self {
        self.default = Some(value);
        self
    }

    pub fn shortcut_hint(mut self, text: &'static str) -> Self {
        self.shortcut_hint = Some(text);
        self
    }

    pub fn behavior(mut self, behavior: ParameterBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    pub fn logarithmic(mut self, enabled: bool) -> Self {
        self.logarithmic = enabled;
        self
    }

    pub fn draw(
        &self,
        ui: &mut Ui,
        value: &mut T,
        impact: &mut super::ParameterImpact,
    ) -> Response {
        let mut exposed = (self.transform.to_exposed)(value);
        let slider = egui::Slider::new(&mut exposed, self.range.clone())
            .text(self.label)
            .logarithmic(self.logarithmic);
        let mut resp = ui.add(slider);

        let mut reset_to_default = false;
        if let Some(default) = self.default {
            if resp.secondary_clicked() {
                reset_to_default = true;
            }
            let mut context_reset = false;
            resp.clone().context_menu(|ui| {
                if ui.button("Reset to default").clicked() {
                    context_reset = true;
                    ui.close();
                }
            });
            if context_reset {
                reset_to_default = true;
            }
            if reset_to_default {
                *value = default;
                resp.mark_changed();
                impact.register_behavior(self.behavior);
            }
        }
        if !reset_to_default && resp.changed() {
            *value = (self.transform.from_exposed)(exposed);
            impact.register_behavior(self.behavior);
        }

        let mut tooltip_lines: Vec<String> = Vec::new();
        if let Some(text) = self.tooltip {
            tooltip_lines.push(text.to_owned());
        }
        if self.default.is_some() {
            tooltip_lines.push("Right-click to reset".to_owned());
        }
        if let Some(hint) = self.shortcut_hint {
            tooltip_lines.push(hint.to_owned());
        }
        if !tooltip_lines.is_empty() {
            resp.clone().on_hover_ui(|ui| {
                for line in &tooltip_lines {
                    ui.label(line);
                }
            });
        }

        resp
    }
}

/// Helper for u32 sliders with right-click reset
pub fn u32_cell(
    ui: &mut Ui,
    v: &mut u32,
    range: std::ops::RangeInclusive<u32>,
    reset_to: u32,
) -> Response {
    let mut resp = ui
        .add(
            egui::Slider::new(v, range)
                .clamping(egui::SliderClamping::Edits)
                .step_by(1.0)
                .show_value(true),
        )
        .on_hover_text("Right-click to reset");
    if resp.secondary_clicked() {
        *v = reset_to;
        resp.mark_changed();
    }
    resp
}

/// Update aesthetic parameter in NotesGroup by finding the matching token
pub fn update_notes_group<F>(
    notes: &mut std::collections::BTreeMap<Token, NotesGroup>,
    token: Token,
    update_fn: F,
) where
    F: FnOnce(&mut NotesGroup),
{
    if let Some(ng) = notes.get_mut(&token) {
        update_fn(ng);
    }
}

/// Rescale envelope normalization based on attack/decay values (exported for direct use)
pub fn rescale_envelope(seq: &mut Sequence) {
    let a = 1.0 / seq.attack_decay.0;
    let b = 1.0 / seq.attack_decay.1;
    let rescale_factor = rescale_factor(a, b);
    if rescale_factor.is_normal() {
        seq.normalization = rescale_factor;
    }
}
