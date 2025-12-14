use super::super::{
    helpers::{ParameterBehavior, SliderParam},
    ParameterImpact,
};
use crate::app::property_panel::hover_texts::{GROOVE_OFFSET_TEXT, TIME_QUANTUM_TEXT};
use crate::app::property_panel::rhythm;
use crate::engine::score::{default_params::default_tail_multiplier, sequence::Sequence};
use egui::Ui;
use std::vec::Vec;

pub fn show_rhythm_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    impact: &mut ParameterImpact,
    edit_vec_fn: &dyn Fn(&mut Ui, &mut Vec<usize>, usize),
    promote: Option<&mut dyn FnMut()>,
) {
    ui.heading("Rhythm");
    ui.horizontal(|ui| {
        let mut numerator = seq.time_quantum.numerator();
        let mut denominator = seq.time_quantum.denominator();
        ui.label("Time quantum:").on_hover_text(TIME_QUANTUM_TEXT);
        if ui
            .add(egui::DragValue::new(&mut numerator).range(1..=128))
            .changed()
        {
            seq.time_quantum
                .set_numerator(numerator)
                .expect("UI enforces valid numerator");
            impact.require_regeneration();
        };
        ui.label("/");
        if ui
            .add(egui::DragValue::new(&mut denominator).range(1..=128))
            .changed()
        {
            seq.time_quantum
                .set_denominator(denominator)
                .expect("UI enforces valid denominator");
            impact.require_regeneration();
        };
    });

    ui.separator();

    if rhythm::inclusion_section(ui, seq, |ui, gens, default| {
        edit_vec_fn(ui, gens, default);
    }) {
        impact.require_regeneration();
    }

    ui.separator();

    if rhythm::exclusion_section(ui, seq, |ui, gens, default| {
        edit_vec_fn(ui, gens, default);
    }) {
        impact.require_regeneration();
    }

    ui.separator();
    ui.horizontal(|ui| {
        let mut tmp_beat_offset = seq.beat_offset;
        ui.label("Groove offset:").on_hover_text(GROOVE_OFFSET_TEXT);
        if ui
            .add(egui::DragValue::new(&mut tmp_beat_offset).range(0..=256))
            .changed()
        {
            seq.beat_offset = tmp_beat_offset;
            impact.require_regeneration();
        };
    });
    let tail_param = SliderParam::new("Tail multiplier", 1.0..=10.0)
        .default(default_tail_multiplier())
        .behavior(ParameterBehavior::StructuralImmediate)
        .tooltip("Extends the final note duration for each window");
    tail_param.draw(ui, &mut seq.tail_multiplier, impact);

    if let Some(promote) = promote {
        if ui
            .small_button("Promote rhythm to parent override")
            .on_hover_text("Copy this rhythm to the parent override and clear it here")
            .clicked()
        {
            promote();
        }
    }
}
