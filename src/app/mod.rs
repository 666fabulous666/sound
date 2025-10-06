mod load;
mod property_panel;
mod save;
mod start_page;
mod timeline_panel;
mod top_panel;

use crate::{
    engine::{
        score::{
            default_params::default_delays, sequence::Sequence, track_node::TrackNode, NotesGroup,
            Score,
        },
        waves::WaveType,
    },
    shortcuts::*,
    texts::README_MD,
    time_freq::Time,
    Token, TokenGen, GENERATE_EARLY, GROOVE_DEFAULTS, NOTE_LINGER_TIME,
};
use arc_swap::ArcSwap;
use cpal::Stream;
use cpal::{traits::DeviceTrait, Device};
use eframe::{egui, App, CreationContext};
use egui::{Color32, Layout, ScrollArea, TextureHandle};
use egui_commonmark::CommonMarkCache;
use instant::Duration;
#[cfg(target_arch = "wasm32")]
use instant::Instant;
use rand::{rngs::ThreadRng, thread_rng};
use serde::{Deserialize, Serialize};
use std::{
    ops::DerefMut,
    sync::{atomic::AtomicU64, Arc},
};

const ALL_WAVES: [WaveType; 10] = [
    WaveType::Mute,
    WaveType::Sine,
    WaveType::Square,
    WaveType::Triangle,
    WaveType::Sawtooth,
    WaveType::HiHat,
    WaveType::Kick,
    WaveType::Snare,
    WaveType::Ride,
    WaveType::Darbuka,
];

const DRUM_WAVES: [WaveType; 5] = [
    WaveType::HiHat,
    WaveType::Kick,
    WaveType::Snare,
    WaveType::Ride,
    WaveType::Darbuka,
];

pub struct GuiApp {
    tempo: f64,
    score: Score,
    // notes: Vec<NotesGroup>,
    // shared_notes: Arc<ArcSwap<Vec<NotesGroup>>>,
    // sequences: Vec<Sequence>,
    clock: Arc<AtomicU64>,
    rng: ThreadRng,
    selected: Option<usize>,
    // last_token: TokenGen,
    stream: Option<Stream>,
    device: Device,
    sample_rate: f64,
    // delays: (Vec<f64>, Vec<f64>),
    shared_delays: Arc<ArcSwap<(Vec<f64>, Vec<f64>)>>,
    #[cfg(target_arch = "wasm32")]
    pub(crate) pending_loaded_bytes: std::rc::Rc<std::cell::RefCell<Option<Vec<u8>>>>,
    #[cfg(target_arch = "wasm32")]
    instant: Instant,
    #[cfg(target_arch = "wasm32")]
    fps: f64,
    min_fps: f64,
    show_start: bool,
    show_doc: bool,
    markdown_cache: CommonMarkCache,
    show_default_picker: bool,
    default_pick_idx: usize,
    logo: Option<TextureHandle>,
    property_panel_width: f32,
}

use serde::Deserializer;

#[derive(Deserialize)]
#[serde(untagged)]
enum TrackNodeOrSequence {
    Node(TrackNode), // e.g. { "Seq": { ... } } or future { "Group": {...} }
    Seq(Sequence),   // legacy: raw Sequence object at array element
}

