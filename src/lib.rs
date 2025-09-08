#![cfg(target_arch = "wasm32")]

const DEFAULT_LOOP_LEN: f64 = 16.0; // seconds

mod app;
mod engine;

use crate::engine::notes::{ChorusParams, Note};
use crate::engine::scheduler::{self, Scheduler};
use crate::engine::waves::generate_wave;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui;
use std::f64::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

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
        gui: app::GuiApp,
        #[allow(dead_code)]
        stream: Option<cpal::Stream>,
        scheduler: scheduler::Scheduler, // single-threaded on the web
        note_queue: Arc<Mutex<Vec<(usize, Vec<Note>)>>>,
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
                    gui: app::GuiApp::construct(
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

use crate::engine::waves::basics::{hi_hat, kick, snare};

fn start_cpal_web(
    note_queue: Arc<Mutex<Vec<(usize, Vec<Note>)>>>,
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
    let sample_rate_hz = config.sample_rate.0 as f64;
    let sample_duration = 1.0f64 / sample_rate_hz;

    device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _info| {
                // === this matches your desktop loop (minus reverb & recording) ===
                let mut notes = note_queue.lock().unwrap();
                let mut clock = sample_clock.lock().unwrap();

                for frame in data.chunks_mut(channels) {
                    let elapsed = *clock;
                    let mut dry_left = 0.0f64;
                    let mut dry_right = 0.0f64;

                    for (_, notes_from_seq) in notes.iter_mut() {
                        notes_from_seq.retain(|note| {
                            if elapsed < note.time {
                                true
                            } else if elapsed <= note.time + note.duration {
                                let t = elapsed - note.time;

                                // identical to your desktop volume expression
                                let volume = note.volume
                                    / (1.5
                                        + (0.5 * note.time).fract()
                                        + (1.2 * note.time).fract()
                                        + (2.5 * note.time).fract()
                                        + (3.0 * note.time).fract());

                                let dry = volume
                                    * generate_wave(
                                        &note.wave_type,
                                        440.0f64 * note.interval.compute(), // freq0 * interval
                                        t,
                                        note.duration,
                                        note.attack_decay,
                                        note.attack_freq_modulation,
                                        note.vibrato,
                                        &note.chorus,
                                        note.pow_fact,
                                    );

                                dry_left += (1.0 - note.spacial) * dry;
                                dry_right += note.spacial * dry;
                                true
                            } else if elapsed > note.time + note.duration + 12.0 {
                                false
                            } else {
                                true
                            }
                        });
                    }

                    // no reverb (yet) – write dry to outputs
                    let left = dry_left;
                    let right = dry_right;

                    if channels >= 2 {
                        frame[0] = left as f32;
                        frame[1] = right as f32;
                    } else {
                        frame[0] = (left + right) as f32;
                    }

                    *clock += sample_duration;
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
