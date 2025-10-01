use egui::{RichText, ScrollArea};
// use egui_double_slider::DoubleSlider;

use crate::{
    app::{GuiApp, ALL_WAVES, DRUM_WAVES},
    engine::notes::{
        default_params::*, ChorusParams, DetRythm, Interval, RdRythm, Rythm, Sequence,
    },
    layout_left,
    rescale_factor,
    shortcuts::*,
    time_freq::{Freq, Time}, // range_slider::*,
};

impl GuiApp {
    pub fn property_panel(&mut self, ctx: &egui::Context) {
        let len = self.sequences.len();
        egui::SidePanel::left("props")
            .min_width(self.property_panel_width.max(240.0))
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    enum Action {
                        None,
                        Delete,
                        Clone,
                        Up,
                        Down,
                    }

                    let mut action = Action::None;
                    let mut edited_seq: Option<Sequence> = None;

                    if let Some(sel) = self.selected {
                        if sel < len {
                            let seq = (&mut self.sequences)[sel].clone();

                            ui.heading(format!("Track {}", sel + 1));
                            ui.horizontal(|ui| {
                                if ui
                                    .button("Delete")
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new(shortcut(DELETE)).weak());
                                    })
                                    .clicked()
                                    || ui.input(|i| i.key_pressed(DELETE))
                                {
                                    action = Action::Delete;
                                }
                                if ui
                                    .button("Clone")
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new(shortcut(CLONE)).weak());
                                    })
                                    .clicked()
                                    || ui.input(|i| i.key_pressed(CLONE))
                                {
                                    action = Action::Clone;
                                }
                                if (ui
                                    .button("Up")
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new(shortcut(SWAP_UP)).weak());
                                    })
                                    .clicked()
                                    || ui.input(|i| i.key_pressed(SWAP_UP)))
                                    && sel > 0
                                {
                                    action = Action::Up;
                                }
                                if (ui
                                    .button("Down")
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new(shortcut(SWAP_DOWN)).weak());
                                    })
                                    .clicked()
                                    || ui.input(|i| i.key_pressed(SWAP_DOWN)))
                                    && sel + 1 < len
                                {
                                    action = Action::Down;
                                }
                                if ui
                                    .button(if seq.mute { "Unute" } else { "Mute" })
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new(shortcut(MUTE)).weak());
                                    })
                                    .clicked()
                                    || ui.input(|i| i.key_pressed(MUTE))
                                {
                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .mute ^= true;
                                }
                            });

                                                        {

                                ui.horizontal(|ui| {
                                    let seq_mut = &mut self.sequences[sel];
                                    let mut vol = seq_mut.volume;

                                    let vol_resp = slider_with_reset(
                                        ui,
                                        &mut vol,
                                        0.0..=32.0,
                                        "Volume",
                                        Some("+ / - (Shift×10)"),
                                        default_volume(),
                                        false,
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label(egui::RichText::new("Hold + / - to change").weak());
                                    });

                                    let (kb_changed, vol_after_kb) = ui.ctx().input(|i| {
                                        let step = if i.modifiers.shift { 0.5 } else { 0.05 };
                                        let mut v = vol;
                                        let mut changed = false;
                                        if i.key_down(egui::Key::Plus) {
                                            v = (v + step).min(32.0);
                                            changed = true;
                                        }
                                        if i.key_down(egui::Key::Minus) {
                                            v = (v - step).max(0.0);
                                            changed = true;
                                        }
                                        (changed, v)
                                    });

                                    if vol_resp.changed() || vol_resp.secondary_clicked() || kb_changed {
                                        seq_mut.volume = vol_after_kb;
                                    }
                                });

                                ui.horizontal(|ui| {
                                    let seq_mut = &mut self.sequences[sel];
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
                                        ui.label(egui::RichText::new("Right-click to reset").weak());
                                    });

                                    if pan_resp.changed() || pan_resp.secondary_clicked() {
                                        seq_mut.spacial = pan.clamp(0.0, 1.0);
                                        if let Some(ng) = self.notes.iter_mut().find(|ng| ng.token == seq.token) {
                                            ng.spacial = seq_mut.spacial;
                                        }
                                    }
                                });
                            }

                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Wave:");
                                let mut w_choice = seq.wave_type;

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
                                if w_choice != seq.wave_type {
                                    let e = edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone());
                                    e.wave_type = w_choice;
                                    if DRUM_WAVES.contains(&w_choice) {
                                        e.attack_decay = default_drum_attack_decay();
                                        e.normalization = default_drum_normalization();
                                    }
                                }
                            });
                            {
                                // compute step once
                                let step =
                                    Time(seq.time_quantum.0 as f64 / seq.time_quantum.1 as f64);

                                // ---- UI (runs only when open) ----
                                let mut t_min = seq.t_min.as_secs();
                                let mut t_max = seq.t_max.as_secs();

                                egui::CollapsingHeader::new("Sequence position").show(ui, |ui| {
                                    ui.add(
                                        egui::Slider::new(&mut t_min, 0.0..=t_max).text("t_min"),
                                    );
                                    ui.add(
                                        egui::Slider::new(
                                            &mut t_max,
                                            t_min..=seq.loop_len.as_secs(),
                                        )
                                        .text("t_max"),
                                    );

                                    t_max = t_max.clamp(t_min, seq.loop_len.as_secs());

                                    if (t_min - seq.t_min.as_secs()).abs() > f64::EPSILON {
                                        edited_seq
                                            .get_or_insert(self.sequences[sel].clone())
                                            .t_min = step * (Time(t_min) / step).round();
                                    }
                                    if (Time(t_max) - seq.t_max).as_secs().abs() > f64::EPSILON {
                                        edited_seq
                                            .get_or_insert(self.sequences[sel].clone())
                                            .t_max = step * (Time(t_max) / step).round();
                                    }
                                });

                                // Only react to arrows if no text field wants the keyboard:
                                if !ui.ctx().wants_keyboard_input() {
                                    // Read keys & modifiers in one go
                                    let (left, right, mods) = ui.ctx().input(|i| {
                                        (
                                            i.key_pressed(egui::Key::ArrowLeft),
                                            i.key_pressed(egui::Key::ArrowRight),
                                            i.modifiers,
                                        )
                                    });

                                    if left || right {
                                        let step = Time(
                                            seq.time_quantum.0 as f64 / seq.time_quantum.1 as f64,
                                        );
                                        let dir = if left { -1.0 } else { 1.0 };

                                        // Logical "command": Ctrl on Windows/Linux, ⌘ on macOS.
                                        let cmd = mods.command; // <- egui maps this for you
                                        let alt = mods.alt;

                                        // Start from the current values
                                        let mut new_min = seq.t_min.as_secs();
                                        let mut new_max = seq.t_max.as_secs();
                                        let s = step.as_secs();

                                        match (cmd, alt) {
                                            // Only t_min (Ctrl / Command)
                                            (true, false) => {
                                                new_min = (new_min + dir * s).clamp(0.0, new_max);
                                            }
                                            // Only t_max (Alt)
                                            (false, true) => {
                                                new_max = (new_max + dir * s)
                                                    .clamp(new_min, seq.loop_len.as_secs());
                                            }
                                            // Both or none → move window together
                                            _ => {
                                                // Shift both; keep span, clamp to [0, loop_len]
                                                let span = new_max - new_min;
                                                new_min = (new_min + dir * s).clamp(
                                                    0.0,
                                                    (seq.loop_len.as_secs() - span).max(0.0),
                                                );
                                                new_max =
                                                    (new_min + span).min(seq.loop_len.as_secs());
                                            }
                                        }

                                        let e =
                                            edited_seq.get_or_insert(self.sequences[sel].clone());
                                        e.t_min = Time(new_min);
                                        e.t_max = Time(new_max);
                                    }
                                }
                            }
                            ui.collapsing("Envelope", |ui| {
                                let mut attack = self.sequences[sel].attack_decay.0;
                                let mut decay  = self.sequences[sel].attack_decay.1;

                                let def = if DRUM_WAVES.contains(&seq.wave_type) {
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

                                let changed =
                                    attack_resp.changed() || attack_resp.secondary_clicked() ||
                                    decay_resp.changed()  || decay_resp.secondary_clicked();

                                if changed {
                                    let seq_mut = &mut self.sequences[sel];
                                    seq_mut.attack_decay = (attack, decay);
                                    rescale_envelope(seq_mut);

                                    if let Some(ng) = self.notes.iter_mut().find(|ng| ng.token == seq.token) {
                                        ng.attack_decay = seq_mut.attack_decay;
                                    }
                                }
                            });

                            ui.collapsing("Bend", |ui| {
                                let seq_bend = &mut self.sequences[sel].bend; // (f64, f64)
                                let mut ng_bend_opt = self
                                    .notes
                                    .iter_mut()
                                    .find(|ng| ng.token == seq.token)
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
                                let token = self.sequences[sel].token;
                                let seq_vibr = &mut self.sequences[sel].vibrato;
                                let mut ng_vibr_opt = self
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
                                let fq_changed = fq_resp.changed() || fq_resp.secondary_clicked();

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
                            if !DRUM_WAVES.contains(&seq.wave_type) {
                                let header = ui.collapsing("Chorus (Unison Detune)", |ui| {
                                    let token = self.sequences[sel].token;
                                    let seq_chorus = &mut self.sequences[sel].chorus;
                                    let mut ng_chorus_opt = self
                                        .notes
                                        .iter_mut()
                                        .find(|ng| ng.token == token)
                                        .map(|ng| &mut ng.chorus);

                                    // voices (int slider; no log)
                                    let mut voices = seq_chorus.voices;
                                    let voices_resp = ui
                                        .add(
                                            egui::Slider::new(&mut voices, 1..=10)
                                                .text("Voice layers"),
                                        )
                                        .on_hover_text(concat!(
                                            "Each layer adds one detuned voice\n",
                                            "above and below f₀.\n",
                                            "Voices total = 1 + 2 × (layers − 1).",
                                        ));
                                    if voices_resp.secondary_clicked() {
                                        voices = ChorusParams::default().voices;
                                    }

                                    // delta (Detune), delta_shift, time_dependency, sym, asym
                                    let mut delta = seq_chorus.delta;
                                    let mut delta_shift = seq_chorus.delta_shift;
                                    let mut time_dep = seq_chorus.time_dependency;
                                    let mut sym = seq_chorus.sym;
                                    let mut asym = seq_chorus.asym;

                                    let delta_resp = slider_with_reset(
                                        ui,
                                        &mut delta,
                                        0.0..=1.0,
                                        "Detune (Δf)",
                                        None,
                                        ChorusParams::default().delta,
                                        true,
                                    )
                                    .on_hover_text(concat!(
                                        "Detune amount between voices around f₀.\n",
                                        "\n",
                                        "Use very small values for slow beating;\n",
                                        "increase for a wider chorus.",
                                    ));

                                    let delta_shift_resp = slider_with_reset(
                                        ui,
                                        &mut delta_shift,
                                        -1.0..=1.0,
                                        "Detune shift",
                                        None,
                                        ChorusParams::default().delta_shift,
                                        false,
                                    )
                                    .on_hover_text("Shift voices frequencies asymmetrically to avoid beatings.");

                                    let time_dep_resp = slider_with_reset(
                                        ui,
                                        &mut time_dep,
                                        Freq(-5.0)..=Freq(5.0),
                                        "Detune over time",
                                        None,
                                        ChorusParams::default().time_dependency,
                                        false,
                                    )
                                    .on_hover_text(concat!(
                                        "Modulates Δf over time.\n",
                                        " > 0 : Δf increases over time.\n",
                                        " < 0 : Δf decreases over time.\n",
                                        " = 0 : static detune."
                                    ));

                                    ui.label("Weighting (around f₀)").on_hover_text(concat!(
                                        "Sets how much outer voices contribute relative to the center.\n",
                                        " • |value| > 1 -> outer voices amplified\n",
                                        " • |value| = 1 -> constant voice levels\n",
                                        " • |value| < 1 -> outer voices attenuated\n",
                                        " •  value < 0  -> outer voices inverted in phase"
                                    ));

                                    let sym_resp = slider_with_reset(
                                        ui,
                                        &mut sym,
                                        -2.0..=2.0,
                                        "Even",
                                        None,
                                        ChorusParams::default().sym,
                                        false,
                                    )
                                    .on_hover_text("Even (symmetric) weighting across ±Δf");

                                    let asym_resp = slider_with_reset(
                                        ui,
                                        &mut asym,
                                        -2.0..=2.0,
                                        "Odd",
                                        None,
                                        ChorusParams::default().asym,
                                        false,
                                    )
                                    .on_hover_text("Odd (asymmetric) weighting across ±Δf.");

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
                                        seq_chorus.voices = voices;
                                        seq_chorus.delta = delta;
                                        seq_chorus.delta_shift = delta_shift;
                                        seq_chorus.time_dependency = time_dep;
                                        seq_chorus.sym = sym;
                                        seq_chorus.asym = asym;

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
                                header.header_response.on_hover_text(concat!(
                                    "Adds multiple voices detuned\n",
                                    "around the main frequency f₀\n",
                                    "to create width and motion.",
                                ));

                                let header = ui.collapsing("Power factor", |ui| {
                                    let token = self.sequences[sel].token;
                                    let seq_pow = &mut self.sequences[sel].pow_fact; // (f64, Freq)
                                    let mut ng_pow_opt = self
                                        .notes
                                        .iter_mut()
                                        .find(|ng| ng.token == token)
                                        .map(|ng| &mut ng.pow_fact);

                                    // initial value (native)
                                    let mut initial = seq_pow.0;

                                    // evolution displayed with signed sqrt mapping
                                    let mut evol_disp = Freq(
                                        seq_pow.1.as_hz().signum() * seq_pow.1.as_hz().abs().sqrt()
                                    );

                                    // defaults (display mapping for evolution)
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
                                    .on_hover_text("Increase/Decrease over time.");

                                    let initial_changed = initial_resp.changed() || initial_resp.secondary_clicked();
                                    let evol_changed    = evol_resp.changed()    || evol_resp.secondary_clicked();

                                    if initial_changed {
                                        seq_pow.0 = initial;
                                        if let Some(p) = ng_pow_opt.as_deref_mut() {
                                            p.0 = seq_pow.0;
                                        }
                                    }
                                    if evol_changed {
                                        seq_pow.1 = Freq(evol_disp.as_hz().signum() * evol_disp.as_hz() * evol_disp.as_hz());
                                        if let Some(p) = ng_pow_opt.as_deref_mut() {
                                            p.1 = seq_pow.1;
                                        }
                                    }
                                });
                                header.header_response.on_hover_text(concat!(
                                    "Produces distortion or metallic timbre\n",
                                    "\n",
                                    " • |value| = 0 -> square wave\n",
                                    " • |value| < 1 -> distortion\n",
                                    " • |value| = 1 -> unchanged wave\n",
                                    " • |value| > 1 -> metallic",
                                ));

                            };
                            ui.collapsing("Rythm", |ui| {
                                {
                                    ui.horizontal(|ui| {
                                        let mut tmp_quantum = seq.time_quantum.clone();
                                        ui.label("Time quantum:").on_hover_text(concat!(
                                            "Duration of the base time unit for beats.\n",
                                            "\n",
                                            "Rhythm inclusions and exclusions are tested\n",
                                            "for divisibility against this quantum."
                                        ));
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut tmp_quantum.0)
                                                    .range(1..=128),
                                            )
                                            .changed()
                                        {
                                            edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone())
                                                .time_quantum
                                                .0 = tmp_quantum.0;
                                        };
                                        ui.label("/");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut tmp_quantum.1)
                                                    .range(1..=128),
                                            )
                                            .changed()
                                        {
                                            edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone())
                                                .time_quantum
                                                .1 = tmp_quantum.1;
                                        };
                                    });
                                }
                                ui.separator();
                                {
                                    ui.label("Rythm inclusions:");
                                    let mut tmp_inclusions = seq.inclusions.clone();
                                    if let Rythm::Rd(_) = tmp_inclusions {
                                        if ui
                                            .button("Use deterministic inclusion generators")
                                            .clicked()
                                        {
                                            edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone())
                                                .inclusions = Rythm::Det(DetRythm::default());
                                        }
                                    } else {
                                        if ui.button("Use random inclusion generators").clicked() {
                                            edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone())
                                                .inclusions = Rythm::Rd(RdRythm::default());
                                        }
                                    }
                                    match tmp_inclusions {
                                        Rythm::Rd(ref mut rd_rythm) => {
                                            ui.vertical(|ui| {
                                                ui.label("Random inclusion generators:")
                                                    .on_hover_text(concat!(
                                                    "Rules that randomly place beats.\n",
                                                    "\n",
                                                    "A set of n inclusion generators is picked\n",
                                                    "randomly from [1, N].\n",
                                                    "\n",
                                                    "Any beat whose time unit is a multiple of\n",
                                                    "one of these values will be included."
                                                ));
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
                                                        if let Rythm::Rd(ref mut edited_rd_rythm) =
                                                            edited_seq
                                                                .get_or_insert(
                                                                    (&mut self.sequences)[sel]
                                                                        .clone(),
                                                                )
                                                                .inclusions
                                                        {
                                                            edited_rd_rythm.amount = rd_rythm
                                                                .amount
                                                                .min(rd_rythm.length);
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
                                                        if let Rythm::Rd(ref mut edited_rd_rythm) =
                                                            edited_seq
                                                                .get_or_insert(
                                                                    (&mut self.sequences)[sel]
                                                                        .clone(),
                                                                )
                                                                .inclusions
                                                        {
                                                            edited_rd_rythm.length = rd_rythm
                                                                .length
                                                                .max(rd_rythm.amount);
                                                        }
                                                    };
                                                });
                                            });
                                        }
                                        Rythm::Det(det_rythm) => {
                                            ui.vertical(|ui| {
                                                ui.label("Deterministic inclusion generators:")
                                                    .on_hover_text(concat!(
                                                    "Rules that deterministically place beats.\n",
                                                    "\n",
                                                    "Choose inclusion generators: any beat whose\n",
                                                    "time unit is a multiple of one of these\n",
                                                    "values will be included.\n",
                                                    "\n",
                                                    "Generators ≤ 1 are ignored; use values > 1.",
                                                ));
                                                let mut gens = det_rythm.generators;
                                                let old_val = gens.clone();
                                                Self::edit_vec(ui, &mut gens, 2, layout_left());
                                                if gens != old_val {
                                                    // TODO: do better
                                                    if let Rythm::Det(ref mut edited_det_rythm) =
                                                        edited_seq
                                                            .get_or_insert(
                                                                (&mut self.sequences)[sel].clone(),
                                                            )
                                                            .inclusions
                                                    {
                                                        edited_det_rythm.generators = gens
                                                            .into_iter()
                                                            .filter(|g| *g > 0)
                                                            .collect();
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                                ui.separator();
                                {
                                    let mut tmp_exclusions = seq.exclusions.clone();
                                    if let Rythm::Rd(_) = tmp_exclusions {
                                        ui.label("Rythm exclusions:");
                                        if ui
                                            .button("Use deterministic exclusion generators")
                                            .clicked()
                                        {
                                            edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone())
                                                .exclusions = Rythm::Det(DetRythm::default());
                                        }
                                    } else {
                                        if ui.button("Use random exclusion generators").clicked() {
                                            edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone())
                                                .exclusions = Rythm::Rd(RdRythm::default());
                                        }
                                    }
                                    match tmp_exclusions {
                                        Rythm::Rd(ref mut rd_rythm) => {
                                            ui.vertical(|ui| {
                                                ui.label("Random exclusion generators:")
                                                    .on_hover_text(concat!(
                                                    "Rules that randomly skip beats.\n",
                                                    "\n",
                                                    "A set of n exclusion generators is picked\n",
                                                    "randomly from [2, N+1].\n",
                                                    "\n",
                                                    "Any beat whose time unit shifted forward\n",
                                                    "by 1 is a multiple of one of these\n",
                                                    "values will be excluded, ensuring the\n",
                                                    "first beat is never excluded.\n",
                                                    "\n",
                                                    "(Generator 1 is not allowed,\n",
                                                    "as it would exclude every beat.)"
                                                ));
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
                                                        if let Rythm::Rd(ref mut edited_rd_rythm) =
                                                            edited_seq
                                                                .get_or_insert(
                                                                    (&mut self.sequences)[sel]
                                                                        .clone(),
                                                                )
                                                                .exclusions
                                                        {
                                                            edited_rd_rythm.amount = rd_rythm
                                                                .amount
                                                                .min(rd_rythm.length);
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
                                                        if let Rythm::Rd(ref mut edited_rd_rythm) =
                                                            edited_seq
                                                                .get_or_insert(
                                                                    (&mut self.sequences)[sel]
                                                                        .clone(),
                                                                )
                                                                .exclusions
                                                        {
                                                            edited_rd_rythm.length = rd_rythm
                                                                .length
                                                                .max(rd_rythm.amount);
                                                        }
                                                    };
                                                });
                                            });
                                        }
                                        Rythm::Det(det_rythm) => {
                                            ui.vertical(|ui| {
                                                ui.label("Deterministic exclusion generators:")
                                                    .on_hover_text(concat!(
                                                    "Rules that deterministically skip beats.\n",
                                                    "\n",
                                                    "Choose exclusion generators: any beat whose\n",
                                                    "time unit shifted forward by 1 is\n",
                                                    "a multiple of one of these\n",
                                                    "values will be excluded.\n",
                                                    "\n",
                                                    "Beats are tested with their time unit\n",
                                                    "shifted forward by 1, ensuring\n",
                                                    "the first beat is never excluded.\n",
                                                    "\n",
                                                    "Generators ≤ 1 are ignored; use values > 1."
                                                ));
                                                let mut gens = det_rythm.generators;
                                                let old_val = gens.clone();
                                                Self::edit_vec(ui, &mut gens, 2, layout_left());
                                                if gens != old_val {
                                                    // TODO: do better
                                                    if let Rythm::Det(ref mut edited_det_rythm) =
                                                        edited_seq
                                                            .get_or_insert(
                                                                (&mut self.sequences)[sel].clone(),
                                                            )
                                                            .exclusions
                                                    {
                                                        edited_det_rythm.generators = gens
                                                            .into_iter()
                                                            .filter(|g| *g > 1)
                                                            .collect();
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        let mut tmp_beat_offset = seq.beat_offset.clone();
                                        ui.label("Groove offset:").on_hover_text(concat!(
                                            "Shifts the rhythmic grid used to place notes.\n",
                                            "\n",
                                            "It offsets the index of the\n",
                                            "time quanta tested for divisibility.\n",
                                            "\n",
                                            "This changes where note onsets are more likely\n",
                                            "to occur, creating an off-beat feel.\n",
                                            "\n",
                                            "Expressed in the unit of the time quantum.",
                                        ));
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut tmp_beat_offset)
                                                    .range(0..=256),
                                            )
                                            .changed()
                                        {
                                            edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone())
                                                .beat_offset = tmp_beat_offset;
                                        };
                                    });
                                    ui.horizontal(|ui| {
                                        let mut loop_len = seq.loop_len.clone();
                                        ui.label("Loop length:").on_hover_text(concat!(
                                            "Length of the loop for this sequence.\n",
                                            "\n",
                                            "When the end is reached, playback jumps\n",
                                            "back to zero immediately, independent of\n",
                                            "the loop lengths of other sequences."
                                        ));
                                        let slider = ui.add(
                                            egui::DragValue::new(&mut loop_len).range(0.0..=512.0),
                                        );
                                        if slider.changed() {
                                            loop_len = loop_len.max(Time(0.0));
                                            let tmp_edited_seq = edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone());
                                            tmp_edited_seq.loop_len = loop_len.max(Time(0.0));
                                            tmp_edited_seq.t_max =
                                                tmp_edited_seq.t_max.min(loop_len);
                                        };
                                        let mut repeat = seq.repeat.clone();
                                        ui.label("Repeat:").on_hover_text(concat!(
                                            "How many times the sequence will be repeated.\n",
                                        ));
                                        let slider =
                                            ui.add(egui::DragValue::new(&mut repeat).range(1..=64));
                                        if slider.changed() {
                                            let tmp_edited_seq = edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone());
                                            tmp_edited_seq.repeat = repeat;
                                        };
                                    });
                                }
                            });
                            ui.collapsing("Harmony", |ui| {
                                {
                                    ui.horizontal(|ui| {
                                        let mut tmp_tolerance = seq.tolerance.clone();
                                        ui.label("Tolerance:").on_hover_text(concat!(
                                            "Tolerance defines how much to look\n",
                                            "before the note starts and after it ends.\n",
                                            "\n",
                                            "Use this to follow notes across their edges\n",
                                            "while generating new notes.\n",
                                            "Negative values are allowed.\n",
                                            "\n",
                                            "(See generating logics for more details)",
                                        ));
                                        ui.label("<-");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut tmp_tolerance.0)
                                                    .range(-4.0..=16.0),
                                            )
                                            .changed()
                                        {
                                            edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone())
                                                .tolerance
                                                .0 = tmp_tolerance.0;
                                        };
                                        ui.label(",");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut tmp_tolerance.1)
                                                    .range(-4.0..=16.0),
                                            )
                                            .changed()
                                        {
                                            edited_seq
                                                .get_or_insert((&mut self.sequences)[sel].clone())
                                                .tolerance
                                                .1 = tmp_tolerance.1;
                                        };
                                        ui.label("->");
                                    });
                                }

                                {
                                    let mut changed = false;
                                    let mut interval = seq.interval.clone();
                                    let mut shuffle = seq.shuffle;
                                    if let Interval::RDTempered(
                                        ref mut nb_rd_steps,
                                        ref mut tones,
                                        ref mut octave,
                                    ) = interval
                                    {
                                        // octave
                                        ui.horizontal(|ui| {
                                            ui.label("Octave:").on_hover_text(
                                            "Base octave where notes of this sequence are placed.",
                                        );
                                            if ui.add(egui::Slider::new(octave, -4..=4)).changed() {
                                                changed = true;
                                            };
                                        });
                                        // nb_rd_steps
                                        ui.horizontal(|ui| {
                                            ui.label("Variation steps:").on_hover_text(concat!(
                                                "Maximum number of random variations to apply.\n",
                                                "\n",
                                                "The note is chosen from visible ones\n",
                                                "(based on tolerance),\n",
                                                "then shifted step by step using\n",
                                                "the allowed intervals.\n",
                                                "\n",
                                                "Higher values allow more chained shifts."
                                            ));
                                            if ui
                                                .add(egui::Slider::new(nb_rd_steps, 0..=16))
                                                .changed()
                                            {
                                                changed = true;
                                            };
                                        });
                                        // ----- RDTempered tones (–11 … 11) ---------------------------------
                                        ui.label("Variation intervals:").on_hover_text(concat!(
                                            "The set of semitone intervals used for variation.\n",
                                            "\n",
                                            "Each step shifts the note by one of these values.\n",
                                            "Multiple steps can combine, wrapping around octaves\n",
                                            "(12 semitones)."
                                        ));
                                        ui.horizontal_wrapped(|ui| {
                                            for tone in -11..=11 {
                                                let mut selected = tones.contains(&tone);

                                                if ui
                                                    .checkbox(&mut selected, tone.to_string())
                                                    .changed()
                                                {
                                                    if selected {
                                                        if !tones.contains(&tone) {
                                                            tones.push(tone);
                                                            tones.sort_unstable();
                                                        }
                                                    } else {
                                                        if let Some(pos) =
                                                            tones.iter().position(|&v| v == tone)
                                                        {
                                                            tones.remove(pos);
                                                        }
                                                    }
                                                    changed = !tones.is_empty();
                                                }
                                            }
                                        });
                                        ui.separator();
                                        changed |= ui
                                            .checkbox(&mut shuffle, "Shuffle")
                                            .on_hover_text(concat!(
                                                "Generate notes from the sequence in a\n",
                                                "random order, affecting which notes follow\n",
                                                "one another.\n",
                                                "\n",
                                                "A note may only follow a previously\n",
                                                "generated other one (see tolerance for\n",
                                                "more settings about this point.",
                                            ))
                                            .changed()
                                    }
                                    if changed {
                                        let e = edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone());
                                        e.interval = interval;
                                        e.shuffle = shuffle;
                                    }
                                }
                            });
                            ui.collapsing("Accents", |ui| {
                                let mut accents = seq.accents;

                                let mut mag_val = 1.0 / accents.0.max(f64::MIN_POSITIVE);
                                let base_slider = ui.add(
                                    egui::Slider::new(&mut mag_val, 0.01..=100.0)
                                        .text("Magnitude")
                                        .logarithmic(true),
                                );
                                if base_slider.changed() {
                                    accents.0 = 1.0 / mag_val;
                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .accents = accents.clone();
                                }

                                let mut gens: Vec<f64> = accents
                                    .1
                                    .iter()
                                    .map(|&x| 1.0 / x.max(f64::MIN_POSITIVE))
                                    .collect();

                                let old_gens = gens.clone();
                                ui.label("Generators");
                                Self::edit_vec(
                                    ui,
                                    &mut gens,
                                    // Some("Generators"),
                                    1.0,
                                    layout_left(),
                                );

                                if gens != old_gens && !gens.contains(&0.0) {
                                    let restored: Vec<f64> = gens
                                        .into_iter()
                                        .map(|x| 1.0 / x.max(f64::MIN_POSITIVE))
                                        .collect();

                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .accents
                                        .1 = restored;
                                }
                            });
                        }
                    } else {
                        ui.label("Click a block to edit");
                    }

                    // -------- perform structural edit after UI borrow ends --------
                    match action {
                        Action::None => {}
                        Action::Delete => {
                            if let Some(sel) = self.selected {
                                self.del_seq(sel);
                                self.selected = if sel > 0 {
                                    Some(sel - 1)
                                } else if self.sequences.len() > 1 {
                                    Some(sel)
                                } else {
                                    None
                                };
                            }
                        }
                        Action::Clone => {
                            if let Some(sel) = self.selected {
                                self.clone_seq(sel);
                                let last = self.sequences.len() - 1;
                                for k in (sel + 1..last).rev() {
                                    self.swap_seqs_at(k + 1, k);
                                }
                            }
                        }
                        Action::Up => {
                            if let Some(sel) = self.selected {
                                self.swap_seqs_at(sel, sel - 1);
                                self.selected = Some(sel - 1);
                            }
                        }
                        Action::Down => {
                            if let Some(sel) = self.selected {
                                self.swap_seqs_at(sel, sel + 1);
                                self.selected = Some(sel + 1);
                            }
                        }
                    }
                    if let Some(edited_seq) = edited_seq {
                        if let Some(sel) = self.selected {
                            if ui.input(|i| !i.pointer.button_down(egui::PointerButton::Primary)) {
                                self.edit_seq_at(edited_seq, sel);
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
    N: egui::emath::Numeric + Copy,
{
    let resp = ui
        .add(egui::Slider::new(value, range).text(label).logarithmic(log))
        .on_hover_ui(|ui| {
            ui.label(egui::RichText::new("Right-click to reset").weak());
            if let Some(shortcut) = shortcut {
                ui.label(egui::RichText::new(format!("Shortcut: {}", shortcut)).weak());
            }
        });
    if resp.secondary_clicked() {
        *value = reset_to;
    }
    resp
}
