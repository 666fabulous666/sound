pub mod app;
pub mod engine;
pub mod stream;

pub const DEFAULT_LOOP_LEN: f64 = 8.0;
pub const NOTE_LINGER_TIME: f64 = 12.0;
pub const F0: f64 = 440.0;
pub const REVERB_BUFFER_LEN: usize = 100_000;
pub const SCHEDULER_STEP: f64 = 1e-2;
pub const SCHEDULER_WAKE_EARLY: f64 = 1.0;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    // nicer panics in the browser console
    console_error_panic_hook::set_once();

    // (optional) basic logging in browser; remove if you don't use `log`
    let _ = eframe::WebLogger::init(log::LevelFilter::Info);

    // --- minimal state so the UI can run on web (audio/threads omitted) ---
    use std::sync::{Arc, Mutex};

    use crate::engine::scheduler::Scheduler;

    // shared sequences (empty to start)
    let shared = Arc::new(Mutex::new(Vec::<engine::notes::Sequence>::new()));

    // left/right delay buffers (empty to start)
    let left_delays: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let right_delays: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    // scheduler + message channel:
    let sample_clock = Arc::new(Mutex::new(0f64));
    let (scheduler, sender) = Scheduler::new(sample_clock.clone());

    // canvas in your index.html: <canvas id="the_canvas_id"></canvas>
    const CANVAS: &str = "the_canvas_id";

    let web_options = eframe::WebOptions::default();

    eframe::WebRunner::new()
        .start(
            CANVAS,
            web_options,
            Box::new(move |cc| {
                Ok(app::make_app_for_web(
                    cc,
                    None, // no audio clock on web (for now)
                    Arc::clone(&shared),
                    scheduler, // minimal scheduler (noop/default)
                    sender.clone(),
                    (Arc::clone(&left_delays), Arc::clone(&right_delays)),
                ))
            }),
        )
        .await
}
