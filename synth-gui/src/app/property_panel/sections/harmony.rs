use super::super::helpers::{i32_cell, u32_cell};
use super::super::{apply_octave_shift, ParameterImpact};
use crate::app::property_panel::hover_texts::{
    HARMONISE_TEXT, OCTAVE_TEXT, SHUFFLE_TEXT, TOLERENCE_TEXT, VARIATION_INTERVALS_TEXT,
    VARIATION_STEPS_TEXT,
};
use crate::engine::score::default_params::{default_harmoniser, default_replicator_distance};
use crate::engine::score::sequence::{IntervalAffinity, ReplicatorStep, Sequence};
use crate::engine::score::{Interval, NotesGroup};
use crate::Token;
use egui::{Grid, RichText, Ui};
use std::collections::BTreeMap;

pub fn show_harmony_section(
    ui: &mut Ui,
    seq: &mut Sequence,
    notes: &mut BTreeMap<Token, NotesGroup>,
    impact: &mut ParameterImpact,
    promote: Option<&mut dyn FnMut()>,
) {
    ui.collapsing("Harmony", |ui| {
        ui.checkbox(&mut seq.glide, "Glide");
        let mut harmonise = seq.harmonise;
        if ui
            .checkbox(&mut harmonise, "Harmonise")
            .on_hover_text(HARMONISE_TEXT)
            .changed()
        {
            seq.harmonise = harmonise;
            impact.require_regeneration();
        }
        ui.separator();

        ui.horizontal(|ui| {
            let mut tmp_tolerance = seq.tolerance;
            ui.label("Tolerance:").on_hover_text(TOLERENCE_TEXT);
            ui.label("<-");
            if ui
                .add(egui::DragValue::new(&mut tmp_tolerance.0).range(-4.0..=16.0))
                .changed()
            {
                seq.tolerance.0 = tmp_tolerance.0;
                impact.require_regeneration();
            };
            ui.label(",");
            if ui
                .add(egui::DragValue::new(&mut tmp_tolerance.1).range(-4.0..=16.0))
                .changed()
            {
                seq.tolerance.1 = tmp_tolerance.1;
                impact.require_regeneration();
            };
            ui.label("->");
        });

        let mut changed = false;
        let mut octave_changed = false;
        let mut interval = seq.interval.clone();
        let mut shuffle = seq.shuffle;
        if let Interval::RDTempered(ref mut nb_rd_steps, ref mut tones, ref mut octave) = interval {
            let old_octave = *octave;
            ui.horizontal(|ui| {
                ui.label("Octave:").on_hover_text(OCTAVE_TEXT);
                if ui.add(egui::Slider::new(octave, -4..=4)).changed() {
                    octave_changed = true;
                };
            });
            if octave_changed {
                let octave_shift = *octave - old_octave;
                apply_octave_shift(notes, seq.token, octave_shift);
                if let Interval::RDTempered(_, _, ref mut seq_octave) = seq.interval {
                    *seq_octave = *octave;
                }
            }
            if !seq.harmonise {
                ui.horizontal(|ui| {
                    ui.label("Variation steps:")
                        .on_hover_text(VARIATION_STEPS_TEXT);
                    if ui.add(egui::Slider::new(nb_rd_steps, 0..=16)).changed() {
                        changed = true;
                    };
                });
                ui.label("Variation intervals:")
                    .on_hover_text(VARIATION_INTERVALS_TEXT);
                ui.horizontal_wrapped(|ui| {
                    for tone in -11..=11 {
                        let mut selected = tones.contains(&tone);
                        if ui.checkbox(&mut selected, tone.to_string()).changed() {
                            if selected {
                                if !tones.contains(&tone) {
                                    tones.push(tone);
                                    tones.sort_unstable();
                                }
                            } else if let Some(pos) = tones.iter().position(|&v| v == tone) {
                                tones.remove(pos);
                            }
                            changed = !tones.is_empty();
                        }
                    }
                });
            } else {
                ui.collapsing("Harmoniser", |ui| {
                    let mut harmoniser = seq.harmoniser;
                    let mut melodiser = seq.melodiser.clone();
                    let mut melodise = seq.melodise;
                    let mut melody_order_affinity = seq.melody_order_affinity;
                    let mut replicator = seq.replicator;
                    let mut replicator_steps = seq.replicator_steps.clone();
                    let mut harmoniser_changed = false;
                    let mut melodiser_changed = false;
                    let mut replicator_changed = false;

                    ui.collapsing("Harmony settings", |ui| {
                        ui.label(RichText::new("Weights per interval (0..=6)").weak());
                        ui.add_space(4.0);

                        Grid::new("harmoniser_grid_inverted")
                            .striped(true)
                            .num_columns(3)
                            .show(ui, |ui| {
                                ui.label(RichText::new("Interval").weak());
                                ui.label(RichText::new("Tension").weak());
                                ui.end_row();

                                for i in 0..=6 {
                                    ui.label(format!("{i}"));
                                    let r = u32_cell(ui, &mut harmoniser[i], 0..=32, 0);
                                    if r.changed() {
                                        harmoniser_changed = true;
                                    }
                                    ui.end_row();
                                }
                            });

                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Reset all to 16").clicked() {
                                harmoniser = [16; 7];
                                harmoniser_changed = true;
                            }
                            if ui.button("Reset defaults").clicked() {
                                harmoniser = default_harmoniser();
                                harmoniser_changed = true;
                            }
                        });
                    });

                    ui.collapsing("Melody settings", |ui| {
                        if ui.checkbox(&mut melodise, "Melodise").changed() {
                            melodiser_changed = true;
                        }

                        if melodise {
                            ui.label(RichText::new("Melodic affinities (-32..=32)").weak());
                            ui.add_space(4.0);

                            if ui.button("Add interval").clicked() {
                                melodiser.push(IntervalAffinity::default());
                                melodiser_changed = true;
                            }

                            let mut removal: Option<usize> = None;
                            for (idx, entry) in melodiser.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(format!("Interval {}", idx + 1));
                                    let mut interval_val = entry.interval as i32;
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut interval_val, 0..=23)
                                                .text("Interval"),
                                        )
                                        .changed()
                                    {
                                        entry.interval = interval_val.clamp(0, 23) as u8;
                                        melodiser_changed = true;
                                    }
                                    ui.label("Affinity");
                                    let r = i32_cell(ui, &mut entry.affinity, -32..=32, 0);
                                    if r.changed() {
                                        melodiser_changed = true;
                                    }
                                    if ui.button("Remove").clicked() {
                                        removal = Some(idx);
                                    }
                                });
                            }
                            if let Some(idx) = removal {
                                if idx < melodiser.len() {
                                    melodiser.remove(idx);
                                    melodiser_changed = true;
                                }
                            }

                            ui.horizontal(|ui| {
                                ui.label("Keep direction affinity");
                                if ui
                                    .add(egui::Slider::new(&mut melody_order_affinity, -32..=32))
                                    .changed()
                                {
                                    melodiser_changed = true;
                                }
                            });
                        }
                    });

                    ui.collapsing("Replicator settings", |ui| {
                        if ui
                            .checkbox(&mut replicator, "Replicator")
                            .on_hover_text(
                                "Add interval affinity against the note N time quanta earlier.",
                            )
                            .changed()
                        {
                            replicator_changed = true;
                        }
                        if replicator {
                            ui.horizontal(|ui| {
                                ui.label("Steps back");
                                if ui
                                    .small_button("Add")
                                    .on_hover_text("Add another time-quantum offset")
                                    .clicked()
                                {
                                    replicator_steps.push(ReplicatorStep::default());
                                    replicator_changed = true;
                                }
                            });

                            let mut removal: Option<usize> = None;
                            for (idx, step) in replicator_steps.iter_mut().enumerate() {
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label(format!("Step {}", idx + 1));
                                    let dist_resp = u32_cell(
                                        ui,
                                        &mut step.distance,
                                        1..=64,
                                        default_replicator_distance(),
                                    );
                                    if dist_resp.changed() {
                                        replicator_changed = true;
                                    }
                                    if ui.button("Remove").clicked() {
                                        removal = Some(idx);
                                    }
                                });

                                ui.label(RichText::new("Replication affinities (-32..=32)").weak());
                                if ui.button("Add interval").clicked() {
                                    step.affinities.push(IntervalAffinity::default());
                                    replicator_changed = true;
                                }

                                let mut remove_aff: Option<usize> = None;
                                for (a_idx, affinity) in step.affinities.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Interval {}", a_idx + 1));
                                        let mut interval_val = affinity.interval as i32;
                                        if ui
                                            .add(
                                                egui::Slider::new(&mut interval_val, 0..=23)
                                                    .text("Interval"),
                                            )
                                            .changed()
                                        {
                                            affinity.interval = interval_val.clamp(0, 23) as u8;
                                            replicator_changed = true;
                                        }
                                        ui.label("Affinity");
                                        let r =
                                            i32_cell(ui, &mut affinity.affinity, -32..=32, 0);
                                        if r.changed() {
                                            replicator_changed = true;
                                        }
                                        if ui.button("Remove").clicked() {
                                            remove_aff = Some(a_idx);
                                        }
                                    });
                                }
                                if let Some(idx) = remove_aff {
                                    if idx < step.affinities.len() {
                                        step.affinities.remove(idx);
                                        replicator_changed = true;
                                    }
                                }
                            }
                            if let Some(idx) = removal {
                                if idx < replicator_steps.len() {
                                    replicator_steps.remove(idx);
                                    replicator_changed = true;
                                }
                            }
                        }
                    });

                    if harmoniser_changed || melodiser_changed || replicator_changed {
                        seq.harmoniser = harmoniser;
                        seq.melodise = melodise;
                        seq.melodiser = melodiser;
                        seq.melody_order_affinity = melody_order_affinity;
                        seq.replicator = replicator;
                        seq.replicator_steps = replicator_steps;
                        impact.require_regeneration();
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
            seq.interval = interval;
            seq.shuffle = shuffle;
            impact.require_regeneration();
        }

        if seq.harmonise {
            if ui
                .add(egui::Slider::new(&mut seq.chord, 1..=12).text("Chord"))
                .changed()
            {
                impact.require_regeneration();
            }
            if ui
                .add(egui::Slider::new(&mut seq.skip_harmonised, 0..=100).text("Skip harmonised"))
                .on_hover_text("Number of most harmonious note combinations to skip. Higher values create more dissonance/tension.")
                .changed()
            {
                impact.require_regeneration();
            }
            if ui
                .add(egui::Slider::new(&mut seq.arpegio, -4.0..=4.0).text("Arpegio"))
                .changed()
            {
                impact.require_regeneration();
            }
            if ui
                .add(egui::Slider::new(&mut seq.reverse_prob, 0.0..=1.0).text("Reverse prob"))
                .changed()
            {
                impact.require_regeneration();
            }
            if ui
                .add(egui::Slider::new(&mut seq.shuffle_prob, 0.0..=1.0).text("Shuffle prob"))
                .changed()
            {
                impact.require_regeneration();
            }
            if ui
                .add(egui::Checkbox::new(
                    &mut seq.random_chord,
                    "Random skip chord notes",
                ))
                .changed()
            {
                impact.require_regeneration();
            }
        }
        if let Some(promote) = promote {
            if ui
                .small_button("Promote harmony to parent override")
                .on_hover_text(
                    "Copy these harmony settings to the parent override and clear them here",
                )
                .clicked()
            {
                promote();
            }
        }
    });
}
