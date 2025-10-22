pub mod app;
pub mod engine;
// pub mod range_slider;
pub mod shortcuts;
pub mod stream;
pub mod texts;
pub mod time_freq;
use std::ops::Deref;

pub const DEFAULT_LOOP_LEN: Time = Time(4.0);
pub const NOTE_LINGER_TIME: Time = Time(12.0);
pub const F0: Freq = Freq(440.0);
pub const REVERB_BUFFER_LEN: usize = 65535;
pub const SCHEDULER_WAKE_EARLY: f64 = 0.1;
pub const GENERATE_EARLY: Time = Time(1e-1);
pub const GLOBAL_VOLUME: f64 = 0.1;
#[cfg(target_arch = "wasm32")]
pub const MAX_FPS: f64 = 60.0;
#[cfg(not(target_arch = "wasm32"))]
pub const MAX_FPS: f64 = 120.0;
const TREE_DEPTH_WIDTH: f32 = 10.0;
pub const GROOVE_DEFAULTS: &[(&str, &str)] = &[
    (
        "Happy Melancholia",
        include_str!("../assets/Happy Melancholia.json"),
    ),
    ("Chart Of Doom", include_str!("../assets/ChartOfDoom.json")),
    ("Unicorns", include_str!("../assets/Unicorns.json")),
    ("Afro Q-Bit", include_str!("../assets/AfroQBit.json")),
    ("Blue Jinn", include_str!("../assets/Blue Jinn.json")),
    (
        "Videogame Groove",
        include_str!("../assets/GameGroove.json"),
    ),
    (
        "Spins of Birds",
        include_str!("../assets/SpinsOfBirds.json"),
    ),
];
pub fn layout_left() -> Layout {
    Layout::left_to_right(egui::Align::Min)
}

pub fn sign_f<T: num_traits::Signed>(arg: T, f: impl Fn(T) -> T) -> T {
    arg.signum() * f(arg.abs())
}

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

pub fn rescale_factor(a: f64, b: f64) -> f64 {
    let denom = a + b;
    (a.powf(a) * b.powf(b)) / denom.powf(denom)
}

use egui::Layout;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::time_freq::{Freq, Time};

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
