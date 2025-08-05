use eframe::{egui, App, CreationContext, NativeOptions};
use serde_json as json;

use crate::{notes::Sequence, waves::WaveType}; // reuse your existing data type

/// Fixed loop length (seconds) for the horizontal axis
const LOOP_LEN: f64 = 64.0;

pub struct GuiApp {
    seqs: Vec<Sequence>,            // all sequences from notes.json
    tracks: Vec<WaveType>,          // unique `w` values = lanes
    start_wall: std::time::Instant, // for play-time read-out (optional)
}

impl GuiApp {
    pub fn new(_cc: &CreationContext<'_>) -> Self {
        // Load notes.json once at start. If parsing fails, start empty.
        let seqs: Vec<Sequence> = std::fs::read_to_string("notes.json")
            .ok()
            .and_then(|s| json::from_str(&s).ok())
            .unwrap_or_default();

        // Collect unique track names (= Sequence.w) preserving insertion order.
        let mut tracks = Vec::<WaveType>::new();
        for s in &seqs {
            if !tracks.contains(&s.w) {
                tracks.push(s.w.clone());
            }
        }
        // if tracks.is_empty() {
        //     tracks.push("Track".into());
        // }

        Self {
            seqs,
            tracks,
            start_wall: std::time::Instant::now(),
        }
    }

    /// Convert time (0..64) → x-coordinate inside the provided rectangle
    fn t_to_x(rect: egui::Rect, t: f64) -> f32 {
        let clamped = t.clamp(0.0, LOOP_LEN) as f32;
        rect.left() + clamped / LOOP_LEN as f32 * rect.width()
    }
}

impl App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎛 Sequences timeline (read-only)");
            ui.label(format!(
                "Loaded {} sequences, {} tracks",
                self.seqs.len(),
                self.tracks.len()
            ));
            ui.separator();

            // Reserve the rest of the panel for the timeline drawing.
            let (rect, _resp) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), ui.available_height()),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(rect);

            // Geometry constants
            let lanes = self.tracks.len();
            let lane_h = rect.height() / lanes as f32;
            let block_h = lane_h * 0.6;
            let lane_gap = (lane_h - block_h) * 0.5;

            // Draw grid every 4s (optional)
            for s in (0..=16).map(|i| i as f64 * 4.0) {
                // 0,4,8,…,64
                let x = Self::t_to_x(rect, s);
                let col = if (s as i32) % 16 == 0 {
                    egui::Color32::from_gray(100)
                } else {
                    egui::Color32::from_gray(60)
                };
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(1.0, col),
                );
            }

            // Draw each lane outline + label
            for (lane_idx, name) in self.tracks.iter().enumerate() {
                let top = rect.top() + lane_idx as f32 * lane_h;
                let lane_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left(), top),
                    egui::vec2(rect.width(), lane_h),
                );
                painter.rect_stroke(
                    lane_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
                );
                painter.text(
                    egui::pos2(lane_rect.left() + 6.0, lane_rect.top() + 4.0),
                    egui::Align2::LEFT_TOP,
                    name,
                    egui::TextStyle::Body.resolve(ui.style()),
                    egui::Color32::LIGHT_GRAY,
                );
            }

            // Draw blocks for each sequence
            for seq in &self.seqs {
                let lane_idx = self.tracks.iter().position(|s| s == &seq.w).unwrap_or(0);
                let top = rect.top() + lane_idx as f32 * lane_h + lane_gap;
                let y0 = top;
                let y1 = top + block_h;
                let x0 = Self::t_to_x(rect, seq.t_min);
                let x1 = Self::t_to_x(rect, seq.t_max).max(x0 + 4.0); // min width

                let block_rect = egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1));
                let fill = egui::Color32::from_rgb(
                    (hash32(&(&seq.w).to_string()) & 0xFF) as u8,
                    ((hash32(&(&seq.w).to_string()) >> 8) & 0xFF) as u8,
                    ((hash32(&(&seq.w).to_string()) >> 16) & 0xFF) as u8,
                );
                painter.rect_filled(block_rect, 4.0, fill);
            }

            // Optional: running-time display in top-left corner
            painter.text(
                egui::pos2(rect.left() + 4.0, rect.top() + 4.0),
                egui::Align2::LEFT_TOP,
                format!("t {:.1}s", self.start_wall.elapsed().as_secs_f64()),
                egui::TextStyle::Small.resolve(ui.style()),
                egui::Color32::LIGHT_GRAY,
            );
        });

        // Repaint 60 FPS so the running-time counter updates.
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

/// Fast but stable hash → pseudo-random colour per track
fn hash32(s: &str) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish() as u32
}

/// Call this from `main.rs`; it blocks until the window is closed.
pub fn run_gui() {
    let native_options = NativeOptions::default();
    let _ = eframe::run_native(
        "Notes GUI",
        native_options,
        Box::new(|cc| Ok(Box::new(GuiApp::new(cc)))),
    );
}
