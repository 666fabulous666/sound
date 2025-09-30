use egui::Align2;

use crate::{
    app::{hsl_to_color32, GuiApp, NotesGroup},
    engine::{notes::Interval, waves::envelope},
    time_freq::Time,
    NOTE_LINGER_TIME,
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
            let playhead = NOTE_LINGER_TIME.min(max_loop_len);

            let track_display_length = max_loop_len + playhead;
            // grid
            let sub_grids = self.sequences.iter().map(|s| s.time_quantum.1 as isize);
            for sub_grid in sub_grids {
                let n = track_display_length.as_secs() as isize * sub_grid;
                for s in -n..=2 * n {
                    let x = Self::t_to_x(
                        rect,
                        Time(s as f64 / sub_grid as f64) - self.now().rem_euclid(max_loop_len)
                            + playhead,
                        track_display_length,
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
                let track_rect = egui::Rect::from_min_max(
                    egui::pos2(Self::t_to_x(rect, Time::new(0.0), track_display_length), y0),
                    egui::pos2(
                        Self::t_to_x(rect, track_display_length, track_display_length),
                        y1,
                    ),
                );
                // let x0 = Self::t_to_x(
                //     track_rect,
                //     (seq.t_min - current_time).rem_euclid(seq.loop_len) + playhead,
                //     track_display_length,
                // );
                // let x1 = Self::t_to_x(
                //     track_rect,
                //     (seq.t_max - current_time).rem_euclid(seq.loop_len) + playhead,
                //     track_display_length,
                // );

                // let block_rect = egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1));
                // let block_rect_l = egui::Rect::from_min_max(
                //     egui::pos2(Self::t_to_x(track_rect, playhead, track_display_length), y0),
                //     egui::pos2(x1, y1),
                // );
                // let block_rect_r = egui::Rect::from_min_max(
                //     egui::pos2(x0, y0),
                //     egui::pos2(
                //         Self::t_to_x(track_rect, seq.loop_len + playhead, track_display_length),
                //         y1,
                //     ),
                // );
                // let mut col = Self::hash_color(&seq.wave_type);
                // if self.selected == Some(idx) {
                //     col = Self::brighten(col);
                //     for k in -16..16 {
                //         let tmp = (30 + k) as f32;
                //         painter.rect_filled(
                //             track_rect.expand2(egui::Vec2 {
                //                 x: 0.0,
                //                 y: k as f32,
                //             }),
                //             tmp.sqrt(),
                //             col.gamma_multiply(1.0 / tmp),
                //         );
                //     }
                // }

                // if x0 < x1 {
                //     painter.rect_filled(block_rect, 4.0, col);
                //     painter.rect_stroke(
                //         block_rect,
                //         4.0,
                //         egui::Stroke::new(1.0, egui::Color32::BLACK),
                //         egui::StrokeKind::Middle,
                //     );
                // } else {
                //     painter.rect_filled(block_rect_l, 4.0, col);
                //     painter.rect_stroke(
                //         block_rect_l,
                //         4.0,
                //         egui::Stroke::new(1.0, egui::Color32::BLACK),
                //         egui::StrokeKind::Middle,
                //     );
                //     painter.rect_filled(block_rect_r, 4.0, col);
                //     painter.rect_stroke(
                //         block_rect_r,
                //         4.0,
                //         egui::Stroke::new(1.0, egui::Color32::BLACK),
                //         egui::StrokeKind::Middle,
                //     );
                // }
                // --- WINDOW REPEATS: draw [t_min, t_max) modulo loop_len across the visible span ---

                let loop_len = seq.loop_len;
                let win_len = seq.t_max - seq.t_min;

                // Where does this sequence’s window start, relative to the playhead-centered view?
                // (shifted so that playhead is at `playhead` along the X axis)
                let start0 = (seq.t_min - current_time).rem_euclid(loop_len) + playhead;

                // How many repetitions do we need to cover the whole visible width?
                let repeats =
                    (track_display_length.as_secs() / loop_len.as_secs()).ceil() as i32 + 2;

                // Color (highlight if selected)
                let mut col = Self::hash_color(&seq.wave_type);
                if self.selected == Some(idx) {
                    col = Self::brighten(col);
                    // keep your selected-lane glow if you like:
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

                // Draw each repeated window tile if it intersects the visible range [0, track_display_length)
                for n in -repeats..repeats {
                    let shift = loop_len * (n as f64);
                    let s = start0 + shift;
                    let e = s + win_len;

                    // Skip if completely off-screen
                    if e <= Time(0.0) || s >= track_display_length {
                        continue;
                    }

                    // Clamp to visible range
                    let s_clamped = s.max(Time(0.0));
                    let e_clamped = e.min(track_display_length);

                    let x_s = Self::t_to_x(track_rect, s_clamped, track_display_length);
                    let x_e = Self::t_to_x(track_rect, e_clamped, track_display_length);

                    if x_e > x_s {
                        let block_rect =
                            egui::Rect::from_min_max(egui::pos2(x_s, y0), egui::pos2(x_e, y1));
                        painter.rect_filled(block_rect, 4.0, col);
                        painter.rect_stroke(
                            block_rect,
                            4.0,
                            egui::Stroke::new(1.0, egui::Color32::BLACK),
                            egui::StrokeKind::Middle,
                        );
                    }
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
                                    Self::t_to_x(
                                        track_rect,
                                        n.time - current_time + playhead,
                                        track_display_length,
                                    ),
                                    0.5 * (track_rect.bottom() + track_rect.top())
                                        + dy * (degree as f32 + 0.5) / 24.0,
                                ),
                                egui::pos2(
                                    Self::t_to_x(
                                        track_rect,
                                        (n.time + n.duration - current_time).min(seq.loop_len)
                                            + playhead,
                                        track_display_length,
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
                    if ui.visuals().dark_mode {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::BLACK
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

                // double bar
                // if seq.loop_len != max_loop_len {
                if true {
                    let bar_pos = seq.loop_len + playhead - (self.now()).rem_euclid(seq.loop_len);
                    let reps =
                        seq.repeat - ((self.now() / seq.loop_len) as usize).rem_euclid(seq.repeat);
                    painter.text(
                        egui::pos2(
                            Self::t_to_x(rect, bar_pos, track_display_length),
                            y0 - 0.333 * lane_gap,
                        ),
                        Align2::CENTER_BOTTOM,
                        format!("x{}", reps as u32),
                        egui::TextStyle::Body.resolve(ui.style()),
                        bar_color,
                    );
                    painter.line_segment(
                        [
                            egui::pos2(Self::t_to_x(rect, bar_pos, track_display_length), y0),
                            egui::pos2(Self::t_to_x(rect, bar_pos, track_display_length), y1),
                        ],
                        egui::Stroke::new(2.0, bar_color),
                    );
                    painter.line_segment(
                        [
                            egui::pos2(Self::t_to_x(rect, bar_pos, track_display_length) + 4.0, y0),
                            egui::pos2(Self::t_to_x(rect, bar_pos, track_display_length) + 4.0, y1),
                        ],
                        egui::Stroke::new(2.0, bar_color),
                    );
                    painter.circle_filled(
                        egui::pos2(
                            Self::t_to_x(rect, bar_pos, track_display_length) - 4.0,
                            0.75 * y0 + 0.25 * y1,
                        ),
                        2.0,
                        bar_color,
                    );
                    painter.circle_filled(
                        egui::pos2(
                            Self::t_to_x(rect, bar_pos, track_display_length) - 4.0,
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
            // grid
            let x = Self::t_to_x(rect, playhead, track_display_length);

            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(3.0, egui::Color32::GOLD),
            );
        });
    }
}
