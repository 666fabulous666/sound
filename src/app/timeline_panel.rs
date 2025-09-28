use crate::{
    app::{hsl_to_color32, GuiApp, NotesGroup},
    engine::{notes::Interval, waves::envelope},
    time_freq::Time,
};

impl GuiApp {
    pub fn timeline_panel(&mut self, ctx: &egui::Context) {
        let len = self.sequences.len();
        let current_time = self.now();
        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, _resp) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), ui.available_height()),
                egui::Sense::click_and_drag(),
            );
            let painter = ui.painter_at(rect);

            let lanes = len.max(1);
            let lane_h = rect.height() / lanes as f32;
            let block_h = lane_h * 0.6;
            let lane_gap = (lane_h - block_h) * 0.5;
            let max_loop_len = (&self.sequences)
                .iter()
                .fold(Time(0.0), |acc, seq| acc.max(seq.loop_len));

            // grid
            // let sub_grids = [6, 8, 10, 14];
            let sub_grids = self.sequences.iter().map(|s| s.time_quantum.1);
            for sub_grid in sub_grids {
                for s in 0..=2 * max_loop_len.as_secs() as usize * sub_grid {
                    let x = Self::t_to_x(
                        rect,
                        Time(s as f64 / sub_grid as f64) - self.now().rem_euclid(max_loop_len),
                        max_loop_len,
                    );
                    let base_col = hsl_to_color32(((279 * sub_grid) % 360) as _, 0.5, 0.5);

                    let col;
                    let thickness;
                    if s % sub_grid == 0 {
                        col = base_col.gamma_multiply(0.75);
                        thickness = 2.0;
                    } else if s % (sub_grid / 2) == 0 {
                        col = base_col.gamma_multiply(0.25);
                        thickness = 1.0;
                    } else {
                        col = base_col.gamma_multiply(0.125);
                        thickness = 1.0;
                    };

                    painter.line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        egui::Stroke::new(thickness, col),
                    );
                }
            }

            // sequences
            for (idx, seq) in (&self.sequences).iter().enumerate() {
                let top = rect.top() + idx as f32 * lane_h + lane_gap;
                let y0 = top;
                let y1 = top + block_h;
                let x0 = Self::t_to_x(
                    rect,
                    (seq.t_min - current_time).rem_euclid(seq.loop_len),
                    max_loop_len,
                );
                let x1 = Self::t_to_x(
                    rect,
                    (seq.t_max - current_time).rem_euclid(seq.loop_len),
                    max_loop_len,
                );

                let block_rect = egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1));
                let block_rect_l = egui::Rect::from_min_max(
                    egui::pos2(Self::t_to_x(rect, Time(0.0), max_loop_len), y0),
                    egui::pos2(x1, y1),
                );
                let block_rect_r = egui::Rect::from_min_max(
                    egui::pos2(x0, y0),
                    egui::pos2(Self::t_to_x(rect, seq.loop_len, max_loop_len), y1),
                );
                let track_rect = egui::Rect::from_min_max(
                    egui::pos2(Self::t_to_x(rect, Time(0.0), max_loop_len), y0),
                    egui::pos2(Self::t_to_x(rect, max_loop_len, max_loop_len), y1),
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
                        egui::StrokeKind::Middle,
                    );
                } else {
                    painter.rect_filled(block_rect_l, 4.0, col);
                    painter.rect_stroke(
                        block_rect_l,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                        egui::StrokeKind::Middle,
                    );
                    painter.rect_filled(block_rect_r, 4.0, col);
                    painter.rect_stroke(
                        block_rect_r,
                        4.0,
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                        egui::StrokeKind::Middle,
                    );
                }

                self.notes
                    .iter()
                    .filter(|NotesGroup { token, .. }| *token == seq.token)
                    .flat_map(|NotesGroup { notes, .. }| notes.iter())
                    .filter(|n| n.time < self.now() + seq.loop_len)
                    .collect::<Vec<_>>()
                    .iter()
                    .for_each(|n| {
                        if let Interval::Tempered(degree, _) = n.interval {
                            let dy = track_rect.top() - track_rect.bottom();

                            let note_rect = egui::Rect::from_min_max(
                                egui::pos2(
                                    Self::t_to_x(track_rect, n.time - current_time, max_loop_len),
                                    0.5 * (track_rect.bottom() + track_rect.top())
                                        + dy * (degree as f32 + 0.5) / 24.0,
                                ),
                                egui::pos2(
                                    Self::t_to_x(
                                        track_rect,
                                        (n.time + n.duration - current_time).min(seq.loop_len),
                                        max_loop_len,
                                    ),
                                    0.5 * (track_rect.bottom() + track_rect.top())
                                        + dy * (degree as f32 - 0.5) / 24.0,
                                ),
                            );

                            let tmp = 100f32.min(note_rect.width()).floor();
                            let tmp_inv = 1.0 / tmp;
                            let es: Vec<_> = (0..tmp as _)
                                .map(|i| {
                                    envelope(seq.attack_decay.0, seq.attack_decay.1, n.duration)(
                                        n.duration * i as f64 * tmp_inv as f64,
                                    ) as f32
                                })
                                .collect();
                            for (i, e) in es.iter().enumerate() {
                                let fract = i as f32 * tmp_inv;
                                let tmp = note_rect
                                    .with_min_x(note_rect.left() + note_rect.width() * fract)
                                    .with_max_x(
                                        note_rect.left() + note_rect.width() * (fract + tmp_inv),
                                    );
                                painter.rect_filled(
                                    tmp,
                                    0.0,
                                    egui::Color32::BLACK
                                        .gamma_multiply(e / seq.normalization as f32),
                                );
                            }
                        }
                    });

                let bar_color = col.lerp_to_gamma(
                    if self.selected == Some(idx) {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::GRAY
                    },
                    0.5,
                );
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
                    ),
                    egui::TextStyle::Body.resolve(ui.style()),
                    bar_color,
                );
                if seq.loop_len != max_loop_len {
                    painter.line_segment(
                        [
                            egui::pos2(Self::t_to_x(rect, seq.loop_len, max_loop_len), y0),
                            egui::pos2(Self::t_to_x(rect, seq.loop_len, max_loop_len), y1),
                        ],
                        egui::Stroke::new(2.0, bar_color),
                    );
                    painter.line_segment(
                        [
                            egui::pos2(Self::t_to_x(rect, seq.loop_len, max_loop_len) + 4.0, y0),
                            egui::pos2(Self::t_to_x(rect, seq.loop_len, max_loop_len) + 4.0, y1),
                        ],
                        egui::Stroke::new(2.0, bar_color),
                    );
                    painter.circle_filled(
                        egui::pos2(
                            Self::t_to_x(rect, seq.loop_len, max_loop_len) - 4.0,
                            0.75 * y0 + 0.25 * y1,
                        ),
                        2.0,
                        bar_color,
                    );
                    painter.circle_filled(
                        egui::pos2(
                            Self::t_to_x(rect, seq.loop_len, max_loop_len) - 4.0,
                            0.25 * y0 + 0.75 * y1,
                        ),
                        2.0,
                        bar_color,
                    );
                }
                if ui
                    .interact(track_rect, egui::Id::new(idx), egui::Sense::click())
                    .clicked()
                {
                    self.selected = Some(idx);
                }
            }
        });
    }
}
