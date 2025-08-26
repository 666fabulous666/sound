use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

use crate::{
    notes::{Note, Sequence},
    LOOP_LEN,
};

pub enum Message {
    Sequence(Sequence),
    NewScore,
}

pub struct Scheduler {
    notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>>,
    sequences: Arc<Mutex<Vec<Sequence>>>,
    sample_clock: Arc<Mutex<f64>>,
    messages_rx: Receiver<Message>,
    loop_start: f64,
}

impl Scheduler {
    pub fn new(sample_clock: Arc<Mutex<f64>>) -> (Self, Sender<Message>) {
        let notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>> = Arc::new(Mutex::new(Vec::new()));
        let sequences: Arc<Mutex<Vec<Sequence>>> = Arc::new(Mutex::new(Vec::new()));

        // Channel replaces Arc<Mutex<Vec<Message>>>
        let (messages_tx, messages_rx) = mpsc::channel();

        let sched = Self {
            notes,
            sequences,
            sample_clock,
            messages_rx,
            loop_start: 0.0,
        };

        (sched, messages_tx)
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

    pub fn run(mut self, running_sched: Arc<AtomicBool>) -> JoinHandle<()> {
        std::thread::spawn(move || {
            let mut rng = rand::thread_rng();
            while running_sched.load(Ordering::Relaxed) {
                // ---- handle inbound messages (drain channel) ----
                loop {
                    match self.messages_rx.try_recv() {
                        Ok(Message::Sequence(sequence)) => {
                            // enqueue a new sequence
                            self.sequences.lock().unwrap().push(sequence);
                        }
                        Ok(Message::NewScore) => {
                            // clear existing sequences
                            self.sequences.lock().unwrap().clear();
                        }
                        Err(TryRecvError::Empty) => break, // no more messages this tick
                        Err(TryRecvError::Disconnected) => {
                            // Sender dropped; you can choose to break the loop if desired
                            break;
                        }
                    }
                }

                // ---- generate notes from sequences that need it ----
                let mut tmp = Vec::<(usize, Vec<Note>)>::new();
                let now = self.now();

                {
                    // IMPORTANT: don't clone; mutate the real sequences so timestamps persist
                    let mut seqs = self.sequences.lock().unwrap();
                    for seq in seqs.iter_mut() {
                        if now > seq.last_generation_time + LOOP_LEN {
                            seq.draw(&mut tmp, &mut rng, self.loop_start);
                            seq.last_generation_time = now;
                        }
                    }
                }

                // Append new notes
                {
                    self.notes.lock().unwrap().extend(tmp);
                }

                // ---- sleep logic (unchanged) ----
                self.loop_start += 1.0;
                if self.loop_start > now {
                    // wake up a little early to avoid missing the boundary
                    let wake_early = 0.5;
                    let sleep_s = (self.loop_start - self.now() - wake_early).max(0.0);
                    std::thread::sleep(Duration::from_secs_f64(sleep_s));
                }
            }
        })
    }
}
