#![cfg(target_arch = "wasm32")]

const DEFAULT_LOOP_LEN: f64 = 16.0; // seconds

mod gui;
mod notes;
mod scheduler;
mod time;
mod waves;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

use scheduler::Scheduler;

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    let _ = eframe::WebLogger::init(log::LevelFilter::Info);

    // ==== shared state (same as main.rs) ====
    let sample_clock = Arc::new(Mutex::new(0f64));
    let (mut scheduler, sender) = Scheduler::new(sample_clock.clone());
    let shared_seqs = scheduler.sequences();
    let note_queue = scheduler.notes();
    let running = Arc::new(AtomicBool::new(true));
    let left_delays = Arc::new(Mutex::new(vec![1]));
    let right_delays = Arc::new(Mutex::new(vec![1]));

    // ==== Web wrapper app: ticks scheduler + holds stream ====
    struct WebWrapper {
        gui: gui::GuiApp,
        #[allow(dead_code)]
        stream: Option<cpal::Stream>,
        scheduler: scheduler::Scheduler, // single-threaded on the web
        note_queue: Arc<Mutex<Vec<(usize, Vec<notes::Note>)>>>,
        sample_clock: Arc<Mutex<f64>>,
        audio_started: bool,
    }

    impl eframe::App for WebWrapper {
        fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
            // 1) tick scheduler (web)
            self.scheduler.tick_once();

            // 2) lazy-start audio on first user gesture (click/touch/keypress)
            if !self.audio_started {
                let started = ctx.input(|i| {
                    i.pointer.any_pressed()
                        || i.key_pressed(egui::Key::Enter)
                        || i.key_pressed(egui::Key::Space)
                });
                if started {
                    self.stream = Some(start_cpal_web(
                        self.note_queue.clone(),
                        self.sample_clock.clone(),
                    ));
                    self.audio_started = true;
                }
            }

            // 3) draw your real GUI
            self.gui.update(ctx, frame);

            // keep repainting regularly to drive tick/audio UI
            ctx.request_repaint();
        }
    }

    // ==== boot eframe with our wrapper ====
    let web_options = eframe::WebOptions::default();
    eframe::WebRunner::new()
        .start(
            "the_canvas_id",
            web_options,
            Box::new(move |cc| {
                Ok(Box::new(WebWrapper {
                    gui: gui::GuiApp::construct(
                        cc,
                        Some(sample_clock.clone()),
                        shared_seqs.clone(),
                        sender.clone(),
                        (left_delays.clone(), right_delays.clone()),
                    ),
                    stream: None,
                    scheduler, // moved in
                    note_queue: note_queue.clone(),
                    sample_clock: sample_clock.clone(),
                    audio_started: false,
                }))
            }),
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    // (We never set running=false on web; the page unload ends everything.)
    Ok(())
}

// === CPAL stream for the web (WebAudio backend) ===
// This is adapted from your main.rs callback; adjust if you add reverb here.
fn start_cpal_web(
    note_queue: Arc<Mutex<Vec<(usize, Vec<notes::Note>)>>>,
    sample_clock: Arc<Mutex<f64>>,
) -> cpal::Stream {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device");
    let config = device
        .default_output_config()
        .expect("no default output config")
        .config();

    let channels = config.channels as usize;
    let sample_rate = config.sample_rate.0 as f32;

    device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _info| {
                // pull notes and render like your desktop audio callback does
                let mut notes = note_queue.lock().unwrap();
                let mut clock = sample_clock.lock().unwrap();

                for frame in data.chunks_mut(channels) {
                    let elapsed = *clock;

                    // ---- mix your voices here ----
                    // For a minimal "it works" path, output silence until your scheduler feeds notes.
                    let mut left = 0.0f32;
                    let mut right = 0.0f32;

                    // TODO: port the exact mixing from your main.rs here (kick/snare/hihat, voices, reverb, etc.)
                    // Right now, this will be silent until you wire the mixing logic.
                    // If you already did in main.rs, copy that loop body here 1:1.

                    for (i, out) in frame.iter_mut().enumerate() {
                        *out = if i % 2 == 0 { left } else { right };
                    }

                    // advance sample-clock in seconds
                    *clock += 1.0 / sample_rate as f64;
                }
            },
            move |e| web_sys::console::error_1(&format!("stream error: {e}").into()),
            None,
        )
        .expect("failed to build stream")
        .tap(|s| s.play().expect("failed to start stream"))
}

// tiny helper to call play() inline
trait Tap: Sized {
    fn tap(self, f: impl FnOnce(&Self)) -> Self {
        f(&self);
        self
    }
}
impl<T> Tap for T {}
