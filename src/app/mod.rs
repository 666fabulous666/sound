mod load;
mod property_panel;
mod save;
mod timeline_panel;
mod top_panel;

use crate::{
    engine::{
        notes::{Note, Sequence},
        waves::WaveType,
    },
    Token, TokenGen, GENERATE_EARLY, GROOVE_JSON, NOTE_LINGER_TIME,
};
use arc_swap::ArcSwap;
use cpal::Stream;
use cpal::{traits::DeviceTrait, Device};
use eframe::{egui, App, CreationContext};
use egui::WidgetText;
use instant::{Duration, Instant};
use rand::{rngs::ThreadRng, thread_rng};
use serde::{Deserialize, Serialize};
use std::{
    ops::DerefMut,
    sync::{atomic::AtomicU64, Arc},
};

const ALL_WAVES: [WaveType; 8] = [
    WaveType::Mute,
    WaveType::Sine,
    WaveType::Square,
    WaveType::Triangle,
    WaveType::Sawtooth,
    WaveType::HiHat,
    WaveType::Kick,
    WaveType::Snare,
];

const DRUM_WAVES: [WaveType; 3] = [WaveType::HiHat, WaveType::Kick, WaveType::Snare];

// ------------------------------------------------------------

pub struct GuiApp {
    notes: Vec<(Token, Vec<Note>)>,
    shared_notes: Arc<ArcSwap<Vec<(Token, Vec<Note>)>>>,
    sequences: Vec<Sequence>,
    clock: Arc<AtomicU64>,
    rng: ThreadRng,
    selected: Option<usize>,
    last_token: TokenGen,
    stream: Option<Stream>,
    device: Device,
    sample_rate: f64,
    delays: (Vec<f64>, Vec<f64>),
    shared_delays: Arc<ArcSwap<(Vec<f64>, Vec<f64>)>>,
    #[cfg(target_arch = "wasm32")]
    pub(crate) pending_loaded_bytes: std::rc::Rc<std::cell::RefCell<Option<Vec<u8>>>>,
    instant: Instant,
    fps: f64,
    min_fps: f64,
}

fn default_delays() -> (Vec<f64>, Vec<f64>) {
    (vec![31.0, 63.0, 128.0], vec![33.0, 61.0, 124.0])
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GuiState {
    seqs: Vec<Sequence>,
    selected: Option<usize>,
    #[serde(default = "default_delays")]
    delays: (Vec<f64>, Vec<f64>),
}

impl GuiApp {
    pub fn new(cc: &CreationContext<'_>, device: Device) -> Self {
        let mut app = Self {
            selected: None,
            last_token: TokenGen(0),
            stream: None,
            notes: Vec::new(),
            shared_notes: Arc::new(ArcSwap::from_pointee(Vec::new())),
            delays: default_delays(),
            shared_delays: Arc::new(ArcSwap::from_pointee((Vec::new(), Vec::new()))),
            sequences: Vec::new(),
            clock: Arc::new(AtomicU64::new(0)),
            rng: thread_rng(),
            sample_rate: device.default_output_config().unwrap().sample_rate().0 as f64,
            device: device,
            #[cfg(target_arch = "wasm32")]
            pending_loaded_bytes: std::rc::Rc::new(std::cell::RefCell::new(None)),
            instant: Instant::now(),
            fps: 60.0,
            min_fps: 60.0,
        };
        app.try_load_default(&cc.egui_ctx);
        app
    }

    fn t_to_x(rect: egui::Rect, t: f64, loop_len: f64) -> f32 {
        rect.left() + t as f32 / loop_len as f32 * rect.width()
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
        let a = (hash & 0xFF) as u8;
        let b = ((hash >> 8) & 0xFF) as u8;
        let c = ((hash >> 16) & 0xFF) as u8;
        egui::Color32::from_rgb(a / 4 * 3, b / 7 * 3, c / 5 * 3)
    }

    fn now(&self) -> f64 {
        self.clock.load(std::sync::atomic::Ordering::Relaxed) as f64 / self.sample_rate
    }

    fn exit(&self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    fn edit_vec<T: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        mut vec: impl DerefMut<Target = Vec<T>>,
        label: Option<impl Into<WidgetText>>,
        default_value: T,
    ) {
        ui.vertical(|ui| {
            if let Some(label) = label {
                ui.label(label);
            }
            ui.horizontal(|ui| {
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
    }

    fn retain_notes(&mut self, now: f64) {
        let _ = self
            .notes
            .iter_mut()
            .for_each(|(_, ns)| ns.retain(|n| n.time - NOTE_LINGER_TIME <= now));
    }
}

impl App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style: egui::Style = (*ctx.style()).clone();
        style.interaction.tooltip_delay = 0.01;
        ctx.set_style(style);
        let mut save = false;
        let mut load = false;
        let mut exit = false;
        #[cfg(target_arch = "wasm32")]
        self.poll_loaded_state();
        if !self.sequences.is_empty() {
            let len = self.sequences.len();

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                self.selected = Some(match self.selected {
                    Some(n) => (n + len - 1) % len,
                    None => len - 1,
                });
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
                self.selected = Some(match self.selected {
                    Some(n) => (n + 1) % len,
                    None => 0,
                });
            }
        }
        self.top_panel(ctx, &mut save, &mut load, &mut exit);
        if save {
            self.save_state();
        }
        if load {
            self.load_state(ctx);
        }
        if exit || ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.exit(ctx);
        }
        self.property_panel(ctx);
        self.timeline_panel(ctx);
        ctx.request_repaint_after(Duration::from_millis((1000.0 / self.min_fps) as _));
        self.generate_notes();
        let now = self.now();
        self.retain_notes(now);
        self.shared_notes.store(Arc::new(self.notes.clone()));
        self.shared_delays.store(Arc::new(self.delays.clone()));
    }
}

// simple deterministic hash for colour
fn hash32(s: &str) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish() as u32
}

