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

fn play_notes(
    items: Vec<Instrument>,
    sample_rate: f32,
    pitch: f32,
    rng: &mut rand::prelude::ThreadRng,
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

    let stream = device
        .build_output_stream(
            &config,
            {
                let sample_clock = sample_clock.clone();
                let notes = notes.clone();
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut clock = sample_clock.lock().unwrap();
                    for sample in data.iter_mut() {
                        let elapsed = *clock / sample_rate;
                        let mut value = 0.0;
                        for note in notes.iter() {
                            if (elapsed >= note.t) && (elapsed <= note.t + note.d) {
                                let t = elapsed - note.t;
                                value += generate_wave(
                                    &note.w,
                                    // note.frequency
                                    pitch * note.f.clone().compute(),
                                    t,
                                    note.d,
                                );
                            }
                        }
                        *sample = value;
                        *clock += 1.0;
                    }
                }
            },
            err_fn,
            None, // Specify latency as None
        )
        .unwrap();

    stream.play().unwrap();

    // Wait until the last note finishes
    let total_duration = notes.iter().map(|n| n.t + n.d).fold(0.0, f32::max);
    thread::sleep(Duration::from_secs_f32(total_duration * 0.5));
}

fn main() {
    let mut rng: rand::prelude::ThreadRng = rand::thread_rng();
    let freq0 = 440.0;
    loop {
        let items = read_notes_from_json("notes.json");
        let (_config, sample_rate, _channels) = setup_audio_stream();
        play_notes(items, sample_rate, freq0, &mut rng);
    }
}
