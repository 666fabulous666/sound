use std::sync::{Arc, Mutex};

use egui::ScrollArea;

use crate::{
    app::{GuiApp, ALL_WAVES},
    engine::{
        notes::{Interval, Sequence},
        scheduler::Message,
    },
};

impl GuiApp {
    pub fn property_panel(
        &mut self,
        ctx: &egui::Context,
        len: usize,
        seqs: &Arc<Mutex<Vec<Sequence>>>,
    ) {
        egui::SidePanel::left("props")
            .default_width(230.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    // Which structural edit (if any) should happen after the UI is drawn?
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
                            let seq = seqs.lock().unwrap()[sel].clone();

                            ui.heading(format!("Track {}", sel + 1));

                            // ── delete / move buttons ─────────────────────────
                            let can_up = sel > 0;
                            let can_down = sel + 1 < len; // ← NEW: use cached len

                            ui.horizontal(|ui| {
                                if ui.button("Delete").clicked() {
                                    action = Action::Delete;
                                }
                                if ui.button("Clone").clicked() {
                                    action = Action::Clone;
                                }
                                if ui.button("move up").clicked() && can_up {
                                    action = Action::Up;
                                }
                                if ui.button("move down").clicked() && can_down {
                                    action = Action::Down;
                                }
                            });

                            ui.separator();
                            // ---- WaveType picker ----
                            let mut w_choice = seq.wave_type;

                            egui::ComboBox::from_id_source("wave_type_combo")
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
                                edited_seq
                                    .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                    .wave_type = w_choice;
                            }

                            ui.separator();

                            // ---- numeric fields ----
                            let mut t_min = seq.t_min;
                            let mut t_max = seq.t_max;
                            ui.add(egui::Slider::new(&mut t_min, 0.0..=t_max).text("t_min"));
                            ui.add(
                                egui::Slider::new(&mut t_max, t_min..=seq.loop_len).text("t_max"),
                            );
                            t_max = t_max.clamp(t_min, seq.loop_len);
                            let step_f64 = seq.step.0 as f64 / seq.step.1 as f64;
                            if (t_min - seq.t_min).abs() > f64::EPSILON {
                                edited_seq
                                    .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                    .t_min = (t_min / step_f64).round() * step_f64;
                            }
                            if (t_max - seq.t_max).abs() > f64::EPSILON {
                                edited_seq
                                    .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                    .t_max = (t_max / step_f64).round() * step_f64;
                            }

                            ui.separator();
                            {
                                let mut attack_decay = seq.attack_decay;
                                let attack = ui.add(
                                    egui::Slider::new(&mut attack_decay.0, 0.01..=100.0)
                                        .text("attack")
                                        .show_value(true)
                                        .logarithmic(true),
                                );
                                let decay = ui.add(
                                    egui::Slider::new(&mut attack_decay.1, 0.01..=100.0)
                                        .text("decay")
                                        .logarithmic(true),
                                );
                                if attack.changed() || decay.changed() {
                                    edited_seq
                                        .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                        .attack_decay = attack_decay;
                                };
                            }
                            ui.separator();
                            {
                                let mut attack_freq_modulation = seq.attack_freq_modulation;
                                let mag = ui.add(
                                    egui::Slider::new(&mut attack_freq_modulation.0, -0.01..=0.01)
                                        .text("attack freq mod mag"),
                                );
                                let speed = ui.add(
                                    egui::Slider::new(&mut attack_freq_modulation.1, 1.0..=100.0)
                                        .text("attack freq mod speed")
                                        .logarithmic(true),
                                );
                                if mag.changed() || speed.changed() {
                                    edited_seq
                                        .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                        .attack_freq_modulation = attack_freq_modulation;
                                };
                            }
                            ui.separator();
                            {
                                let mut vibrato = seq.vibrato;
                                let mut vibrato_mag_display = vibrato.0 * 1e6;
                                let mag = ui.add(
                                    egui::Slider::new(&mut vibrato_mag_display, 0.0..=500.0)
                                        .text("vibrato mag"),
                                );

                                let fq = ui.add(
                                    egui::Slider::new(&mut vibrato.1, 0.01..=100.0)
                                        .text("vibrato fq")
                                        .logarithmic(true),
                                );
                                if mag.changed() || fq.changed() {
                                    edited_seq
                                        .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                        .vibrato = (vibrato_mag_display * 1e-6, vibrato.1);
                                };
                            }
                            ui.separator();
                            {
                                let mut chorus = seq.chorus;
                                let n = ui.add(
                                    egui::Slider::new(&mut chorus.number_of_heads, 1..=10)
                                        .text("chorus n"),
                                );
                                let delta = ui.add(
                                    egui::Slider::new(&mut chorus.delta, 5e-4..=2e-1)
                                        .text("chorus delta")
                                        .logarithmic(true),
                                );
                                let symmetric = ui.add(
                                    egui::Slider::new(&mut chorus.sym, 0.0..=1.0).text("symmetric"),
                                );
                                let asymmetric = ui.add(
                                    egui::Slider::new(&mut chorus.asym, 0.0..=1.0)
                                        .text("asymmetric"),
                                );
                                let time_dep = ui.add(
                                    egui::Slider::new(&mut chorus.time_dependency, -50.0..=50.0)
                                        .text("time_dependency"),
                                );
                                if [n, delta, symmetric, asymmetric, time_dep]
                                    .iter()
                                    .any(|x| x.changed())
                                {
                                    edited_seq
                                        .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                        .chorus = chorus;
                                };
                            }
                            ui.separator();
                            {
                                let mut pow_fact = seq.pow_fact;
                                if ui
                                    .add(
                                        egui::Slider::new(&mut pow_fact, -50.0..=50.0)
                                            .text("pow factor"), // .logarithmic(true),
                                    )
                                    .changed()
                                {
                                    edited_seq
                                        .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                        .pow_fact = pow_fact;
                                };
                            }
                            ui.separator();
                            {
                                ui.horizontal(|ui| {
                                    let mut tmp_step = seq.step.clone();
                                    ui.label("step:");
                                    if ui
                                        .add(egui::DragValue::new(&mut tmp_step.0).range(1..=128))
                                        .changed()
                                    {
                                        edited_seq
                                            .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                            .step
                                            .0 = tmp_step.0;
                                    };
                                    ui.label("/");
                                    if ui
                                        .add(egui::DragValue::new(&mut tmp_step.1).range(1..=128))
                                        .changed()
                                    {
                                        edited_seq
                                            .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                            .step
                                            .1 = tmp_step.1;
                                    };
                                });
                            }

                            {
                                ui.horizontal(|ui| {
                                    let mut tmp_skips = seq.skips.clone();
                                    ui.label("skips:");
                                    if ui
                                        .add(egui::DragValue::new(&mut tmp_skips.0).range(0..=512))
                                        .changed()
                                    {
                                        edited_seq
                                            .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                            .skips
                                            .0 = tmp_skips.0.min(tmp_skips.1);
                                    };
                                    ui.label(",");
                                    if ui
                                        .add(egui::DragValue::new(&mut tmp_skips.1).range(0..=512))
                                        .changed()
                                    {
                                        edited_seq
                                            .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                            .skips
                                            .1 = tmp_skips.1.max(tmp_skips.0);
                                    };
                                });
                            }

                            {
                                ui.horizontal(|ui| {
                                    let mut tmp_tolerance = seq.tolerance.clone();
                                    ui.label("tolerance:");
                                    ui.label("<-");
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut tmp_tolerance.0)
                                                .range(-4.0..=16.0),
                                        )
                                        .changed()
                                    {
                                        edited_seq
                                            .get_or_insert(seqs.lock().unwrap()[sel].clone())
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
                                            .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                            .tolerance
                                            .1 = tmp_tolerance.1;
                                    };
                                    ui.label("->");
                                });
                            }

                            {
                                ui.horizontal(|ui| {
                                    let mut loop_len = seq.loop_len.clone();
                                    ui.label("loop_len:");
                                    let slider = ui.add(
                                        egui::DragValue::new(&mut loop_len).range(0.0..=512.0),
                                    );
                                    if slider.changed() {
                                        loop_len = loop_len.max(0.0);
                                        let tmp_edited_seq = edited_seq
                                            .get_or_insert(seqs.lock().unwrap()[sel].clone());
                                        tmp_edited_seq.loop_len = loop_len.max(0.0);
                                        tmp_edited_seq.t_max = tmp_edited_seq.t_max.min(loop_len);
                                    };
                                });
                            }

                            {
                                let mut changed = false;
                                let mut f = seq.interval.clone();
                                if let Interval::RDTempered(
                                    ref mut nb_rd_steps,
                                    ref mut tones,
                                    ref mut octave,
                                ) = f
                                {
                                    // octave
                                    ui.horizontal(|ui| {
                                        ui.label("octave:");
                                        if ui
                                            .add(egui::DragValue::new(octave).range(-5..=5))
                                            .changed()
                                        {
                                            changed = true;
                                        };
                                    });
                                    // nb_rd_steps
                                    ui.horizontal(|ui| {
                                        ui.label("nb_rd_steps:");
                                        if ui
                                            .add(egui::DragValue::new(nb_rd_steps).range(0..=16))
                                            .changed()
                                        {
                                            changed = true;
                                        };
                                    });
                                    // ----- RDTempered tones (–11 … 11) ---------------------------------
                                    ui.label("RD tones:");
                                    ui.horizontal_wrapped(|ui| {
                                        for tone in -11..=11 {
                                            let mut selected = tones.contains(&tone);

                                            // show the checkbox; the label *is* the number
                                            if ui
                                                .checkbox(&mut selected, tone.to_string())
                                                .changed()
                                            {
                                                if selected {
                                                    // add if absent
                                                    if !tones.contains(&tone) {
                                                        tones.push(tone);
                                                        tones.sort_unstable();
                                                    }
                                                } else {
                                                    // remove if present
                                                    if let Some(pos) =
                                                        tones.iter().position(|&v| v == tone)
                                                    {
                                                        tones.remove(pos);
                                                    }
                                                }
                                                changed = true;
                                            }
                                        }
                                    });
                                }
                                if changed {
                                    edited_seq
                                        .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                        .interval = f;
                                }
                            }

                            // beat_offset
                            ui.horizontal(|ui| {
                                let mut tmp_beat_offset = seq.beat_offset.clone();
                                ui.label("beat_offset:");
                                if ui
                                    .add(egui::DragValue::new(&mut tmp_beat_offset).range(0..=256))
                                    .changed()
                                {
                                    edited_seq
                                        .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                        .beat_offset = tmp_beat_offset;
                                };
                            });

                            // volume
                            ui.horizontal(|ui| {
                                let mut tmp_volume = seq.volume.clone();
                                ui.label("volume:");
                                if ui
                                    .add(
                                        egui::Slider::new(&mut tmp_volume, 0.0..=32.0)
                                            .text("volume"),
                                    )
                                    .changed()
                                {
                                    edited_seq
                                        .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                        .volume = tmp_volume;
                                };
                            });

                            // spacial
                            ui.horizontal(|ui| {
                                let mut tmp_spacial = seq.spacial.clone();
                                ui.label("spacial:");
                                if ui
                                    .add(
                                        egui::Slider::new(&mut tmp_spacial, 0.0..=1.0)
                                            .text("spacial"),
                                    )
                                    .changed()
                                {
                                    edited_seq
                                        .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                        .spacial = tmp_spacial.clamp(0.0, 1.0);
                                };
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
                                self.sender.send(Message::DeleteSequence(sel)).unwrap();
                                self.selected = if sel == 0 { None } else { Some(sel - 1) };
                            }
                        }
                        Action::Clone => {
                            let last_token = self
                                .last_token
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if let Some(sel) = self.selected {
                                self.sender
                                    .send(Message::CloneSequence(sel, last_token + 1))
                                    .unwrap();
                            }
                        }
                        Action::Up => {
                            if let Some(sel) = self.selected {
                                self.sender
                                    .send(Message::SwapSequences(sel, sel - 1))
                                    .unwrap();
                                self.selected = Some(sel - 1);
                            }
                        }
                        Action::Down => {
                            if let Some(sel) = self.selected {
                                self.sender
                                    .send(Message::SwapSequences(sel, sel + 1))
                                    .unwrap();
                                self.selected = Some(sel + 1);
                            }
                        }
                    }
                    if let Some(edited_seq) = edited_seq {
                        if let Some(sel) = self.selected {
                            if ui.input(|i| !i.pointer.button_down(egui::PointerButton::Primary)) {
                                self.sender
                                    .send(Message::EditSequence(sel, edited_seq))
                                    .unwrap();
                            }
                        }
                    }
                });
            });
    }
}
