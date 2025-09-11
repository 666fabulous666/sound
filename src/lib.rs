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
    use cpal::traits::HostTrait;

    use crate::engine::scheduler::Scheduler;
    use std::sync::{Arc, Mutex};
    const CANVAS: &str = "the_canvas_id";

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let sample_clock = Arc::new(Mutex::new(0f64));
    let (scheduler, sender) = Scheduler::new(sample_clock.clone());

    let web_options = eframe::WebOptions::default();

    let clock = Some(Arc::clone(&sample_clock));
    let delays = (
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(Vec::new())),
    );
    eframe::WebRunner::new()
        .start(
            CANVAS,
            web_options,
            Box::new(move |cc| {
                use crate::app::GuiApp;

                Ok(Box::new(GuiApp::new(
                    cc,
                    device,
                    clock.clone(),
                    scheduler.sequences(),
                    scheduler.notes(),
                    scheduler,
                    sender,
                    delays,
                )))
            }),
        )
        .await
}
