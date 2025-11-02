use egui::{Response, Ui};
use std::fmt::Display;

use super::hover_texts::{
    ASYM_DETUNE_TEXT, DETUNE_SHIFT_TEXT, DETUNE_TEXT, DETUNE_TIME_DEP_TEXT,
    DETUNE_WEIGHTING_TEXT, POW_FACT_EVOL_TEXT, SYM_DETUNE_TEXT, VOICE_LAYERS_TEXT,
};
use crate::{
    engine::score::{default_params::*, sequence::Sequence, ChorusParams, NotesGroup},
    rescale_factor,
    time_freq::Freq,
    Token,
};

/// Helper for sliders with right-click reset functionality
pub fn slider_with_reset<'a, N>(
    ui: &mut Ui,
    value: &'a mut N,
    range: std::ops::RangeInclusive<N>,
    label: &str,
    shortcut: Option<&str>,
    reset_to: N,
    log: bool,
) -> Response
where
    N: egui::emath::Numeric + Copy + Display,
{
    let mut resp = ui
        .add(egui::Slider::new(value, range).text(label).logarithmic(log))
        .on_hover_ui(|ui| {
            ui.label(egui::RichText::new(format!("Right-click to reset to {}", reset_to)).weak());
            if let Some(shortcut) = shortcut {
                ui.label(egui::RichText::new(format!("Shortcut: {}", shortcut)).weak());
            }
        });
    if resp.secondary_clicked() {
        *value = reset_to;
        resp.mark_changed();
    }
    resp
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
pub fn update_notes_group<F>(notes: &mut [NotesGroup], token: Token, update_fn: F)
where
    F: FnOnce(&mut NotesGroup),
{
    if let Some(ng) = notes.iter_mut().find(|ng| ng.token == token) {
        update_fn(ng);
    }
}

/// Handle volume slider with keyboard shortcuts and aesthetic update
pub fn volume_control(
    ui: &mut Ui,
    volume: &mut f64,
    notes: &mut [NotesGroup],
    token: Token,
) -> bool {
    let vol_resp = slider_with_reset(
        ui,
        volume,
        0.0..=2.0,
        "Volume",
        Some("+ / - (Shift×10)"),
        default_volume(),
        false,
    )
    .on_hover_ui(|ui| {
        ui.label(egui::RichText::new("Hold + / - to change").weak());
    });

    let kb_changed = ui.ctx().input(|i| {
        let step = if i.modifiers.shift { 0.5 } else { 0.05 };
        let mut changed = false;
        if i.key_down(egui::Key::Plus) {
            *volume = (*volume + step).min(32.0);
            changed = true;
        }
        if i.key_down(egui::Key::Minus) {
            *volume = (*volume - step).max(0.0);
            changed = true;
        }
        changed
    });

    let changed = vol_resp.changed() || vol_resp.secondary_clicked() || kb_changed;
    if changed {
        update_notes_group(notes, token, |ng| {
            ng.volume = *volume;
        });
    }
    changed
}

/// Handle spacial/pan slider with aesthetic update
pub fn spacial_control(
    ui: &mut Ui,
    spacial: &mut f64,
    notes: &mut [NotesGroup],
    token: Token,
) -> bool {
    let pan_resp = slider_with_reset(
        ui,
        spacial,
        0.0..=1.0,
        "Stereo",
        None,
        default_spacial(),
        false,
    )
    .on_hover_ui(|ui| {
        ui.label("0.5 is centered");
        ui.label(egui::RichText::new("Right-click to reset").weak());
    });

    let changed = pan_resp.changed() || pan_resp.secondary_clicked();
    if changed {
        *spacial = spacial.clamp(0.0, 1.0);
        update_notes_group(notes, token, |ng| {
            ng.pan = *spacial;
        });
    }
    changed
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

/// Handle envelope section (attack, decay, lowpass)
pub fn envelope_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    notes: &mut [NotesGroup],
    is_drum: bool,
) {
    let mut attack = seq.attack_decay.0;
    let mut decay = seq.attack_decay.1;

    let def = if is_drum {
        default_drum_attack_decay()
    } else {
        default_attack_decay()
    };

    let attack_resp = slider_with_reset(ui, &mut attack, 0.01..=100.0, "Attack", None, def.0, true);
    let decay_resp = slider_with_reset(ui, &mut decay, 0.01..=100.0, "Decay", None, def.1, true);

    // Lowpass filter checkbox
    let lowpass_checkbox = ui.checkbox(&mut seq.lowpass_enabled, "Enable Lowpass Filter");

    let mut lp_changed = false;
    if seq.lowpass_enabled {
        let mut lp_attack = seq.lp_attack_decay.0;
        let mut lp_decay = seq.lp_attack_decay.1;
        let mut lp_cutoff_multiplier = seq.cutoff_multiplier;
        let mut lp_order = seq.lp_order;

        let lp_order_resp = u32_cell(ui, &mut lp_order, 1..=5, default_lp_order())
            .on_hover_text("Filter order (1-5): higher order = steeper rolloff");

        let lp_attack_resp = slider_with_reset(
            ui,
            &mut lp_attack,
            0.01..=100.0,
            "Lowpass Attack",
            None,
            def.0,
            true,
        );
        let lp_decay_resp = slider_with_reset(
            ui,
            &mut lp_decay,
            0.01..=100.0,
            "Lowpass Decay",
            None,
            def.1,
            true,
        );
        let lp_cutoff_multiplier_resp = slider_with_reset(
            ui,
            &mut lp_cutoff_multiplier,
            0.01..=100.0,
            "Lowpass cutoff muliplier",
            None,
            def.1,
            true,
        );

        lp_changed = lp_order_resp.changed()
            || lp_order_resp.secondary_clicked()
            || lp_attack_resp.changed()
            || lp_attack_resp.secondary_clicked()
            || lp_decay_resp.changed()
            || lp_decay_resp.secondary_clicked()
            || lp_cutoff_multiplier_resp.changed()
            || lp_cutoff_multiplier_resp.secondary_clicked();

        if lp_changed {
            seq.lp_order = lp_order;
            seq.lp_attack_decay = (lp_attack, lp_decay);
            seq.cutoff_multiplier = lp_cutoff_multiplier;
        }
    }

    let changed = attack_resp.changed()
        || attack_resp.secondary_clicked()
        || decay_resp.changed()
        || decay_resp.secondary_clicked()
        || lowpass_checkbox.changed()
        || lp_changed;

    if changed {
        seq.attack_decay = (attack, decay);
        rescale_envelope(seq);

        update_notes_group(notes, seq.token, |ng| {
            ng.attack_decay = seq.attack_decay;
            ng.lp_attack_decay = seq.lp_attack_decay;
            ng.cutoff_multiplier = seq.cutoff_multiplier;
            ng.lowpass_enabled = seq.lowpass_enabled;
            ng.lp_order = seq.lp_order;
        });
    }
}

/// Handle bend controls with aesthetic update
pub fn bend_section(ui: &mut Ui, seq: &mut Sequence, notes: &mut [NotesGroup]) {
    let seq_bend = &mut seq.bend;
    let mut mag_disp = seq_bend.0 * 1e4;
    let mut speed = seq_bend.1;

    let mag_resp = slider_with_reset(
        ui,
        &mut mag_disp,
        -200.0..=200.0,
        "Magnitude",
        None,
        default_bend().0 * 1e4,
        false,
    );
    let speed_resp = slider_with_reset(
        ui,
        &mut speed,
        1.0..=1000.0,
        "Speed",
        None,
        default_bend().1,
        true,
    );

    if mag_resp.changed() {
        seq_bend.0 = mag_disp * 1e-4;
        update_notes_group(notes, seq.token, |ng| {
            ng.bend.0 = seq_bend.0;
        });
    }
    if speed_resp.changed() {
        seq_bend.1 = speed;
        update_notes_group(notes, seq.token, |ng| {
            ng.bend.1 = seq_bend.1;
        });
    }
}

/// Handle vibrato controls with aesthetic update
pub fn vibrato_section(ui: &mut Ui, seq: &mut Sequence, notes: &mut [NotesGroup]) {
    let seq_vibr = &mut seq.vibrato;
    let mut mag_disp = seq_vibr.0 * 1e6;
    let mut freq = seq_vibr.1;

    let mag_resp = slider_with_reset(
        ui,
        &mut mag_disp,
        0.0..=1000.0,
        "Magnitude",
        None,
        default_vibrato().0 * 1e6,
        false,
    );
    let fq_resp = slider_with_reset(
        ui,
        &mut freq,
        Freq(1.0)..=Freq(100.0),
        "Frequency",
        None,
        default_vibrato().1,
        true,
    );

    if mag_resp.changed() || mag_resp.secondary_clicked() {
        seq_vibr.0 = mag_disp * 1e-6;
        update_notes_group(notes, seq.token, |ng| {
            ng.vibrato.0 = seq_vibr.0;
        });
    }
    if fq_resp.changed() || fq_resp.secondary_clicked() {
        seq_vibr.1 = freq;
        update_notes_group(notes, seq.token, |ng| {
            ng.vibrato.1 = seq_vibr.1;
        });
    }
}

/// Handle chorus controls with aesthetic update
pub fn chorus_section(ui: &mut Ui, seq: &mut Sequence, notes: &mut [NotesGroup]) {
    let seq_chorus = &mut seq.chorus;

    let voices = &mut seq_chorus.voices;
    let delta = &mut seq_chorus.delta;
    let delta_shift = &mut seq_chorus.delta_shift;
    let time_dep = &mut seq_chorus.time_dependency;
    let sym = &mut seq_chorus.sym;
    let asym = &mut seq_chorus.asym;

    let voices_resp = slider_with_reset(
        ui,
        voices,
        1..=10,
        "Voice layers",
        None,
        ChorusParams::default().voices,
        false,
    )
    .on_hover_text(VOICE_LAYERS_TEXT);

    let delta_resp = slider_with_reset(
        ui,
        delta,
        0.0..=1.0,
        "Detune (Δf)",
        None,
        ChorusParams::default().delta,
        true,
    )
    .on_hover_text(DETUNE_TEXT);

    let delta_shift_resp = slider_with_reset(
        ui,
        delta_shift,
        -1.0..=1.0,
        "Detune shift",
        None,
        ChorusParams::default().delta_shift,
        false,
    )
    .on_hover_text(DETUNE_SHIFT_TEXT);

    let time_dep_resp = slider_with_reset(
        ui,
        time_dep,
        Freq(-5.0)..=Freq(5.0),
        "Detune over time",
        None,
        ChorusParams::default().time_dependency,
        false,
    )
    .on_hover_text(DETUNE_TIME_DEP_TEXT);

    ui.label("Weighting (around f₀)")
        .on_hover_text(DETUNE_WEIGHTING_TEXT);

    let sym_resp = slider_with_reset(
        ui,
        sym,
        -2.0..=2.0,
        "Even",
        None,
        ChorusParams::default().sym,
        false,
    )
    .on_hover_text(SYM_DETUNE_TEXT);

    let asym_resp = slider_with_reset(
        ui,
        asym,
        -2.0..=2.0,
        "Odd",
        None,
        ChorusParams::default().asym,
        false,
    )
    .on_hover_text(ASYM_DETUNE_TEXT);

    let changed = voices_resp.changed()
        || voices_resp.secondary_clicked()
        || delta_resp.changed()
        || delta_resp.secondary_clicked()
        || delta_shift_resp.changed()
        || delta_shift_resp.secondary_clicked()
        || time_dep_resp.changed()
        || time_dep_resp.secondary_clicked()
        || sym_resp.changed()
        || sym_resp.secondary_clicked()
        || asym_resp.changed()
        || asym_resp.secondary_clicked();

    if changed {
        update_notes_group(notes, seq.token, |ng| {
            ng.chorus.voices = *voices;
            ng.chorus.delta = *delta;
            ng.chorus.delta_shift = *delta_shift;
            ng.chorus.time_dependency = *time_dep;
            ng.chorus.sym = *sym;
            ng.chorus.asym = *asym;
        });
    }
}

/// Handle power factor controls with aesthetic update
pub fn power_factor_section(ui: &mut Ui, seq: &mut Sequence, notes: &mut [NotesGroup]) {
    let seq_pow = &mut seq.pow_fact;
    let mut initial = seq_pow.0;
    let mut evol_disp = Freq(seq_pow.1.as_hz().signum() * seq_pow.1.as_hz().abs().sqrt());

    let def = default_pow_fact();
    let def_evol_disp = Freq(def.1.as_hz().signum() * def.1.as_hz().abs().sqrt());

    let initial_resp = slider_with_reset(
        ui,
        &mut initial,
        0.0..=1000.0,
        "Initial value",
        None,
        def.0,
        true,
    );
    let evol_resp = slider_with_reset(
        ui,
        &mut evol_disp,
        Freq(-10.0)..=Freq(10.0),
        "Evolution",
        None,
        def_evol_disp,
        false,
    )
    .on_hover_text(POW_FACT_EVOL_TEXT);

    if initial_resp.changed() || initial_resp.secondary_clicked() {
        seq_pow.0 = initial;
        update_notes_group(notes, seq.token, |ng| {
            ng.pow_fact.0 = seq_pow.0;
        });
    }
    if evol_resp.changed() || evol_resp.secondary_clicked() {
        seq_pow.1 = Freq(evol_disp.as_hz().signum() * evol_disp.as_hz() * evol_disp.as_hz());
        update_notes_group(notes, seq.token, |ng| {
            ng.pow_fact.1 = seq_pow.1;
        });
    }
}
