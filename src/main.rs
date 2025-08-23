use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use notes::{Note, Sequence};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

mod gui;
mod notes;
mod waves;
use waves::{basics::*, WaveType};
mod reverb;
use reverb::Reverb;

use crossterm::event::{poll, read, Event, KeyCode};

// // // // around 1/3 s
// const LEFT_DELAYS: [usize; 5] = [1, 14699, 14713, 14717, 14723];
// const RIGHT_DELAYS: [usize; 5] = [1, 14633, 14651, 14657, 14669];
// const LEFT_DELAYS: [usize; 4] = [1, 14699, 22037, 7351];
// const RIGHT_DELAYS: [usize; 4] = [1, 14713, 22051, 7349];
const LEFT_DELAYS: [usize; 1] = [1];
const RIGHT_DELAYS: [usize; 1] = [1];

const LOOP_LEN: f64 = 16.0; // seconds

fn wait_for_exit_signal() -> bool {
    if poll(Duration::from_millis(100)).unwrap() {
        if let Event::Key(event) = read().unwrap() {
            return matches!(event.code, KeyCode::Char('q') | KeyCode::Esc);
        }
    }
    false
}

fn save_to_wav(filename: &str, sample_rate: f64, samples: &[f64], channels: u16) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let path = format!("../audio/{}.wav", filename);
    let mut writer = hound::WavWriter::create(&path, spec).expect("Failed to create WAV file");

    let max_amp = 1.0;

    for &sample in samples {
        let scaled =
            (sample / max_amp * i16::MAX as f64).clamp(i16::MIN as f64, i16::MAX as f64) as i16;
        writer.write_sample(scaled).unwrap();
    }

    writer.finalize().expect("Failed to finalize WAV file");

    println!(
        "✅ Saved '{}' — {:.2} sec, {} channels at {} Hz",
        path,
        samples.len() as f64 / sample_rate / channels as f64,
        channels,
        sample_rate
    );
}

fn envelope(attack: f64, decay: f64, note_duration: f64) -> impl Fn(f64) -> f64 {
    move |time: f64| {
        let time_fraction = time / note_duration;
        0.1 * (time_fraction.powf(1.0 / attack) * (1.0 - time_fraction).powf(1.0 / decay)) as f64
    }
}

fn generate_wave(
    wave_type: &WaveType,
    freq: f64,
    time: f64,
    duration: f64,
    attack_decay: (f64, f64),
) -> f64 {
    envelope(attack_decay.0, attack_decay.1, duration)(time)
        * match wave_type {
            WaveType::Sine => sine_wave(freq, time),
            WaveType::Square => square_wave(freq, time),
            WaveType::Triangle => triangle_wave(freq, time),
            WaveType::Sawtooth => sawtooth_wave(freq, time),
            WaveType::DistOrg => dist_org(freq, time),
            WaveType::Custom2 => custom2(freq, time),
            WaveType::Droplet => droplet_wave(freq, time),
            WaveType::DropletOct => droplet_oct_wave(freq, time),
            WaveType::HiHat => hi_hat(freq, time),
            WaveType::Kick => kick(freq, time),
            WaveType::Snare => snare(freq, time),
            WaveType::Ride => ride(freq, time),
            WaveType::Mute => mute_wave(freq, time),
            WaveType::Xylo => xylophone_wave(freq, time),
        }
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

    let note_queue: Arc<Mutex<Vec<Note>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded_samples = Arc::new(Mutex::new(Vec::new()));
    let sample_clock = Arc::new(Mutex::new(0f64));
    let running = Arc::new(AtomicBool::new(true));
    let shared_seqs: Arc<Mutex<Vec<Sequence>>> = Arc::new(Mutex::new(Vec::new()));

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

                        notes.retain(|note| {
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
                                    );
                                true
                            } else if elapsed > note.t + note.d + 1.0 {
                                false
                            } else {
                                true
                            }
                        });

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

    let note_queue_sched = note_queue.clone();
    let sample_clock_sched = sample_clock.clone();
    let recorded_samples_sched = recorded_samples.clone();
    let shared_seqs_sched = shared_seqs.clone();
    let running_sched = running.clone();

    let scheduler = std::thread::spawn(move || {
        let mut rng = rand::thread_rng();

        println!("🎵 Press 'q' or 'Esc' in this terminal to quit...");
        let mut batch_index: usize = 0;
        let batch_interval = LOOP_LEN; // seconds; one full loop per batch

        while running_sched.load(Ordering::Relaxed) {
            // Absolute time at which this batch starts
            let start_time = batch_interval * batch_index as f64;

            // 1) Snapshot sequences ONCE per batch (no change detection here)
            let seqs = {
                // keep lock scope tiny
                shared_seqs_sched.lock().unwrap().clone()
            };

            // 2) Render one full batch in local time [seq.t_min, seq.t_max],
            //    preserving cross-sequence context, then shift by start_time.
            let mut context = Vec::<Note>::new(); // shared for interaction between sequences
            let mut flat = Vec::<Note>::new();

            for seq in &seqs {
                let before = context.len();
                seq.draw(&mut context, &mut rng); // writes this seq's notes to `context`
                let mut group = context[before..].to_vec();
                for n in &mut group {
                    n.t += start_time; // shift to absolute time in this batch
                }
                flat.extend(group);
            }

            // 3) Publish this batch’s notes to the audio thread
            note_queue_sched.lock().unwrap().extend(flat);

            // 4) Prepare next batch
            batch_index += 1;

            // 5) Sleep until (just before) the next batch boundary
            let now = *sample_clock_sched.lock().unwrap();
            let target = batch_interval * batch_index as f64;

            if target > now {
                // wake up a little early to avoid missing the boundary
                let wake_early = 1.0;
                let sleep_s = (target - now - wake_early).max(0.0);
                std::thread::sleep(std::time::Duration::from_secs_f64(sleep_s));
            }

            // 6) Allow terminal quit + WAV export (unchanged from your code)
            if wait_for_exit_signal() {
                println!("Exiting. Please enter a filename:");
                let mut name = String::new();
                std::io::stdin().read_line(&mut name).unwrap();
                let name = name.trim();

                let buffer = recorded_samples_sched.lock().unwrap();
                save_to_wav(name, sample_rate, &buffer, channels);

                running_sched.store(false, Ordering::Relaxed); // tell other threads to stop
                break;
            }
        }
    });

    gui::run_gui(Some(Arc::clone(&sample_clock)), Arc::clone(&shared_seqs));

    running.store(false, Ordering::Relaxed); // <- tell the scheduler to finish

    // Wait for the scheduler thread to finish (it will exit automatically
    // if the user already pressed q/Esc; otherwise closing the GUI window
    // doesn’t stop it, so you may want a channel/flag – see below).
    scheduler.join().ok();
}
