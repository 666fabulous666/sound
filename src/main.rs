use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use synth::{
    app,
    engine::{reverb, scheduler::Scheduler},
    stream::stream,
    F0,
};

use reverb::Reverb;

fn main() {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let config = device.default_output_config().unwrap().config();
    let sample_rate = config.sample_rate.0 as f64;
    let sample_duration = 1.0 / sample_rate;
    let channels = config.channels;

    let sample_clock = Arc::new(Mutex::new(0f64));
    let (scheduler, sender) = Scheduler::new(sample_clock.clone());
    let shared_seqs = scheduler.sequences();
    let note_queue = scheduler.notes();
    let recorded_samples = Arc::new(Mutex::new(Vec::new()));
    let running = Arc::new(AtomicBool::new(true));
    let (left_delays, right_delays) =
        (Arc::new(Mutex::new(vec![1])), Arc::new(Mutex::new(vec![1])));

    let stream = stream(
        F0,
        device,
        config,
        sample_duration,
        channels,
        &sample_clock,
        note_queue,
        recorded_samples,
        (
            Reverb::new(0.5, 0.5, left_delays.clone()),
            Reverb::new(0.5, 0.5, right_delays.clone()),
        ),
    );

    stream.play().unwrap();

    let _ = app::run_gui(
        Some(Arc::clone(&sample_clock)),
        Arc::clone(&shared_seqs),
        scheduler,
        sender,
        (left_delays.clone(), right_delays.clone()),
    );

    running.store(false, Ordering::Relaxed);
}