fn compat_nodes<'de, D>(d: D) -> Result<Vec<TrackNode>, D::Error>
where
    D: Deserializer<'de>,
{
    let items: Vec<TrackNodeOrSequence> = Deserialize::deserialize(d)?;
    Ok(items
        .into_iter()
        .map(|it| match it {
            TrackNodeOrSequence::Node(n) => n,
            TrackNodeOrSequence::Seq(s) => TrackNode::Seq(s),
        })
        .collect())
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GuiState {
    #[serde(deserialize_with = "compat_nodes")]
    pub seqs: Vec<TrackNode>, // now works with both old and new files
    pub selected: Option<usize>,
    #[serde(default = "default_delays")]
    pub delays: (Vec<f64>, Vec<f64>),
}

impl GuiApp {
    pub fn new(_cc: &CreationContext<'_>, device: Device) -> Self {
        let app = Self {
            tempo: default_tempo(),
            selected: None,
            stream: None,
            shared_delays: Arc::new(ArcSwap::from_pointee((Vec::new(), Vec::new()))),
            clock: Arc::new(AtomicU64::new(0)),
            rng: thread_rng(),
            sample_rate: device.default_output_config().unwrap().sample_rate().0 as f64,
            device: device,
            #[cfg(target_arch = "wasm32")]
            pending_loaded_bytes: std::rc::Rc::new(std::cell::RefCell::new(None)),
            #[cfg(target_arch = "wasm32")]
            instant: Instant::now(),
            #[cfg(target_arch = "wasm32")]
            fps: 60.0,
            min_fps: 60.0,
            show_start: true,
            show_doc: false,
            markdown_cache: egui_commonmark::CommonMarkCache::default(),
            show_default_picker: false,
            default_pick_idx: 0,
            logo: None,
            property_panel_width: 270.0,
            score: Score {
                notes: Vec::new(),
                sequences: Vec::new(),
                last_token: TokenGen(0),
                delays: default_delays(),
                shared_notes: Arc::new(ArcSwap::from_pointee(Vec::new())),
            },
        };
        app
    }
    pub fn score(&self) -> &Score {
        &self.score
    }
    pub fn score_mut(&mut self) -> &mut Score {
        &mut self.score
    }
    pub fn load_logo(&mut self, ctx: &egui::Context) {
        if self.logo.is_none() {
            let image = if ctx.style().visuals.dark_mode {
                let bytes = include_bytes!("../../assets/QuantumHarmonicsTmpWhite.png");

                image::load_from_memory(bytes)
                    .expect("Failed to load logo")
                    .to_rgba8()
            } else {
                let bytes = include_bytes!("../../assets/QuantumHarmonicsTmp.png");
                image::load_from_memory(bytes)
                    .expect("Failed to load logo")
                    .to_rgba8()
            };
            let size = [image.width() as usize, image.height() as usize];

            let pixels = image.into_vec();
            let texture = ctx.load_texture(
                "logo",
                egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                Default::default(),
            );
            self.logo = Some(texture);
        }
    }

    fn doc_page(&mut self, ctx: &egui::Context) {
        use egui_commonmark::CommonMarkViewer;

        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.heading("README");
                ui.separator();

                CommonMarkViewer::new().show(ui, &mut self.markdown_cache, README_MD);

                ui.add_space(12.0);
                if ui.button("Exit README").clicked() {
                    self.show_doc = false;
                }
            });
        });
    }
    fn t_to_x(rect: egui::Rect, t: Time, loop_len: Time) -> f32 {
        rect.left() + t.as_secs() as f32 / loop_len.as_secs() as f32 * rect.width()
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
        let h = hash32(&txt) % 360;
        hsl_to_color32(h as _, 0.25, 0.5)
    }

    fn now(&self) -> Time {
        Time(self.clock.load(std::sync::atomic::Ordering::Relaxed) as f64 / self.sample_rate)
    }

    fn exit(&self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    fn edit_vec<T: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        mut vec: impl DerefMut<Target = Vec<T>>,
        // label: Option<impl Into<WidgetText>>,
        default_value: T,
        layout: Layout,
    ) {
        ui.vertical(|ui| {
            // if let Some(label) = label {
            //     ui.label(label);
            // }
            ui.horizontal(|ui| {
                ui.with_layout(layout, |ui| {
                    vec.retain_mut(|d| !ui.add(egui::DragValue::new(d)).secondary_clicked()); // TODO: return true
                    if ui
                        .button("+")
                        .on_hover_text("Right click an item to remove it.")
                        .clicked()
                    {
                        vec.push(default_value);
                    } else {
                    }
                });
            });
        });
    }

    fn retain_notes(&mut self, now: Time) {
        let _ = self
            .score
            .notes
            .iter_mut()
            .for_each(|NotesGroup { notes, .. }| {
                notes.retain(|n| n.time + NOTE_LINGER_TIME >= now)
            });
    }
}

