use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use crossterm::event::{poll, read, Event, KeyCode};

use crate::{
    notes::{Note, Sequence},
    LOOP_LEN,
};
pub struct Scheduler {
    handle: std::thread::JoinHandle<()>,
}

pub enum SchedulerMessage {}

impl Scheduler {
    pub fn new(
        sample_rate: f64,
        channels: u16,
        note_queue_sched: Arc<Mutex<Vec<(usize, Vec<Note>)>>>,
        sample_clock_sched: Arc<Mutex<f64>>,
        recorded_samples_sched: Arc<Mutex<Vec<f64>>>,
        shared_seqs_sched: Arc<Mutex<Vec<Sequence>>>,
        running_sched: Arc<AtomicBool>,
    ) -> Self {
        let scheduler = std::thread::spawn(move || {
            let mut rng = rand::thread_rng();

            println!("🎵 Press 'q' or 'Esc' in this terminal to quit...");
            let mut batch_index: usize = 0;
            let batch_interval = LOOP_LEN; // seconds; one full loop per batch

            while running_sched.load(Ordering::Relaxed) {
                // Absolute time at which this batch starts
                let start_time = batch_interval * batch_index as f64;

                let mut context = Vec::<(usize, Vec<Note>)>::new(); // shared for interaction between sequences

                {
                    // keep lock scope tiny
                    shared_seqs_sched.lock().unwrap().clone()
                }
                .iter()
                .for_each(|seq| {
                    seq.draw(&mut context, &mut rng, start_time); // writes this seq's notes to `context`
                });

                {
                    note_queue_sched.lock().unwrap().extend(context)
                };

                batch_index += 1;

                let now = *sample_clock_sched.lock().unwrap();
                let target = batch_interval * batch_index as f64;

                if target > now {
                    // wake up a little early to avoid missing the boundary
                    let wake_early = 1.0;
                    let sleep_s = (target - now - wake_early).max(0.0);
                    std::thread::sleep(std::time::Duration::from_secs_f64(sleep_s));
                }

                // if wait_for_exit_signal() {
                //     println!("Exiting. Please enter a filename:");
                //     let mut name = String::new();
                //     std::io::stdin().read_line(&mut name).unwrap();
                //     let name = name.trim();

                //     let buffer = recorded_samples_sched.lock().unwrap();
                //     save_to_wav(name, sample_rate, &buffer, channels);

                //     running_sched.store(false, Ordering::Relaxed); // tell other threads to stop
                //     break;
                // }
            }
        });
        Self { handle: scheduler }
    }
    pub fn handle(self) -> std::thread::JoinHandle<()> {
        self.handle
    }
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
fn wait_for_exit_signal() -> bool {
    if poll(Duration::from_millis(100)).unwrap() {
        if let Event::Key(event) = read().unwrap() {
            return matches!(event.code, KeyCode::Char('q') | KeyCode::Esc);
        }
    }
    false
}
