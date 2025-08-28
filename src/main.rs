use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
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

use waves::{basics::*, WaveType};
mod reverb;
use reverb::Reverb;

// // // // around 1/3 s
// const LEFT_DELAYS: [usize; 5] = [1, 14699, 14713, 14717, 14723];
// const RIGHT_DELAYS: [usize; 5] = [1, 14633, 14651, 14657, 14669];
const LEFT_DELAYS: [usize; 4] = [1, 14699, 22037, 7351];
const RIGHT_DELAYS: [usize; 4] = [1, 14713, 22051, 7349];
// const LEFT_DELAYS: [usize; 1] = [1];
// const RIGHT_DELAYS: [usize; 1] = [1];

const LOOP_LEN: f64 = 32.0; // seconds

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
    chorus: (usize, f64, f64),
) -> f64 {
    let time_bent = time_bender(
        time,
        attack_freq_modulation.0,
        attack_freq_modulation.1,
        vibrato.0,
        vibrato.1,
    );
    let phase = 2.0 * PI * freq * time_bent;
    let tmp = (0..chorus.0)
        .map(|k| {
            (chorus.2).powi(k as i32)
                * ((phase * (1.0 + 2.0f64.powi(k as i32) * chorus.1)).sin()
                    + (phase * (1.0 - 2.0f64.powi(k as i32) * chorus.1)).sin())
        })
        .sum::<f64>()
        / (freq / 440.0).sqrt();
    envelope(attack_decay.0, attack_decay.1, duration)(time) * tmp
    // * match wave_type {
    //     WaveType::Sine => sine_wave(freq, time_bent),
    //     WaveType::Square => square_wave(freq, time_bent),
    //     WaveType::Triangle => triangle_wave(freq, time_bent),
    //     WaveType::Sawtooth => sawtooth_wave(freq, time_bent),
    //     WaveType::DistOrg => dist_org(freq, time_bent),
    //     WaveType::Custom2 => custom2(freq, time_bent),
    //     WaveType::Droplet => droplet_wave(freq, time_bent),
    //     WaveType::DropletOct => droplet_oct_wave(freq, time_bent),
    //     WaveType::HiHat => hi_hat(freq, time_bent),
    //     WaveType::Kick => kick(freq, time_bent),
    //     WaveType::Snare => snare(freq, time_bent),
    //     WaveType::Ride => ride(freq, time_bent),
    //     WaveType::Mute => mute_wave(freq, time_bent),
    //     WaveType::Xylo => xylophone_wave(freq, time_bent),
    // }
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
    // let note_queue: Arc<Mutex<Vec<(usize, Vec<Note>)>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded_samples = Arc::new(Mutex::new(Vec::new()));
    let running = Arc::new(AtomicBool::new(true));
    // let shared_seqs: Arc<Mutex<Vec<Sequence>>> = Arc::new(Mutex::new(Vec::new()));

    let mut reverb_left: Reverb<44100> = Reverb::new(0.5, 0.5, &LEFT_DELAYS);
    let mut reverb_right: Reverb<44100> = Reverb::new(0.5, 0.5, &RIGHT_DELAYS);

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
                        let mut dry = 0.0;

                        for (_, notes_from_seq) in notes.iter_mut() {
                            notes_from_seq.retain(|note| {
                                if elapsed < note.t {
                                    true
                                } else if elapsed <= note.t + note.d {
                                    let t = elapsed - note.t;
                                    let volume = note.volume // TODO: make this parameters
                                    / (0.5
                                        + (0.5 * note.t).fract()
                                        + (1.2 * note.t).fract()
                                        + (2.5 * note.t).fract()
                                        + (3.0 * note.t).fract());
                                    dry += volume
                                        * generate_wave(
                                            &note.w,
                                            freq0 * note.f.compute(),
                                            t,
                                            note.d,
                                            note.attack_decay,
                                            note.attack_freq_modulation,
                                            note.vibrato,
                                            note.chorus,
                                        );
                                    true
                                } else if elapsed > note.t + note.d + 1.0 {
                                    false
                                } else {
                                    true
                                }
                            })
                        }

                        let left = reverb_left.process(dry);
                        let right = reverb_right.process(dry);

                        if channels >= 2 {
                            frame[0] = left as f32;
                            frame[1] = right as f32;
                        } else {
                            frame[0] = (left + right) as f32 * 0.5;
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
    );

    running.store(false, Ordering::Relaxed); // <- tell the scheduler to finish

    handle.join().ok();
}
