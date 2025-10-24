pub mod hover_texts;
mod navigation;
use std::fmt::Display;

use egui::{Grid, RichText, ScrollArea, Slider, TextEdit};

use crate::{
    app::{
        property_panel::{
            hover_texts::{
                ASYM_DETUNE_TEXT, DETERMINISTIC_EXCLUSION_TEXT, DETERMINISTIC_INCLUSION_TEXT,
                DETUNE_SHIFT_TEXT, DETUNE_TEXT, DETUNE_TIME_DEP_TEXT, DETUNE_WEIGHTING_TEXT,
                GROOVE_OFFSET_TEXT, HARMONISE_TEXT, LOOP_LENGTH_TEXT, OCTAVE_TEXT,
                POW_FACT_EVOL_TEXT, POW_FACT_TEXT, RANDOM_EXCLUSION_TEXT, RANDOM_INCLUSION_TEXT,
                REPEAT_TEXT, SHUFFLE_TEXT, SYM_DETUNE_TEXT, TIME_QUANTUM_TEXT, TOLERENCE_TEXT,
                UNISSON_DETUNE_TEXT, VARIATION_INTERVALS_TEXT, VARIATION_STEPS_TEXT,
                VOICE_LAYERS_TEXT,
            },
            navigation::navigation,
        },
        GuiApp, ALL_WAVES, DRUM_WAVES,
    },
    engine::score::{
        default_params::*, sequence::Sequence, track_node::TrackNode, ChorusParams, DetRythm,
        Interval, RdRythm, Rythm,
    },
    layout_left,
    rescale_factor,
    shortcuts::*,
    time_freq::{Freq, Time}, // range_slider::*,
};

#[derive(Clone)]
pub enum Action {
    None,
    Mute,
    Delete,
    Clone,
    Parent,
    FirstChild,
    SelectUp,
    SelectDown,
    MoveUp,
    MoveDown,
    Wrap,
    Promote,
    Dissolve,
    GroupAbove,
    GroupBelow,
}
impl Action {
    pub fn label_and_action(self) -> (&'static str, Self) {
        match self {
            Action::None => ("None", self),
            Action::Delete => ("Delete", self),
            Action::Clone => ("Clone", self),
            Action::Parent => ("Parent", self),
            Action::FirstChild => ("First child", self),
            Action::SelectUp => ("Select up", self),
            Action::SelectDown => ("Select down", self),
            Action::MoveUp => ("Move up", self),
            Action::MoveDown => ("Move down", self),
            Action::Wrap => ("Wrap", self),
            Action::Promote => ("Promote", self),
            Action::Dissolve => ("Dissolve", self),
            Action::GroupAbove => ("Group above", self),
            Action::GroupBelow => ("Group below", self),
            Action::Mute => ("Mute", self),
        }
    }
}

