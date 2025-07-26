use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use notes::Instrument;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod notes;
mod waves;
use waves::{basics::*, WaveType};

use crossterm::event::{poll, read, Event, KeyCode};
use std::io;

mod reverb;
use reverb::Reverb;

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
fn save_to_wav(filename: &str, sample_rate: f32, samples: &[f32], channels: u16) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let path = format!("../audio/{}.wav", filename);
    let mut writer =
        hound::WavWriter::create(format!("{path}"), spec).expect("Failed to create WAV file");

    let max_amp = samples
        .iter()
        .copied()
        .fold(0f32, |a, b| a.max(b.abs()))
        .max(1e-6);

    match channels {
        1 => {
            for &sample in samples {
                let scaled = (sample / max_amp * i16::MAX as f32)
                    .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                writer.write_sample(scaled).unwrap();
            }
        }
        2 => {
            for &sample in samples {
                let scaled = (sample / max_amp * i16::MAX as f32)
                    .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                writer.write_sample(scaled).unwrap();
            }
        }
        _ => panic!("Only mono and stereo output are supported."),
    }

    writer.finalize().expect("Failed to finalize WAV file");

    println!(
        "✅ Saved '{}' — {:.2} sec, {} channels at {} Hz",
        path,
        samples.len() as f32 / sample_rate / channels as f32,
        channels,
        sample_rate
    );
}
fn read_notes_from_json(path: &str) -> Vec<Instrument> {
    let file = File::open(path).expect("Failed to open JSON file");
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).expect("Failed to parse JSON")
}

fn enveloppe(attack: f32, decay: f32, note_duration: f32) -> impl Fn(f32) -> f32 {
    move |time: f32| {
        let time_fraction = time / note_duration;
        time_fraction.powf(1.0 / attack) * (1.0 - time_fraction).powf(1.0 / decay)
    }
}

fn generate_wave(wave_type: &WaveType, frequency: f32, time: f32, note_duration: f32) -> f32 {
    enveloppe(3.0, 1.0, note_duration)(time)
        * match wave_type {
            WaveType::Sine => sine_wave(frequency, time),
            WaveType::Square => square_wave(frequency, time),
            WaveType::Triangle => triangle_wave(frequency, time),
            WaveType::Sawtooth => sawtooth_wave(frequency, time),
            WaveType::Custom1 => custom1(frequency, time),
            WaveType::Custom2 => custom2(frequency, time),
        }
}

fn setup_audio_stream() -> (cpal::StreamConfig, f32, u16) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let config = device.default_output_config().unwrap().config();
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels;

    (config, sample_rate, channels)
}

fn main() {
    let mut rng = rand::thread_rng();
    let freq0 = 440.0;
    let (_config, sample_rate, channels) = setup_audio_stream();

    let mut all_recorded_samples = Vec::new();
    loop {
        let items = read_notes_from_json("notes.json");

        let recorded_samples = Arc::new(Mutex::new(Vec::new()));
        play_notes_with_recording(
            items,
            sample_rate,
            freq0,
            &mut rng,
            recorded_samples.clone(),
        );

        {
            let locked = recorded_samples.lock().unwrap();
            all_recorded_samples.extend(locked.iter());
        }

        if wait_for_exit_signal() {
            println!("Exiting. Please enter a filename:");
            let mut name = String::new();
            io::stdin().read_line(&mut name).unwrap();
            let name = name.trim();
            save_to_wav(name, sample_rate, &all_recorded_samples, channels);
            break;
        }
    }
}
fn play_notes_with_recording(
    items: Vec<Instrument>,
    sample_rate: f32,
    pitch: f32,
    rng: &mut rand::prelude::ThreadRng,
    recorded_samples: Arc<Mutex<Vec<f32>>>,
) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let config = device.default_output_config().unwrap().config();

    let err_fn = |err| eprintln!("An error occurred on the output audio stream: {}", err);

    let sample_clock = Arc::new(Mutex::new(0f32));
    let mut notes = Vec::<notes::Note>::new();
    items.iter().for_each(|item| item.draw(&mut notes, rng));

    let mut reverb_left = Reverb::new(0.5, 0.5, &LEFT_DELAYS);
    let mut reverb_right = Reverb::new(0.5, 0.5, &RIGHT_DELAYS);
    let stream = device
        .build_output_stream(
            &config,
            {
                let sample_clock = sample_clock.clone();
                let notes = notes.clone();
                let recorded_samples = recorded_samples.clone();
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut clock = sample_clock.lock().unwrap();
                    let mut buffer = recorded_samples.lock().unwrap();

                    for frame in data.chunks_mut(config.channels as usize) {
                        let elapsed = *clock / sample_rate;
                        let mut dry = 0.0;

                        for note in notes.iter() {
                            if (elapsed >= note.t) && (elapsed <= note.t + note.d) {
                                let t = elapsed - note.t;
                                dry += generate_wave(
                                    &note.w,
                                    pitch * note.f.clone().compute(),
                                    t,
                                    note.d,
                                );
                            }
                        }

                        let left = reverb_left.process(dry);
                        let right = reverb_right.process(dry); // independent

                        frame[0] = left;
                        frame[1] = right;

                        buffer.push((left + right) * 0.5);
                        *clock += 1.0;
                    }
                }
            },
            err_fn,
            None,
        )
        .unwrap();

    stream.play().unwrap();

    let total_duration = notes.iter().map(|n| n.t + n.d).fold(0.0, f32::max);
    thread::sleep(Duration::from_secs_f32(total_duration * 0.5));
}
