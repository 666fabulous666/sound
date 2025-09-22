mod load;
mod property_panel;
mod save;
mod timeline_panel;
mod top_panel;

use crate::engine::{
    notes::{Note, Sequence},
    scheduler::Scheduler,
    waves::WaveType,
};
use cpal::Device;
use cpal::Stream;
use eframe::{egui, App, CreationContext};
use egui::WidgetText;
use instant::Duration;
use rand::{rngs::ThreadRng, thread_rng};
use serde::{Deserialize, Serialize};
use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
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
    // seqs: Vec<Sequence>,
    notes: Vec<(usize, Vec<Note>)>,
    selected: Option<usize>,
    last_token: usize,
    scheduler: Scheduler,
    score_params: ScoreParams,
    rng: ThreadRng,
    stream: Option<Stream>,
    device: Device,
}

#[derive(Serialize, Deserialize, Clone)]
struct GuiState {
    seqs: Vec<Sequence>,
    selected: Option<usize>,
}

impl GuiApp {
    pub fn new(_cc: &CreationContext<'_>, device: Device) -> Self {
        Self {
            // seqs: Vec::new(),
            notes: Vec::new(),
            selected: None,
            last_token: 0,
            score_params: ScoreParams::default(),
            rng: thread_rng(),
            stream: None,
            device: device,
            scheduler: Scheduler::default(),
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

    fn now(&self) -> Arc<Mutex<f64>> {
        self.scheduler.now()
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
        // self.scheduler.run_once(&mut self.rng)
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
