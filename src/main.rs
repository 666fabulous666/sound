mod app;
mod engine;
mod reverb;
mod scheduler;
mod time;

use crate::engine::waves::{
    basics::{hi_hat, kick, snare},
    WaveType,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use engine::notes::ChorusParams;
use scheduler::Scheduler;
use std::{
    f64::consts::PI,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use reverb::Reverb;

const DEFAULT_LOOP_LEN: f64 = 16.0; // seconds

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use crate::app::run_gui;

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
    let mut reverb_left: Reverb<44100> = Reverb::new(0.5, 0.5, left_delays.clone());
    let mut reverb_right: Reverb<44100> = Reverb::new(0.5, 0.5, right_delays.clone());

    // Start persistent audio stream
    let stream = {
        let note_queue = note_queue.clone();
        let recorded_samples = recorded_samples.clone();
        let sample_clock = sample_clock.clone();

        device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut notes = note_queue.lock().unwrap();
                    let mut buffer = recorded_samples.lock().unwrap();
                    let mut clock = sample_clock.lock().unwrap();

                    for frame in data.chunks_mut(channels as usize) {
                        let elapsed = *clock;
                        let mut dry_left = 0.0;
                        let mut dry_right = 0.0;

                        for (_, notes_from_seq) in notes.iter_mut() {
                            notes_from_seq.retain(|note| {
                                if elapsed < note.time {
                                    true
                                } else if elapsed <= note.time + note.duration {
                                    use crate::engine::waves::generate_wave;

                                    let t = elapsed - note.time;
                                    let volume = note.volume // TODO: make this parameters
                                    / (1.5
                                        + (0.5 * note.time).fract()
                                        + (1.2 * note.time).fract()
                                        + (2.5 * note.time).fract()
                                        + (3.0 * note.time).fract());
                                    let dry = volume
                                        * generate_wave(
                                            &note.wave_type,
                                            freq0 * note.interval.compute(),
                                            t,
                                            note.duration,
                                            note.attack_decay,
                                            note.attack_freq_modulation,
                                            note.vibrato,
                                            &note.chorus,
                                            note.pow_fact,
                                        );
                                    dry_left += (1.0 - note.spacial) * dry;
                                    dry_right += note.spacial * dry;
                                    true
                                } else if elapsed > note.time + note.duration + 12.0 {
                                    false
                                } else {
                                    true
                                }
                            })
                        }

                        let left = reverb_left.process(dry_left);
                        let right = reverb_right.process(dry_right);

                        if channels >= 2 {
                            frame[0] = left as f32;
                            frame[1] = right as f32;
                        } else {
                            frame[0] = (left + right) as f32;
                        }

                        buffer.push(left);
                        buffer.push(right);
                        *clock += sample_duration;
                    }
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )
            .unwrap()
    };

    stream.play().unwrap();

    let running_sched = running.clone();
    let handle = scheduler.run(running_sched);

    run_gui(
        Some(Arc::clone(&sample_clock)),
        Arc::clone(&shared_seqs),
        sender,
        (left_delays.clone(), right_delays.clone()),
    );

    running.store(false, Ordering::Relaxed); // <- tell the scheduler to finish

    handle.join().ok();
}
#[cfg(target_arch = "wasm32")]
fn main() {
    // No-op: the web entry point is in src/lib.rs via #[wasm_bindgen(start)].
}
