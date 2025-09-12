use crate::{
    engine::notes::{Note, Sequence},
    SCHEDULER_STEP, SCHEDULER_WAKE_EARLY,
};
use instant::Duration;
use rand::rngs::ThreadRng;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

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
    receiver: Receiver<Message>,
    sched_start: f64,
}

impl Scheduler {
    pub fn new(sample_clock: Arc<Mutex<f64>>) -> (Self, Sender<Message>) {
        let notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>> = Arc::new(Mutex::new(Vec::new()));
        let sequences: Arc<Mutex<Vec<Sequence>>> = Arc::new(Mutex::new(Vec::new()));

        let (messages_tx, receiver) = mpsc::channel();

        let sched = Self {
            notes,
            sequences,
            sample_clock,
            receiver,
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
    fn run_loop(&mut self, running_sched: Arc<AtomicBool>, mut rng: ThreadRng) {
        while running_sched.load(Ordering::Relaxed) {
            self.run_once(&mut rng);
            self.sleep_for(5e-2);
        }
    }

    pub fn run_once(&mut self, rng: &mut ThreadRng) {
        if self.sched_start < self.now() + SCHEDULER_WAKE_EARLY {
            // WARNING: should it be a while?
            let mut notes_buffer = Vec::<(usize, Vec<Note>)>::new();
            // ---- handle inbound messages (drain channel) ----
            self.drain_messages(rng, &mut notes_buffer);

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
                        self.draw_seq(seq, rng, &mut notes_buffer);
                    }
                }
            }

            // Append new notes
            {
                self.notes.lock().unwrap().extend(notes_buffer);
            }
            self.sched_start += SCHEDULER_STEP;
        }
    }

    fn sleep_for(&mut self, dt: f64) {
        let sched_start_increase = dt;
        self.sched_start += sched_start_increase;
        let wake_early = sched_start_increase * 2.0; // WARINIG: isn't it supposed to be smaller than sched_start_increase?
        if self.sched_start > self.now() {
            let sleep_s = (self.sched_start - self.now() - wake_early).max(0.0);
            std::thread::sleep(Duration::from_secs_f64(sleep_s));
        }
    }

    fn drain_messages(&mut self, rng: &mut ThreadRng, notes_buffer: &mut Vec<(usize, Vec<Note>)>) {
        loop {
            match self.receiver.try_recv() {
                Ok(Message::NewScore) => {
                    self.sequences.lock().unwrap().clear();
                    self.notes.lock().unwrap().clear();
                }
                Ok(Message::NewSequence(mut sequence)) => {
                    self.draw_seq(&mut sequence, rng, notes_buffer);
                    self.sequences.lock().unwrap().push(sequence);
                }
                Ok(Message::EditSequence(a, mut sequence)) => {
                    self.regen_seq(&mut sequence, rng, notes_buffer);
                    let len = {
                        let mut seqs = self.sequences.lock().unwrap();
                        seqs[a] = sequence;
                        seqs.len()
                    };
                    (a + 1..len).for_each(|k| self.regen_seq_at(k, rng, notes_buffer));
                }
                Ok(Message::DeleteSequence(a)) => {
                    let tk = { self.sequences.lock().unwrap()[a].token.clone() };
                    self.remove_seq(tk);
                    self.sequences.lock().unwrap().remove(a);
                }
                Ok(Message::CloneSequence(a, new_token)) => {
                    let mut sequence = { self.sequences.lock().unwrap()[a].clone() };
                    sequence.token = new_token;
                    self.draw_seq(&mut sequence, rng, notes_buffer);
                    self.sequences.lock().unwrap().push(sequence);
                }
                Ok(Message::SwapSequences(a, b)) => {
                    self.sequences.lock().unwrap().swap(a, b);
                    self.regen_seq_at(a, rng, notes_buffer);
                    self.regen_seq_at(b, rng, notes_buffer);
                    let len = self.sequences.lock().unwrap().len();
                    (a.max(b) + 1..len).for_each(|k| self.regen_seq_at(k, rng, notes_buffer));
                }
                Err(TryRecvError::Empty) => break, // no more messages this tick
                Err(TryRecvError::Disconnected) => {
                    // Sender dropped;
                    break;
                }
            }
        }
    }
    pub fn run_thread(mut self, running_sched: Arc<AtomicBool>) -> JoinHandle<()> {
        std::thread::spawn(move || {
            let rng = rand::thread_rng();
            self.run_loop(running_sched, rng)
        })
    }

    fn draw_seq(&self, seq: &mut Sequence, rng: &mut ThreadRng, out: &mut Vec<(usize, Vec<Note>)>) {
        let seq_start = (self.sched_start / seq.loop_len).floor() * seq.loop_len;
        seq.draw(out, rng, seq_start);
        seq.not_generate_until = Some(seq_start + seq.t_min + seq.loop_len - 0.1);
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
