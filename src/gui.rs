use eframe::{egui, App, CreationContext, NativeOptions};
// use serde_json as json;
use std::sync::{atomic::AtomicUsize, mpsc::Sender, Arc, Mutex};

use crate::{
    notes::{Interval, Sequence},
    scheduler::Message,
    waves::WaveType,
    LOOP_LEN,
};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::atomic::Ordering;

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

pub struct GuiApp {
    seqs: Arc<Mutex<Vec<Sequence>>>,     // NEW: live shared sequences
    selected: Option<usize>,             // currently picked sequence index
    clock: Option<Arc<Mutex<f64>>>,      // shared play-head seconds from audio
    fall_back_start: std::time::Instant, // for standalone demo
    last_token: AtomicUsize,
    messages: Sender<Message>,
}

#[derive(Serialize, Deserialize, Clone)]
struct GuiState {
    seqs: Vec<Sequence>,
    selected: Option<usize>,
}

impl GuiApp {
    pub fn new(
        _cc: &CreationContext<'_>,
        clock: Option<Arc<Mutex<f64>>>,
        shared: Arc<Mutex<Vec<Sequence>>>,
        messages: Sender<Message>,
    ) -> Self {
        *shared.lock().unwrap() = Vec::new();

        Self {
            seqs: shared,
            selected: None,
            clock,
            fall_back_start: std::time::Instant::now(),
            last_token: 0.into(),

            messages,
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

    fn exit(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn save_state(&self) {
        // Choose where to save
        if let Some(path) = FileDialog::new()
            .set_title("Save session as JSON")
            .add_filter("JSON", &["json"])
            .save_file()
        {
            // Snapshot current state
            let seqs = self.seqs.lock().unwrap().clone();
            let state = GuiState {
                seqs,
                selected: self.selected,
            };

            match serde_json::to_string_pretty(&state) {
                Ok(text) => {
                    if let Err(e) = fs::write(&path, text) {
                        eprintln!("[save_state] Failed to write file: {e}");
                    }
                }
                Err(e) => eprintln!("[save_state] Failed to serialize: {e}"),
            }
        }
    }

    fn load_state(&mut self) {
        // Pick a file to open
        if let Some(path) = FileDialog::new()
            .set_title("Load session from JSON")
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            match fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<GuiState>(&text) {
                    Ok(state) => {
                        // Clear current score on the audio side
                        let _ = self.messages.send(Message::NewScore);

                        // Send each sequence to the scheduler
                        for seq in &state.seqs {
                            let _ = self.messages.send(Message::NewSequence(seq.clone()));
                        }

                        // Update the shared mirror immediately so the UI reflects it right away
                        if let Ok(mut shared) = self.seqs.lock() {
                            *shared = state.seqs.clone();
                        }

                        // Keep tokens monotonic for future "Add track"
                        let next_token = state
                            .seqs
                            .iter()
                            .map(|s| s.token)
                            .max()
                            .unwrap_or(0)
                            .saturating_add(1);
                        self.last_token.store(next_token, Ordering::Relaxed);

                        // Restore selection (clamped)
                        self.selected = state.selected.and_then(|i| {
                            if i < state.seqs.len() {
                                Some(i)
                            } else {
                                None
                            }
                        });
                    }
                    Err(e) => eprintln!("[load_state] Failed to parse JSON: {e}"),
                },
                Err(e) => eprintln!("[load_state] Failed to read file: {e}"),
            }
        }
    }
}

impl App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_time = self.current_time();
        let len = self.seqs.lock().unwrap().len();
        // -------- top bar --------
        let mut save = false;
        let mut load = false;
        let mut exit = false;
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New score").clicked() {
                    self.messages.send(Message::NewScore).unwrap();
                    self.selected = None;
                }
                if ui.button("Add track").clicked() {
                    let idx = len;
                    self.last_token
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let seq =
                        Sequence::new(self.last_token.load(std::sync::atomic::Ordering::Relaxed));
                    // seqs.push(seq);
                    self.messages.send(Message::NewSequence(seq)).unwrap();
                    self.selected = Some(idx);
                }
                save = ui.button("Save…").clicked();
                load = ui.button("Load…").clicked();
                exit = ui.button("Exit").clicked();
            });
        });

        if save {
            self.save_state();
        }
        if load {
            self.load_state();
        }
        if exit {
            self.exit(ctx);
        }
        let seqs = self.seqs.clone();

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
                let mut edited_seq: Option<Sequence> = None;

                if let Some(sel) = self.selected {
                    if sel < len {
                        let seq = seqs.lock().unwrap()[sel].clone();

                        ui.heading(format!("Track {}", sel + 1));

                        // ── delete / move buttons ─────────────────────────
                        let can_up = sel > 0;
                        let can_down = sel + 1 < len; // ← NEW: use cached len

                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                action = Action::Delete;
                            }
                            if ui.button("move up").clicked() && can_up {
                                action = Action::Up;
                            }
                            if ui.button("move down").clicked() && can_down {
                                action = Action::Down;
                            }
                        });

                        ui.separator();
                        // ---- WaveType picker ----
                        // let mut w_choice = seq.w;
                        let mut w_choice = seq.w;

                        egui::ComboBox::from_id_source("wave_type_combo")
                            .selected_text((&w_choice).to_string())
                            .show_ui(ui, |ui| {
                                for var in ALL_WAVES.iter() {
                                    ui.selectable_value(&mut w_choice, *var, (&var).to_string());
                                }
                            });
                        if w_choice != seq.w {
                            edited_seq
                                .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                .w = w_choice;
                        }

                        ui.separator();

                        // ---- numeric fields ----
                        let mut t_min = seq.t_min;
                        let mut t_max = seq.t_max;
                        ui.add(egui::Slider::new(&mut t_min, 0.0..=LOOP_LEN).text("t_min"));
                        ui.add(egui::Slider::new(&mut t_max, 0.0..=LOOP_LEN).text("t_max"));
                        if t_max < t_min {
                            t_max = t_min;
                        }
                        if (t_min - seq.t_min).abs() > f64::EPSILON {
                            edited_seq
                                .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                .t_min = t_min;
                        }
                        if (t_max - seq.t_max).abs() > f64::EPSILON {
                            edited_seq
                                .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                .t_max = t_max;
                        }

                        let mut attack_decay = seq.attack_decay;
                        if ui
                            .add(
                                egui::Slider::new(&mut attack_decay.0, 0.01..=100.0)
                                    .text("attack")
                                    .logarithmic(true),
                            )
                            .changed()
                            || ui
                                .add(
                                    egui::Slider::new(&mut attack_decay.1, 0.01..=100.0)
                                        .text("decay")
                                        .logarithmic(true),
                                )
                                .changed()
                        {
                            edited_seq
                                .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                .attack_decay = attack_decay;
                        };

                        let mut attack_freq_modulation = seq.attack_freq_modulation;
                        if ui
                            .add(
                                egui::Slider::new(&mut attack_freq_modulation.0, -0.1..=0.1)
                                    .text("attack freq mod mag"),
                            )
                            .changed()
                            || ui
                                .add(
                                    egui::Slider::new(&mut attack_freq_modulation.1, 1.0..=100.0)
                                        .text("attack freq mod time")
                                        .logarithmic(true),
                                )
                                .changed()
                        {
                            edited_seq
                                .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                .attack_freq_modulation = attack_freq_modulation;
                        };

                        let mut vibrato = seq.vibrato;
                        let mut vibrato_mag_display = vibrato.0 * 1e6;
                        if ui
                            .add(
                                egui::Slider::new(&mut vibrato_mag_display, 0.0..=500.0)
                                    .text("vibrato mag"),
                            )
                            .changed()
                            || ui
                                .add(
                                    egui::Slider::new(&mut vibrato.1, 0.01..=100.0)
                                        .text("vibrato fq")
                                        .logarithmic(true),
                                )
                                .changed()
                        {
                            edited_seq
                                .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                .vibrato = (vibrato_mag_display * 1e-6, vibrato.1);
                        };

                        let mut chorus = seq.chorus;
                        if ui
                            .add(egui::Slider::new(&mut chorus.0, 1..=10).text("chorus n"))
                            .changed()
                            || ui
                                .add(
                                    egui::Slider::new(&mut chorus.1, 1e-4..=1e-2)
                                        .text("chorus delta")
                                        .logarithmic(true),
                                )
                                .changed()
                            || ui
                                .add(
                                    egui::Slider::new(&mut chorus.2, 0.0..=1.0)
                                        .text("chorus decay")
                                        .logarithmic(true),
                                )
                                .changed()
                        {
                            edited_seq
                                .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                .chorus = chorus;
                        };

                        // step
                        ui.horizontal(|ui| {
                            let mut tmp_step = seq.step.clone();
                            ui.label("step:");
                            if ui
                                .add(egui::DragValue::new(&mut tmp_step.0).range(1..=128))
                                .changed()
                            {
                                edited_seq
                                    .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                    .step
                                    .0 = tmp_step.0
                            };
                            ui.label("/");
                            if ui
                                .add(egui::DragValue::new(&mut tmp_step.1).range(1..=128))
                                .changed()
                            {
                                edited_seq
                                    .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                    .step
                                    .1 = tmp_step.1
                            };
                        });

                        // skips
                        ui.horizontal(|ui| {
                            let mut tmp_skips = seq.skips.clone();
                            ui.label("skips:");
                            if ui
                                .add(egui::DragValue::new(&mut tmp_skips.0).range(0..=512))
                                .changed()
                            {
                                edited_seq
                                    .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                    .skips
                                    .0 = tmp_skips.0
                            };
                            ui.label(",");
                            if ui
                                .add(egui::DragValue::new(&mut tmp_skips.1).range(0..=512))
                                .changed()
                            {
                                edited_seq
                                    .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                    .skips
                                    .1 = tmp_skips.1
                            };
                        });

                        {
                            let mut changed = false;
                            let mut f = seq.f.clone();
                            if let Interval::RDTempered(
                                ref mut nb_rd_steps,
                                ref mut tones,
                                ref mut octave,
                            ) = f
                            {
                                // octave
                                ui.horizontal(|ui| {
                                    ui.label("octave:");
                                    if ui.add(egui::DragValue::new(octave).range(-5..=5)).changed()
                                    {
                                        changed = true;
                                    };
                                });
                                // nb_rd_steps
                                ui.horizontal(|ui| {
                                    ui.label("nb_rd_steps:");
                                    if ui
                                        .add(egui::DragValue::new(nb_rd_steps).range(0..=16))
                                        .changed()
                                    {
                                        changed = true;
                                    };
                                });
                                // ----- RDTempered tones (–11 … 11) ---------------------------------
                                ui.label("RD tones:");
                                ui.horizontal_wrapped(|ui| {
                                    for tone in -11..=11 {
                                        let mut selected = tones.contains(&tone);

                                        // show the checkbox; the label *is* the number
                                        if ui.checkbox(&mut selected, tone.to_string()).changed() {
                                            if selected {
                                                // add if absent
                                                if !tones.contains(&tone) {
                                                    tones.push(tone);
                                                    tones.sort_unstable();
                                                }
                                            } else {
                                                // remove if present
                                                if let Some(pos) =
                                                    tones.iter().position(|&v| v == tone)
                                                {
                                                    tones.remove(pos);
                                                }
                                            }
                                            changed = true;
                                        }
                                    }
                                });
                            }
                            if changed {
                                edited_seq
                                    .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                    .f = f;
                            }
                        }

                        // beat_offset
                        ui.horizontal(|ui| {
                            let mut tmp_beat_offset = seq.beat_offset.clone();
                            ui.label("beat_offset:");
                            if ui
                                .add(egui::DragValue::new(&mut tmp_beat_offset).range(0..=256))
                                .changed()
                            {
                                edited_seq
                                    .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                    .beat_offset = tmp_beat_offset;
                            };
                        });

                        // volume
                        ui.horizontal(|ui| {
                            let mut tmp_volume = seq.volume.clone();
                            ui.label("volume:");
                            if ui
                                .add(egui::Slider::new(&mut tmp_volume, 0.0..=32.0).text("volume"))
                                .changed()
                            {
                                edited_seq
                                    .get_or_insert(seqs.lock().unwrap()[sel].clone())
                                    .volume = tmp_volume;
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
                            self.messages.send(Message::DeleteSequence(sel)).unwrap();
                            self.selected = if sel == 0 { None } else { Some(sel - 1) };
                        }
                    }
                    Action::Up => {
                        if let Some(sel) = self.selected {
                            self.messages
                                .send(Message::SwapSequences(sel, sel - 1))
                                .unwrap();
                            self.selected = Some(sel - 1);
                        }
                    }
                    Action::Down => {
                        if let Some(sel) = self.selected {
                            self.messages
                                .send(Message::SwapSequences(sel, sel + 1))
                                .unwrap();
                            self.selected = Some(sel + 1);
                        }
                    }
                }
                if let Some(edited_seq) = edited_seq {
                    if let Some(sel) = self.selected {
                        self.messages
                            .send(Message::EditSequence(sel, edited_seq))
                            .unwrap();
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

            let lanes = len.max(1);
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
            for (idx, seq) in seqs.lock().unwrap().iter().enumerate() {
                let top = rect.top() + idx as f32 * lane_h + lane_gap;
                let y0 = top;
                let y1 = top + block_h;
                let x0 = Self::t_to_x(rect, (seq.t_min - current_time).rem_euclid(LOOP_LEN));
                let x1 = Self::t_to_x(rect, (seq.t_max - current_time).rem_euclid(LOOP_LEN));

                let block_rect = egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1));
                let block_rect_l = egui::Rect::from_min_max(
                    egui::pos2(Self::t_to_x(rect, 0.0), y0),
                    egui::pos2(x1, y1),
                );
                let block_rect_r = egui::Rect::from_min_max(
                    egui::pos2(x0, y0),
                    egui::pos2(Self::t_to_x(rect, LOOP_LEN), y1),
                );
                let track_rect = egui::Rect::from_min_max(
                    egui::pos2(Self::t_to_x(rect, 0.0), y0),
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

        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

// ------------------------------------------------------------

pub fn run_gui(
    clock: Option<Arc<Mutex<f64>>>,
    shared: Arc<Mutex<Vec<Sequence>>>,
    messages: Sender<Message>,
) {
    let native_options = NativeOptions::default();
    let _ = eframe::run_native(
        "Notes GUI",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(GuiApp::new(
                cc,
                clock.clone(),
                shared.clone(),
                messages,
            )))
        }),
    );
}

// simple deterministic hash for colour
fn hash32(s: &str) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish() as u32
}
