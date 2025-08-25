use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

use crossterm::event::{poll, read, Event, KeyCode};

use crate::{
    notes::{Note, Sequence},
    LOOP_LEN,
};

pub enum Message {
    NewSeq(Sequence),
}
pub struct Scheduler {
    notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>>,
    sequences: Arc<Mutex<Vec<Sequence>>>,
    sample_clock: Arc<Mutex<f64>>,
    messages: Arc<Mutex<Vec<Message>>>,
    loop_start: f64,
}

impl Scheduler {
    pub fn new(sample_clock: Arc<Mutex<f64>>) -> Self {
        let notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>> = Arc::new(Mutex::new(Vec::new()));
        let sequences: Arc<Mutex<Vec<Sequence>>> = Arc::new(Mutex::new(Vec::new()));
        let messages: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));
        Self {
            notes,
            sequences,
            sample_clock,
            messages,
            loop_start: 0.0,
        }
    }
    fn now(&self) -> f64 {
        *self.sample_clock.lock().unwrap()
    }
    pub fn notes(&self) -> Arc<Mutex<Vec<(usize, Vec<Note>)>>> {
        self.notes.clone()
    }
    pub fn sequences(&self) -> Arc<Mutex<Vec<Sequence>>> {
        self.sequences.clone()
    }
    pub fn messages(&self) -> Arc<Mutex<Vec<Message>>> {
        self.messages.clone()
    }
    pub fn run(mut self, running_sched: Arc<AtomicBool>) -> JoinHandle<()> {
        std::thread::spawn(move || {
            let mut rng = rand::thread_rng();
            while running_sched.load(Ordering::Relaxed) {
                let mut tmp = Vec::<(usize, Vec<Note>)>::new();

                {
                    // keep lock scope tiny
                    self.sequences.lock().unwrap().clone()
                }
                .iter_mut()
                .for_each(|seq| {
                    if self.now() > seq.last_generation_time + LOOP_LEN {
                        seq.draw(&mut tmp, &mut rng, self.loop_start); // writes this seq's notes to `context`
                        seq.last_generation_time = self.now();
                    }
                });

                {
                    self.notes.lock().unwrap().extend(tmp)
                };
                self.loop_start += 1.0;
                if self.loop_start > self.now() {
                    // wake up a little early to avoid missing the boundary
                    let wake_early = 0.5;
                    let sleep_s = (self.loop_start - self.now() - wake_early).max(0.0);
                    std::thread::sleep(std::time::Duration::from_secs_f64(sleep_s));
                }
            }
        })
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
