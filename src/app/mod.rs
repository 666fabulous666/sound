mod load;
mod property_panel;
mod save;
mod timeline_panel;
mod top_panel;

use crate::engine::{
    notes::{Note, Sequence},
    scheduler::{Message, Scheduler},
    waves::WaveType,
};
#[cfg(not(target_arch = "wasm32"))]
use cpal::Device;
use cpal::Stream;
#[cfg(not(target_arch = "wasm32"))]
use eframe::NativeOptions;
use eframe::{egui, App, CreationContext};
use instant::{Duration, Instant};
use rand::{rngs::ThreadRng, thread_rng};
use serde::{Deserialize, Serialize};
use std::sync::{atomic::AtomicUsize, mpsc::Sender, Arc, Mutex};

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
    delays: (Arc<Mutex<Vec<usize>>>, Arc<Mutex<Vec<usize>>>),
}

impl Default for ScoreParams {
    fn default() -> Self {
        Self {
            delays: (Arc::new(Mutex::new(vec![])), Arc::new(Mutex::new(vec![]))),
        }
    }
}

pub struct GuiApp {
    seqs: Arc<Mutex<Vec<Sequence>>>,
    notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>>,
    selected: Option<usize>,
    clock: Option<Arc<Mutex<f64>>>,
    fall_back_start: Instant,
    last_token: AtomicUsize,
    scheduler: Scheduler,
    messages: Sender<Message>,
    score_params: ScoreParams,
    rng: ThreadRng,
    stream: Option<Stream>,
    is_playing: bool,
    device: Device,
}

#[derive(Serialize, Deserialize, Clone)]
struct GuiState {
    seqs: Vec<Sequence>,
    selected: Option<usize>,
}

impl GuiApp {
    pub fn new(
        _cc: &CreationContext<'_>,
        device: Device,
        clock: Option<Arc<Mutex<f64>>>,
        seqs: Arc<Mutex<Vec<Sequence>>>,
        notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>>,
        scheduler: Scheduler,
        messages: Sender<Message>,
        delays: (Arc<Mutex<Vec<usize>>>, Arc<Mutex<Vec<usize>>>),
    ) -> Self {
        *seqs.lock().unwrap() = Vec::new();

        Self {
            seqs,
            notes,
            selected: None,
            clock,
            fall_back_start: Instant::now(),
            last_token: 0.into(),

            scheduler,
            messages,
            score_params: ScoreParams { delays },
            rng: thread_rng(),
            stream: None,
            is_playing: false,
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
        #[cfg(not(target_arch = "wasm32"))]
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);

        #[cfg(target_arch = "wasm32")]
        todo!()
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

        self.top_panel(ctx, len, &mut save, &mut load, &mut exit);

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

        self.property_panel(ctx, len, &seqs);

        self.timeline_panel(ctx, current_time, len, seqs);

        ctx.request_repaint_after(Duration::from_millis(16));
        self.scheduler.run_once(&mut self.rng)
    }
}

// ------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
pub fn run_gui(
    device: Device,
    clock: Option<Arc<Mutex<f64>>>,
    seqs: Arc<Mutex<Vec<Sequence>>>,
    notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>>,
    scheduler: Scheduler,
    messages: Sender<Message>,
    // delays: (Arc<Mutex<Vec<usize>>>, Arc<Mutex<Vec<usize>>>),
) {
    let delays = (
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(Vec::new())),
    );
    let native_options = NativeOptions::default();
    let _ = eframe::run_native(
        "Notes GUI",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(GuiApp::new(
                cc,
                device,
                clock.clone(),
                seqs.clone(),
                notes,
                scheduler,
                messages,
                delays,
            )))
        }),
    );
}

#[cfg(target_arch = "wasm32")]
pub fn make_app_for_web(
    cc: &CreationContext<'_>,
    clock: Option<Arc<Mutex<f64>>>,
    shared: Arc<Mutex<Vec<Sequence>>>,
    scheduler: Scheduler,
    messages: Sender<Message>,
    delays: (Arc<Mutex<Vec<usize>>>, Arc<Mutex<Vec<usize>>>),
) -> Box<dyn App> {
    Box::new(GuiApp::new(cc, clock, shared, scheduler, messages, delays))
}

// simple deterministic hash for colour
fn hash32(s: &str) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish() as u32
}