fn default_tempo() -> f64 {
    60.0
}

impl App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.load_logo(ctx);

        let mut style: egui::Style = (*ctx.style()).clone();
        style.interaction.tooltip_delay = 0.01;
        ctx.set_style(style);
        let mut save = false;
        let mut load = false;
        let mut exit = false;
        #[cfg(target_arch = "wasm32")]
        self.poll_loaded_state();
        if !self.score.sequences.is_empty() {
            let len = self.score.sequences.len();

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, SELECT_UP)) {
                self.selected = Some(match self.selected {
                    Some(n) => (n + len - 1) % len,
                    None => len - 1,
                });
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, SELECT_DOWN)) {
                self.selected = Some(match self.selected {
                    Some(n) => (n + 1) % len,
                    None => 0,
                });
            }
        }
        if !self.show_start {
            self.top_panel(ctx, &mut save, &mut load, &mut exit);
        }
        if save {
            self.save_state();
        }
        if load {
            self.load_state(ctx);
        }
        if exit || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.exit(ctx);
        }
        if self.show_default_picker {
            self.default_picker_window(ctx);
        }
        if self.show_doc {
            self.doc_page(ctx);
            if self.show_doc {
                return;
            };
        }
        if self.show_start || self.score.sequences.is_empty() {
            self.start_page(ctx);
            if self.show_start {
                return;
            };
        }
        self.property_panel(ctx);
        self.timeline_panel(ctx);
        ctx.request_repaint_after(Duration::from_millis((1000.0 / self.min_fps) as _));
        self.generate_notes();
        let now = self.now();
        self.retain_notes(now);
        self.score
            .shared_notes
            .store(Arc::new(self.score.notes.clone()));
        self.shared_delays
            .store(Arc::new(self.score.delays.clone()));
    }
}

