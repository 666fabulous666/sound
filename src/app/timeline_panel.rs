use std::sync::{Arc, Mutex};

use crate::{
    app::GuiApp,
    engine::notes::{Interval, Sequence},
};

impl GuiApp {
    pub fn timeline_panel(
        &mut self,
        ctx: &egui::Context,
        current_time: f64,
        len: usize,
        seqs: Arc<Mutex<Vec<Sequence>>>,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, _resp) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), ui.available_height()),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(rect);

            let lanes = len.max(1);
            let lane_h = rect.height() / lanes as f32;
            let block_h = lane_h * 0.6;
            let lane_gap = (lane_h - block_h) * 0.5;
            let loop_len = seqs
                .lock()
                .unwrap()
                .iter()
                .fold(0.0f64, |acc, seq| acc.max(seq.loop_len));

            // grid
            for s in (0..=16).map(|i| i as f64 * 4.0) {
                let x = Self::t_to_x(rect, s, loop_len);
                let col = if (s as i32) % 16 == 0 {
                    egui::Color32::from_gray(120)
                } else {
                    egui::Color32::from_gray(70)
                };
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(1.0, col),
                );
            }

            // sequences
            for (idx, seq) in seqs.lock().unwrap().iter().enumerate() {
                let top = rect.top() + idx as f32 * lane_h + lane_gap;
                let y0 = top;
                let y1 = top + block_h;
                let x0 = Self::t_to_x(
                    rect,
                    (seq.t_min - current_time).rem_euclid(seq.loop_len),
                    loop_len,
                );
                let x1 = Self::t_to_x(
                    rect,
                    (seq.t_max - current_time).rem_euclid(seq.loop_len),
                    loop_len,
                );

                let block_rect = egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1));
                let block_rect_l = egui::Rect::from_min_max(
                    egui::pos2(Self::t_to_x(rect, 0.0, loop_len), y0),
                    egui::pos2(x1, y1),
                );
                let block_rect_r = egui::Rect::from_min_max(
                    egui::pos2(x0, y0),
                    egui::pos2(Self::t_to_x(rect, seq.loop_len, loop_len), y1),
                );
                let track_rect = egui::Rect::from_min_max(
                    egui::pos2(Self::t_to_x(rect, 0.0, loop_len), y0),
                    egui::pos2(Self::t_to_x(rect, loop_len, loop_len), y1),
                );
                let mut col = Self::hash_color(&seq.wave_type);
                if self.selected == Some(idx) {
                    col = Self::brighten(col);
                    for k in -16..16 {
                        let tmp = (30 + k) as f32;
                        painter.rect_filled(
                            track_rect.expand2(egui::Vec2 {
                                x: 0.0,
                                y: k as f32,
                            }),
                            tmp.sqrt(),
                            // egui::Color32::from_rgba_unmultiplied(200, 225, 255, 1),
                            col.gamma_multiply(1.0 / tmp),
                        );
                    }
                }

                if x0 < x1 {
                    painter.rect_filled(block_rect, 4.0, col);
                    painter.rect_stroke(
                        block_rect,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                    );
                } else {
                    painter.rect_filled(block_rect_l, 4.0, col);
                    painter.rect_stroke(
                        block_rect_l,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                    );
                    painter.rect_filled(block_rect_r, 4.0, col);
                    painter.rect_stroke(
                        block_rect_r,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                    );
                }

                self.notes
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|(token, _)| *token == seq.token)
                    .flat_map(|(_, ns)| ns.iter())
                    .collect::<Vec<_>>()
                    .iter()
                    .for_each(|n| {
                        if let Interval::Tempered(degree, _) = n.interval {
                            let dy = track_rect.top() - track_rect.bottom();
                            let y = 0.5 * (track_rect.bottom() + track_rect.top())
                                + dy * degree as f32 / 24.0; // FIXME: why 24?
                            painter.line_segment(
                                [
                                    egui::pos2(
                                        Self::t_to_x(
                                            track_rect,
                                            n.time - self.current_time(),
                                            loop_len,
                                        ) + 1.0,
                                        y,
                                    ),
                                    egui::pos2(
                                        Self::t_to_x(
                                            track_rect,
                                            n.time + n.duration - self.current_time(),
                                            loop_len,
                                        ) - 1.0,
                                        y,
                                    ),
                                ],
                                egui::Stroke::new(5.0, egui::Color32::BLACK),
                            );
                        }
                    });

                if ui
                    .interact(track_rect, egui::Id::new(idx), egui::Sense::click())
                    .clicked()
                {
                    self.selected = Some(idx);
                }

                // lane label
                painter.text(
                    egui::pos2(
                        rect.right() - 4.0,
                        rect.top() + (idx as f32 + 0.5) * lane_h + 4.0,
                    ),
                    egui::Align2::RIGHT_CENTER,
                    format!(
                        "{} oct {}",
                        (&seq.wave_type).to_string(),
                        if let Interval::RDTempered(_nb_rd_steps, _tones, octave) = &seq.interval {
                            octave
                        } else {
                            todo!()
                        },
                        // idx + 1,
                    ),
                    egui::TextStyle::Body.resolve(ui.style()),
                    egui::Color32::WHITE,
                );
            }
        });
    }
}
