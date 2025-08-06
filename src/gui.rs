use eframe::{egui, App, CreationContext, NativeOptions};
use serde_json as json;
use std::sync::{Arc, Mutex};

use crate::{notes::Sequence, waves::WaveType};

// list of all wave variants for the ComboBox
const ALL_WAVES: [WaveType; 14] = [
    WaveType::Sine,
    WaveType::Square,
    WaveType::Triangle,
    WaveType::Sawtooth,
    WaveType::DistOrg,
    WaveType::Custom2,
    WaveType::Droplet,
    WaveType::DropletOct,
    WaveType::HiHat,
    WaveType::Kick,
    WaveType::Snare,
    WaveType::Ride,
    WaveType::Mute,
    WaveType::Xylo,
];

// ------------------------------------------------------------
const LOOP_LEN: f64 = 64.0; // seconds

pub struct GuiApp {
    seqs: Vec<Sequence>,                 // editable score
    selected: Option<usize>,             // currently picked sequence index
    clock: Option<Arc<Mutex<f64>>>,      // shared play‑head seconds from audio
    dirty: bool,                         // unsaved edits?
    fall_back_start: std::time::Instant, // for standalone demo
}

impl GuiApp {
    pub fn new(_cc: &CreationContext<'_>, clock: Option<Arc<Mutex<f64>>>) -> Self {
        let seqs: Vec<Sequence> = std::fs::read_to_string("notes.json")
            .ok()
            .and_then(|s| json::from_str(&s).ok())
            .unwrap_or_default();
        Self {
            seqs,
            selected: None,
            clock,
            dirty: false,
            fall_back_start: std::time::Instant::now(),
        }
    }

    fn t_to_x(rect: egui::Rect, t: f64) -> f32 {
        rect.left() + t as f32 / LOOP_LEN as f32 * rect.width()
    }

    fn brighten(col: egui::Color32) -> egui::Color32 {
        let [r, g, b, a] = col.to_array();
        egui::Color32::from_rgba_premultiplied(
            r.saturating_add(40),
            g.saturating_add(40),
            b.saturating_add(40),
            a,
        )
    }

    fn hash_color(w: &WaveType) -> egui::Color32 {
        let txt = format!("{:?}", w.to_string());
        let hash = hash32(&txt);
        egui::Color32::from_rgb(
            (hash & 0xFF) as u8,
            ((hash >> 8) & 0xFF) as u8,
            ((hash >> 16) & 0xFF) as u8,
        )
    }

    fn current_time(&self) -> f64 {
        if let Some(clk) = &self.clock {
            *clk.lock().unwrap()
        } else {
            self.fall_back_start.elapsed().as_secs_f64()
        }
    }

    fn save_to_file(&mut self) {
        match json::to_string_pretty(&self.seqs) {
            Ok(s) => {
                if std::fs::write("notes.json", s).is_ok() {
                    self.dirty = false;
                }
            }
            Err(e) => eprintln!("Save failed: {e}"),
        }
    }
}

