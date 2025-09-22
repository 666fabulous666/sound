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
    GENERATE_EARLY, NOTE_LINGER_TIME, SCHEDULER_STEP,
};
use arc_swap::ArcSwap;
use cpal::Stream;
use cpal::{traits::DeviceTrait, Device};
use eframe::{egui, App, CreationContext};
use egui::WidgetText;
use instant::Duration;
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

// ------------------------------------------------------------

pub struct ScoreParams {
    delays: (Vec<f64>, Vec<f64>),
}

impl Default for ScoreParams {
    fn default() -> Self {
        Self {
            delays: (Vec::new(), Vec::new()),
        }
    }
}

pub struct GuiApp {
    notes: Vec<(usize, Vec<Note>)>,
    shared_notes: Arc<ArcSwap<Vec<(usize, Vec<Note>)>>>,
    sequences: Vec<Sequence>,
    clock: Arc<AtomicU64>,
    sched_start: f64,
    rng: ThreadRng,
    selected: Option<usize>,
    last_token: usize,
    score_params: ScoreParams,
    stream: Option<Stream>,
    device: Device,
    sample_rate: f64,
}

#[derive(Serialize, Deserialize, Clone)]
struct GuiState {
    seqs: Vec<Sequence>,
    selected: Option<usize>,
}

impl GuiApp {
    pub fn new(_cc: &CreationContext<'_>, device: Device) -> Self {
        Self {
            selected: None,
            last_token: 0,
            score_params: ScoreParams::default(),
            stream: None,
            notes: Vec::new(),
            shared_notes: Arc::new(ArcSwap::from_pointee(Vec::new())),
            sequences: Vec::new(),
            clock: Arc::new(AtomicU64::new(0)),
            sched_start: 0.0,
            rng: thread_rng(),
            sample_rate: device.default_output_config().unwrap().sample_rate().0 as f64,
            device: device,
        }
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

        #[cfg(target_arch = "wasm32")]
        todo!()
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
        self.top_panel(ctx, &mut save, &mut load, &mut exit);
        if save {
            self.save_state();
        }
        if load {
            self.load_state();
        }
        if exit {
            self.exit(ctx);
        }
        self.property_panel(ctx);
        self.timeline_panel(ctx);
        ctx.request_repaint_after(Duration::from_millis(16));
        self.generate_notes();
        let now = self.now();
        self.retain_notes(now);
        self.shared_notes.store(self.notes.clone().into());
        // println!("{now}");
    }
}

// ------------------------------------------------------------

// #[cfg(not(target_arch = "wasm32"))]
// pub fn run_gui_native(
//     device: Device,
//     clock: Option<Arc<Mutex<f64>>>,
//     scheduler: Scheduler,
//     sender: Sender<Message>,
// ) {
//     let delays = (
//         Arc::new(Mutex::new(Vec::new())),
//         Arc::new(Mutex::new(Vec::new())),
//     );
//     let native_options = NativeOptions::default();
//     let _ = eframe::run_native(
//         "Notes GUI",
//         native_options,
//         Box::new(move |cc| {
//             Ok(Box::new(GuiApp::new(
//                 cc,
//                 device,
//                 clock.clone(),
//                 scheduler.sequences(),
//                 scheduler.notes(),
//                 scheduler,
//                 sender,
//                 delays,
//             )))
//         }),
//     );
// }

// #[cfg(target_arch = "wasm32")]
// pub fn make_app_for_web(
//     cc: &CreationContext<'_>,
//     clock: Option<Arc<Mutex<f64>>>,
//     shared: Arc<Mutex<Vec<Sequence>>>,
//     scheduler: Scheduler,
//     messages: Sender<Message>,
//     delays: (Arc<Mutex<Vec<usize>>>, Arc<Mutex<Vec<usize>>>),
// ) -> Box<dyn App> {
//     Box::new(GuiApp::new(cc, clock, shared, scheduler, messages, delays))
// }

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
        let tokens: Vec<_> = self
            .sequences
            .iter()
            .filter(|seq| {
                seq.not_generate_until
                    .as_ref()
                    .map_or(true, |until| now >= *until)
            })
            .map(|seq| seq.token)
            .collect();

        for token in tokens {
            self.draw_seq_at(token);
        }

        self.sched_start += SCHEDULER_STEP;
    }
    fn new_score(&mut self) {
        self.sequences.clear();
        self.notes.clear();
    }
    fn new_seq(&mut self, mut sequence: Sequence) {
        self.draw_seq(&mut sequence);
        self.sequences.push(sequence);
    }
    fn edit_seq(&mut self, mut sequence: Sequence, a: usize) {
        self.regen_seq(&mut sequence);
        self.sequences[a] = sequence;
        let len = self.sequences.len();
        (a + 1..len).for_each(|k| self.regen_seq_at(k));
    }
    fn del_seq(&mut self, a: usize) {
        let tk = self.sequences[a].token;
        self.remove_seq(tk);
        self.sequences.remove(a);
    }
    fn clone_seq(&mut self, a: usize, new_token: usize) {
        let mut sequence = self.sequences[a].clone();
        sequence.token = new_token;
        self.draw_seq(&mut sequence);
        self.sequences.push(sequence);
    }
    fn swap_seqs(&mut self, a: usize, b: usize) {
        self.sequences.swap(a, b);
        self.regen_seq_at(a);
        self.regen_seq_at(b);
        let len = self.sequences.len();
        (a.max(b) + 1..len).for_each(|k| self.regen_seq_at(k));
    }
    fn draw_seq(&mut self, seq: &mut Sequence) {
        let seq_start = (self.sched_start / seq.loop_len).floor() * seq.loop_len;
        seq.draw(&mut self.notes, &mut self.rng, seq_start);
        seq.not_generate_until =
            Some(seq_start + seq.t_min + seq.repeat as f64 * seq.loop_len - GENERATE_EARLY);
    }

    // fn draw_seq_at(&mut self, a: usize, out: &mut Vec<(usize, Vec<Note>)>) {
    fn draw_seq_at(&mut self, a: usize) {
        let seq = &mut self.sequences[a];
        let seq_start = (self.sched_start / seq.loop_len).floor() * seq.loop_len;
        seq.draw(&mut self.notes, &mut self.rng, seq_start);
        seq.not_generate_until =
            Some(seq_start + seq.t_min + seq.repeat as f64 * seq.loop_len - GENERATE_EARLY);
    }
    fn remove_seq(&mut self, tk: usize) {
        self.notes.retain(|(token, _)| *token != tk);
    }
    fn regen_seq(&mut self, seq: &mut Sequence) {
        self.remove_seq(seq.token);
        self.draw_seq(seq);
    }
    fn regen_seq_at(&mut self, a: usize) {
        self.remove_seq(a);
        self.draw_seq_at(a);
    }
}
