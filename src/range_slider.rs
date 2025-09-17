use egui::{Id, Pos2, Response, Sense, Ui, Vec2, Widget};

pub struct RangeSlider<'a> {
    min: &'a mut f64,
    max: &'a mut f64,
    range: std::ops::RangeInclusive<f64>,
    step: f64,
    min_span: f64,
    label: Option<String>,
}

impl<'a> RangeSlider<'a> {
    pub fn new(min: &'a mut f64, max: &'a mut f64, range: std::ops::RangeInclusive<f64>) -> Self {
        Self {
            min,
            max,
            range,
            step: 0.0,
            min_span: 0.0,
            label: None,
        }
    }
    pub fn step(mut self, step: f64) -> Self {
        self.step = step.max(0.0);
        self
    }
    pub fn min_span(mut self, span: f64) -> Self {
        self.min_span = span.max(0.0);
        self
    }
    pub fn text(mut self, t: impl Into<String>) -> Self {
        self.label = Some(t.into());
        self
    }
}

impl<'a> Widget for RangeSlider<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            min,
            max,
            range,
            step,
            min_span,
            label,
        } = self;

        let inner = ui.horizontal(|ui| {
            if let Some(lbl) = &label {
                ui.label(lbl);
            }

            let h = ui.spacing().interact_size.y;
            let w = ui.available_width();
            let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click_and_drag());

            // helpers
            let r0 = *range.start();
            let r1 = *range.end();
            let to_px = |v: f64| -> f32 {
                let t = ((v - r0) / (r1 - r0)).clamp(0.0, 1.0);
                rect.left() + t as f32 * rect.width()
            };
            let to_val = |x: f32| -> f64 {
                let t = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
                r0 + t * (r1 - r0)
            };
            let snap = |v: f64| {
                if step > 0.0 {
                    (v / step).round() * step
                } else {
                    v
                }
            };

            // persistent state across frames
            let id_vals: Id = resp.id.with("_vals"); // (preview_min, preview_max)
            let id_active: Id = resp.id.with("_active_min"); // which thumb

            // preview values (fallback to current committed output values)
            let mut preview = ui
                .memory(|m| m.data.get_temp::<(f64, f64)>(id_vals))
                .unwrap_or((*min, *max));
            preview.0 = preview.0.clamp(r0, r1);
            preview.1 = preview.1.clamp(r0, r1);

            // choose active thumb on drag start
            if resp.drag_started() {
                if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let xm = to_px(preview.0);
                    let xM = to_px(preview.1);
                    let active_min = (pos.x - xm).abs() <= (pos.x - xM).abs();
                    ui.memory_mut(|m| {
                        m.data.insert_temp(id_active, active_min);
                        m.data.insert_temp(id_vals, preview);
                    });
                }
            }

            // during drag: update preview, write-through (like Slider), mark changed
            if resp.dragged() {
                if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let active_min = ui
                        .memory(|m| m.data.get_temp::<bool>(id_active))
                        .unwrap_or(true);
                    let mut v = snap(to_val(pos.x)).clamp(r0, r1);
                    if active_min {
                        v = v.min(preview.1 - min_span.max(step));
                        if (v - preview.0).abs() > f64::EPSILON {
                            preview.0 = v;
                        }
                    } else {
                        v = v.max(preview.0 + min_span.max(step));
                        if (v - preview.1).abs() > f64::EPSILON {
                            preview.1 = v;
                        }
                    }
                    ui.memory_mut(|m| m.data.insert_temp(id_vals, preview));

                    // write-through every drag frame (matches Slider behavior)
                    let mut out_min = snap(preview.0).clamp(r0, r1);
                    let mut out_max = snap(preview.1).clamp(r0, r1);
                    if out_max <= out_min {
                        out_max = (out_min + step.max(min_span)).min(r1);
                    }
                    if (*min - out_min).abs() > f64::EPSILON {
                        *min = out_min;
                        resp.mark_changed();
                    }
                    if (*max - out_max).abs() > f64::EPSILON {
                        *max = out_max;
                        resp.mark_changed();
                    }
                }
            }

            // commit on release (or if mouse is up but preview still around)
            let mouse_up_now = ui.input(|i| !i.pointer.button_down(egui::PointerButton::Primary));
            let lost_preview = ui
                .memory(|m| m.data.get_temp::<(f64, f64)>(id_vals))
                .is_none();

            if resp.drag_stopped() || (mouse_up_now && !lost_preview) {
                // final snap & commit
                let mut out_min = snap(preview.0).clamp(r0, r1);
                let mut out_max = snap(preview.1).clamp(r0, r1);
                if out_max <= out_min {
                    out_max = (out_min + step.max(min_span)).min(r1);
                }

                let wrote_min = if (*min - out_min).abs() > f64::EPSILON {
                    *min = out_min;
                    true
                } else {
                    false
                };
                let wrote_max = if (*max - out_max).abs() > f64::EPSILON {
                    *max = out_max;
                    true
                } else {
                    false
                };

                // **ensure changed on the release frame** (critical for your flow)
                if wrote_min || wrote_max {
                    resp.mark_changed();
                } else {
                    // even if identical after snapping, emulate Slider's "release frame can still be observed"
                    resp.mark_changed();
                }

                // clear state
                ui.memory_mut(|m| {
                    m.data.remove::<bool>(id_active);
                    m.data.remove::<(f64, f64)>(id_vals);
                });
            }

            // paint with preview so thumbs move smoothly
            let visuals = ui.style().visuals.clone();
            let painter = ui.painter();
            let y = rect.center().y;

            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                visuals.widgets.inactive.fg_stroke,
            );

            let x_min = to_px(preview.0);
            let x_max = to_px(preview.1);

            painter.line_segment(
                [Pos2::new(x_min, y), Pos2::new(x_max, y)],
                visuals.selection.stroke,
            );

            let r = (h * 0.35).clamp(5.0, 9.0);
            painter.circle_filled(Pos2::new(x_min, y), r, visuals.widgets.inactive.bg_fill);
            painter.circle_stroke(Pos2::new(x_min, y), r, visuals.widgets.active.fg_stroke);
            painter.circle_filled(Pos2::new(x_max, y), r, visuals.widgets.inactive.bg_fill);
            painter.circle_stroke(Pos2::new(x_max, y), r, visuals.widgets.active.fg_stroke);

            resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        });

        inner.response
    }
}
