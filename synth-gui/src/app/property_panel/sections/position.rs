use super::super::ParameterImpact;
use crate::app::property_panel::hover_texts::{LOOP_LENGTH_TEXT, LOOP_OFFSET_TEXT, REPEAT_TEXT};
use crate::engine::score::sequence::Sequence;
use crate::time_freq::Beat;
use egui::Ui;

pub fn show_position_section(ui: &mut Ui, seq: &mut Sequence, impact: &mut ParameterImpact) {
    ui.heading("Position");

    // Loop length and repeat
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

    // Loop offset
    ui.horizontal(|ui| {
        let mut loop_offset = seq.loop_offset.as_beats();
        ui.label("Loop offset (beats):")
            .on_hover_text(LOOP_OFFSET_TEXT);
        if ui
            .add(egui::DragValue::new(&mut loop_offset).range(0.0..=512.0).speed(0.1))
            .changed()
        {
            seq.loop_offset = Beat(loop_offset.max(0.0));
            impact.require_regeneration();
        };
    });

    ui.separator();

    // Sequence position (t_min / t_max)
    let step = seq.time_quantum.beat_step();
    let mut t_min = seq.t_min.as_beats();
    let mut t_max = seq.t_max.as_beats();

    ui.label("Sequence position (beats):");
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
}
