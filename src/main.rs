use cpal::traits::HostTrait;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use synth::{app, engine::scheduler::Scheduler};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let sample_clock = Arc::new(Mutex::new(0f64));
    let (scheduler, sender) = Scheduler::new(sample_clock.clone());
    let seqs = scheduler.sequences();
    let notes = scheduler.notes();
    // let recorded_samples = Arc::new(Mutex::new(Vec::<f64>::new()));
    let running = Arc::new(AtomicBool::new(true));

    let _ = app::run_gui(
        device,
        Some(Arc::clone(&sample_clock)),
        Arc::clone(&seqs),
        notes.clone(),
        scheduler,
        sender,
    );

    running.store(false, Ordering::Relaxed);
}

#[cfg(target_arch = "wasm32")]
fn main() {}
