use std::collections::HashMap;

use egui::Align2;

use crate::{
    app::{hsl_to_color32, GuiApp, NotesGroup},
    engine::{
        score::{
            track_node::{all_paths, TrackNode},
            Interval,
        },
        waves::envelope,
    },
    time_freq::Time,
    NOTE_LINGER_TIME, TREE_DEPTH_WIDTH,
};

impl GuiApp {
    pub fn timeline_panel(&mut self, ctx: &egui::Context) {
        // If there are no sequences at all, show a placeholder and bail out
        let has_any_seq = self.score.track_root.sequences().next().is_some();

        if !has_any_seq {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.heading("No tracks yet");
                    ui.label("Use “Add track” or load an example to get started.");
                });
            });
            return;
        }

        // ----- PRECOMPUTE OWNED DATA (no long borrows) -----
        let node_paths = all_paths(&self.score.track_root); // owned paths
        let visible_paths: Vec<Vec<usize>> = node_paths
            .iter()
            .filter(|p| self.score.track_root.path_visible(p) && p.len() > 0)
            .cloned()
            .collect();

        let current_time = self.now();

        // Grid params based on sequences only:
        let max_loop_len = self
            .score
            .track_root
            .sequences()
            .fold(Time(0.0), |acc, seq| acc.max(seq.loop_len));
        let playhead = NOTE_LINGER_TIME.min(max_loop_len);
        let track_display_length = max_loop_len + playhead;

        // Precompute sub-grids so we don't borrow during the closure:
        let sub_grids: Vec<isize> = self
            .score
            .track_root
            .sequences()
            .map(|s| s.time_quantum.1 as isize)
            .collect();

        // ----- UI -----
        egui::CentralPanel::default().show(ctx, |ui| {
            let black_or_white = if ui.visuals().dark_mode {
                egui::Color32::WHITE
            } else {
                egui::Color32::BLACK
            };
            let (rect, _resp) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), ui.available_height()),
                egui::Sense::click_and_drag(),
            );
            let painter = ui.painter_at(rect);

            let lanes = visible_paths.len().max(1);
            let lane_h = rect.height() / lanes as f32;
            let block_h = lane_h * 0.6;
            let lane_gap = (lane_h - block_h) * 0.5;

            // --- grid (uses precomputed sub_grids) ---
            for sub_grid in &sub_grids {
                let n = (track_display_length.as_secs() as isize) * *sub_grid;
                for s in -n..=2 * n {
                    let x = Self::t_to_x(
                        rect,
                        Time(s as f64 / *sub_grid as f64) - current_time.rem_euclid(max_loop_len)
                            + playhead,
                        track_display_length,
                    );
                    let base_col = hsl_to_color32(((279 * *sub_grid) % 360) as _, 0.5, 0.5);
                    let (col, thickness) = if s % *sub_grid == 0 {
                        (base_col.gamma_multiply(0.75), 2.0)
                    } else if *sub_grid != 0 && s % (*sub_grid / 2) == 0 {
                        (base_col.gamma_multiply(0.25), 1.0)
                    } else {
                        (base_col.gamma_multiply(0.125), 1.0)
                    };
                    painter.line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        egui::Stroke::new(thickness, col),
                    );
                }
            }

            // --- lanes: iterate owned paths, fetch node on-demand ---
            let mut anchor_points_with_path = Vec::new();
            for (i, path) in visible_paths.iter().enumerate() {
                let top = rect.top() + i as f32 * lane_h + lane_gap;
                let y0 = top;
                let y1 = top + block_h;
                let track_rect = egui::Rect::from_min_max(
                    egui::pos2(Self::t_to_x(rect, Time::new(0.0), track_display_length), y0),
                    egui::pos2(
                        Self::t_to_x(rect, track_display_length, track_display_length),
                        y1,
                    ),
                );

                let node = match self.score.track_root.get(path) {
                    Some(n) => n,
                    None => continue,
                };

                let is_selected = self
                    .selected
                    .as_ref()
                    .map(|p| p.as_slice() == path.as_slice())
                    .unwrap_or(false);

                match node {
                    TrackNode::Group { name, children, .. } => {
                        let col = highlight_if_selected(
                            &painter,
                            lane_gap,
                            track_rect,
                            is_selected,
                            egui::Color32::from_gray(128),
                        );
                        let bar_color = col.lerp_to_gamma(black_or_white, 0.6);
                        let group_prefix = path.clone();
                        let (first_y, last_y) = first_last_y(
                            &visible_paths,
                            rect,
                            lane_h,
                            block_h,
                            lane_gap,
                            group_prefix,
                        );

                        if first_y.is_finite() && last_y.is_finite() && last_y > first_y {
                            let subbox_offset = TREE_DEPTH_WIDTH * path.len() as f32 + 2.0;
                            let left = rect.left() + subbox_offset;
                            let right = rect.right() - subbox_offset;
                            let encompass_rect = egui::Rect::from_min_max(
                                egui::pos2(left, first_y - 0.25 * lane_gap),
                                egui::pos2(right, last_y + 0.25 * lane_gap),
                            );

                            if is_selected {
                                highlight(black_or_white, &painter, encompass_rect);
                            }

                            let header_rect = track_rect;
                            painter.rect_filled(header_rect, 6.0, col.gamma_multiply(0.35));
                            painter.rect_stroke(
                                header_rect,
                                6.0,
                                egui::Stroke::new(1.0, black_or_white.gamma_multiply(0.5)),
                                egui::StrokeKind::Middle,
                            );
                        }

                        let label = if name.is_empty() {
                            format!("Group ({})", children.len())
                        } else {
                            format!("{} ({})", name, children.len())
                        };
                        painter.text(
                            egui::pos2(
                                rect.left() + 8.0 + TREE_DEPTH_WIDTH * (path.len() + 1) as f32,
                                rect.top() + (i as f32 + 0.5) * lane_h,
                            ),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::TextStyle::Body.resolve(ui.style()),
                            bar_color,
                        );

                        if ui
                            .interact(track_rect, egui::Id::new(("grp", i)), egui::Sense::click())
                            .clicked()
                        {
                            self.selected = Some(path.to_vec());
                        }
                    }

                    TrackNode::Seq(seq) => {
                        let col = highlight_if_selected(
                            &painter,
                            lane_gap,
                            track_rect,
                            is_selected,
                            Self::hash_color(&seq.wave_type),
                        );
                        // Repeat window tiling modulo loop
                        let loop_len = seq.loop_len;
                        let win_len = seq.t_max - seq.t_min;
                        let start0 = (seq.t_min - current_time).rem_euclid(loop_len) + playhead;
                        let repeats =
                            (track_display_length.as_secs() / loop_len.as_secs()).ceil() as i32 + 2;

                        for n in -repeats..repeats {
                            let shift = loop_len * (n as f64);
                            let s = start0 + shift;
                            let e = s + win_len;

                            if e <= Time(0.0) || s >= track_display_length {
                                continue;
                            }

                            let s_clamped = s.max(Time(0.0));
                            let e_clamped = e.min(track_display_length);

                            let x_s = Self::t_to_x(track_rect, s_clamped, track_display_length);
                            let x_e = Self::t_to_x(track_rect, e_clamped, track_display_length);

                            if x_e > x_s {
                                let block_rect = egui::Rect::from_min_max(
                                    egui::pos2(x_s, y0),
                                    egui::pos2(x_e, y1),
                                );
                                painter.rect_filled(block_rect, 4.0, col);
                                painter.rect_stroke(
                                    block_rect,
                                    4.0,
                                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                                    egui::StrokeKind::Middle,
                                );
                            }
                        }

                        // notes rendering (unchanged)
                        self.score
                            .notes
                            .iter()
                            .filter(|NotesGroup { token, .. }| *token == seq.token)
                            .flat_map(|NotesGroup { notes, .. }| notes.iter())
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
                                                (n.time + n.duration - current_time) + playhead,
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
                                            envelope(
                                                seq.attack_decay.0,
                                                seq.attack_decay.1,
                                                n.duration,
                                            )(
                                                n.duration * i as f64 * tmp_inv as f64
                                            ) as f32
                                        })
                                        .collect();
                                    for (i, e) in es.iter().enumerate() {
                                        let fract = i as f32 * tmp_inv;
                                        let tmp = note_rect
                                            .with_min_x(
                                                note_rect.left() + note_rect.width() * fract,
                                            )
                                            .with_max_x(
                                                note_rect.left()
                                                    + note_rect.width() * (fract + tmp_inv),
                                            );
                                        painter.rect_filled(
                                            tmp,
                                            0.0,
                                            black_or_white
                                                .gamma_multiply(e / seq.normalization as f32),
                                        );
                                    }
                                }
                            });

                        let bar_color = col.lerp_to_gamma(black_or_white, 0.5);

                        let label = format!(
                            "{} oct {}",
                            (&seq.wave_type).to_string(),
                            if let Interval::RDTempered(_, _, octave) = &seq.interval {
                                octave
                            } else {
                                &0
                            },
                        );
                        painter.text(
                            egui::pos2(
                                rect.left() + 8.0 + TREE_DEPTH_WIDTH * (path.len() + 1) as f32,
                                rect.top() + (i as f32 + 0.5) * lane_h,
                            ),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::TextStyle::Body.resolve(ui.style()),
                            bar_color,
                        );

                        // repeat bars (unchanged)
                        let rep_loop_len = seq.loop_len * seq.repeat as f64;
                        let bar_pos =
                            rep_loop_len + playhead - (self.now()).rem_euclid(rep_loop_len);
                        (0..seq.repeat).for_each(|j| {
                            let pos = seq.loop_len * j as f64 + playhead
                                - (self.now()).rem_euclid(rep_loop_len);
                            painter.text(
                                egui::pos2(
                                    Self::t_to_x(rect, pos, track_display_length),
                                    y0 - 0.333 * lane_gap,
                                ),
                                Align2::CENTER_BOTTOM,
                                format!("{}/{}", j + 1, seq.repeat),
                                egui::TextStyle::Body.resolve(ui.style()),
                                bar_color,
                            );
                        });
                        painter.text(
                            egui::pos2(
                                Self::t_to_x(rect, bar_pos, track_display_length),
                                y0 - 0.333 * lane_gap,
                            ),
                            Align2::CENTER_BOTTOM,
                            format!("x{}", seq.repeat),
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
                                egui::pos2(
                                    Self::t_to_x(rect, bar_pos, track_display_length) + 4.0,
                                    y0,
                                ),
                                egui::pos2(
                                    Self::t_to_x(rect, bar_pos, track_display_length) + 4.0,
                                    y1,
                                ),
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

                        // click to select
                        if ui
                            .interact(track_rect, egui::Id::new(("seq", i)), egui::Sense::click())
                            .clicked()
                        {
                            self.selected = Some(path.clone());
                        }
                    }
                }
                anchor_points_with_path.push((
                    egui::pos2(
                        track_rect.left() + TREE_DEPTH_WIDTH * (path.len() + 1) as f32,
                        0.5 * (track_rect.top() + track_rect.bottom()),
                    ),
                    path,
                ));
            }

            // tree
            // 1) Build a lookup map for parent anchors.
            let mut anchor_by_path: HashMap<Vec<usize>, egui::Pos2> =
                HashMap::with_capacity(anchor_points_with_path.len());
            for (anchor, path) in &anchor_points_with_path {
                anchor_by_path.insert(path.to_vec(), *anchor);
            }

            // 2) Draw one connector per node that has a visible parent.
            for (anchor, path) in &anchor_points_with_path {
                // root has no parent → skip
                if path.is_empty() {
                    continue;
                }

                // parent path = path without last index
                let parent_path = &path[..path.len() - 1];

                if let Some(parent_anchor) = anchor_by_path.get(parent_path) {
                    let angle_anchor = egui::Pos2::new(parent_anchor.x, anchor.y);
                    painter.line_segment(
                        [*parent_anchor, angle_anchor],
                        egui::Stroke::new(1.0, black_or_white),
                    );
                    painter.line_segment(
                        [angle_anchor, *anchor],
                        egui::Stroke::new(1.0, black_or_white),
                    );
                }
                // else: parent not visible → no line
            }

            // playhead
            let x = Self::t_to_x(rect, playhead, track_display_length);
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(3.0, egui::Color32::GOLD),
            );
        });
    }
}

