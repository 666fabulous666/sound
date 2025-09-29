#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use cpal::traits::HostTrait;

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");

    let _ = {
        use eframe::NativeOptions;

        let native_options = NativeOptions {
            viewport: egui::ViewportBuilder::default()
                // .with_inner_size([1280.0, 720.0]) // optional "starting size"
                .with_maximized(true),
            ..Default::default()
        };
        let _ = eframe::run_native(
            "Notes GUI",
            native_options,
            Box::new(move |cc| {
                use synth::app::GuiApp;

                Ok(Box::new(GuiApp::new(cc, device)))
            }),
        );
    };
}

#[cfg(target_arch = "wasm32")]
fn main() {}
