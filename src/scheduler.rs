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

use crate::notes::{Note, Sequence};

pub enum Message {
    NewScore,
    NewSequence(Sequence),
    EditSequence(usize, Sequence),
    DeleteSequence(usize),
    CloneSequence(usize, usize),
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
                            self.draw_seq(&mut sequence, &mut rng, &mut notes_buffer);
                            self.sequences.lock().unwrap().push(sequence);
                        }
                        Ok(Message::EditSequence(a, mut sequence)) => {
                            self.regen_seq(&mut sequence, &mut rng, &mut notes_buffer);
                            let len = {
                                let mut seqs = self.sequences.lock().unwrap();
                                seqs[a] = sequence;
                                seqs.len()
                            };
                            (a + 1..len)
                                .for_each(|k| self.regen_seq_at(k, &mut rng, &mut notes_buffer));
                        }
                        Ok(Message::DeleteSequence(a)) => {
                            let tk = { self.sequences.lock().unwrap()[a].token.clone() };
                            self.remove_seq(tk);
                            self.sequences.lock().unwrap().remove(a);
                        }
                        Ok(Message::CloneSequence(a, new_token)) => {
                            let mut sequence = { self.sequences.lock().unwrap()[a].clone() };
                            sequence.token = new_token;
                            self.draw_seq(&mut sequence, &mut rng, &mut notes_buffer);
                            self.sequences.lock().unwrap().push(sequence);
                        }
                        Ok(Message::SwapSequences(a, b)) => {
                            self.sequences.lock().unwrap().swap(a, b);
                            self.regen_seq_at(a, &mut rng, &mut notes_buffer);
                            self.regen_seq_at(b, &mut rng, &mut notes_buffer);
                            let len = self.sequences.lock().unwrap().len();
                            (a.max(b) + 1..len)
                                .for_each(|k| self.regen_seq_at(k, &mut rng, &mut notes_buffer));
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
                        if seq.not_generate_until.is_none()
                            || seq
                                .not_generate_until
                                .as_ref()
                                .is_some_and(|until| self.now() > *until)
                        {
                            self.draw_seq(seq, &mut rng, &mut notes_buffer);
                        }
                    }
                }

                // Append new notes
                {
                    self.notes.lock().unwrap().extend(notes_buffer);
                }

                // ---- sleep logic ----
                let sched_start_increase = 5e-2;
                self.sched_start += sched_start_increase;
                let wake_early = sched_start_increase * 2.0; // WARINIG: isn't it supposed to be smaller than sched_start_increase?
                if self.sched_start > self.now() {
                    let sleep_s = (self.sched_start - self.now() - wake_early).max(0.0);
                    std::thread::sleep(Duration::from_secs_f64(sleep_s));
                }
            }
        })
    }

    fn draw_seq(
        &self,
        seq: &mut Sequence,
        rng: &mut ThreadRng,
        notes_buffer: &mut Vec<(usize, Vec<Note>)>,
    ) {
        let loop_len = seq.loop_len;
        let seq_start = (self.sched_start / loop_len).floor() * loop_len;
        seq.draw(notes_buffer, rng, seq_start);
        seq.not_generate_until = Some(seq_start + loop_len - 0.1);
    }
    fn remove_seq(&self, tk: usize) {
        self.notes.lock().unwrap().retain(|(token, _)| *token != tk);
    }
    fn regen_seq(
        &self,
        seq: &mut Sequence,
        rng: &mut ThreadRng,
        notes_buffer: &mut Vec<(usize, Vec<Note>)>,
    ) {
        self.remove_seq(seq.token);
        self.draw_seq(seq, rng, notes_buffer);
    }
    fn regen_seq_at(
        &self,
        a: usize,
        rng: &mut ThreadRng,
        notes_buffer: &mut Vec<(usize, Vec<Note>)>,
    ) {
        let s = &mut self.sequences.lock().unwrap()[a];
        self.regen_seq(s, rng, notes_buffer);
    }
}
