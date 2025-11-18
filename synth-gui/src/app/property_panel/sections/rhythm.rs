use super::super::{
    helpers::{ParameterBehavior, SliderParam},
    ParameterImpact,
};
use crate::app::property_panel::hover_texts::{
    GROOVE_OFFSET_TEXT, LOOP_LENGTH_TEXT, REPEAT_TEXT, TIME_QUANTUM_TEXT,
};
use crate::app::property_panel::rhythm;
use crate::engine::score::{default_params::default_tail_multiplier, sequence::Sequence};
use crate::time_freq::Beat;
use egui::Ui;
use std::vec::Vec;

pub fn show_rhythm_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    impact: &mut ParameterImpact,
    edit_vec_fn: &dyn Fn(&mut Ui, &mut Vec<usize>, usize),
) {
    ui.collapsing("Rythm", |ui| {
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

        let step = seq.time_quantum.beat_step();
        let mut t_min = seq.t_min.as_beats();
        let mut t_max = seq.t_max.as_beats();

        egui::CollapsingHeader::new("Sequence position (beats)").show(ui, |ui| {
            ui.add(egui::Slider::new(&mut t_min, 0.0..=t_max).text("t_min"));
            ui.add(egui::Slider::new(&mut t_max, t_min..=seq.loop_len.as_beats()).text("t_max"));

            t_max = t_max.clamp(t_min, seq.loop_len.as_beats());

            if (t_min - seq.t_min.as_beats()).abs() > f64::EPSILON {
                let quantized = step * ((Beat(t_min) / step).round());
                seq.t_min = quantized;
                impact.require_regeneration();
            }
            if (t_max - seq.t_max.as_beats()).abs() > f64::EPSILON {
                let quantized = step * ((Beat(t_max) / step).round());
                seq.t_max = quantized;
                impact.require_regeneration();
            }
        });

        if !ui.ctx().wants_keyboard_input() {
            let (left, right, mods) = ui.ctx().input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowLeft),
                    i.key_pressed(egui::Key::ArrowRight),
                    i.modifiers,
                )
            });
            if left || right {
                let dir = if left { -1.0 } else { 1.0 };
                let mut new_min = seq.t_min.as_beats();
                let mut new_max = seq.t_max.as_beats();
                let s = step.as_beats();

                match (mods.command, mods.alt) {
                    (true, false) => {
                        new_min = (new_min + dir * s).clamp(0.0, new_max);
                    }
                    (false, true) => {
                        new_max = (new_max + dir * s).clamp(new_min, seq.loop_len.as_beats());
                    }
                    _ => {
                        let span = new_max - new_min;
                        new_min = (new_min + dir * s)
                            .clamp(0.0, (seq.loop_len.as_beats() - span).max(0.0));
                        new_max = (new_min + span).min(seq.loop_len.as_beats());
                    }
                }

                seq.t_min = Beat(new_min);
                seq.t_max = Beat(new_max);
                impact.require_regeneration();
            }
        }

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
        ui.horizontal(|ui| {
            let mut loop_len = seq.loop_len.as_beats();
            ui.label("Loop length (beats):")
                .on_hover_text(LOOP_LENGTH_TEXT);
            let loop_slider = ui.add(egui::DragValue::new(&mut loop_len).range(0.0..=512.0));
            if loop_slider.changed() {
                let loop_len = Beat(loop_len.max(0.0));
                seq.loop_len = loop_len;
                seq.t_max = seq.t_max.min(loop_len);
                impact.require_regeneration();
            };
            let mut repeat = seq.repeat;
            ui.label("Repeat:").on_hover_text(REPEAT_TEXT);
            let repeat_slider = ui.add(egui::DragValue::new(&mut repeat).range(1..=64));
            if repeat_slider.changed() {
                seq.repeat = repeat;
                impact.require_regeneration();
            };
        });
        let tail_param = SliderParam::new("Tail multiplier", 1.0..=10.0)
            .default(default_tail_multiplier())
            .behavior(ParameterBehavior::StructuralImmediate)
            .tooltip("Extends the final note duration for each window");
        tail_param.draw(ui, &mut seq.tail_multiplier, impact);
    });
}
