pub mod app;
pub mod engine;
// pub mod range_slider;
pub mod stream;

pub const DEFAULT_LOOP_LEN: f64 = 8.0;
pub const NOTE_LINGER_TIME: f64 = 12.0;
pub const F0: f64 = 440.0;
pub const REVERB_BUFFER_LEN: usize = 65535;
pub const SCHEDULER_STEP: f64 = 1e-2;
pub const SCHEDULER_WAKE_EARLY: f64 = 0.1;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    fn make_canvas(id: &str) -> Result<HtmlCanvasElement, JsValue> {
        use eframe::web_sys::window;

        let document = window().unwrap().document().unwrap();
        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        canvas.set_id(id);
        canvas.style().set_property("width", "100vw")?;
        canvas.style().set_property("height", "100vh")?;
        canvas.style().set_property("display", "block")?;
        document.body().unwrap().append_child(&canvas)?;
        Ok(canvas)
    }

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();
    // log::log!(log::Level::Error, "test init");
    use cpal::traits::HostTrait;
    use eframe::web_sys::HtmlCanvasElement;

    use crate::engine::scheduler::Scheduler;
    use std::sync::{Arc, Mutex};
    const CANVAS: &str = "the_canvas_id";
    let canvas = make_canvas(CANVAS)?;

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let sample_clock = Arc::new(Mutex::new(0f64));
    let (scheduler, sender) = Scheduler::new(Arc::clone(&sample_clock));

    let web_options = eframe::WebOptions::default();

    let clock = Some(Arc::clone(&sample_clock));
    let delays = (
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(Vec::new())),
    );

    eframe::WebRunner::new()
        .start(
            canvas,
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