fn highlight_if_selected(
    painter: &egui::Painter,
    lane_gap: f32,
    track_rect: egui::Rect,
    is_selected: bool,
    col: egui::Color32,
) -> egui::Color32 {
    let col = {
        let mut col = col;
        if is_selected {
            col = brighten(col, 20);
            highlight_glow_smooth(
                col.gamma_multiply(0.5),
                painter,
                track_rect.expand(lane_gap),
            );
        } else {
            col = col.gamma_multiply(0.5);
        }
        col
    };
    col
}

fn highlight(black_or_white: egui::Color32, painter: &egui::Painter, encompass_rect: egui::Rect) {
    painter.rect_stroke(
        encompass_rect,
        6.0,
        egui::Stroke::new(1.0, black_or_white.gamma_multiply(0.5)),
        egui::StrokeKind::Outside,
    );
    painter.rect_filled(encompass_rect, 6.0, black_or_white.gamma_multiply(0.15));
}

fn highlight_glow(col: egui::Color32, painter: &egui::Painter, track_rect: egui::Rect) {
    for k in -8..8 {
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
fn highlight_glow_smooth(col: egui::Color32, painter: &egui::Painter, track_rect: egui::Rect) {
    use egui::epaint::{Mesh, Vertex};

    let (top, bottom) = (track_rect.top(), track_rect.bottom());
    let center = 0.5 * (top + bottom);
    let height = bottom - top;

    // Top fades out → col → col → fades out
    let top_color = col.gamma_multiply(0.0);
    let mid_color = col;
    let bottom_color = col.gamma_multiply(0.0);

    let mut mesh = Mesh::default();

    // We’ll make a vertical gradient mesh with more interpolation control
    let steps = 32; // higher = smoother
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;

        let y0 = egui::lerp(top..=bottom, t0);
        let y1 = egui::lerp(top..=bottom, t1);

        // Gradient intensity based on distance to center
        let i0 = 1.0 - ((y0 - center).abs() / (0.5 * height)).powf(2.0);
        let i1 = 1.0 - ((y1 - center).abs() / (0.5 * height)).powf(2.0);

        let c0 = mid_color.gamma_multiply(i0.clamp(0.0, 1.0));
        let c1 = mid_color.gamma_multiply(i1.clamp(0.0, 1.0));

        let idx = mesh.vertices.len() as u32;
        mesh.vertices.extend_from_slice(&[
            Vertex {
                pos: egui::pos2(track_rect.left(), y0),
                uv: Default::default(),
                color: c0,
            },
            Vertex {
                pos: egui::pos2(track_rect.right(), y0),
                uv: Default::default(),
                color: c0,
            },
            Vertex {
                pos: egui::pos2(track_rect.right(), y1),
                uv: Default::default(),
                color: c1,
            },
            Vertex {
                pos: egui::pos2(track_rect.left(), y1),
                uv: Default::default(),
                color: c1,
            },
        ]);
        mesh.indices
            .extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
    }

    painter.add(egui::Shape::mesh(mesh));
}

fn first_last_y(
    visible_paths: &Vec<Vec<usize>>,
    rect: egui::Rect,
    lane_h: f32,
    block_h: f32,
    lane_gap: f32,
    group_prefix: Vec<usize>,
) -> (f32, f32) {
    let mut first_y = f32::MAX;
    let mut last_y = f32::MIN;

    // find visible children (and self)
    for (j, child_path) in visible_paths.iter().enumerate() {
        if child_path.starts_with(&group_prefix) {
            let top_j = rect.top() + j as f32 * lane_h + lane_gap;
            let bottom_j = top_j + block_h;
            first_y = first_y.min(top_j);
            last_y = last_y.max(bottom_j);
        }
    }
    (first_y, last_y)
}

fn brighten(col: egui::Color32, add: u8) -> egui::Color32 {
    let [r, g, b, a] = col.to_array();
    egui::Color32::from_rgba_premultiplied(
        r.saturating_add(add),
        g.saturating_add(add),
        b.saturating_add(add),
        a,
    )
}
