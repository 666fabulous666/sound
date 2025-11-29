use super::super::ParameterImpact;
use crate::engine::score::sequence::Sequence;
use crate::time_freq::Beat;
use egui::Ui;

pub fn show_position_section(ui: &mut Ui, seq: &mut Sequence, impact: &mut ParameterImpact) {
    ui.heading("Position");

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
