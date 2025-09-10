use std::sync::{Arc, Mutex};

use crate::{app::GuiApp, engine::notes::Sequence};

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
                    egui::pos2(Self::t_to_x(rect, seq.loop_len, loop_len), y1),
                );
                let mut col = Self::hash_color(&seq.wave_type);
                if self.selected == Some(idx) {
                    col = Self::brighten(col);
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

                if ui
                    .interact(track_rect, egui::Id::new(idx), egui::Sense::click())
                    .clicked()
                {
                    self.selected = Some(idx);
                }

                // lane label
                painter.text(
                    egui::pos2(rect.left() + 4.0, rect.top() + idx as f32 * lane_h + 4.0),
                    egui::Align2::LEFT_TOP,
                    format!("Track {}", idx + 1),
                    egui::TextStyle::Small.resolve(ui.style()),
                    egui::Color32::WHITE,
                );
            }
        });
    }
}
