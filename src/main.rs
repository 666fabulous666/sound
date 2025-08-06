use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use notes::{Note, Sequence};
use std::fs::File;
use std::io::BufReader;
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

// // // around 1/3 s
// const LEFT_DELAYS: [usize; 5] = [1, 14699, 14713, 14717, 14723];
// const RIGHT_DELAYS: [usize; 5] = [1, 14633, 14651, 14657, 14669];

// const LEFT_DELAYS: [usize; 4] = [1, 14699, 22037, 7351];
// const RIGHT_DELAYS: [usize; 4] = [1, 14713, 22051, 7349];

// const LEFT_DELAYS: [usize; 1] = [14713];
// const RIGHT_DELAYS: [usize; 1] = [14651];
// const LEFT_DELAYS: [usize; 3] = [1, 14713, 22337];
// const RIGHT_DELAYS: [usize; 3] = [1, 14651, 22051];
// const LEFT_DELAYS: [usize; 2] = [14713, 22337];
// const RIGHT_DELAYS: [usize; 2] = [14651, 22051];

const LEFT_DELAYS: [usize; 3] = [1, 2, 3];
const RIGHT_DELAYS: [usize; 3] = [1, 2, 3];

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

    // let max_amp = samples
    //     .iter()
    //     .copied()
    //     .fold(0f64, |a, b| a.max(b.abs()))
    //     .max(1e-6);
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

fn read_notes_from_json(path: &str) -> Vec<Sequence> {
    let file = File::open(path).expect("Failed to open JSON file");
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).expect("Failed to parse JSON")
}

fn envelope(attack: f64, decay: f64, note_duration: f64) -> impl Fn(f64) -> f64 {
    move |time: f64| {
        let time_fraction = time / note_duration;
        0.1 * (time_fraction.powf(1.0 / attack) * (1.0 - time_fraction).powf(1.0 / decay)) as f64
    }
}

fn generate_wave(wave_type: &WaveType, frequency: f64, time: f64, note_duration: f64) -> f64 {
    envelope(4.0, 0.33, note_duration)(time)
        * match wave_type {
            WaveType::Sine => sine_wave(frequency, time),
            WaveType::Square => square_wave(frequency, time),
            WaveType::Triangle => triangle_wave(frequency, time),
            WaveType::Sawtooth => sawtooth_wave(frequency, time),
            WaveType::DistOrg => dist_org(frequency, time),
            WaveType::Custom2 => custom2(frequency, time),
            WaveType::Droplet => droplet_wave(frequency, time),
            WaveType::DropletOct => droplet_oct_wave(frequency, time),
            WaveType::HiHat => hi_hat(frequency, time),
            WaveType::Kick => kick(frequency, time),
            WaveType::Snare => snare(frequency, time),
            WaveType::Ride => ride(frequency, time),
            WaveType::Mute => mute_wave(frequency, time),
            WaveType::Xylo => xylophone_wave(frequency, time),
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

    let mut reverb_left: Reverb<44100> = Reverb::new(0.4, 0.5, &LEFT_DELAYS);
    let mut reverb_right: Reverb<44100> = Reverb::new(0.4, 0.5, &RIGHT_DELAYS);

    // Start persistent audio stream
    let stream = {
        let note_queue = Arc::clone(&note_queue);
        let recorded_samples = Arc::clone(&recorded_samples);
        let sample_clock = Arc::clone(&sample_clock);

        device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut notes = note_queue.lock().unwrap();
                    let mut buffer = recorded_samples.lock().unwrap();
                    let mut clock = sample_clock.lock().unwrap();

                    for frame in data.chunks_mut(channels as usize) {
                        // let elapsed = *clock / sample_rate;
                        let elapsed = *clock;
                        let mut dry = 0.0;

                        notes.retain(|note| {
                            if elapsed < note.t {
                                true
                            } else if elapsed <= note.t + note.d {
                                let t = elapsed - note.t;
                                let volume = note.volume
                                    / (0.5
                                        + (0.5 * note.t).fract()
                                        + (1.2 * note.t).fract()
                                        + (2.5 * note.t).fract()
                                        + (3.0 * note.t).fract());
                                // TODO: this could be part of the sequence's parameter
                                // println!("time: {elapsed}");
                                dry += volume
                                    * generate_wave(&note.w, freq0 * note.f.compute(), t, note.d); // TODO: not computing note.f here
                                true
                            } else if elapsed > note.t + note.d + 16.0 {
                                // FIXME: make this 16 automatic
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

    // 1.  Make *new* handles for the scheduler thread
    let note_queue_sched = Arc::clone(&note_queue);
    let sample_clock_sched = Arc::clone(&sample_clock);
    let recorded_samples_sched = Arc::clone(&recorded_samples);

    let running_sched = Arc::clone(&running);
    let scheduler = std::thread::spawn(move || {
        let mut rng = rand::thread_rng(); // local RNG (Send not required)

        println!("🎵 Press 'q' or 'Esc' in this terminal to quit...");
        let mut batch_index = 0;
        let batch_interval = 64.0;

        while running_sched.load(Ordering::Relaxed) {
            let start_time = batch_interval * batch_index as f64;

            // ----- create notes exactly like before -----
            let instruments = read_notes_from_json("notes.json");
            let mut new_notes = Vec::new();
            for inst in instruments {
                inst.draw(&mut new_notes, &mut rng);
            }
            for note in &mut new_notes {
                note.t += start_time;
            }
            note_queue_sched.lock().unwrap().extend(new_notes);
            batch_index += 1;

            // ----- timing -----
            let now = *sample_clock_sched.lock().unwrap();
            let target = batch_interval * batch_index as f64;
            if target > now {
                std::thread::sleep(std::time::Duration::from_secs_f64(target - now - 1.0));
            }

            // ----- exit? -----
            if wait_for_exit_signal() {
                println!("Exiting. Please enter a filename:");
                let mut name = String::new();
                std::io::stdin().read_line(&mut name).unwrap();
                let name = name.trim();

                let buffer = recorded_samples_sched.lock().unwrap();
                save_to_wav(name, sample_rate, &buffer, channels);
                break; // leave the loop → thread ends
            }
            // manual quit?
            if wait_for_exit_signal() {
                println!("Exiting. Please enter a filename:");
                // … WAV export …
                running_sched.store(false, Ordering::Relaxed); // tell everybody else to stop
                break;
            }
        }
    });

    // macOS: GUI must be on main thread
    gui::run_gui(Some(Arc::clone(&sample_clock))); // <- pass the shared clock
                                                   // blocks; returns when window is closed

    running.store(false, Ordering::Relaxed); // <- tell the scheduler to finish

    // Wait for the scheduler thread to finish (it will exit automatically
    // if the user already pressed q/Esc; otherwise closing the GUI window
    // doesn’t stop it, so you may want a channel/flag – see below).
    scheduler.join().ok();
}
