pub mod app;
pub mod engine;
// pub mod range_slider;
pub mod stream;

pub const DEFAULT_LOOP_LEN: f64 = 4.0;
pub const NOTE_LINGER_TIME: f64 = 12.0;
pub const F0: f64 = 440.0;
pub const REVERB_BUFFER_LEN: usize = 65535;
pub const SCHEDULER_WAKE_EARLY: f64 = 0.1;
pub const GENERATE_EARLY: f64 = 1e-1;
pub const GLOBAL_VOLUME: f64 = 0.01;
pub const TARGET_FPS: u64 = 30;
const GROOVE_JSON: &str = include_str!("../assets/groove.json");

#[derive(Serialize, Deserialize, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct Token(usize);

impl Deref for Token {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct TokenGen(usize);
impl TokenGen {
    pub fn new() -> Self {
        Self(0)
    }
    pub fn next(&mut self) -> Token {
        self.0 += 1;
        Token(self.0)
    }
}

use std::ops::Deref;

use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    use crate::app::GuiApp;
    use cpal::traits::HostTrait;
    use eframe::web_sys::HtmlCanvasElement;

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

    const CANVAS: &str = "the_canvas_id";

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();
    // log::log!(log::Level::Error, "test init");

    let canvas = make_canvas(CANVAS)?;
    let web_options = eframe::WebOptions::default();

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");

    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(move |cc| Ok(Box::new(GuiApp::new(cc, device)))),
        )
        .await
}