impl GuiApp {
    pub fn property_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("props")
            .min_width(self.property_panel_width.max(240.0))
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    let mut action = Action::None;
                    let mut edited_seq = false;
                    if let Some(sel) = self.selected.clone() {
                        if let Some(track_node_mut) = self.score.track_root.get_mut(&sel) {
                            navigation(ui, &mut action, track_node_mut);
                            if let Some(seq_mut) = track_node_mut.as_seq_mut() {
                                ui.heading(format!("Sequence {:?}", sel));

                                {
                                    ui.horizontal(|ui| {
                                        let vol_resp = slider_with_reset(
                                            ui,
                                            &mut seq_mut.volume,
                                            0.0..=2.0,
                                            "Volume",
                                            Some("+ / - (Shift×10)"),
                                            default_volume(),
                                            false,
                                        )
                                        .on_hover_ui(|ui| {
                                            ui.label(
                                                egui::RichText::new("Hold + / - to change").weak(),
                                            );
                                        });

                                        let kb_changed = ui.ctx().input(|i| {
                                            let step = if i.modifiers.shift { 0.5 } else { 0.05 };
                                            let mut changed = false;
                                            if i.key_down(egui::Key::Plus) {
                                                *&mut seq_mut.volume =
                                                    (*&mut seq_mut.volume + step).min(32.0);
                                                changed = true;
                                            }
                                            if i.key_down(egui::Key::Minus) {
                                                *&mut seq_mut.volume =
                                                    (*&mut seq_mut.volume - step).max(0.0);
                                                changed = true;
                                            }
                                            changed
                                        });

                                        if vol_resp.changed()
                                            || vol_resp.secondary_clicked()
                                            || kb_changed
                                        {
                                            // *vol = *vol_after_kb;
                                            if let Some(ng) = self
                                                .score
                                                .notes
                                                .iter_mut()
                                                .find(|ng| ng.token == seq_mut.token)
                                            {
                                                ng.volume = *&mut seq_mut.volume;
                                            }
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        let mut pan = seq_mut.spacial;

                                        let pan_resp = slider_with_reset(
                                            ui,
                                            &mut pan,
                                            0.0..=1.0,
                                            "Stereo",
                                            None,
                                            default_spacial(),
                                            false,
                                        )
                                        .on_hover_ui(|ui| {
                                            ui.label("0.5 is centered");
                                            ui.label(
                                                egui::RichText::new("Right-click to reset").weak(),
                                            );
                                        });

                                        if pan_resp.changed() || pan_resp.secondary_clicked() {
                                            seq_mut.spacial = pan.clamp(0.0, 1.0);
                                            if let Some(ng) = self
                                                .score
                                                .notes
                                                .iter_mut()
                                                .find(|ng| ng.token == seq_mut.token)
                                            {
                                                ng.spacial = seq_mut.spacial;
                                            }
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        let mut proba = seq_mut.proba;

                                        let proba_resp = slider_with_reset(
                                            ui,
                                            &mut proba,
                                            0.0..=1.0,
                                            "proba",
                                            None,
                                            default_proba(),
                                            false,
                                        )
                                        .on_hover_ui(|ui| {
                                            ui.label(
                                                egui::RichText::new("Right-click to reset").weak(),
                                            );
                                        });

                                        if proba_resp.changed() || proba_resp.secondary_clicked() {
                                            seq_mut.proba = proba.clamp(0.0, 1.0);
                                        }
                                    });
                                }

                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("Wave:");
                                    let mut w_choice = seq_mut.wave_type;

                                    egui::ComboBox::from_id_salt("wave_type_combo")
                                        .selected_text((&w_choice).to_string())
                                        .show_ui(ui, |ui| {
                                            for var in ALL_WAVES.iter() {
                                                ui.selectable_value(
                                                    &mut w_choice,
                                                    *var,
                                                    (&var).to_string(),
                                                );
                                            }
                                        });
                                    if w_choice != seq_mut.wave_type {
                                        seq_mut.wave_type = w_choice;
                                        if let Some(ng) = self
                                            .score
                                            .notes
                                            .iter_mut()
                                            .find(|ng| ng.token == seq_mut.token)
                                        {
                                            ng.wave_type = seq_mut.wave_type;
                                        }
                                    }
                                });

                                ui.collapsing("Envelope", |ui| {
                                    let mut attack = seq_mut.attack_decay.0;
                                    let mut decay = seq_mut.attack_decay.1;
                                    let mut lp_attack = seq_mut.lp_attack_decay.0;
                                    let mut lp_decay = seq_mut.lp_attack_decay.1;

                                    let def = if DRUM_WAVES.contains(&seq_mut.wave_type) {
                                        default_drum_attack_decay()
                                    } else {
                                        default_attack_decay()
                                    };

                                    let attack_resp = slider_with_reset(
                                        ui,
                                        &mut attack,
                                        0.01..=100.0,
                                        "Attack",
                                        None,
                                        def.0,
                                        true,
                                    );
                                    let decay_resp = slider_with_reset(
                                        ui,
                                        &mut decay,
                                        0.01..=100.0,
                                        "Decay",
                                        None,
                                        def.1,
                                        true,
                                    );
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

                                    let changed = attack_resp.changed()
                                        || attack_resp.secondary_clicked()
                                        || decay_resp.changed()
                                        || decay_resp.secondary_clicked()
                                        || lp_attack_resp.changed()
                                        || lp_attack_resp.secondary_clicked()
                                        || lp_decay_resp.changed()
                                        || lp_decay_resp.secondary_clicked();

                                    if changed {
                                        seq_mut.attack_decay = (attack, decay);
                                        seq_mut.lp_attack_decay = (lp_attack, lp_decay);
                                        rescale_envelope(seq_mut);

                                        if let Some(ng) = self
                                            .score
                                            .notes
                                            .iter_mut()
                                            .find(|ng| ng.token == seq_mut.token)
                                        {
                                            ng.attack_decay = seq_mut.attack_decay;
                                            ng.lp_attack_decay = seq_mut.lp_attack_decay;
                                        }
                                    }
                                });

                                ui.collapsing("Bend", |ui| {
                                    let seq_bend = &mut seq_mut.bend;
                                    let mut ng_bend_opt = self
                                        .score
                                        .notes
                                        .iter_mut()
                                        .find(|ng| ng.token == seq_mut.token)
                                        .map(|ng| &mut ng.bend);
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
                                        if let Some(b) = ng_bend_opt.as_deref_mut() {
                                            b.0 = seq_bend.0;
                                        }
                                    }
                                    if speed_resp.changed() {
                                        seq_bend.1 = speed;
                                        if let Some(b) = ng_bend_opt.as_deref_mut() {
                                            b.1 = seq_bend.1;
                                        }
                                    }
                                });
                                ui.collapsing("Vibrato", |ui| {
                                    let token = seq_mut.token;
                                    let seq_vibr = &mut seq_mut.vibrato;
                                    let mut ng_vibr_opt = self
                                        .score
                                        .notes
                                        .iter_mut()
                                        .find(|ng| ng.token == token)
                                        .map(|ng| &mut ng.vibrato);
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
                                    let mag_changed =
                                        mag_resp.changed() || mag_resp.secondary_clicked();
                                    let fq_changed =
                                        fq_resp.changed() || fq_resp.secondary_clicked();

                                    if mag_changed {
                                        seq_vibr.0 = mag_disp * 1e-6;
                                        if let Some(v) = ng_vibr_opt.as_deref_mut() {
                                            v.0 = seq_vibr.0;
                                        }
                                    }
                                    if fq_changed {
                                        seq_vibr.1 = freq;
                                        if let Some(v) = ng_vibr_opt.as_deref_mut() {
                                            v.1 = seq_vibr.1;
                                        }
                                    }
                                });
                                if !DRUM_WAVES.contains(&seq_mut.wave_type) {
                                    let header = ui.collapsing("Chorus (Unison Detune)", |ui| {
                                        let token = seq_mut.token;
                                        let seq_chorus = &mut seq_mut.chorus;
                                        let mut ng_chorus_opt = self
                                            .score
                                            .notes
                                            .iter_mut()
                                            .find(|ng| ng.token == token)
                                            .map(|ng| &mut ng.chorus);

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
                                            seq_chorus.voices = *voices;
                                            seq_chorus.delta = *delta;
                                            seq_chorus.delta_shift = *delta_shift;
                                            seq_chorus.time_dependency = *time_dep;
                                            seq_chorus.sym = *sym;
                                            seq_chorus.asym = *asym;

                                            if let Some(ch) = ng_chorus_opt.as_deref_mut() {
                                                ch.voices = seq_chorus.voices;
                                                ch.delta = seq_chorus.delta;
                                                ch.delta_shift = seq_chorus.delta_shift;
                                                ch.time_dependency = seq_chorus.time_dependency;
                                                ch.sym = seq_chorus.sym;
                                                ch.asym = seq_chorus.asym;
                                            }
                                        }
                                    });
                                    header.header_response.on_hover_text(UNISSON_DETUNE_TEXT);
                                };

                                let header = ui.collapsing("Power factor", |ui| {
                                    let token = seq_mut.token;
                                    let seq_pow = &mut seq_mut.pow_fact;
                                    let mut ng_pow_opt = self
                                        .score
                                        .notes
                                        .iter_mut()
                                        .find(|ng| ng.token == token)
                                        .map(|ng| &mut ng.pow_fact);

                                    let mut initial = seq_pow.0;

                                    let mut evol_disp = Freq(
                                        seq_pow.1.as_hz().signum() * seq_pow.1.as_hz().abs().sqrt(),
                                    );

                                    let def = default_pow_fact();
                                    let def_evol_disp =
                                        Freq(def.1.as_hz().signum() * def.1.as_hz().abs().sqrt());

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

                                    let initial_changed =
                                        initial_resp.changed() || initial_resp.secondary_clicked();
                                    let evol_changed =
                                        evol_resp.changed() || evol_resp.secondary_clicked();

                                    if initial_changed {
                                        seq_pow.0 = initial;
                                        if let Some(p) = ng_pow_opt.as_deref_mut() {
                                            p.0 = seq_pow.0;
                                        }
                                    }
                                    if evol_changed {
                                        seq_pow.1 = Freq(
                                            evol_disp.as_hz().signum()
                                                * evol_disp.as_hz()
                                                * evol_disp.as_hz(),
                                        );
                                        if let Some(p) = ng_pow_opt.as_deref_mut() {
                                            p.1 = seq_pow.1;
                                        }
                                    }
                                });
                                header.header_response.on_hover_text(POW_FACT_TEXT);
                                ui.collapsing("Rythm", |ui| {
                                    {
                                        ui.horizontal(|ui| {
                                            let mut tmp_quantum = seq_mut.time_quantum.clone();
                                            ui.label("Time quantum:")
                                                .on_hover_text(TIME_QUANTUM_TEXT);
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut tmp_quantum.0)
                                                        .range(1..=128),
                                                )
                                                .changed()
                                            {
                                                seq_mut.time_quantum.0 = tmp_quantum.0;
                                                edited_seq ^= true;
                                            };
                                            ui.label("/");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut tmp_quantum.1)
                                                        .range(1..=128),
                                                )
                                                .changed()
                                            {
                                                seq_mut.time_quantum.1 = tmp_quantum.1;
                                                edited_seq ^= true;
                                            };
                                        });
                                    }
                                    ui.separator();
                                    {
                                        let step = Time(
                                            seq_mut.time_quantum.0 as f64
                                                / seq_mut.time_quantum.1 as f64,
                                        );

                                        let mut t_min = seq_mut.t_min.as_secs();
                                        let mut t_max = seq_mut.t_max.as_secs();

                                        egui::CollapsingHeader::new("Sequence position").show(
                                            ui,
                                            |ui| {
                                                ui.add(
                                                    egui::Slider::new(&mut t_min, 0.0..=t_max)
                                                        .text("t_min"),
                                                );
                                                ui.add(
                                                    egui::Slider::new(
                                                        &mut t_max,
                                                        t_min..=seq_mut.loop_len.as_secs(),
                                                    )
                                                    .text("t_max"),
                                                );

                                                t_max =
                                                    t_max.clamp(t_min, seq_mut.loop_len.as_secs());

                                                if (t_min - seq_mut.t_min.as_secs()).abs()
                                                    > f64::EPSILON
                                                {
                                                    seq_mut.t_min =
                                                        step * (Time(t_min) / step).round();
                                                    edited_seq ^= true;
                                                }
                                                if (Time(t_max) - seq_mut.t_max).as_secs().abs()
                                                    > f64::EPSILON
                                                {
                                                    seq_mut.t_max =
                                                        step * (Time(t_max) / step).round();
                                                    edited_seq ^= true;
                                                }
                                            },
                                        );

                                        if !ui.ctx().wants_keyboard_input() {
                                            let (left, right, mods) = ui.ctx().input(|i| {
                                                (
                                                    i.key_pressed(egui::Key::ArrowLeft),
                                                    i.key_pressed(egui::Key::ArrowRight),
                                                    i.modifiers,
                                                )
                                            });

                                            if left || right {
                                                let step = Time(
                                                    seq_mut.time_quantum.0 as f64
                                                        / seq_mut.time_quantum.1 as f64,
                                                );
                                                let dir = if left { -1.0 } else { 1.0 };

                                                let cmd = mods.command;
                                                let alt = mods.alt;

                                                let mut new_min = seq_mut.t_min.as_secs();
                                                let mut new_max = seq_mut.t_max.as_secs();
                                                let s = step.as_secs();

                                                match (cmd, alt) {
                                                    (true, false) => {
                                                        new_min =
                                                            (new_min + dir * s).clamp(0.0, new_max);
                                                    }
                                                    (false, true) => {
                                                        new_max = (new_max + dir * s).clamp(
                                                            new_min,
                                                            seq_mut.loop_len.as_secs(),
                                                        );
                                                    }
                                                    _ => {
                                                        let span = new_max - new_min;
                                                        new_min = (new_min + dir * s).clamp(
                                                            0.0,
                                                            (seq_mut.loop_len.as_secs() - span)
                                                                .max(0.0),
                                                        );
                                                        new_max = (new_min + span)
                                                            .min(seq_mut.loop_len.as_secs());
                                                    }
                                                }

                                                seq_mut.t_min = Time(new_min);
                                                seq_mut.t_max = Time(new_max);
                                                edited_seq ^= true;
                                            }
                                        }
                                    }
                                    ui.separator();
                                    {
                                        ui.label("Rythm inclusions:");
                                        let mut tmp_inclusions = seq_mut.inclusions.clone();
                                        if let Rythm::Rd(_) = tmp_inclusions {
                                            if ui
                                                .button("Use deterministic inclusion generators")
                                                .clicked()
                                            {
                                                seq_mut.inclusions =
                                                    Rythm::Det(DetRythm::default());
                                                edited_seq ^= true;
                                            }
                                        } else {
                                            if ui
                                                .button("Use random inclusion generators")
                                                .clicked()
                                            {
                                                seq_mut.inclusions = Rythm::Rd(RdRythm::default());
                                                edited_seq ^= true;
                                            }
                                        }
                                        match tmp_inclusions {
                                            Rythm::Rd(ref mut rd_rythm) => {
                                                ui.vertical(|ui| {
                                                    ui.label("Random inclusion generators:")
                                                        .on_hover_text(RANDOM_INCLUSION_TEXT);
                                                    ui.horizontal(|ui| {
                                                        ui.label("n:");
                                                        if ui
                                                            .add(
                                                                egui::DragValue::new(
                                                                    &mut rd_rythm.amount,
                                                                )
                                                                .range(0..=rd_rythm.length),
                                                            )
                                                            .changed()
                                                        {
                                                            if let Rythm::Rd(
                                                                ref mut edited_rd_rythm,
                                                            ) = seq_mut.inclusions
                                                            {
                                                                edited_rd_rythm.amount = rd_rythm
                                                                    .amount
                                                                    .min(rd_rythm.length);
                                                                edited_seq ^= true;
                                                            }
                                                        };
                                                        ui.label("N:");
                                                        if ui
                                                            .add(
                                                                egui::DragValue::new(
                                                                    &mut rd_rythm.length,
                                                                )
                                                                .range(rd_rythm.amount..=512),
                                                            )
                                                            .changed()
                                                        {
                                                            if let Rythm::Rd(
                                                                ref mut edited_rd_rythm,
                                                            ) = seq_mut.inclusions
                                                            {
                                                                edited_rd_rythm.length = rd_rythm
                                                                    .length
                                                                    .max(rd_rythm.amount);
                                                                edited_seq ^= true;
                                                            }
                                                        };
                                                    });
                                                });
                                            }
                                            Rythm::Det(det_rythm) => {
                                                ui.vertical(|ui| {
                                                    ui.label("Deterministic inclusion generators:")
                                                        .on_hover_text(
                                                            DETERMINISTIC_INCLUSION_TEXT,
                                                        );
                                                    let mut gens = det_rythm.generators;
                                                    let old_val = gens.clone();
                                                    Self::edit_vec(ui, &mut gens, 2, layout_left());
                                                    if gens != old_val {
                                                        // TODO: do better
                                                        if let Rythm::Det(
                                                            ref mut edited_det_rythm,
                                                        ) = seq_mut.inclusions
                                                        {
                                                            edited_det_rythm.generators = gens
                                                                .into_iter()
                                                                .filter(|g| *g > 0)
                                                                .collect();
                                                            edited_seq ^= true;
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                    ui.separator();
                                    {
                                        let mut tmp_exclusions = seq_mut.exclusions.clone();
                                        if let Rythm::Rd(_) = tmp_exclusions {
                                            ui.label("Rythm exclusions:");
                                            if ui
                                                .button("Use deterministic exclusion generators")
                                                .clicked()
                                            {
                                                seq_mut.exclusions =
                                                    Rythm::Det(DetRythm::default());
                                                edited_seq ^= true;
                                            }
                                        } else {
                                            if ui
                                                .button("Use random exclusion generators")
                                                .clicked()
                                            {
                                                seq_mut.exclusions = Rythm::Rd(RdRythm::default());
                                                edited_seq ^= true;
                                            }
                                        }
                                        match tmp_exclusions {
                                            Rythm::Rd(ref mut rd_rythm) => {
                                                ui.vertical(|ui| {
                                                    ui.label("Random exclusion generators:")
                                                        .on_hover_text(RANDOM_EXCLUSION_TEXT);
                                                    ui.horizontal(|ui| {
                                                        ui.label("n:");
                                                        if ui
                                                            .add(
                                                                egui::DragValue::new(
                                                                    &mut rd_rythm.amount,
                                                                )
                                                                .range(0..=512),
                                                            )
                                                            .changed()
                                                        {
                                                            if let Rythm::Rd(
                                                                ref mut edited_rd_rythm,
                                                            ) = seq_mut.exclusions
                                                            {
                                                                edited_rd_rythm.amount = rd_rythm
                                                                    .amount
                                                                    .min(rd_rythm.length);
                                                                edited_seq ^= true;
                                                            }
                                                        };
                                                        ui.label("N:");
                                                        if ui
                                                            .add(
                                                                egui::DragValue::new(
                                                                    &mut rd_rythm.length,
                                                                )
                                                                .range(0..=512),
                                                            )
                                                            .changed()
                                                        {
                                                            if let Rythm::Rd(
                                                                ref mut edited_rd_rythm,
                                                            ) = seq_mut.exclusions
                                                            {
                                                                edited_rd_rythm.length = rd_rythm
                                                                    .length
                                                                    .max(rd_rythm.amount);
                                                                edited_seq ^= true;
                                                            }
                                                        };
                                                    });
                                                });
                                            }
                                            Rythm::Det(det_rythm) => {
                                                ui.vertical(|ui| {
                                                    ui.label("Deterministic exclusion generators:")
                                                        .on_hover_text(
                                                            DETERMINISTIC_EXCLUSION_TEXT,
                                                        );
                                                    let mut gens = det_rythm.generators;
                                                    let old_val = gens.clone();
                                                    Self::edit_vec(ui, &mut gens, 2, layout_left());
                                                    if gens != old_val {
                                                        // TODO: do better
                                                        if let Rythm::Det(
                                                            ref mut edited_det_rythm,
                                                        ) = seq_mut.exclusions
                                                        {
                                                            edited_det_rythm.generators = gens
                                                                .into_iter()
                                                                .filter(|g| *g > 1)
                                                                .collect();
                                                            edited_seq ^= true;
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                        ui.separator();
                                        ui.horizontal(|ui| {
                                            let mut tmp_beat_offset = seq_mut.beat_offset.clone();
                                            ui.label("Groove offset:")
                                                .on_hover_text(GROOVE_OFFSET_TEXT);
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut tmp_beat_offset)
                                                        .range(0..=256),
                                                )
                                                .changed()
                                            {
                                                seq_mut.beat_offset = tmp_beat_offset;
                                                edited_seq ^= true;
                                            };
                                        });
                                        ui.horizontal(|ui| {
                                            let mut loop_len = seq_mut.loop_len.clone();
                                            ui.label("Loop length:")
                                                .on_hover_text(LOOP_LENGTH_TEXT);
                                            let slider = ui.add(
                                                egui::DragValue::new(&mut loop_len)
                                                    .range(0.0..=512.0),
                                            );
                                            if slider.changed() {
                                                loop_len = loop_len.max(Time(0.0));
                                                seq_mut.loop_len = loop_len.max(Time(0.0));
                                                seq_mut.t_max = seq_mut.t_max.min(loop_len);
                                                edited_seq ^= true;
                                            };
                                            let mut repeat = seq_mut.repeat.clone();
                                            ui.label("Repeat:").on_hover_text(REPEAT_TEXT);
                                            let slider = ui.add(
                                                egui::DragValue::new(&mut repeat).range(1..=64),
                                            );
                                            if slider.changed() {
                                                seq_mut.repeat = repeat;
                                                edited_seq ^= true;
                                            };
                                        });
                                    }
                                });
                                ui.collapsing("Harmony", |ui| {
                                    ui.checkbox(&mut seq_mut.glide, "Glide");
                                    let mut harmonise = seq_mut.harmonise;
                                    if ui
                                        .checkbox(&mut harmonise, "Harmonise")
                                        .on_hover_text(HARMONISE_TEXT)
                                        .changed()
                                    {
                                        seq_mut.harmonise = harmonise;
                                        edited_seq = true
                                    }
                                    ui.separator();
                                    {
                                        ui.horizontal(|ui| {
                                            let mut tmp_tolerance = seq_mut.tolerance.clone();
                                            ui.label("Tolerance:").on_hover_text(TOLERENCE_TEXT);
                                            ui.label("<-");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut tmp_tolerance.0)
                                                        .range(-4.0..=16.0),
                                                )
                                                .changed()
                                            {
                                                seq_mut.tolerance.0 = tmp_tolerance.0;
                                                edited_seq ^= true;
                                            };
                                            ui.label(",");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut tmp_tolerance.1)
                                                        .range(-4.0..=16.0),
                                                )
                                                .changed()
                                            {
                                                seq_mut.tolerance.1 = tmp_tolerance.1;
                                                edited_seq ^= true;
                                            };
                                            ui.label("->");
                                        });
                                    }

                                    {
                                        let mut changed = false;
                                        let mut interval = seq_mut.interval.clone();
                                        let mut shuffle = seq_mut.shuffle;
                                        if let Interval::RDTempered(
                                            ref mut nb_rd_steps,
                                            ref mut tones,
                                            ref mut octave,
                                        ) = interval
                                        {
                                            // octave
                                            ui.horizontal(|ui| {
                                                ui.label("Octave:").on_hover_text(OCTAVE_TEXT);
                                                if ui
                                                    .add(egui::Slider::new(octave, -4..=4))
                                                    .changed()
                                                {
                                                    changed = true;
                                                };
                                            });
                                            if !harmonise {
                                                // nb_rd_steps
                                                ui.horizontal(|ui| {
                                                    ui.label("Variation steps:")
                                                        .on_hover_text(VARIATION_STEPS_TEXT);
                                                    if ui
                                                        .add(egui::Slider::new(nb_rd_steps, 0..=16))
                                                        .changed()
                                                    {
                                                        changed = true;
                                                    };
                                                });
                                                ui.label("Variation intervals:")
                                                    .on_hover_text(VARIATION_INTERVALS_TEXT);
                                                ui.horizontal_wrapped(|ui| {
                                                    for tone in -11..=11 {
                                                        let mut selected = tones.contains(&tone);

                                                        if ui
                                                            .checkbox(
                                                                &mut selected,
                                                                tone.to_string(),
                                                            )
                                                            .changed()
                                                        {
                                                            if selected {
                                                                if !tones.contains(&tone) {
                                                                    tones.push(tone);
                                                                    tones.sort_unstable();
                                                                }
                                                            } else {
                                                                if let Some(pos) = tones
                                                                    .iter()
                                                                    .position(|&v| v == tone)
                                                                {
                                                                    tones.remove(pos);
                                                                }
                                                            }
                                                            changed = !tones.is_empty();
                                                        }
                                                    }
                                                });
                                            } else {
                                                ui.collapsing("Harmoniser", |ui| {
                                                    let mut harmoniser = seq_mut.harmoniser; // [[u32;7];2]
                                                    let mut changed = false;

                                                    ui.label(
                                                        RichText::new(
                                                            "Weights per interval (0..=6)",
                                                        )
                                                        .weak(),
                                                    );
                                                    ui.add_space(4.0);

                                                    Grid::new("harmoniser_grid_inverted")
                                                        .striped(true)
                                                        .num_columns(3) // interval label + 2 modes
                                                        .show(ui, |ui| {
                                                            // Header
                                                            ui.label(
                                                                RichText::new("Interval").weak(),
                                                            );
                                                            ui.label(
                                                                RichText::new("Overlapping").weak(),
                                                            );
                                                            ui.label(
                                                                RichText::new("Non-overlapping")
                                                                    .weak(),
                                                            );
                                                            ui.end_row();

                                                            // Rows: one per interval
                                                            for i in 0..=6 {
                                                                ui.label(format!("{i}"));
                                                                for j in 0..=1 {
                                                                    let r = u32_cell(
                                                                        ui,
                                                                        &mut harmoniser[j][i],
                                                                        0..=32,
                                                                        0,
                                                                    );
                                                                    if r.changed() {
                                                                        changed = true;
                                                                    }
                                                                }
                                                                ui.end_row();
                                                            }
                                                        });

                                                    ui.horizontal_wrapped(|ui| {
                                                        if ui.button("Reset all to 16").clicked() {
                                                            harmoniser = [[16; 7]; 2];
                                                            changed = true;
                                                        }
                                                        if ui.button("Reset defaults").clicked() {
                                                            harmoniser = default_harmoniser();
                                                            changed = true;
                                                        }
                                                        if ui.button("Copy Overlapping to Non-overlapping").clicked() {
                                                            harmoniser[1] = harmoniser[0];
                                                            changed = true;
                                                        }
                                                        if ui.button("Copy Non-overlapping to Overlapping").clicked() {
                                                            harmoniser[0] = harmoniser[1];
                                                            changed = true;
                                                        }
                                                    });

                                                    if changed {
                                                        seq_mut.harmoniser = harmoniser;
                                                    }
                                                });
                                            }
                                            ui.separator();
                                            changed |= ui
                                                .checkbox(&mut shuffle, "Shuffle")
                                                .on_hover_text(SHUFFLE_TEXT)
                                                .changed();
                                        }
                                        if changed {
                                            seq_mut.interval = interval;
                                            seq_mut.shuffle = shuffle;
                                            edited_seq = true;
                                        }
                                    }
                                });
                                ui.collapsing("Accents", |ui| {
                                    let def = default_accents();
                                    let mut mag_disp =
                                        1.0 / seq_mut.accents.0.max(f64::MIN_POSITIVE);
                                    let mag_resp = slider_with_reset(
                                        ui,
                                        &mut mag_disp,
                                        0.01..=100.0,
                                        "Magnitude",
                                        None,
                                        1.0 / def.0.max(f64::MIN_POSITIVE),
                                        true,
                                    );
                                    if mag_resp.changed() {
                                        seq_mut.accents.0 = 1.0 / mag_disp.max(f64::MIN_POSITIVE);
                                    }

                                    let mut gens_disp: Vec<f64> = seq_mut
                                        .accents
                                        .1
                                        .iter()
                                        .map(|&x| 1.0 / x.max(f64::MIN_POSITIVE))
                                        .collect();

                                    let old = gens_disp.clone();
                                    ui.label("Generators");
                                    Self::edit_vec(ui, &mut gens_disp, 1.0, layout_left());

                                    if gens_disp != old && !gens_disp.iter().any(|&v| v == 0.0) {
                                        let restored: Vec<f64> = gens_disp
                                            .into_iter()
                                            .map(|x| 1.0 / x.max(f64::MIN_POSITIVE))
                                            .collect();
                                        seq_mut.accents.1 = restored;
                                    }
                                });
                            } else {
                                ui.heading(format!("Group {:?}", sel));
                                match track_node_mut {
                                    TrackNode::Group {
                                        name,
                                        collapsed,
                                        volume,
                                        proba,
                                        ..
                                    } => {
                                        // Name field
                                        ui.horizontal(|ui| {
                                            ui.label("Name:");
                                            let resp = ui.add(
                                                TextEdit::singleline(name)
                                                    .hint_text("Group name…")
                                                    .desired_width(200.0),
                                            );
                                            // Optional: commit on Enter
                                            if resp.lost_focus()
                                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                            {
                                                ui.memory_mut(|m| m.surrender_focus(resp.id));
                                            }
                                        });

                                        // Collapse / expand
                                        if ui
                                            .button(if *collapsed {
                                                "Uncollapse"
                                            } else {
                                                "Collapse"
                                            })
                                            .on_hover_ui(|ui| {
                                                ui.label(RichText::new(shortcut(COLLAPSE)).weak());
                                            })
                                            .clicked()
                                            || (!ui.ctx().wants_keyboard_input()
                                                && ui.input(|i| i.key_pressed(COLLAPSE)))
                                        {
                                            *collapsed = !*collapsed;
                                        }

                                        // Group volume
                                        ui.add(Slider::new(volume, 0.0..=5.0).text("Volume"));
                                        // Group proba
                                        ui.add(Slider::new(proba, 0.0..=1.0).text("Proba"));
                                    }
                                    TrackNode::Seq(_) => unreachable!(),
                                }
                            }
                        }
                    } else {
                        ui.label("Click a block to edit");
                    }

                    if let Some(mut sel) = self.selected.clone() {
                        match action {
                            Action::None => {}
                            Action::Delete => {
                                // Compute selection target BEFORE deletion
                                let (idx, parent_path, siblings) = {
                                    let idx = *sel.last().unwrap();
                                    let parent_path = &sel[..sel.len().saturating_sub(1)];
                                    let siblings = self
                                        .score
                                        .track_root
                                        .get(parent_path)
                                        .map(|p| p.child_count())
                                        .unwrap_or(0);
                                    (idx, parent_path.to_vec(), siblings)
                                };

                                // Perform deletion
                                self.del_node(&sel);

                                // Decide new selection
                                self.selected = if siblings > 1 {
                                    // There will be at least one sibling left after deletion
                                    let mut p = parent_path.clone();
                                    // Prefer next sibling at the same index (which now points to what was "next")
                                    let new_idx = if idx < siblings - 1 {
                                        idx
                                    } else {
                                        idx.saturating_sub(1)
                                    };
                                    p.push(new_idx);
                                    Some(p)
                                } else {
                                    // No siblings left: select the parent (or None if we deleted the only root child)
                                    if parent_path.is_empty() {
                                        None
                                    } else {
                                        Some(parent_path)
                                    }
                                };
                            }
                            Action::Clone => {
                                // Clone the selected node (implementation assumed: inserts right after original)
                                self.clone_node(&sel);

                                // Move selection to the new clone (original index + 1)
                                if let Some(last) = sel.last_mut() {
                                    *last += 1;
                                }
                                self.selected = Some(sel);
                            }
                            Action::MoveUp => {
                                if let Some(new_path) = self.score.swap_with_prev(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::MoveDown => {
                                if let Some(new_path) = self.score.swap_with_next(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::Wrap => {
                                if let Some(new_path) =
                                    self.score.wrap_into_group_at(&sel, "Group".into())
                                {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::Promote => {
                                if let Some(new_path) = self.score.promote_one_rank(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::Dissolve => {
                                if let Some(new_path) = self.score.dissolve_group_at(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::GroupAbove => {
                                if let Some(new_path) = self.score.move_into_prev_group(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::GroupBelow => {
                                if let Some(new_path) = self.score.move_into_next_group(&sel) {
                                    self.selected = Some(new_path);
                                }
                            }
                            Action::SelectUp => {
                                if let Some(ref path) = self.selected {
                                    self.selected = self.score.prev_sibling(path, true);
                                }
                            }
                            Action::SelectDown => {
                                if let Some(ref path) = self.selected {
                                    self.selected = self.score.next_sibling(path, true);
                                }
                            }
                            Action::Parent => {
                                self.selected = self.score.parent_of(&sel);
                            }
                            Action::FirstChild => {
                                self.selected = self.score.first_child_of(&sel);
                            }
                            Action::Mute => {
                                self.score
                                    .track_root
                                    .get_mut(&sel)
                                    .as_mut()
                                    .map(|track_node| {
                                        track_node.toggle_mute();
                                        edited_seq = true;
                                    });
                            }
                        }
                    }
                    if edited_seq {
                        if let Some(sel) = self.selected.clone() {
                            if ui.input(|i| !i.pointer.button_down(egui::PointerButton::Primary)) {
                                let mut sequence =
                                    self.score.track_root.get_mut(&sel).unwrap().clone();
                                if let Some(Sequence {
                                    ref mut not_generate_until,
                                    ..
                                }) = sequence.as_seq_mut()
                                {
                                    *not_generate_until = None;
                                }
                                self.edit_node_at(sequence, &sel);
                            }
                        }
                    }
                });
                self.property_panel_width = ui.available_width();
            });
    }
}
fn rescale_envelope(e: &mut Sequence) {
    let a = 1.0 / e.attack_decay.0;
    let b = 1.0 / e.attack_decay.1;
    let rescale_factor = rescale_factor(a, b);
    if rescale_factor.is_normal() {
        e.normalization = rescale_factor
    }
}
fn slider_with_reset<'a, N>(
    ui: &mut egui::Ui,
    value: &'a mut N,
    range: std::ops::RangeInclusive<N>,
    label: &str,
    shortcut: Option<&str>,
    reset_to: N,
    log: bool,
) -> egui::Response
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
/// Small helper: u32 slider with right-click reset to `reset_to`.
fn u32_cell(
    ui: &mut egui::Ui,
    v: &mut u32,
    range: std::ops::RangeInclusive<u32>,
    reset_to: u32,
) -> egui::Response {
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
        // mark as changed so caller can detect it
        resp.mark_changed();
    }
    resp
}
