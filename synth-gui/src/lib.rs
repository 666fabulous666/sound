//! Synth-gui: GUI editor for synth-core
//!
//! This library provides a graphical editor for creating and manipulating audio synthesis scores.

pub mod app;
pub mod shortcuts;
pub mod texts;

// Re-export core functionality
pub use synth_core::*;

// GUI-specific constants
#[cfg(target_arch = "wasm32")]
pub const MAX_FPS: f64 = 60.0;
#[cfg(not(target_arch = "wasm32"))]
pub const MAX_FPS: f64 = 120.0;

pub const TREE_DEPTH_WIDTH: f32 = 10.0;
pub const TOOLBAR_ICON_SIZE: f32 = 64.0;

/// Default preset grooves included with the application
pub const GROOVE_DEFAULTS: &[(&str, &str)] = &[
    (
        "Happy Melancholia",
        include_str!("../../assets/Happy Melancholia.json"),
    ),
    (
        "Chart Of Doom",
        include_str!("../../assets/ChartOfDoom.json"),
    ),
    ("Unicorns", include_str!("../../assets/Unicorns.json")),
    ("Afro Q-Bit", include_str!("../../assets/AfroQBit.json")),
    ("Blue Jinn", include_str!("../../assets/Blue Jinn.json")),
    (
        "Videogame Groove",
        include_str!("../../assets/GameGroove.json"),
    ),
    (
        "Spins of Birds",
        include_str!("../../assets/SpinsOfBirds.json"),
    ),
];

// Built-in instrument presets (auto-generated from assets/presets/*.json)
include!(concat!(env!("OUT_DIR"), "/preset_defaults.rs"));

/// Helper for left-aligned egui layout
pub fn layout_left() -> egui::Layout {
    egui::Layout::left_to_right(egui::Align::Min)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    use crate::app::GuiApp;
    use cpal::traits::HostTrait;
    use eframe::web_sys::HtmlCanvasElement;
    use wasm_bindgen::JsCast;

    fn make_canvas(id: &str) -> Result<HtmlCanvasElement, wasm_bindgen::JsValue> {
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
