use egui::epaint::{Mesh, Vertex};
use egui::{Color32, Painter, Rect};
use std::sync::Arc;

use egui::Align2;

use crate::{
    app::GuiApp,
    engine::{
        score::{
            sequence::Sequence,
            track_node::{all_paths, NodeKind},
            Interval,
        },
        waves::envelope,
    },
    time_freq::{Beat, Tempo, Time},
    TREE_DEPTH_WIDTH,
};

const LASER_STRIP_RATIOS: [f32; 7] = [0.0, 0.12, 0.3, 0.5, 0.7, 0.88, 1.0];
const LASER_FALLOFF_POWER: f32 = 2.2;
const LASER_CORE_COLOR: Color32 = Color32::from_rgb(184, 255, 255);
const LASER_GLOW_COLOR: Color32 = Color32::from_rgb(32, 104, 255);
const LASER_DEEP_COLOR: Color32 = Color32::from_rgb(6, 18, 64);

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
            // .filter(|p| self.score.track_root.path_visible(p) && p.len() > 0)
            .filter(|p| self.score.track_root.path_visible(p))
            .cloned()
            .collect();

        let tempo = self.score.tempo();
        let current_time = self.now();

        let (max_loop_len, _padding, track_display_length) = self.timeline_lengths(tempo);

        // Precompute sub-grids so we don't borrow during the closure:
        let sub_grids: Vec<isize> = self
            .score
            .track_root
            .sequences()
            .map(|s| s.time_quantum.denominator() as isize)
            .collect();

        // ----- UI -----
        egui::CentralPanel::default().show(ctx, |ui| {
            let text_color = ui.visuals().text_color();
            let lanes = visible_paths.len().max(1);

            if self.fixed_track_height {
                let lane_h = self.track_lane_height.max(12.0);
                let total_height = lane_h * lanes as f32;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let draw_height = total_height.max(ui.available_height());
                    let (rect, _resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), draw_height),
                        egui::Sense::click_and_drag(),
                    );
                    self.render_timeline_contents(
                        ui,
                        ctx,
                        rect,
                        lane_h,
                        &visible_paths,
                        current_time,
                        max_loop_len,
                        track_display_length,
                        &sub_grids,
                        text_color,
                        tempo,
                    );
                });
            } else {
                let (rect, _resp) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), ui.available_height()),
                    egui::Sense::click_and_drag(),
                );
                let lane_h = rect.height() / lanes as f32;
                self.render_timeline_contents(
                    ui,
                    ctx,
                    rect,
                    lane_h,
                    &visible_paths,
                    current_time,
                    max_loop_len,
                    track_display_length,
                    &sub_grids,
                    text_color,
                    tempo,
                );
            }
        });
    }

    fn render_timeline_contents(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rect: egui::Rect,
        lane_h: f32,
        visible_paths: &[Vec<usize>],
        current_time: Time,
        max_loop_len: Time,
        track_display_length: Time,
        sub_grids: &[isize],
        text_color: Color32,
        tempo: Tempo,
    ) {
        let painter = ui.painter_at(rect);

        let (scroll_vec, zoom_modifier) = ui.ctx().input(|i| {
            let delta = if i.smooth_scroll_delta.length_sq() > 0.0 {
                i.smooth_scroll_delta
            } else {
                i.raw_scroll_delta
            };
            (delta, i.modifiers.command || i.modifiers.mac_cmd)
        });
        let pointer_pos = ui.ctx().pointer_hover_pos();
        let scroll_scalar = if scroll_vec.y.abs() >= scroll_vec.x.abs() {
            scroll_vec.y
        } else {
            scroll_vec.x
        };
        if zoom_modifier && scroll_scalar.abs() > f32::EPSILON {
            if let Some(pointer) = pointer_pos {
                if rect.contains(pointer) {
                    let focus_ratio = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                    let multiplier = (1.0f32 + scroll_scalar * 0.08).clamp(0.5, 2.0);
                    self.adjust_timeline_zoom(multiplier, focus_ratio, track_display_length, rect);
                }
            }
        }

        let view_span = self.timeline_view_span(track_display_length);
        self.clamp_timeline_view(track_display_length, view_span);
        let view_start = self.timeline_view_start;
        let view_end = view_start + view_span;
        let playhead = view_start + Time::new(view_span.as_secs() * 0.5);

        let block_h = lane_h * 0.6;
        let lane_gap = (lane_h - block_h) * 0.5;

        paint_grid(
            current_time,
            max_loop_len,
            playhead,
            view_start,
            view_span,
            sub_grids,
            text_color,
            rect,
            &painter,
            tempo,
        );

        let tree_band_rect = rect.with_max_x(rect.left() + rect.width() * 0.25);

        let mut lane_infos = Vec::new();
        for (i, path) in visible_paths.iter().enumerate() {
            let top = rect.top() + i as f32 * lane_h + lane_gap;
            let y0 = top;
            let y1 = top + block_h;
            let lane_rect =
                egui::Rect::from_min_max(egui::pos2(rect.left(), y0), egui::pos2(rect.right(), y1));
            let track_rect = egui::Rect::from_min_max(
                egui::pos2(Self::t_to_x(rect, view_start, view_start, view_span), y0),
                egui::pos2(Self::t_to_x(rect, view_end, view_start, view_span), y1),
            );
            let tree_rect = egui::Rect::from_min_max(
                egui::pos2(tree_band_rect.left(), y0),
                egui::pos2(tree_band_rect.right(), y1),
            );

            let node = match self.score.track_root.get(path) {
                Some(n) => n,
                None => continue,
            };

            lane_infos.push(LaneGeometry {
                path: path.clone(),
                lane_rect,
                track_rect,
                tree_rect,
                depth: path.len(),
                is_group: matches!(node.kind, NodeKind::Group { .. }),
                child_count: node.child_count(),
            });
        }

        for lane in &lane_infos {
            let Some(node) = self.score.track_root.get(&lane.path).cloned() else {
                continue;
            };

            let node_hue = node.hue;
            let track_rect = lane.track_rect;
            let y0 = track_rect.top();
            let y1 = track_rect.bottom();

            let drag_resp = ui
                .interact(
                    lane.track_rect,
                    egui::Id::new(("lane_drag", &lane.path)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_cursor(egui::CursorIcon::Grab);

            let is_selected = self
                .selected
                .as_ref()
                .map(|p| p.as_slice() == lane.path.as_slice())
                .unwrap_or(false);

            match node.kind {
                NodeKind::Group { collapsed, .. } => {
                    let col = highlight_if_selected(
                        &painter,
                        lane_gap,
                        track_rect,
                        is_selected,
                        GuiApp::group_color(node_hue),
                        ui.visuals().panel_fill,
                    );
                    if let Some((first_y, last_y)) = group_y_span(&lane_infos, lane.path.as_slice())
                    {
                        let subbox_offset = TREE_DEPTH_WIDTH * lane.depth as f32;
                        let left = rect.left() + subbox_offset;
                        let right = rect.right();
                        let encompass_rect = egui::Rect::from_min_max(
                            egui::pos2(left, first_y - lane_gap),
                            egui::pos2(right, last_y + lane_gap),
                        );

                        if is_selected && !collapsed {
                            highlight_group(text_color, &painter, encompass_rect, node.muted, node.solo);
                        }

                        painter.rect_filled(
                            track_rect,
                            6.0,
                            if is_selected {
                                col
                            } else {
                                col.gamma_multiply(0.35)
                            },
                        );
                        painter.rect_stroke(
                            track_rect,
                            6.0,
                            egui::Stroke::new(1.0, text_color.gamma_multiply(0.5)),
                            egui::StrokeKind::Middle,
                        );
                    }
                }

                NodeKind::Seq(seq) => {
                    let col = highlight_if_selected(
                        &painter,
                        lane_gap,
                        track_rect,
                        is_selected,
                        GuiApp::seq_color(&seq.wave_type, node_hue),
                        ui.visuals().panel_fill,
                    );
                    let loop_len = tempo.beats_to_time(seq.loop_len);
                    let loop_offset_time = tempo.beats_to_time(seq.loop_offset);
                    let t_min_time = tempo.beats_to_time(seq.t_min);
                    let t_max_time = tempo.beats_to_time(seq.t_max);
                    let win_len = t_max_time - t_min_time;
                    // Account for loop_offset: shift the phase of when the loop appears
                    let adjusted_current = current_time - loop_offset_time;
                    let start0 = (t_min_time - adjusted_current).rem_euclid(loop_len) + playhead;
                    let visible_end = view_end + win_len;

                    if loop_len.as_secs() > f64::EPSILON {
                        let start_n = ((view_start.as_secs() - start0.as_secs())
                            / loop_len.as_secs())
                        .floor() as i32
                            - 2;
                        let end_n = ((visible_end.as_secs() - start0.as_secs())
                            / loop_len.as_secs())
                        .ceil() as i32
                            + 2;

                        for n in start_n..=end_n {
                            let shift = loop_len * (n as f64);
                            let s = start0 + shift;
                            let e = s + win_len;

                            if e <= view_start || s >= view_end {
                                continue;
                            }

                            let x_s = Self::t_to_x(track_rect, s, view_start, view_span);
                            let x_e = Self::t_to_x(track_rect, e, view_start, view_span);

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

                                // Muted indicator: gray stripes
                                if node.muted {
                                    draw_stripes(&painter, block_rect, egui::Color32::GRAY.gamma_multiply(0.6));
                                }

                                // Solo indicator: golden border
                                if node.solo {
                                    painter.rect_stroke(
                                        block_rect,
                                        4.0,
                                        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 50)),
                                        egui::StrokeKind::Inside,
                                    );
                                }

                                self.sequence_drag_handles(
                                    ui, ctx, &painter, lane, &seq, block_rect, n,
                                );
                            }
                        }
                    }

                    let envelope_params = self.score.track_root.resolve_envelope(&lane.path).value;

                    if let Some(group) = self.score.notes.get(&seq.token) {
                        group.notes.iter().for_each(|n| {
                            if let Interval::Tempered(degree, _) = n.interval {
                                let dy = track_rect.top() - track_rect.bottom();
                                let note_rect = egui::Rect::from_min_max(
                                    egui::pos2(
                                        Self::t_to_x(
                                            track_rect,
                                            n.time - current_time + playhead,
                                            view_start,
                                            view_span,
                                        ),
                                        0.5 * (track_rect.bottom() + track_rect.top())
                                            + dy * (degree as f32 + 0.5) / 24.0,
                                    ),
                                    egui::pos2(
                                        Self::t_to_x(
                                            track_rect,
                                            (n.time + n.duration - current_time) + playhead,
                                            view_start,
                                            view_span,
                                        ),
                                        0.5 * (track_rect.bottom() + track_rect.top())
                                            + dy * (degree as f32 - 0.5) / 24.0,
                                    ),
                                );

                                let note_width = note_rect.width();
                                let note_height = note_rect.height();
                                if note_width <= f32::EPSILON || note_height <= f32::EPSILON {
                                    return;
                                }
                                let subdivisions = note_width.min(100.0).floor() as usize;
                                if subdivisions == 0 {
                                    return;
                                }
                                let inv = 1.0 / subdivisions as f32;
                                let time_step = inv as f64;
                                let env_curve = envelope(
                                    envelope_params.attack,
                                    envelope_params.decay,
                                    n.duration,
                                );
                                let envelope_values: Vec<f32> = (0..subdivisions)
                                    .map(|i| (env_curve(n.duration * i as f64 * time_step)) as f32)
                                    .collect();
                                let normalization = {
                                    let value = envelope_params.normalization as f32;
                                    if !value.is_finite() || value.abs() <= f32::EPSILON {
                                        1.0
                                    } else {
                                        value
                                    }
                                };
                                let note_left = note_rect.left();
                                let note_top = note_rect.top();
                                let mut mesh = Mesh::default();
                                let rows = LASER_STRIP_RATIOS.len();
                                for (i, e) in envelope_values.iter().enumerate() {
                                    let base_value = (e / normalization).clamp(0.0, 1.0);
                                    let left = note_left + note_width * (i as f32 * inv);
                                    let right = note_left + note_width * ((i + 1) as f32 * inv);
                                    let vertex_base = mesh.vertices.len() as u32;
                                    for &ratio in LASER_STRIP_RATIOS.iter() {
                                        let y = note_top + ratio * note_height;
                                        let color = laser_color(
                                            laser_value_at_ratio(base_value, ratio),
                                            ratio,
                                        );
                                        mesh.vertices.push(Vertex {
                                            pos: egui::pos2(left, y),
                                            uv: egui::Pos2::ZERO,
                                            color,
                                        });
                                        mesh.vertices.push(Vertex {
                                            pos: egui::pos2(right, y),
                                            uv: egui::Pos2::ZERO,
                                            color,
                                        });
                                    }
                                    for row in 0..rows - 1 {
                                        let idx = vertex_base + (row as u32) * 2;
                                        mesh.indices.extend_from_slice(&[
                                            idx,
                                            idx + 2,
                                            idx + 1,
                                            idx + 1,
                                            idx + 2,
                                            idx + 3,
                                        ]);
                                    }
                                }
                                if !mesh.indices.is_empty() {
                                    painter.add(egui::Shape::mesh(mesh));
                                }
                            }
                        });
                    }

                    let bar_color = col.lerp_to_gamma(text_color, 0.5);
                    let rep_loop_len = tempo.beats_to_time(seq.loop_len * seq.repeat as f64);
                    let current = self.now();
                    // Use adjusted_current for repeat markers to account for loop_offset
                    let adjusted_current_for_repeat = current - loop_offset_time;
                    if rep_loop_len.as_secs() > 0.0 {
                        let bar_pos = rep_loop_len + playhead - adjusted_current_for_repeat.rem_euclid(rep_loop_len);
                        (0..seq.repeat).for_each(|j| {
                            let pos = tempo.beats_to_time(seq.loop_len * j as f64) + playhead
                                - adjusted_current_for_repeat.rem_euclid(rep_loop_len);
                            painter.text(
                                egui::pos2(
                                    Self::t_to_x(rect, pos, view_start, view_span),
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
                                Self::t_to_x(rect, bar_pos, view_start, view_span),
                                y0 - 0.333 * lane_gap,
                            ),
                            Align2::CENTER_BOTTOM,
                            format!("x{}", seq.repeat),
                            egui::TextStyle::Body.resolve(ui.style()),
                            bar_color,
                        );
                        painter.line_segment(
                            [
                                egui::pos2(Self::t_to_x(rect, bar_pos, view_start, view_span), y0),
                                egui::pos2(Self::t_to_x(rect, bar_pos, view_start, view_span), y1),
                            ],
                            egui::Stroke::new(2.0, bar_color),
                        );
                        painter.line_segment(
                            [
                                egui::pos2(
                                    Self::t_to_x(rect, bar_pos, view_start, view_span) + 4.0,
                                    y0,
                                ),
                                egui::pos2(
                                    Self::t_to_x(rect, bar_pos, view_start, view_span) + 4.0,
                                    y1,
                                ),
                            ],
                            egui::Stroke::new(2.0, bar_color),
                        );
                        painter.circle_filled(
                            egui::pos2(
                                Self::t_to_x(rect, bar_pos, view_start, view_span) - 4.0,
                                0.75 * y0 + 0.25 * y1,
                            ),
                            2.0,
                            bar_color,
                        );
                        painter.circle_filled(
                            egui::pos2(
                                Self::t_to_x(rect, bar_pos, view_start, view_span) - 4.0,
                                0.25 * y0 + 0.75 * y1,
                            ),
                            2.0,
                            bar_color,
                        );
                    }
                }
            }

            self.handle_lane_widget_interaction(ctx, &lane.path, &drag_resp);
            if drag_resp.clicked() {
                self.selected = Some(lane.path.clone());
            }
        }

        self.update_sequence_drag_runtime(ctx, &lane_infos, tempo, view_span);

        self.paint_tree_band(ui, &painter, tree_band_rect, rect, &lane_infos);

        if let Some(preview) = self.update_tree_drag_preview(ctx, &lane_infos, rect) {
            paint_drop_preview(&painter, &preview);
        }
        if let Some(drag_state) = self.tree_drag.as_ref() {
            self.paint_tree_drag_ghost(ui, &painter, rect, &lane_infos, drag_state);
        }

        if self.tree_drag.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        }

        // playhead
        let x = Self::t_to_x(rect, playhead, view_start, view_span);
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, ui.visuals().strong_text_color()),
        );
    }

    fn handle_lane_widget_interaction(
        &mut self,
        ctx: &egui::Context,
        path: &[usize],
        response: &egui::Response,
    ) {
        if response.clicked() {
            self.selected = Some(path.to_vec());
        }

        if response.drag_started() {
            if path.is_empty() {
                return;
            }
            let pointer_pos = response
                .interact_pointer_pos()
                .unwrap_or_else(|| response.rect.center());
            self.tree_drag = Some(TreeDragState {
                source_path: path.to_vec(),
                source_id: response.id,
                pointer_pos,
                drop_slot: None,
            });
        }

        if let Some(state) = &self.tree_drag {
            if state.source_id == response.id && response.drag_stopped() {
                self.finish_tree_drag();
            }
        }

        if self
            .tree_drag
            .as_ref()
            .map(|drag| drag.source_id == response.id)
            .unwrap_or(false)
        {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        }
    }

    fn finish_tree_drag(&mut self) {
        if let Some(state) = self.tree_drag.take() {
            if let Some(slot) = state.drop_slot {
                eprintln!(
                    "DND drop: source={:?}, target_parent={:?}, index={}, kind={:?}",
                    state.source_path, slot.parent_path, slot.insert_index, slot.kind
                );
                if let Some(new_path) = self.score.move_node_to(
                    &state.source_path,
                    &slot.parent_path,
                    slot.insert_index,
                ) {
                    eprintln!("DND result: new_path={:?}", new_path);
                    self.selected = Some(new_path);
                }
            }
        }
    }

    fn update_tree_drag_preview(
        &mut self,
        ctx: &egui::Context,
        lanes: &[LaneGeometry],
        rect: egui::Rect,
    ) -> Option<DropPreview> {
        let drag_state = self.tree_drag.as_mut()?;
        if let Some(pos) = ctx.pointer_latest_pos() {
            drag_state.pointer_pos = pos;
        }
        let preview =
            compute_drop_preview(drag_state.pointer_pos, lanes, rect, &drag_state.source_path);
        drag_state.drop_slot = preview.as_ref().map(|p| p.slot.clone());
        preview
    }

    fn paint_tree_drag_ghost(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        panel_rect: egui::Rect,
        lanes: &[LaneGeometry],
        drag_state: &TreeDragState,
    ) {
        let Some(source_lane) = lanes
            .iter()
            .find(|lane| lane.path == drag_state.source_path)
        else {
            return;
        };
        let Some(node) = self.score.track_root.get(&source_lane.path) else {
            return;
        };

        let width = panel_rect.width().max(1.0);
        let height = source_lane
            .lane_rect
            .height()
            .min(panel_rect.height())
            .max(1.0);
        let min_y = panel_rect.top() + height * 0.5;
        let max_y = panel_rect.bottom() - height * 0.5;
        let center_y = if min_y > max_y {
            panel_rect.center().y
        } else {
            drag_state.pointer_pos.y.clamp(min_y, max_y)
        };
        let ghost_rect = egui::Rect::from_center_size(
            egui::pos2(panel_rect.center().x, center_y),
            egui::vec2(width, height),
        );

        let (accent, default_label) = match &node.kind {
            NodeKind::Group { .. } => (GuiApp::group_color(node.hue), "Group".to_string()),
            NodeKind::Seq(seq) => (
                GuiApp::seq_color(&seq.wave_type, node.hue),
                (&seq.wave_type).to_string(),
            ),
        };
        let ghost_fill =
            egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 70);
        let ghost_border = egui::Stroke::new(1.5, accent.gamma_multiply(0.7));

        painter.rect_filled(ghost_rect, 8.0, ghost_fill);
        painter.rect_stroke(ghost_rect, 8.0, ghost_border, egui::StrokeKind::Inside);

        let label = if node.name.trim().is_empty() {
            default_label
        } else {
            node.name.clone()
        };
        painter.text(
            ghost_rect.left_center() + egui::vec2(12.0, 0.0),
            Align2::LEFT_CENTER,
            label,
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().strong_text_color(),
        );
    }

    fn sequence_drag_handles(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        painter: &egui::Painter,
        lane: &LaneGeometry,
        seq: &Sequence,
        block_rect: egui::Rect,
        block_instance: i32,
    ) {
        let handle_width = block_rect.width().min(8.0).max(3.0);
        let left_rect = egui::Rect::from_min_max(
            block_rect.left_top(),
            egui::pos2(block_rect.left() + handle_width, block_rect.bottom()),
        );
        let right_rect = egui::Rect::from_min_max(
            egui::pos2(block_rect.right() - handle_width, block_rect.top()),
            block_rect.right_bottom(),
        );
        let shift_height = (block_rect.height() * 0.35).clamp(4.0, 12.0);
        let shift_rect = egui::Rect::from_min_max(
            egui::pos2(block_rect.left(), block_rect.bottom() - shift_height),
            block_rect.right_bottom(),
        );

        let handle_color = ui.visuals().widgets.active.bg_fill.gamma_multiply(0.6);
        painter.rect_filled(left_rect, 2.0, handle_color);
        painter.rect_filled(right_rect, 2.0, handle_color);
        painter.rect_filled(shift_rect, 2.0, handle_color.gamma_multiply(0.8));

        let start_resp = ui
            .interact(
                left_rect,
                egui::Id::new(("seq_start", &lane.path, block_instance)),
                egui::Sense::click_and_drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        self.process_sequence_handle_response(
            ctx,
            lane,
            seq,
            SequenceDragKind::ResizeStart,
            &start_resp,
        );

        let end_resp = ui
            .interact(
                right_rect,
                egui::Id::new(("seq_end", &lane.path, block_instance)),
                egui::Sense::click_and_drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        self.process_sequence_handle_response(
            ctx,
            lane,
            seq,
            SequenceDragKind::ResizeEnd,
            &end_resp,
        );

        let shift_resp = ui
            .interact(
                shift_rect,
                egui::Id::new(("seq_shift", &lane.path, block_instance)),
                egui::Sense::click_and_drag(),
            )
            .on_hover_cursor(egui::CursorIcon::Grab);
        self.process_sequence_handle_response(
            ctx,
            lane,
            seq,
            SequenceDragKind::MoveWindow,
            &shift_resp,
        );
    }

    fn process_sequence_handle_response(
        &mut self,
        ctx: &egui::Context,
        lane: &LaneGeometry,
        seq: &Sequence,
        kind: SequenceDragKind,
        response: &egui::Response,
    ) {
        let cursor_icon = match kind {
            SequenceDragKind::ResizeStart | SequenceDragKind::ResizeEnd => {
                egui::CursorIcon::ResizeHorizontal
            }
            SequenceDragKind::MoveWindow => egui::CursorIcon::Grabbing,
        };
        if response.drag_started() {
            ctx.set_cursor_icon(cursor_icon);
            let pointer_pos = response
                .interact_pointer_pos()
                .unwrap_or_else(|| response.rect.center());
            self.sequence_drag = Some(SequenceDragState {
                path: lane.path.clone(),
                source_id: response.id,
                kind,
                pointer_start: pointer_pos,
                t_min_start: seq.t_min,
                t_max_start: seq.t_max,
            });
            self.selected = Some(lane.path.clone());
        }

        if let Some(state) = &self.sequence_drag {
            if state.source_id == response.id {
                if response.drag_stopped() {
                    self.finish_sequence_drag();
                } else if response.is_pointer_button_down_on() {
                    ctx.set_cursor_icon(cursor_icon);
                }
            }
        }
    }

    fn update_sequence_drag_runtime(
        &mut self,
        ctx: &egui::Context,
        lanes: &[LaneGeometry],
        tempo: Tempo,
        visible_span: Time,
    ) {
        let Some(state) = self.sequence_drag.as_ref() else {
            return;
        };
        let Some(lane) = lanes.iter().find(|l| l.path == state.path) else {
            return;
        };
        let Some(pointer) = ctx.pointer_latest_pos() else {
            return;
        };

        let delta_ratio = (pointer.x - state.pointer_start.x) / lane.track_rect.width().max(1.0);
        let delta_secs = visible_span.as_secs() * delta_ratio as f64;
        let delta_beats = tempo.time_to_beats(Time(delta_secs)).as_beats();

        if let Some(node) = self.score.track_root.get_mut(&state.path) {
            if let NodeKind::Seq(seq) = &mut node.kind {
                let step = seq.time_quantum.beat_step().as_beats().max(f64::EPSILON);
                let snapped_delta = (delta_beats / step).round() * step;
                let mut new_min = state.t_min_start.as_beats();
                let mut new_max = state.t_max_start.as_beats();

                match state.kind {
                    SequenceDragKind::ResizeStart => {
                        new_min =
                            (new_min + snapped_delta).clamp(0.0, state.t_max_start.as_beats());
                        new_min = new_min.min(new_max);
                    }
                    SequenceDragKind::ResizeEnd => {
                        new_max = (new_max + snapped_delta)
                            .clamp(state.t_min_start.as_beats(), seq.loop_len.as_beats());
                        new_max = new_max.max(new_min);
                    }
                    SequenceDragKind::MoveWindow => {
                        let span =
                            (state.t_max_start.as_beats() - state.t_min_start.as_beats()).max(0.0);
                        let max_start = (seq.loop_len.as_beats() - span).max(0.0);
                        new_min = (new_min + snapped_delta).clamp(0.0, max_start);
                        new_max = (new_min + span).min(seq.loop_len.as_beats());
                    }
                }

                seq.t_min = Beat(new_min);
                seq.t_max = Beat(new_max);
            }
        }
    }

    fn finish_sequence_drag(&mut self) {
        if let Some(state) = self.sequence_drag.take() {
            self.score.refresh_notes_for_path(&state.path);
            self.score
                .shared_notes
                .store(Arc::new(self.score.notes.clone()));
        }
    }
    fn paint_tree_band(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        band_rect: egui::Rect,
        rect: egui::Rect,
        lanes: &[LaneGeometry],
    ) {
        use crate::engine::score::Interval;

        use egui::{Align2, Pos2, Stroke};

        let text_color = ui.visuals().text_color();

        // --- background gradient for the band (left→right fade) -------------
        horizontal_fade_rect(
            painter,
            band_rect,
            ui.visuals().panel_fill.gamma_multiply(0.75),
        );

        // --- compute anchors (left X depends on depth) -----------------------
        let mut anchors: Vec<(Pos2, &LaneGeometry)> = Vec::with_capacity(lanes.len());
        for lane in lanes {
            let anchor = egui::pos2(
                rect.left() + TREE_DEPTH_WIDTH * (lane.depth + 1) as f32,
                lane.lane_rect.center().y,
            );
            anchors.push((anchor, lane));
        }

        // --- lookup for parent anchors ---------------------------------------
        let mut anchor_by_path: std::collections::HashMap<Vec<usize>, Pos2> =
            std::collections::HashMap::with_capacity(anchors.len());
        for (a, lane) in &anchors {
            anchor_by_path.insert(lane.path.clone(), *a);
        }

        // --- draw connectors (roots have no parent) --------------------------
        for (anchor, lane) in &anchors {
            if lane.path.is_empty() {
                continue;
            }
            let parent_path = &lane.path[..lane.path.len() - 1];
            if let Some(parent_anchor) = anchor_by_path.get(parent_path) {
                let stroke = Stroke::new(1.0, text_color);
                let elbow = Pos2::new(parent_anchor.x, anchor.y);
                painter.line_segment([*parent_anchor, elbow], stroke);
                painter.line_segment([elbow, *anchor], stroke);
                painter.circle_filled(elbow, 2.0, text_color);
            }
        }

        // --- draw labels in the band (group/seq info) ------------------------
        for (anchor, lane) in anchors.iter() {
            // Resolve node briefly
            let Some(node) = self.score.track_root.get_mut(&lane.path) else {
                continue;
            };

            let y = lane.lane_rect.center().y;

            // Left text X with a small padding beyond the vertical line
            let label_pos = egui::pos2(
                rect.left() + 8.0 + TREE_DEPTH_WIDTH * (lane.depth + 1) as f32,
                y,
            );

            // Selection highlight color mix target (white in dark mode, black in light)
            let base_text_col = text_color;
            // check selection
            let is_selected = self
                .selected
                .as_ref()
                .map(|p| p.as_slice() == lane.path.as_slice())
                .unwrap_or(false);

            let text_style = if is_selected {
                // stronger visual presence (slightly larger or bold)
                egui::TextStyle::Heading
            } else {
                egui::TextStyle::Body
            };

            let bullet_radius = if is_selected { 5.0 } else { 3.0 };
            match &mut node.kind {
                NodeKind::Group {
                    children,
                    ref mut collapsed,
                    ..
                } => {
                    let label = if node.name.is_empty() {
                        format!("Group ({})", children.len())
                    } else {
                        format!("{} ({})", node.name, children.len())
                    };

                    // let base_col = text_color.gamma_multiply(if is_selected { 1.5 } else { 0.9 });
                    painter.text(
                        label_pos,
                        Align2::LEFT_CENTER,
                        label,
                        text_style.resolve(ui.style()),
                        base_text_col,
                    );
                    let fill = if *collapsed {
                        ui.visuals().panel_fill
                    } else {
                        text_color
                    };
                    painter.circle_filled(*anchor, bullet_radius, fill);
                    painter.circle_stroke(
                        *anchor,
                        bullet_radius,
                        Stroke::new(1.0, ui.visuals().strong_text_color()),
                    );
                    if ui
                        .interact(
                            egui::Rect::from_center_size(
                                *anchor,
                                egui::vec2(bullet_radius * 2.5, bullet_radius * 2.5),
                            ),
                            egui::Id::new(("circle", &lane.path)),
                            egui::Sense::click(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        *collapsed ^= true;
                    }
                }
                NodeKind::Seq(seq) => {
                    let label = format!(
                        "{} oct {}",
                        (&seq.wave_type).to_string(),
                        if let Interval::RDTempered(_, _, octave) = &seq.interval {
                            *octave
                        } else {
                            0
                        },
                    );
                    painter.text(
                        label_pos,
                        Align2::LEFT_CENTER,
                        label,
                        text_style.resolve(ui.style()),
                        base_text_col,
                    );
                    let wave_color = GuiApp::seq_color(&seq.wave_type, node.hue);
                    painter.circle_filled(*anchor, bullet_radius, wave_color);
                    painter.circle_stroke(*anchor, bullet_radius, Stroke::new(1.0, text_color));
                }
            }

            let tree_resp = ui
                .interact(
                    lane.tree_rect,
                    egui::Id::new(("tree_lane", &lane.path)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_cursor(egui::CursorIcon::Grab);
            self.handle_lane_widget_interaction(ui.ctx(), &lane.path, &tree_resp);
        }
    }
}

fn paint_grid(
    current_time: Time,
    max_loop_len: Time,
    playhead: Time,
    view_start: Time,
    view_span: Time,
    sub_grids: &[isize],
    text_color: Color32,
    rect: Rect,
    painter: &Painter,
    tempo: Tempo,
) {
    let track_beats = tempo.time_to_beats(view_span).as_beats().max(0.0);
    for sub_grid in sub_grids {
        if *sub_grid <= 0 {
            continue;
        }
        let subdivisions = (track_beats * *sub_grid as f64).ceil() as isize;
        let n = subdivisions.max(*sub_grid);
        for s in -n..=2 * n {
            let beat = Beat(s as f64 / *sub_grid as f64);
            let line_time =
                tempo.beats_to_time(beat) - current_time.rem_euclid(max_loop_len) + playhead;
            let x = GuiApp::t_to_x(rect, line_time, view_start, view_span);
            let base_col = text_color;
            let thickness = 1.0;
            let half = *sub_grid / 2;
            let col = if s % *sub_grid == 0 {
                base_col.gamma_multiply(0.2)
            } else if half > 0 && s % half == 0 {
                base_col.gamma_multiply(0.1)
            } else {
                base_col.gamma_multiply(0.05)
            };
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(thickness, col),
            );
        }
    }
}

fn highlight_if_selected(
    painter: &egui::Painter,
    lane_gap: f32,
    track_rect: egui::Rect,
    is_selected: bool,
    col: egui::Color32,
    background_color: egui::Color32,
) -> egui::Color32 {
    let col = {
        let mut col = col;
        if is_selected {
            col = col.lerp_to_gamma(background_color, 0.25);
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
/// Draw diagonal stripes on a rectangle (used for muted/solo indicators)
fn draw_stripes(painter: &egui::Painter, rect: egui::Rect, stripe_color: egui::Color32) {
    let spacing = 8.0; // distance between stripes
    let thickness = 1.5; // visual thickness of each stripe

    let top_left = rect.left_top();
    let bottom_right = rect.right_bottom();
    let width = rect.width();
    let height = rect.height();

    // Cover entire rect diagonally
    let diag_len = width + height;
    let step_count = (diag_len / spacing).ceil() as i32;

    // Alpha ramp: 0→1 over [0..0.25], 1 over [0.25..0.75], 1→0 over [0.75..1]
    let stripe_alpha = |x: f32| -> f32 {
        let t = ((x - rect.left()) / width).clamp(0.0, 1.0);
        if t < 0.25 {
            (t / 0.25).clamp(0.0, 1.0)
        } else if t > 0.75 {
            ((1.0 - t) / 0.25).clamp(0.0, 1.0)
        } else {
            1.0
        }
    };

    let mut mesh = egui::epaint::Mesh::default();

    for i in 0..step_count {
        let offset = i as f32 * spacing;

        // Start/end for a 45° stripe before clamping
        let mut start = egui::pos2(top_left.x + offset, top_left.y);
        let mut end = egui::pos2(top_left.x, top_left.y + offset);

        // Clamp to rect bounds by sliding ends to the box
        if start.x > bottom_right.x {
            let dx = start.x - bottom_right.x;
            start.x = bottom_right.x;
            start.y += dx;
        }
        if end.y > bottom_right.y {
            let dy = end.y - bottom_right.y;
            end.y = bottom_right.y;
            end.x += dy;
        }

        // Quick reject
        if !rect.intersects(egui::Rect::from_two_pos(start, end)) {
            continue;
        }

        // Build a thin quad for the stripe with per-vertex color (horizontal alpha)
        let dir = (end - start).normalized();
        if !dir.is_finite() {
            continue;
        }
        let n = egui::Vec2::new(-dir.y, dir.x); // perpendicular
        let half = 0.5 * thickness;

        // Four corners of the stripe quad
        let v0 = start - n * half;
        let v1 = start + n * half;
        let v2 = end + n * half;
        let v3 = end - n * half;

        // Alpha based on horizontal position
        let a0 = stripe_alpha(v0.x);
        let a1 = stripe_alpha(v1.x);
        let a2 = stripe_alpha(v2.x);
        let a3 = stripe_alpha(v3.x);

        let c0 = stripe_color.gamma_multiply(a0);
        let c1 = stripe_color.gamma_multiply(a1);
        let c2 = stripe_color.gamma_multiply(a2);
        let c3 = stripe_color.gamma_multiply(a3);

        let idx = mesh.vertices.len() as u32;
        mesh.vertices.extend_from_slice(&[
            egui::epaint::Vertex {
                pos: v0,
                uv: Default::default(),
                color: c0,
            },
            egui::epaint::Vertex {
                pos: v1,
                uv: Default::default(),
                color: c1,
            },
            egui::epaint::Vertex {
                pos: v2,
                uv: Default::default(),
                color: c2,
            },
            egui::epaint::Vertex {
                pos: v3,
                uv: Default::default(),
                color: c3,
            },
        ]);
        mesh.indices
            .extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
    }

    painter.add(egui::Shape::mesh(mesh));
}

fn highlight_group(
    color: egui::Color32,
    painter: &egui::Painter,
    encompass_rect: egui::Rect,
    muted: bool,
    solo: bool,
) {
    // Base fill
    let rounding = 10.0;
    painter.rect_filled(encompass_rect, rounding, color.gamma_multiply(0.35));

    if muted {
        // Gray stripes for muted
        draw_stripes(painter, encompass_rect, egui::Color32::GRAY.gamma_multiply(0.5));
    }

    if solo {
        // Golden border for solo
        painter.rect_stroke(
            encompass_rect,
            rounding,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 50)),
            egui::StrokeKind::Inside,
        );
    }
}

fn highlight_glow_smooth(col: egui::Color32, painter: &egui::Painter, track_rect: egui::Rect) {
    use egui::epaint::{Mesh, Vertex};

    let (top, bottom) = (track_rect.top(), track_rect.bottom());
    let center = 0.5 * (top + bottom);
    let height = bottom - top;

    let mid_color = col;

    let mut mesh = Mesh::default();

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

pub fn horizontal_fade_rect(painter: &Painter, rect: Rect, left_color: Color32) {
    let mut mesh = Mesh::default();

    let (min, max) = (rect.min, rect.max);

    let right_color = left_color.gamma_multiply(0.0); // fully transparent

    // Four corners: opaque left → transparent right
    let idx_base = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&[
        Vertex {
            pos: egui::pos2(min.x, min.y),
            uv: Default::default(),
            color: left_color,
        },
        Vertex {
            pos: egui::pos2(max.x, min.y),
            uv: Default::default(),
            color: right_color,
        },
        Vertex {
            pos: egui::pos2(max.x, max.y),
            uv: Default::default(),
            color: right_color,
        },
        Vertex {
            pos: egui::pos2(min.x, max.y),
            uv: Default::default(),
            color: left_color,
        },
    ]);

    mesh.indices.extend_from_slice(&[
        idx_base,
        idx_base + 1,
        idx_base + 2,
        idx_base,
        idx_base + 2,
        idx_base + 3,
    ]);

    painter.add(egui::Shape::mesh(mesh));
}

fn group_y_span(lanes: &[LaneGeometry], prefix: &[usize]) -> Option<(f32, f32)> {
    let mut first_y = f32::MAX;
    let mut last_y = f32::MIN;

    for lane in lanes.iter().filter(|lane| lane.path.starts_with(prefix)) {
        first_y = first_y.min(lane.lane_rect.top());
        last_y = last_y.max(lane.lane_rect.bottom());
    }

    if first_y.is_finite() && last_y.is_finite() && last_y > first_y {
        Some((first_y, last_y))
    } else {
        None
    }
}

#[derive(Clone)]
struct LaneGeometry {
    path: Vec<usize>,
    lane_rect: egui::Rect,
    track_rect: egui::Rect,
    tree_rect: egui::Rect,
    depth: usize,
    is_group: bool,
    child_count: usize,
}

#[derive(Clone)]
enum DropIndicator {
    Line { y: f32, x_start: f32, x_end: f32 },
    Rect { rect: egui::Rect },
}

#[derive(Clone)]
struct DropPreview {
    slot: DropSlot,
    indicator: DropIndicator,
}

#[derive(Clone)]
pub(super) struct DropSlot {
    parent_path: Vec<usize>,
    insert_index: usize,
    kind: DropKind,
}

#[derive(Clone)]
pub(super) struct TreeDragState {
    source_path: Vec<usize>,
    source_id: egui::Id,
    pointer_pos: egui::Pos2,
    drop_slot: Option<DropSlot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DropKind {
    Before,
    After,
    Into,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SequenceDragKind {
    ResizeStart,
    ResizeEnd,
    MoveWindow,
}

#[derive(Clone)]
pub(super) struct SequenceDragState {
    path: Vec<usize>,
    source_id: egui::Id,
    kind: SequenceDragKind,
    pointer_start: egui::Pos2,
    t_min_start: Beat,
    t_max_start: Beat,
}

fn compute_drop_preview(
    pointer: egui::Pos2,
    lanes: &[LaneGeometry],
    rect: egui::Rect,
    source_path: &[usize],
) -> Option<DropPreview> {
    if lanes.is_empty() || source_path.is_empty() {
        return None;
    }

    #[derive(Clone, Copy)]
    enum DropZone {
        Before(usize),
        After(usize),
        Into(usize),
    }

    let mut zone: Option<DropZone> = None;
    for (idx, lane) in lanes.iter().enumerate() {
        if pointer.y < lane.lane_rect.top() {
            zone = Some(DropZone::Before(idx));
            break;
        }
        if pointer.y <= lane.lane_rect.bottom() {
            let height = lane.lane_rect.height().max(1.0);
            let frac = (pointer.y - lane.lane_rect.top()) / height;
            if frac < 0.3 {
                zone = Some(DropZone::Before(idx));
            } else if frac > 0.7 {
                zone = Some(DropZone::After(idx));
            } else {
                zone = Some(DropZone::Into(idx));
            }
            break;
        }
    }
    if zone.is_none() {
        zone = Some(DropZone::After(lanes.len() - 1));
    }

    let src_parent: Vec<usize> = source_path[..source_path.len() - 1].to_vec();
    let src_idx = *source_path.last().unwrap();

    let (slot, indicator) = match zone? {
        DropZone::Before(idx) => {
            let lane = &lanes[idx];
            if lane.path.is_empty() {
                return None;
            }
            let parent_path = lane.path[..lane.path.len() - 1].to_vec();
            let insert_index = *lane.path.last().unwrap();
            let indicator = DropIndicator::Line {
                y: lane.lane_rect.top(),
                x_start: rect.left() + TREE_DEPTH_WIDTH * lane.depth as f32,
                x_end: rect.right(),
            };
            (
                DropSlot {
                    parent_path,
                    insert_index,
                    kind: DropKind::Before,
                },
                indicator,
            )
        }
        DropZone::After(idx) => {
            let lane = &lanes[idx];
            if lane.path.is_empty() {
                return None;
            }
            let parent_path = lane.path[..lane.path.len() - 1].to_vec();
            let insert_index = lane.path.last().copied().unwrap() + 1;
            let indicator = DropIndicator::Line {
                y: lane.lane_rect.bottom(),
                x_start: rect.left() + TREE_DEPTH_WIDTH * lane.depth as f32,
                x_end: rect.right(),
            };
            (
                DropSlot {
                    parent_path,
                    insert_index,
                    kind: DropKind::After,
                },
                indicator,
            )
        }
        DropZone::Into(idx) => {
            let lane = &lanes[idx];
            if !lane.is_group {
                return None;
            }
            let parent_path = lane.path.clone();
            let insert_index = lane.child_count;
            let indicator = DropIndicator::Rect {
                rect: lane.lane_rect,
            };
            (
                DropSlot {
                    parent_path,
                    insert_index,
                    kind: DropKind::Into,
                },
                indicator,
            )
        }
    };

    if slot.parent_path.starts_with(source_path) {
        return None;
    }
    if slot.parent_path == src_parent
        && (slot.insert_index == src_idx || slot.insert_index == src_idx + 1)
    {
        return None;
    }

    Some(DropPreview { slot, indicator })
}

fn paint_drop_preview(painter: &egui::Painter, preview: &DropPreview) {
    match &preview.indicator {
        DropIndicator::Line { y, x_start, x_end } => {
            painter.line_segment(
                [egui::pos2(*x_start, *y), egui::pos2(*x_end, *y)],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 210, 0)),
            );
        }
        DropIndicator::Rect { rect } => {
            let fill = egui::Color32::from_rgba_unmultiplied(255, 210, 0, 32);
            let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 210, 0));
            painter.rect_filled(*rect, 6.0, fill);
            painter.rect_stroke(*rect, 6.0, stroke, egui::StrokeKind::Middle);
        }
    }
}

fn laser_value_at_ratio(base_value: f32, ratio: f32) -> f32 {
    if base_value <= 0.0 {
        return 0.0;
    }
    let normalized_ratio = ratio.clamp(0.0, 1.0);
    let distance = ((normalized_ratio - 0.5).abs() / 0.5).min(1.0);
    let falloff = (1.0 - distance).powf(LASER_FALLOFF_POWER);
    base_value * falloff
}

fn laser_color(intensity: f32, ratio: f32) -> Color32 {
    if !intensity.is_finite() || intensity <= f32::EPSILON {
        return Color32::TRANSPARENT;
    }
    let normalized_ratio = ratio.clamp(0.0, 1.0);
    let ridge = (1.0 - ((normalized_ratio - 0.5).abs() / 0.5).powf(1.4)).clamp(0.0, 1.0);
    let glow_tint = lerp_rgba(LASER_GLOW_COLOR, LASER_CORE_COLOR, ridge.powf(1.2));
    let color = lerp_rgba(LASER_DEEP_COLOR, glow_tint, ridge);
    let alpha = (intensity.powf(0.85) * (0.35 + 0.65 * ridge)).clamp(0.0, 1.0);
    color_with_alpha(color, alpha)
}

fn lerp_rgba(a: Color32, b: Color32, t: f32) -> Color32 {
    let clamped = t.clamp(0.0, 1.0);
    let ar = a.r() as f32;
    let ag = a.g() as f32;
    let ab = a.b() as f32;
    let aa = a.a() as f32;
    let br = b.r() as f32;
    let bg = b.g() as f32;
    let bb = b.b() as f32;
    let ba = b.a() as f32;
    Color32::from_rgba_unmultiplied(
        (ar + (br - ar) * clamped) as u8,
        (ag + (bg - ag) * clamped) as u8,
        (ab + (bb - ab) * clamped) as u8,
        (aa + (ba - aa) * clamped) as u8,
    )
}

fn color_with_alpha(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    )
}
