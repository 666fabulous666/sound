mod stream;
use crate::stream::stream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use synth::{
    engine::{reverb, scheduler::Scheduler},
    gui,
};

use reverb::Reverb;

fn main() {
    let freq0 = 440.0f64;

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
    let reverb_left: Reverb<44100> = Reverb::new(0.5, 0.5, left_delays.clone());
    let reverb_right: Reverb<44100> = Reverb::new(0.5, 0.5, right_delays.clone());

    // Start persistent audio stream
    let stream = stream(
        freq0,
        device,
        config,
        sample_duration,
        channels,
        &sample_clock,
        note_queue,
        recorded_samples,
        reverb_left,
        reverb_right,
    );

    stream.play().unwrap();

    let running_sched = running.clone();
    let handle = scheduler.run(running_sched);

    gui::run_gui(
        Some(Arc::clone(&sample_clock)),
        Arc::clone(&shared_seqs),
        sender,
        (left_delays.clone(), right_delays.clone()),
    );

    running.store(false, Ordering::Relaxed); // <- tell the scheduler to finish

    handle.join().ok();
}
