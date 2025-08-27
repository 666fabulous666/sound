use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

use rand::rngs::ThreadRng;

use crate::{
    notes::{Note, Sequence},
    LOOP_LEN,
};

pub enum Message {
    NewScore,
    NewSequence(Sequence),
    EditSequence(usize, Sequence),
    DeleteSequence(usize),
    SwapSequences(usize, usize),
}

pub struct Scheduler {
    notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>>,
    sequences: Arc<Mutex<Vec<Sequence>>>,
    sample_clock: Arc<Mutex<f64>>,
    messages_rx: Receiver<Message>,
    sched_start: f64,
}

impl Scheduler {
    pub fn new(sample_clock: Arc<Mutex<f64>>) -> (Self, Sender<Message>) {
        let notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>> = Arc::new(Mutex::new(Vec::new()));
        let sequences: Arc<Mutex<Vec<Sequence>>> = Arc::new(Mutex::new(Vec::new()));

        let (messages_tx, messages_rx) = mpsc::channel();

        let sched = Self {
            notes,
            sequences,
            sample_clock,
            messages_rx,
            sched_start: 0.0,
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
                let mut notes_buffer = Vec::<(usize, Vec<Note>)>::new();
                // ---- handle inbound messages (drain channel) ----
                loop {
                    match self.messages_rx.try_recv() {
                        Ok(Message::NewScore) => {
                            self.sequences.lock().unwrap().clear();
                            self.notes.lock().unwrap().clear();
                        }
                        Ok(Message::NewSequence(mut sequence)) => {
                            self.draw_seq(&mut sequence, &mut rng, &mut notes_buffer, LOOP_LEN);
                            self.sequences.lock().unwrap().push(sequence);
                        }
                        Ok(Message::EditSequence(a, mut sequence)) => {
                            self.remove_seq(sequence.token);
                            self.draw_seq(&mut sequence, &mut rng, &mut notes_buffer, LOOP_LEN);
                            self.sequences.lock().unwrap()[a] = sequence;
                        }
                        Ok(Message::DeleteSequence(a)) => {
                            let tk = { self.sequences.lock().unwrap()[a].token.clone() };
                            self.remove_seq(tk);
                            self.sequences.lock().unwrap().remove(a);
                        }
                        Ok(Message::SwapSequences(a, b)) => {
                            self.remove_seq_at(a);
                            self.remove_seq_at(b);
                            self.sequences.lock().unwrap().swap(a, b);
                        }
                        Err(TryRecvError::Empty) => break, // no more messages this tick
                        Err(TryRecvError::Disconnected) => {
                            // Sender dropped;
                            break;
                        }
                    }
                }

                // ---- generate notes from sequences that need it ----
                {
                    let mut seqs = self.sequences.lock().unwrap();
                    for seq in seqs.iter_mut() {
                        // if now > seq.last_generation_time + LOOP_LEN {
                        if seq.last_generation_time.is_none()
                            || seq
                                .last_generation_time
                                .as_ref()
                                .is_some_and(|t| self.now() > t + LOOP_LEN)
                        {
                            self.draw_seq(seq, &mut rng, &mut notes_buffer, LOOP_LEN);
                        }
                    }
                }

                // Append new notes
                {
                    self.notes.lock().unwrap().extend(notes_buffer);
                }

                // ---- sleep logic ----
                self.sched_start += 1.0;
                if self.sched_start > self.now() {
                    let sleep_s = (self.sched_start - self.now() - 0.5).max(0.0);
                    std::thread::sleep(Duration::from_secs_f64(sleep_s));
                }
            }
        })
    }

    fn remove_seq_at(&mut self, a: usize) {
        let s = &mut self.sequences.lock().unwrap()[a];
        s.last_generation_time = None;
        self.remove_seq(s.token);
    }
    fn draw_seq(
        &self,
        seq: &mut Sequence,
        rng: &mut ThreadRng,
        notes_buffer: &mut Vec<(usize, Vec<Note>)>,
        loop_len: f64,
    ) {
        seq.draw(
            notes_buffer,
            rng,
            (self.sched_start / loop_len).floor() * loop_len,
        );
        seq.last_generation_time = Some(self.now());
    }
    fn remove_seq(&self, tk: usize) {
        self.notes.lock().unwrap().retain(|(token, _)| *token != tk);
    }
}
