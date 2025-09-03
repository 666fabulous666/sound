use crate::waves::basics::{hi_hat, kick, snare};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use notes::ChorusParams;
use scheduler::Scheduler;
use std::{
    f64::consts::PI,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

mod gui;
mod notes;
mod scheduler;
mod waves;

use waves::WaveType;
mod reverb;
use reverb::Reverb;

const DEFAULT_LOOP_LEN: f64 = 16.0; // seconds

fn envelope(attack: f64, decay: f64, note_duration: f64) -> impl Fn(f64) -> f64 {
    move |time: f64| {
        let time_fraction = time / note_duration;
        0.1 * (time_fraction.powf(1.0 / attack) * (1.0 - time_fraction).powf(1.0 / decay)) as f64
    }
}
fn time_bender(
    time: f64,
    attack_mag: f64,
    attack_time: f64,
    vibrato_mag: f64,
    vibrato_freq: f64,
) -> f64 {
    time + attack_mag * (1.0 + time).powf(-attack_time)
        + vibrato_mag * (2.0 * PI * time * vibrato_freq).sin()
}

fn generate_wave(
    wave_type: &WaveType,
    freq: f64,
    time: f64,
    duration: f64,
    attack_decay: (f64, f64),
    attack_freq_modulation: (f64, f64),
    vibrato: (f64, f64),
    chorus: &ChorusParams,
    pow_fact: f64,
) -> f64 {
    let time_bent = time_bender(
        time,
        attack_freq_modulation.0,
        attack_freq_modulation.1,
        vibrato.0,
        vibrato.1,
    );
    let f = |x: f64| match wave_type {
        WaveType::Mute => 0.0,
        WaveType::Sine => x.sin(),
        WaveType::Square => {
            if x % (2.0 * PI) < PI {
                0.25
            } else {
                -0.25
            }
        }
        WaveType::Triangle => {
            let t = x / (2.0 * PI);
            2.0 * (t - (t + 0.75).floor() + 0.25).abs() - 1.0
        }
        WaveType::Sawtooth => {
            let t = x / (2.0 * PI);
            0.5 * (t - (0.5 + t).floor())
        }
        WaveType::HiHat => hi_hat(freq, time_bent),
        WaveType::Kick => kick(freq, time_bent),
        WaveType::Snare => snare(freq, time_bent),
    };
    let phase = 2.0 * PI * freq * time_bent;
    let pow_fact = (pow_fact * time).exp();
    let tmp = (0..chorus.number_of_heads)
        .map(|k| {
            let delta = chorus.delta * (chorus.time_dependency * time).exp2();
            let two_pow_k = 2f64.powi(k as i32);
            let sym_pow_k = chorus.sym.powi(k as i32);
            let asym_pow_k = chorus.asym.powi(k as i32);
            let tmp1 = f(phase * (1.0 + two_pow_k * delta));
            let tmp2 = f(phase * (1.0 - two_pow_k * delta));
            let tmp1 = tmp1.signum() * tmp1.abs().min(1.0).powf(pow_fact);
            let tmp2 = tmp2.signum() * tmp2.abs().min(1.0).powf(pow_fact);
            let tmp = (sym_pow_k + asym_pow_k) * tmp1 + (sym_pow_k - asym_pow_k) * tmp2;
            tmp
        })
        .sum::<f64>()
        / (freq / 440.0).sqrt();
    envelope(attack_decay.0, attack_decay.1, duration)(time) * tmp
}

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
                                if elapsed < note.t {
                                    true
                                } else if elapsed <= note.t + note.d {
                                    let t = elapsed - note.t;
                                    let volume = note.volume // TODO: make this parameters
                                    / (1.5
                                        + (0.5 * note.t).fract()
                                        + (1.2 * note.t).fract()
                                        + (2.5 * note.t).fract()
                                        + (3.0 * note.t).fract());
                                    let dry = volume
                                        * generate_wave(
                                            &note.w,
                                            freq0 * note.f.compute(),
                                            t,
                                            note.d,
                                            note.attack_decay,
                                            note.attack_freq_modulation,
                                            note.vibrato,
                                            &note.chorus,
                                            note.pow_fact,
                                        );
                                    dry_left += (1.0 - note.spacial) * dry;
                                    dry_right += note.spacial * dry;
                                    true
                                } else if elapsed > note.t + note.d + 1.0 {
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

    gui::run_gui(
        Some(Arc::clone(&sample_clock)),
        Arc::clone(&shared_seqs),
        sender,
        (left_delays.clone(), right_delays.clone()),
    );

    running.store(false, Ordering::Relaxed); // <- tell the scheduler to finish

    handle.join().ok();
}
