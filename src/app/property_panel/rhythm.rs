use egui::Ui;

use crate::{
    engine::score::{sequence::Sequence, DetRythm, RdRythm, Rythm},
    layout_left,
};

use super::hover_texts::{
    DETERMINISTIC_EXCLUSION_TEXT, DETERMINISTIC_INCLUSION_TEXT, RANDOM_EXCLUSION_TEXT,
    RANDOM_INCLUSION_TEXT,
};

/// Handle the inclusion generators UI and return whether edited
pub fn inclusion_section(ui: &mut Ui, seq: &mut Sequence, edit_vec_fn: impl Fn(&mut Ui, &mut Vec<usize>, usize)) -> bool {
    let mut edited = false;
    ui.label("Rythm inclusions:");

    let mut tmp_inclusions = seq.inclusions.clone();
    if let Rythm::Rd(_) = tmp_inclusions {
        if ui
            .button("Use deterministic inclusion generators")
            .clicked()
        {
            seq.inclusions = Rythm::Det(DetRythm::default());
            edited = true;
        }
    } else {
        if ui.button("Use random inclusion generators").clicked() {
            seq.inclusions = Rythm::Rd(RdRythm::default());
            edited = true;
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
                            egui::DragValue::new(&mut rd_rythm.amount)
                                .range(0..=rd_rythm.length),
                        )
                        .changed()
                    {
                        if let Rythm::Rd(ref mut edited_rd_rythm) = seq.inclusions {
                            edited_rd_rythm.amount = rd_rythm.amount.min(rd_rythm.length);
                            edited = true;
                        }
                    }
                    ui.label("N:");
                    if ui
                        .add(
                            egui::DragValue::new(&mut rd_rythm.length)
                                .range(rd_rythm.amount..=512),
                        )
                        .changed()
                    {
                        if let Rythm::Rd(ref mut edited_rd_rythm) = seq.inclusions {
                            edited_rd_rythm.length = rd_rythm.length.max(rd_rythm.amount);
                            edited = true;
                        }
                    }
                });
            });
        }
        Rythm::Det(det_rythm) => {
            ui.vertical(|ui| {
                ui.label("Deterministic inclusion generators:")
                    .on_hover_text(DETERMINISTIC_INCLUSION_TEXT);
                let mut gens = det_rythm.generators;
                let old_val = gens.clone();
                edit_vec_fn(ui, &mut gens, 2);
                if gens != old_val {
                    if let Rythm::Det(ref mut edited_det_rythm) = seq.inclusions {
                        edited_det_rythm.generators =
                            gens.into_iter().filter(|g| *g > 0).collect();
                        edited = true;
                    }
                }
            });
        }
    }

    edited
}

/// Handle the exclusion generators UI and return whether edited
pub fn exclusion_section(ui: &mut Ui, seq: &mut Sequence, edit_vec_fn: impl Fn(&mut Ui, &mut Vec<usize>, usize)) -> bool {
    let mut edited = false;
    let mut tmp_exclusions = seq.exclusions.clone();

    if let Rythm::Rd(_) = tmp_exclusions {
        ui.label("Rythm exclusions:");
        if ui
            .button("Use deterministic exclusion generators")
            .clicked()
        {
            seq.exclusions = Rythm::Det(DetRythm::default());
            edited = true;
        }
    } else {
        if ui.button("Use random exclusion generators").clicked() {
            seq.exclusions = Rythm::Rd(RdRythm::default());
            edited = true;
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
                        .add(egui::DragValue::new(&mut rd_rythm.amount).range(0..=512))
                        .changed()
                    {
                        if let Rythm::Rd(ref mut edited_rd_rythm) = seq.exclusions {
                            edited_rd_rythm.amount = rd_rythm.amount.min(rd_rythm.length);
                            edited = true;
                        }
                    }
                    ui.label("N:");
                    if ui
                        .add(egui::DragValue::new(&mut rd_rythm.length).range(0..=512))
                        .changed()
                    {
                        if let Rythm::Rd(ref mut edited_rd_rythm) = seq.exclusions {
                            edited_rd_rythm.length = rd_rythm.length.max(rd_rythm.amount);
                            edited = true;
                        }
                    }
                });
            });
        }
        Rythm::Det(det_rythm) => {
            ui.vertical(|ui| {
                ui.label("Deterministic exclusion generators:")
                    .on_hover_text(DETERMINISTIC_EXCLUSION_TEXT);
                let mut gens = det_rythm.generators;
                let old_val = gens.clone();
                edit_vec_fn(ui, &mut gens, 2);
                if gens != old_val {
                    if let Rythm::Det(ref mut edited_det_rythm) = seq.exclusions {
                        edited_det_rythm.generators =
                            gens.into_iter().filter(|g| *g > 1).collect();
                        edited = true;
                    }
                }
            });
        }
    }

    edited
}
