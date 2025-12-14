pub mod app;
pub mod engine;
pub mod error;
// pub mod range_slider;
pub mod shortcuts;
pub mod stream;
pub mod texts;
pub mod time_freq;
use std::ops::Deref;

pub const DEFAULT_LOOP_LEN: Time = Time(4.0);
pub const NOTE_LINGER_TIME: Time = Time(32.0);
pub const F0: Freq = Freq(440.0);
// pub const REVERB_BUFFER_LEN: usize = 65535;
pub const REVERB_BUFFER_LEN: usize = 44100;
pub const SCHEDULER_WAKE_EARLY: f64 = 0.1;
pub const GENERATE_EARLY: Time = Time(1e0);
// pub const GLOBAL_VOLUME: f64 = 0.1;
#[cfg(target_arch = "wasm32")]
pub const MAX_FPS: f64 = 60.0;
#[cfg(not(target_arch = "wasm32"))]
pub const MAX_FPS: f64 = 120.0;
const TREE_DEPTH_WIDTH: f32 = 10.0;
pub const GROOVE_DEFAULTS: &[(&str, &str)] = &[
    (
        "The Piper",
        include_str!("../assets/pipe, banjo, tabla and bass2.json"),
    ),
    (
        "Aquareggae",
        include_str!("../assets/reggae.json"),
    ),
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
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

/// Compute a rescaling factor for envelope normalization.
///
/// Given attack time `a` and decay time `b`, computes a normalization factor
/// to ensure consistent perceived loudness regardless of envelope shape.
///
/// # Arguments
/// * `a` - Attack time (must be positive and finite)
/// * `b` - Decay time (must be positive and finite)
///
/// # Returns
/// A finite rescaling factor, or `1.0` if the computation would produce NaN/infinity.
///
/// # Examples
/// ```
/// use synth::rescale_factor;
/// let factor = rescale_factor(1.0, 1.0);
/// assert!(factor.is_finite());
/// assert!(factor > 0.0);
/// ```
pub fn rescale_factor(a: f64, b: f64) -> f64 {
    // Validate inputs
    if !a.is_finite() || !b.is_finite() || a <= 0.0 || b <= 0.0 {
        return 1.0; // Safe fallback
    }

    let denom = a + b;
    let result = (a.powf(a) * b.powf(b)) / denom.powf(denom);

    // Ensure result is finite
    if result.is_finite() && result > 0.0 {
        result
    } else {
        1.0 // Safe fallback
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rescale_factor_normal_values() {
        let factor = rescale_factor(1.0, 1.0);
        assert!(factor.is_finite(), "rescale_factor(1.0, 1.0) should be finite");
        assert!(factor > 0.0, "rescale_factor should be positive");

        let factor2 = rescale_factor(0.5, 2.0);
        assert!(factor2.is_finite());
        assert!(factor2 > 0.0);
    }

    #[test]
    fn test_rescale_factor_edge_cases() {
        // Very small values
        let factor = rescale_factor(0.01, 0.01);
        assert!(factor.is_finite());
        assert!(factor > 0.0);

        // Very large values
        let factor = rescale_factor(100.0, 100.0);
        assert!(factor.is_finite());
        assert!(factor > 0.0);

        // Asymmetric values
        let factor = rescale_factor(0.01, 100.0);
        assert!(factor.is_finite());
        assert!(factor > 0.0);
    }

    #[test]
    fn test_rescale_factor_invalid_inputs() {
        // NaN inputs should return 1.0
        assert_eq!(rescale_factor(f64::NAN, 1.0), 1.0);
        assert_eq!(rescale_factor(1.0, f64::NAN), 1.0);

        // Infinity inputs should return 1.0
        assert_eq!(rescale_factor(f64::INFINITY, 1.0), 1.0);
        assert_eq!(rescale_factor(1.0, f64::INFINITY), 1.0);

        // Zero or negative inputs should return 1.0
        assert_eq!(rescale_factor(0.0, 1.0), 1.0);
        assert_eq!(rescale_factor(-1.0, 1.0), 1.0);
        assert_eq!(rescale_factor(1.0, 0.0), 1.0);
        assert_eq!(rescale_factor(1.0, -1.0), 1.0);
    }

    #[test]
    fn test_token_gen() {
        let mut gen = TokenGen::new();
        let t1 = gen.next();
        let t2 = gen.next();
        let t3 = gen.next();

        assert_ne!(t1, t2);
        assert_ne!(t2, t3);
        assert_eq!(*t1, 1);
        assert_eq!(*t2, 2);
        assert_eq!(*t3, 3);
    }

    #[test]
    fn test_sign_f() {
        assert_eq!(sign_f(5.0, |x| x * x), 25.0);
        assert_eq!(sign_f(-5.0, |x| x * x), -25.0);
        assert_eq!(sign_f(0.0, |x| x * x), 0.0);
    }
}