impl GuiApp {
    fn default_picker_window(&mut self, ctx: &egui::Context) {
        use egui::{Align, Layout, RichText};

        egui::Window::new("Choose a default groove")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_min_width(420.0);

                ui.label("Select a preset to load:");
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        for (i, (name, _json)) in GROOVE_DEFAULTS.iter().enumerate() {
                            let selected = self.default_pick_idx == i;
                            if ui.selectable_label(selected, *name).clicked() {
                                self.default_pick_idx = i;
                            }
                        }
                    });

                ui.add_space(8.0);
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_default_picker = false;
                    }

                    ui.add_space(8.0);

                    // Primary action
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Load preset").strong())
                                .min_size(egui::vec2(140.0, 28.0)),
                        )
                        .clicked()
                    {
                        let (_name, json) = GROOVE_DEFAULTS[self.default_pick_idx];
                        match serde_json::from_str::<GuiState>(json) {
                            Ok(state) => {
                                self.apply_loaded_state(state);
                                self.show_default_picker = false;
                                self.show_start = false;
                            }
                            Err(e) => {
                                eprintln!("[default_picker] Failed to parse preset: {e}");
                            }
                        }
                    }
                });
            });
    }
    fn generate_notes(&mut self) {
        let now = self.now();
        let ids: Vec<_> = self
            .score
            .sequences
            .iter()
            .enumerate()
            .filter(|(_, seq)| {
                seq.seq_unchecked()
                    .not_generate_until
                    .as_ref()
                    .map_or(true, |until| now >= *until)
            })
            .map(|(i, _)| i)
            .collect();

        for i in ids {
            self.draw_seq_at(i);
        }
    }
    fn new_score(&mut self) {
        self.score_mut().sequences.clear();
        self.score.notes.clear();
    }
    fn new_seq(&mut self, mut track_node: TrackNode) {
        self.draw_seq(track_node.seq_mut_unchecked());
        self.score.sequences.push(track_node);
    }
    fn edit_seq_at(&mut self, mut sequence: Sequence, i: usize) {
        self.regen_seq(&mut sequence);
        self.score.sequences[i] = TrackNode::Seq(sequence);
        let len = self.score.sequences.len();
        (i + 1..len).for_each(|k| self.regen_seq_at(k));
    }
    fn del_seq(&mut self, a: usize) {
        let tk = self.score.sequences[a].seq_unchecked().token;
        self.drain_notes_from_seq(tk);
        self.score.sequences.remove(a);
    }
    fn clone_seq(&mut self, i: usize) {
        let mut sequence = self.score.sequences[i].clone();
        sequence.seq_mut_unchecked().token = self.score.last_token.next();
        self.draw_seq(sequence.seq_mut_unchecked());
        self.score.sequences.push(sequence);
    }
    fn swap_seqs_at(&mut self, i: usize, j: usize) {
        self.score.sequences.swap(i, j);
        self.regen_seq_at(i);
        self.regen_seq_at(j);
        let len = self.score.sequences.len();
        (i.max(j) + 1..len).for_each(|k| self.regen_seq_at(k));
    }
    fn draw_seq(&mut self, seq: &mut Sequence) {
        let now = self.now();
        let seq_start = seq.loop_len * (now / seq.loop_len).floor();
        // seq.draw(&mut self.notes, &mut self.rng, seq_start, self.tempo);
        seq.draw(&mut self.score.notes, &mut self.rng, seq_start);
        seq.not_generate_until =
            Some(seq_start + seq.t_min + seq.loop_len * seq.repeat as f64 - GENERATE_EARLY);
    }

    // fn draw_seq_at(&mut self, a: usize, out: &mut Vec<(usize, Vec<Note>)>) {
    fn draw_seq_at(&mut self, a: usize) {
        let now = self.now();
        let seq = self.score.sequences[a].seq_mut_unchecked();
        let seq_start = seq.loop_len * ((now + GENERATE_EARLY) / seq.loop_len).floor();
        // seq.draw(&mut self.notes, &mut self.rng, seq_start, self.tempo);
        seq.draw(&mut self.score.notes, &mut self.rng, seq_start);
        seq.not_generate_until =
            Some(seq_start + seq.t_min + seq.loop_len * seq.repeat as f64 - GENERATE_EARLY);
    }
    fn drain_notes_from_seq(&mut self, tk: Token) {
        self.score
            .notes
            .retain(|NotesGroup { token, .. }| *token != tk);
    }
    fn regen_seq(&mut self, seq: &mut Sequence) {
        self.drain_notes_from_seq(seq.token);
        self.draw_seq(seq);
    }
    fn regen_seq_at(&mut self, i: usize) {
        self.drain_notes_from_seq(self.score.sequences[i].seq_unchecked().token);
        self.draw_seq_at(i);
    }
    #[cfg(target_arch = "wasm32")]
    fn poll_loaded_state(&mut self) {
        let state = {
            let mut slot = self.pending_loaded_bytes.borrow_mut();
            slot.take()
                .and_then(|bytes| serde_json::from_slice::<crate::app::GuiState>(&bytes).ok())
        };
        if let Some(state) = state {
            self.apply_loaded_state(state);
        }
    }
    fn try_load_default(&mut self, _ctx: &egui::Context) {
        self.show_default_picker = true;
    }
}

/// Convert HSL (0.0–360.0, 0.0–1.0, 0.0–1.0) to `Color32`.
pub fn hsl_to_color32(h: f32, s: f32, l: f32) -> Color32 {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());

    let (r1, g1, b1) = match h_prime as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 => (c, 0.0, x),
        _ => (0.0, 0.0, 0.0),
    };

    let m = l - c / 2.0;
    let (r, g, b) = (r1 + m, g1 + m, b1 + m);

    Color32::from_rgb(
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

/// simple deterministic hash for colour
fn hash32(s: &str) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish() as u32
}
