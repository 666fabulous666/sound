use egui::{RichText, ScrollArea};
// use egui_double_slider::DoubleSlider;

use crate::{
    app::{GuiApp, ALL_WAVES, DRUM_WAVES},
    engine::notes::{
        default_params::*, ChorusParams, DetRythm, Interval, RdRythm, Rythm, Sequence,
    },
    layout_left,
    rescale_factor,
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
                                    .button("✖")
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Shortcut: Backspace").weak());
                                    })
                                    .clicked()
                                    || ui.input(|i| i.key_pressed(egui::Key::Backspace))
                                {
                                    action = Action::Delete;
                                }
                                if ui
                                    .button("Clone")
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Shortcut: C").weak());
                                    })
                                    .clicked()
                                    || ui.input(|i| i.key_pressed(egui::Key::C))
                                {
                                    action = Action::Clone;
                                }
                                if (ui
                                    .button("Up")
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Shortcut: U").weak());
                                    })
                                    .clicked()
                                    || ui.input(|i| i.key_pressed(egui::Key::U)))
                                    && sel > 0
                                {
                                    action = Action::Up;
                                }
                                if (ui
                                    .button("Down")
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Shortcut: D").weak());
                                    })
                                    .clicked()
                                    || ui.input(|i| i.key_pressed(egui::Key::D)))
                                    && sel + 1 < len
                                {
                                    action = Action::Down;
                                }
                                if ui
                                    .button(if seq.mute { "Unute" } else { "Mute" })
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Shortcut: \\").weak());
                                    })
                                    .clicked()
                                    || ui.input(|i| i.key_pressed(egui::Key::Backslash))
                                {
                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .mute ^= true;
                                }
                            });

                            {
                                ui.horizontal(|ui| {
                                    let mut tmp_volume = seq.volume.clone();
                                    ui.label("Volume:");
                                    let vol = ui
                                        .add(egui::Slider::new(&mut tmp_volume, 0.0..=32.0))
                                        .on_hover_ui(|ui| {
                                            ui.label(RichText::new("Default: right click").weak());
                                        });
                                    if vol.changed() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .volume = tmp_volume;
                                    };
                                    if vol.secondary_clicked() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .volume = default_volume();
                                    };
                                });
                                ui.horizontal(|ui| {
                                    let mut tmp_spacial = seq.spacial.clone();
                                    ui.label("Stereo:");
                                    let stereo = ui
                                        .add(egui::Slider::new(&mut tmp_spacial, 0.0..=1.0))
                                        .on_hover_ui(|ui| {
                                            ui.label("0.5 is centered");
                                            ui.label(RichText::new("Default: right click").weak());
                                        });
                                    if stereo.changed() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .spacial = tmp_spacial.clamp(0.0, 1.0);
                                    };
                                    if stereo.secondary_clicked() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .spacial = default_spacial();
                                    };
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
                            ui.collapsing("Sequence position", |ui| {
                                ui.label("Sequence position");
                                let mut t_min = seq.t_min.as_secs();
                                let mut t_max = seq.t_max.as_secs();
                                ui.add(egui::Slider::new(&mut t_min, 0.0..=t_max).text("t_min"));
                                ui.add(
                                    egui::Slider::new(&mut t_max, t_min..=seq.loop_len.as_secs())
                                        .text("t_max"),
                                );
                                t_max = t_max.clamp(t_min, seq.loop_len.as_secs());
                                let step_f64 =
                                    Time(seq.time_quantum.0 as f64 / seq.time_quantum.1 as f64);
                                if (t_min - seq.t_min.as_secs()).abs() > f64::EPSILON {
                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .t_min = step_f64 * (Time(t_min) / step_f64).round();
                                }
                                if (Time(t_max) - seq.t_max).as_secs().abs() > f64::EPSILON {
                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .t_max = step_f64 * (Time(t_max) / step_f64).round();
                                }
                            });
                            ui.collapsing("Envelope", |ui| {
                                let mut attack_decay = seq.attack_decay;
                                let attack = ui
                                    .add(
                                        egui::Slider::new(&mut attack_decay.0, 0.01..=100.0)
                                            .text("Attack")
                                            .show_value(true)
                                            .logarithmic(true),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Default: right click").weak());
                                    });
                                let decay = ui
                                    .add(
                                        egui::Slider::new(&mut attack_decay.1, 0.01..=100.0)
                                            .text("Decay")
                                            .logarithmic(true),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Default: right click").weak());
                                    });

                                if attack.changed() || decay.changed() {
                                    let e = edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone());
                                    e.attack_decay = attack_decay;
                                    rescale_envelope(e);
                                };
                                let default = if DRUM_WAVES.contains(&seq.wave_type) {
                                    default_drum_attack_decay
                                } else {
                                    default_attack_decay
                                };
                                if attack.secondary_clicked() {
                                    let e = edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone());
                                    e.attack_decay.0 = default().0;
                                    rescale_envelope(e);
                                }
                                if decay.secondary_clicked() {
                                    let e = edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone());
                                    e.attack_decay.1 = default().1;
                                    rescale_envelope(e);
                                }
                            });
                            ui.collapsing("Bend", |ui| {
                                let mut bend = seq.bend;
                                let mut tmp_mag = bend.0 * 1e4;
                                let mag = ui
                                    .add(
                                        egui::Slider::new(&mut tmp_mag, -200.0..=200.0)
                                            .text("Magnitude"),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Default: right click").weak());
                                    });

                                let speed = ui
                                    .add(
                                        egui::Slider::new(&mut bend.1, 1.0..=1000.0)
                                            .text("Speed")
                                            .logarithmic(true),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Default: right click").weak());
                                    });

                                if mag.changed() || speed.changed() {
                                    let e = edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone());
                                    e.bend.0 = tmp_mag * 1e-4;
                                    e.bend.1 = bend.1;
                                };
                                if mag.secondary_clicked() {
                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .bend
                                        .0 = default_bend().0;
                                };
                                if speed.secondary_clicked() {
                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .bend
                                        .1 = default_bend().1;
                                };
                            });
                            ui.collapsing("Vibrato", |ui| {
                                let mut vibrato = seq.vibrato;
                                let mut vibrato_mag_display = vibrato.0 * 1e6;
                                let mag = ui
                                    .add(
                                        egui::Slider::new(&mut vibrato_mag_display, 0.0..=1000.0)
                                            .text("Magnitude"),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Default: right click").weak());
                                    });

                                let fq = ui
                                    .add(
                                        egui::Slider::new(&mut vibrato.1, Freq(1.0)..=Freq(100.0))
                                            .text("Frequency")
                                            .logarithmic(true),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label(RichText::new("Default: right click").weak());
                                    });

                                if mag.changed() || fq.changed() {
                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .vibrato = (vibrato_mag_display * 1e-6, vibrato.1);
                                };
                                if mag.secondary_clicked() {
                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .vibrato
                                        .0 = default_vibrato().0;
                                };
                                if fq.secondary_clicked() {
                                    edited_seq
                                        .get_or_insert((&mut self.sequences)[sel].clone())
                                        .vibrato
                                        .1 = default_vibrato().1;
                                };
                            });
                            if !DRUM_WAVES.contains(&seq.wave_type) {
                                ui.collapsing("Chorus (Unison Detune)", |ui| {
                                    let mut chorus = seq.chorus;

                                    let voices = ui
                                        .add(
                                            egui::Slider::new(&mut chorus.voices, 1..=10)
                                                .text("Voice layers"),
                                        )
                                        .on_hover_text(concat!(
                                            "Each layer adds one detuned voice\n",
                                            "above and below f₀.\n",
                                            "Voices total = 1 + 2 × (steps − 1).",
                                        ));

                                    let delta = ui
                                        .add(
                                            egui::Slider::new(&mut chorus.delta, 0.0..=1.0)
                                                .text("Detune (Δf)")
                                                .logarithmic(true),
                                        )
                                        .on_hover_text(concat!(
                                            "Detune amount between voices around f₀.\n",
                                            "\n",
                                            "Use very small values for slow beating;\n",
                                            "increase for a wider chorus.",
                                        ));

                                    let delta_shift = ui
                                        .add(
                                            egui::Slider::new(&mut chorus.delta_shift, -1.0..=1.0)
                                                .text("Detune shift"), // .logarithmic(true),
                                        )
                                        .on_hover_text(concat!(
                                            "Shift voices frequencies asymmetrically\n",
                                            "to avoid beatings.",
                                        ));

                                    let time_dep = ui
                                        .add(
                                            egui::Slider::new(
                                                &mut chorus.time_dependency,
                                                Freq(-5.0)..=Freq(5.0),
                                            )
                                            .text("Detune over time"),
                                        )
                                        .on_hover_text(concat!(
                                            "Modulates Δf over time.\n",
                                            "\n",
                                            " > 0 : Δf increases over time.\n",
                                            " < 0 : Δf decreases over time.\n",
                                            " = 0 : static detune."
                                        ));

                                    ui.label("Weighting (around f₀)").on_hover_text(concat!(
                                        "Sets how much outer voices contribute\n",
                                        "relative to the center.\n",
                                        "\n",
                                        " • |value| > 1 -> outer voices are amplified\n",
                                        " • |value| = 1 -> constant voice levels\n",
                                        " • |value| < 1 -> outer voices are attenuated\n",
                                        "                  (so energy concentrates near f₀)\n",
                                        " •  value < 0  -> outer voices are inverted in phase"
                                    ));
                                    let symmetric = ui
                                        .add(
                                            egui::Slider::new(&mut chorus.sym, -2.0..=2.0)
                                                .text("Even"),
                                        )
                                        .on_hover_text(concat!(
                                            "Even (symmetric) weighting across +/− Δf",
                                        ));

                                    let asymmetric = ui
                                        .add(
                                            egui::Slider::new(&mut chorus.asym, -2.0..=2.0)
                                                .text("Odd"),
                                        )
                                        .on_hover_text(concat!(
                                            "Odd (asymmetric) weighting across +/− Δf.",
                                        ));

                                    if voices.secondary_clicked() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .chorus
                                            .voices = ChorusParams::default().voices;
                                    };
                                    if delta.secondary_clicked() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .chorus
                                            .delta = ChorusParams::default().delta;
                                    };
                                    if delta_shift.secondary_clicked() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .chorus
                                            .delta_shift = ChorusParams::default().delta_shift;
                                    };
                                    if symmetric.secondary_clicked() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .chorus
                                            .sym = ChorusParams::default().sym;
                                    };
                                    if asymmetric.secondary_clicked() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .chorus
                                            .asym = ChorusParams::default().asym;
                                    };
                                    if time_dep.secondary_clicked() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .chorus
                                            .time_dependency =
                                            ChorusParams::default().time_dependency;
                                    };
                                    if [voices, delta, delta_shift, symmetric, asymmetric, time_dep]
                                        .iter()
                                        .any(|x| x.changed())
                                    {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .chorus = chorus;
                                    };
                                })
                                .header_response
                                .on_hover_text(concat!(
                                    "Adds multiple voices detuned\n",
                                    "around the main frequency f₀\n",
                                    "to create width and motion.",
                                ));

                                ui.collapsing("Power factor", |ui| {
                                    let mut pow_fact_initial = seq.pow_fact.0;
                                    let initial_val = ui.add(
                                        egui::Slider::new(&mut pow_fact_initial, 0.0..=1000.0)
                                            .logarithmic(true)
                                            .text("Initial value"),
                                    );
                                    if initial_val.changed() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .pow_fact
                                            .0 = pow_fact_initial;
                                    };
                                    let mut time_dep_pow_fact = Freq(
                                        seq.pow_fact.1.as_hz().signum()
                                            * seq.pow_fact.1.as_hz().abs().sqrt(),
                                    );
                                    let evol = ui
                                        .add(
                                            egui::Slider::new(
                                                &mut time_dep_pow_fact,
                                                Freq(-10.0)..=Freq(10.0),
                                            )
                                            .text("Evolution"),
                                        )
                                        .on_hover_text(concat!("Increase/Decrease over time."));
                                    if evol.changed() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .pow_fact
                                            .1 = Freq(
                                            time_dep_pow_fact.as_hz().signum()
                                                * time_dep_pow_fact.as_hz()
                                                * time_dep_pow_fact.as_hz(),
                                        );
                                    };
                                    if initial_val.secondary_clicked() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .pow_fact
                                            .0 = default_pow_fact().0;
                                    };
                                    if evol.secondary_clicked() {
                                        edited_seq
                                            .get_or_insert((&mut self.sequences)[sel].clone())
                                            .pow_fact
                                            .1 = default_pow_fact().1;
                                    };
                                })
                                .header_response
                                .on_hover_text(concat!(
                                    "Produces distortion or metallic timbre\n",
                                    "\n",
                                    " • |value| = 0 -> square wave\n",
                                    " • |value| < 1 -> distortion\n",
                                    " • |value| = 1 -> unchanged wave\n",
                                    " • |value| < 1 -> metallic",
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
