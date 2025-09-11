use cpal::traits::HostTrait;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use synth::engine::scheduler::Scheduler;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let sample_clock = Arc::new(Mutex::new(0f64));
    let (scheduler, sender) = Scheduler::new(sample_clock.clone());
    // let recorded_samples = Arc::new(Mutex::new(Vec::<f64>::new()));
    let running = Arc::new(AtomicBool::new(true));

    let _ = {
        use eframe::NativeOptions;

        let clock = Some(Arc::clone(&sample_clock));
        let delays = (
            Arc::new(Mutex::new(Vec::new())),
            Arc::new(Mutex::new(Vec::new())),
        );
        let native_options = NativeOptions::default();
        let _ = eframe::run_native(
            "Notes GUI",
            native_options,
            Box::new(move |cc| {
                use synth::app::GuiApp;

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
        );
    };

    running.store(false, Ordering::Relaxed);
}

#[cfg(target_arch = "wasm32")]
fn main() {}
