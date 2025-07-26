use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use notes::{Instrument, Note};
use std::fs::File;
use std::io::{self, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod notes;
mod waves;
use waves::{basics::*, WaveType};

mod reverb;
use reverb::Reverb;

use crossterm::event::{poll, read, Event, KeyCode};

const LEFT_DELAYS: [usize; 5] = [14683, 14699, 14713, 14717, 14723];
const RIGHT_DELAYS: [usize; 5] = [14627, 14633, 14651, 14657, 14669];

fn wait_for_exit_signal() -> bool {
    if poll(Duration::from_millis(100)).unwrap() {
        if let Event::Key(event) = read().unwrap() {
            return matches!(event.code, KeyCode::Char('q') | KeyCode::Esc);
        }
    }
    false
}

fn save_to_wav(filename: &str, sample_rate: f64, samples: &[f32], channels: u16) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let path = format!("../audio/{}.wav", filename);
    let mut writer = hound::WavWriter::create(&path, spec).expect("Failed to create WAV file");

    let max_amp = samples
        .iter()
        .copied()
        .fold(0f32, |a, b| a.max(b.abs()))
        .max(1e-6);

    for &sample in samples {
        let scaled =
            (sample / max_amp * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
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

fn read_notes_from_json(path: &str) -> Vec<Instrument> {
    let file = File::open(path).expect("Failed to open JSON file");
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).expect("Failed to parse JSON")
}

fn envelope(attack: f64, decay: f64, note_duration: f64) -> impl Fn(f64) -> f32 {
    move |time: f64| {
        let time_fraction = time / note_duration;
        (time_fraction.powf(1.0 / attack) * (1.0 - time_fraction).powf(1.0 / decay)) as f32
    }
}

fn generate_wave(wave_type: &WaveType, frequency: f64, time: f64, note_duration: f64) -> f32 {
    envelope(3.0, 1.0, note_duration)(time)
        * match wave_type {
            WaveType::Sine => sine_wave(frequency, time) as f32,
            WaveType::Square => square_wave(frequency, time) as f32,
            WaveType::Triangle => triangle_wave(frequency, time) as f32,
            WaveType::Sawtooth => sawtooth_wave(frequency, time) as f32,
            WaveType::Custom1 => custom1(frequency, time) as f32,
            WaveType::Custom2 => custom2(frequency, time) as f32,
        }
}

fn main() {
    let mut rng = rand::thread_rng();
    let freq0 = 440.0f64;

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let config = device.default_output_config().unwrap().config();
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels;

    let note_queue: Arc<Mutex<Vec<Note>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded_samples = Arc::new(Mutex::new(Vec::new()));
    let sample_clock = Arc::new(Mutex::new(0f64));

    let mut reverb_left = Reverb::new(0.5, 0.5, &LEFT_DELAYS);
    let mut reverb_right = Reverb::new(0.5, 0.5, &RIGHT_DELAYS);

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
                        let elapsed = *clock / sample_rate;
                        let mut dry = 0.0;

                        notes.retain(|note| {
                            if elapsed < note.t {
                                true
                            } else if elapsed <= note.t + note.d {
                                let t = elapsed - note.t;
                                dry += generate_wave(&note.w, freq0 * note.f.compute(), t, note.d);
                                true
                            } else {
                                false
                            }
                        });

                        let left = reverb_left.process(dry);
                        let right = reverb_right.process(dry);

                        if channels >= 2 {
                            frame[0] = left;
                            frame[1] = right;
                        } else {
                            frame[0] = (left + right) * 0.5;
                        }

                        let out = (left + right) * 0.25; // NOTE: theoretically this could be 0.5 but it saturates otherwise
                                                         // if out.abs() > 1.0 {
                                                         //     eprintln!("⚠️ Saturation: output = {out}");
                                                         // }
                        buffer.push(out);
                        *clock += 1.0;
                    }
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )
            .unwrap()
    };

    stream.play().unwrap();

    println!("🎵 Press 'q' or 'Esc' to quit...");

    let mut batch_index = 0;
    let batch_interval = 64.0; // seconds

    loop {
        let start_time = batch_interval * batch_index as f64;

        // Read and generate notes
        let instruments = read_notes_from_json("notes.json");
        let mut new_notes = Vec::new();
        for inst in instruments {
            inst.draw(&mut new_notes, &mut rng);
        }

        // Apply time offset
        for note in &mut new_notes {
            note.t += start_time;
        }

        {
            let mut notes = note_queue.lock().unwrap();
            notes.extend(new_notes);
        }

        batch_index += 1;

        // Sleep until next batch is due
        let now = *sample_clock.lock().unwrap() / sample_rate;
        let target = batch_interval * batch_index as f64;

        if target > now {
            thread::sleep(Duration::from_secs_f64(target - now - 1.0));
        }

        if wait_for_exit_signal() {
            println!("Exiting. Please enter a filename:");
            let mut name = String::new();
            io::stdin().read_line(&mut name).unwrap();
            let name = name.trim();

            let buffer = recorded_samples.lock().unwrap();
            save_to_wav(name, sample_rate, &buffer, channels);
            break;
        }
    }
}