impl GuiApp {
    fn generate_notes(&mut self) {
        let now = self.now();
        let ids: Vec<_> = self
            .sequences
            .iter()
            .enumerate()
            .filter(|(_, seq)| {
                seq.not_generate_until
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
        self.sequences.clear();
        self.notes.clear();
    }
    fn new_seq(&mut self, mut sequence: Sequence) {
        self.draw_seq(&mut sequence);
        self.sequences.push(sequence);
    }
    fn edit_seq_at(&mut self, mut sequence: Sequence, i: usize) {
        self.regen_seq(&mut sequence);
        self.sequences[i] = sequence;
        let len = self.sequences.len();
        (i + 1..len).for_each(|k| self.regen_seq_at(k));
    }
    fn del_seq(&mut self, a: usize) {
        let tk = self.sequences[a].token;
        self.drain_notes_from_seq(tk);
        self.sequences.remove(a);
    }
    fn clone_seq(&mut self, i: usize) {
        let mut sequence = self.sequences[i].clone();
        sequence.token = self.last_token.next();
        self.draw_seq(&mut sequence);
        self.sequences.push(sequence);
    }
    fn swap_seqs_at(&mut self, i: usize, j: usize) {
        self.sequences.swap(i, j);
        self.regen_seq_at(i);
        self.regen_seq_at(j);
        let len = self.sequences.len();
        (i.max(j) + 1..len).for_each(|k| self.regen_seq_at(k));
    }
    fn draw_seq(&mut self, seq: &mut Sequence) {
        let now = self.now();
        let seq_start = (now / seq.loop_len).floor() * seq.loop_len;
        seq.draw(&mut self.notes, &mut self.rng, seq_start);
        seq.not_generate_until =
            Some(seq_start + seq.t_min + seq.repeat as f64 * seq.loop_len - GENERATE_EARLY);
    }

    // fn draw_seq_at(&mut self, a: usize, out: &mut Vec<(usize, Vec<Note>)>) {
    fn draw_seq_at(&mut self, a: usize) {
        let now = self.now();
        let seq = &mut self.sequences[a];
        let seq_start = ((now + GENERATE_EARLY) / seq.loop_len).floor() * seq.loop_len;
        seq.draw(&mut self.notes, &mut self.rng, seq_start);
        seq.not_generate_until =
            Some(seq_start + seq.t_min + seq.repeat as f64 * seq.loop_len - GENERATE_EARLY);
    }
    fn drain_notes_from_seq(&mut self, tk: Token) {
        self.notes.retain(|(token, _)| *token != tk);
    }
    fn regen_seq(&mut self, seq: &mut Sequence) {
        self.drain_notes_from_seq(seq.token);
        self.draw_seq(seq);
    }
    fn regen_seq_at(&mut self, i: usize) {
        self.drain_notes_from_seq(self.sequences[i].token);
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
        if let Ok(state) = serde_json::from_str::<GuiState>(GROOVE_JSON) {
            self.apply_loaded_state(state);
        } else {
            eprintln!("Failed to parse embedded groove.json");
        }
    }
}