impl App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // -------- top bar --------
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    self.save_to_file();
                }
                if ui.button("New score").clicked() {
                    self.seqs.clear();
                    self.selected = None;
                    self.dirty = true;
                }
                if ui.button("Add track").clicked() {
                    let idx = self.seqs.len();
                    self.seqs.push(Sequence::default());
                    self.selected = Some(idx);
                    self.dirty = true;
                }
                if self.dirty {
                    ui.colored_label(egui::Color32::YELLOW, "unsaved");
                }
            });
        });

        // -------- property pane --------
        egui::SidePanel::right("props")
            .default_width(230.0)
            .show(ctx, |ui| {
                // Which structural edit (if any) should happen after the UI is drawn?
                enum Action {
                    None,
                    Delete,
                    Up,
                    Down,
                }
                let mut action = Action::None;

                if let Some(sel) = self.selected {
                    let seq_len = self.seqs.len(); // ← NEW (immutable, early)

                    if sel < seq_len {
                        let seq = &mut self.seqs[sel]; // mutable borrow starts here

                        ui.heading(format!("Track {}", sel + 1));

                        // ── delete / move buttons ─────────────────────────
                        let can_up = sel > 0;
                        let can_down = sel + 1 < seq_len; // ← NEW: use cached len

                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                action = Action::Delete;
                                self.dirty = true;
                            }
                            if ui.button("↑").clicked() && can_up {
                                action = Action::Up;
                                self.dirty = true;
                            }
                            if ui.button("↓").clicked() && can_down {
                                action = Action::Down;
                                self.dirty = true;
                            }
                        });

                        ui.separator();
                        // ---- WaveType picker ----
                        let mut w_choice = seq.w;
                        egui::ComboBox::from_id_source("wave_type_combo")
                            .selected_text((&w_choice).to_string())
                            .show_ui(ui, |ui| {
                                for var in ALL_WAVES.iter() {
                                    ui.selectable_value(&mut w_choice, *var, (&var).to_string());
                                }
                            });
                        if w_choice != seq.w {
                            seq.w = w_choice;
                            self.dirty = true;
                        }

                        ui.separator();

                        // ---- numeric fields ----
                        let mut t_min = seq.t_min;
                        let mut t_max = seq.t_max;
                        ui.add(egui::Slider::new(&mut t_min, 0.0..=LOOP_LEN).text("t_min"));
                        ui.add(egui::Slider::new(&mut t_max, 0.0..=LOOP_LEN).text("t_max"));
                        if t_max < t_min {
                            t_max = t_min;
                            self.dirty = true;
                        }
                        if (t_min - seq.t_min).abs() > f64::EPSILON {
                            seq.t_min = t_min;
                            self.dirty = true;
                        }
                        if (t_max - seq.t_max).abs() > f64::EPSILON {
                            seq.t_max = t_max;
                            self.dirty = true;
                        }

                        // step
                        ui.horizontal(|ui| {
                            ui.label("step:");
                            if ui
                                .add(egui::DragValue::new(&mut seq.step.0).range(1..=128))
                                .changed()
                            {
                                self.dirty = true
                            };
                            ui.label("/");
                            if ui
                                .add(egui::DragValue::new(&mut seq.step.1).range(1..=128))
                                .changed()
                            {
                                self.dirty = true
                            };
                        });

                        // skips
                        ui.horizontal(|ui| {
                            ui.label("skips:");
                            if ui
                                .add(egui::DragValue::new(&mut seq.skips.0).range(0..=512))
                                .changed()
                            {
                                self.dirty = true
                            };
                            ui.label(",");
                            if ui
                                .add(egui::DragValue::new(&mut seq.skips.1).range(0..=512))
                                .changed()
                            {
                                self.dirty = true
                            };
                        });

                        // beat_offset
                        ui.horizontal(|ui| {
                            ui.label("beat_offset:");
                            if ui
                                .add(egui::DragValue::new(&mut seq.beat_offset).range(0..=256))
                                .changed()
                            {
                                self.dirty = true
                            };
                        });

                        // volume
                        ui.horizontal(|ui| {
                            ui.label("volume:");
                            if ui
                                .add(egui::Slider::new(&mut seq.volume, 0.0..=32.0).text("volume"))
                                .changed()
                            {
                                self.dirty = true
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
                            self.seqs.remove(sel);
                            self.selected = if sel == 0 { None } else { Some(sel - 1) };
                            self.dirty = true;
                        }
                    }
                    Action::Up => {
                        if let Some(sel) = self.selected {
                            self.seqs.swap(sel, sel - 1);
                            self.selected = Some(sel - 1);
                            self.dirty = true;
                        }
                    }
                    Action::Down => {
                        if let Some(sel) = self.selected {
                            self.seqs.swap(sel, sel + 1);
                            self.selected = Some(sel + 1);
                            self.dirty = true;
                        }
                    }
                }
            });

        // -------- main timeline --------
        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, _resp) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), ui.available_height()),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(rect);

            let lanes = self.seqs.len().max(1);
            let lane_h = rect.height() / lanes as f32;
            let block_h = lane_h * 0.6;
            let lane_gap = (lane_h - block_h) * 0.5;

            // grid
            for s in (0..=16).map(|i| i as f64 * 4.0) {
                let x = Self::t_to_x(rect, s);
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
            for (idx, seq) in self.seqs.iter().enumerate() {
                let top = rect.top() + idx as f32 * lane_h + lane_gap;
                let y0 = top;
                let y1 = top + block_h;
                let x0 = Self::t_to_x(rect, (seq.t_min - self.current_time()).rem_euclid(LOOP_LEN));
                let x1 = Self::t_to_x(rect, (seq.t_max - self.current_time()).rem_euclid(LOOP_LEN));

                let block_rect = egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1));
                let block_rect_l =
                    egui::Rect::from_min_max(egui::pos2(0.0, y0), egui::pos2(x1, y1));
                let block_rect_r = egui::Rect::from_min_max(
                    egui::pos2(x0, y0),
                    egui::pos2(Self::t_to_x(rect, LOOP_LEN), y1),
                );
                let mut col = Self::hash_color(&seq.w);
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
                    .interact(block_rect, egui::Id::new(idx), egui::Sense::click())
                    .clicked()
                    || ui
                        .interact(block_rect_l, egui::Id::new(idx), egui::Sense::click())
                        .clicked()
                    || ui
                        .interact(block_rect_r, egui::Id::new(idx), egui::Sense::click())
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

            // // play‑head
            // let ph = self.current_time() % LOOP_LEN;
            // let x = Self::t_to_x(rect, ph);
            // painter.line_segment(
            //     [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            //     egui::Stroke::new(2.0, egui::Color32::LIGHT_GREEN),
            // );
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

// ------------------------------------------------------------

pub fn run_gui(clock: Option<Arc<Mutex<f64>>>) {
    let native_options = NativeOptions::default();
    let _ = eframe::run_native(
        "Notes GUI",
        native_options,
        Box::new(move |cc| Ok(Box::new(GuiApp::new(cc, clock.clone())))),
    );
}

// simple deterministic hash for colour
fn hash32(s: &str) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish() as u32
}
