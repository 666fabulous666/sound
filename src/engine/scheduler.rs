use crate::{
    engine::notes::{Note, Sequence},
    GENERATE_EARLY, SCHEDULER_STEP,
};
use rand::{rngs::ThreadRng, thread_rng};
use std::sync::{Arc, Mutex};

pub enum Message {
    NewScore,
    NewSequence(Sequence),
    EditSequence(usize, Sequence),
    DeleteSequence(usize),
    CloneSequence(usize, usize),
    SwapSequences(usize, usize),
}

pub struct Scheduler {
    pub notes: Vec<(usize, Vec<Note>)>,
    pub sequences: Vec<Sequence>,
    pub clock: Arc<Mutex<f64>>, // TODO: see if it can be an Atomic, maybe using ticks instead of secs
    pub sched_start: f64,
    rng: ThreadRng,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(Arc::new(Mutex::new(0.0)))
    }
}

impl Scheduler {
    pub fn new(clock: Arc<Mutex<f64>>) -> Self {
        // let notes: Arc<Mutex<Vec<(usize, Vec<Note>)>>> = Arc::new(Mutex::new(Vec::new()));
        // let sequences: Arc<Mutex<Vec<Sequence>>> = Arc::new(Mutex::new(Vec::new()));

        Self {
            notes: Vec::new(),
            sequences: Vec::new(),
            clock,
            sched_start: 0.0,
            rng: thread_rng(),
        }
    }

    pub fn now(&self) -> Arc<Mutex<f64>> {
        self.clock.clone()
    }

    // pub fn notes(&self) -> &Vec<(usize, Vec<Note>)> {
    //     self.notes.clone()
    // }

    // pub fn sequences(&self) -> Arc<Mutex<Vec<Sequence>>> {
    //     self.sequences.clone()
    // }
    // fn run_loop(&mut self, running_sched: Arc<AtomicBool>, mut rng: ThreadRng) {
    //     while running_sched.load(Ordering::Relaxed) {
    //         self.run_once(&mut rng);
    //         self.sleep_for(5e-2);
    //     }
    // }

    pub fn generate_notes(&mut self) {
        let now = self.now().lock().unwrap().clone();
        let mut notes_buffer = Vec::<(usize, Vec<Note>)>::new(); // FIXME: should not need it
        for seq in self.sequences.clone().iter() {
            // TODO: don't clone
            if seq.not_generate_until.is_none()
                || seq
                    .not_generate_until
                    .as_ref()
                    .is_some_and(|until| now >= *until)
            {
                self.draw_seq_at(seq.token, &mut notes_buffer);
                println!("generate token {}, {} notes", seq.token, notes_buffer.len());
            }
        }
        self.notes.extend(notes_buffer);
        self.sched_start += SCHEDULER_STEP;
    }

    // pub fn run_once(&mut self) {
    //     // if self.sched_start < self.now() + SCHEDULER_WAKE_EARLY {
    //     while self.sched_start < now + SCHEDULER_WAKE_EARLY {
    //         // WARNING: should it be a while?
    //         let mut notes_buffer = Vec::<(usize, Vec<Note>)>::new(); // FIXME: should'n it see other notes???

    //         // ---- handle inbound messages (drain channel) ----
    //         // self.drain_messages(rng, &mut notes_buffer);

    //         // ---- generate notes from sequences that need it ----
    //         {
    //             for seq in self.sequences.clone().iter() {
    //                 // TODO: don't clone
    //                 if seq.not_generate_until.is_none()
    //                     || seq
    //                         .not_generate_until
    //                         .as_ref()
    //                         .is_some_and(|until| now >= *until)
    //                 {
    //                     self.draw_seq_at(seq.token, &mut notes_buffer);
    //                 }
    //             }
    //         }

    //         // Append new notes
    //         {
    //             self.notes.extend(notes_buffer);
    //         }
    //         self.sched_start += SCHEDULER_STEP;
    //     }
    // }

    // fn sleep_for(&mut self, dt: f64) {
    //     let sched_start_increase = dt;
    //     self.sched_start += sched_start_increase;
    //     let wake_early = sched_start_increase * 2.0; // WARINIG: isn't it supposed to be smaller than sched_start_increase?
    //     if self.sched_start > self.now() {
    //         let sleep_s = (self.sched_start - self.now() - wake_early).max(0.0);
    //         std::thread::sleep(Duration::from_secs_f64(sleep_s));
    //     }
    // }

    // fn drain_messages(&mut self, rng: &mut ThreadRng, notes_buffer: &mut Vec<(usize, Vec<Note>)>) {
    //     loop {
    //         match self.receiver.try_recv() {
    //             Ok(Message::NewScore) => {
    //                 self.sequences.lock().unwrap().clear();
    //                 self.notes.lock().unwrap().clear();
    //             }
    //             Ok(Message::NewSequence(mut sequence)) => {
    //                 self.draw_seq(&mut sequence, rng, notes_buffer);
    //                 self.sequences.lock().unwrap().push(sequence);
    //             }
    //             Ok(Message::EditSequence(a, mut sequence)) => {
    //                 self.regen_seq(&mut sequence, rng, notes_buffer);
    //                 let len = {
    //                     let mut seqs = self.sequences.lock().unwrap();
    //                     seqs[a] = sequence;
    //                     seqs.len()
    //                 };
    //                 (a + 1..len).for_each(|k| self.regen_seq_at(k, rng, notes_buffer));
    //             }
    //             Ok(Message::DeleteSequence(a)) => {
    //                 let tk = { self.sequences.lock().unwrap()[a].token.clone() };
    //                 self.remove_seq(tk);
    //                 self.sequences.lock().unwrap().remove(a);
    //             }
    //             Ok(Message::CloneSequence(a, new_token)) => {
    //                 let mut sequence = { self.sequences.lock().unwrap()[a].clone() };
    //                 sequence.token = new_token;
    //                 self.draw_seq(&mut sequence, rng, notes_buffer);
    //                 self.sequences.lock().unwrap().push(sequence);
    //             }
    //             Ok(Message::SwapSequences(a, b)) => {
    //                 self.sequences.lock().unwrap().swap(a, b);
    //                 self.regen_seq_at(a, rng, notes_buffer);
    //                 self.regen_seq_at(b, rng, notes_buffer);
    //                 let len = self.sequences.lock().unwrap().len();
    //                 (a.max(b) + 1..len).for_each(|k| self.regen_seq_at(k, rng, notes_buffer));
    //             }
    //             Err(TryRecvError::Empty) => break, // no more messages this tick
    //             Err(TryRecvError::Disconnected) => {
    //                 // Sender dropped;
    //                 break;
    //             }
    //         }
    //     }
    // }
    pub fn new_score(&mut self) {
        self.sequences.clear();
        self.notes.clear();
    }
    pub fn new_seq(&mut self, mut sequence: Sequence, notes_buffer: &mut Vec<(usize, Vec<Note>)>) {
        self.draw_seq(&mut sequence, notes_buffer);
        self.sequences.push(sequence);
    }
    pub fn edit_seq(
        &mut self,
        mut sequence: Sequence,
        notes_buffer: &mut Vec<(usize, Vec<Note>)>,
        a: usize,
    ) {
        self.regen_seq(&mut sequence, notes_buffer);
        self.sequences[a] = sequence;
        let len = self.sequences.len();
        (a + 1..len).for_each(|k| self.regen_seq_at(k, notes_buffer));
    }
    pub fn del_seq(&mut self, a: usize) {
        let tk = self.sequences[a].token;
        self.remove_seq(tk);
        self.sequences.remove(a);
    }
    pub fn clone_seq(
        &mut self,
        a: usize,
        new_token: usize,
        notes_buffer: &mut Vec<(usize, Vec<Note>)>,
    ) {
        let mut sequence = self.sequences[a].clone();
        sequence.token = new_token;
        self.draw_seq(&mut sequence, notes_buffer);
        self.sequences.push(sequence);
    }
    pub fn swap_seqs(&mut self, a: usize, b: usize, notes_buffer: &mut Vec<(usize, Vec<Note>)>) {
        self.sequences.swap(a, b);
        self.regen_seq_at(a, notes_buffer);
        self.regen_seq_at(b, notes_buffer);
        let len = self.sequences.len();
        (a.max(b) + 1..len).for_each(|k| self.regen_seq_at(k, notes_buffer));
    }
    // pub fn run_thread(mut self, running_sched: Arc<AtomicBool>) -> JoinHandle<()> {
    //     std::thread::spawn(move || {
    //         let rng = rand::thread_rng();
    //         self.run_loop(running_sched, rng)
    //     })
    // }

    fn draw_seq(&mut self, seq: &mut Sequence, notes_buffer: &mut Vec<(usize, Vec<Note>)>) {
        let seq_start = (self.sched_start / seq.loop_len).floor() * seq.loop_len;
        seq.draw(notes_buffer, &mut self.rng, seq_start);
        seq.not_generate_until =
            Some(seq_start + seq.t_min + seq.repeat as f64 * seq.loop_len - GENERATE_EARLY);
    }
    fn draw_seq_at(&mut self, a: usize, out: &mut Vec<(usize, Vec<Note>)>) {
        let seq = &mut self.sequences[a];
        let seq_start = (self.sched_start / seq.loop_len).floor() * seq.loop_len;
        seq.draw(out, &mut self.rng, seq_start);
        seq.not_generate_until =
            Some(seq_start + seq.t_min + seq.repeat as f64 * seq.loop_len - GENERATE_EARLY);
    }
    fn remove_seq(&mut self, tk: usize) {
        self.notes.retain(|(token, _)| *token != tk);
    }
    fn regen_seq(&mut self, seq: &mut Sequence, notes_buffer: &mut Vec<(usize, Vec<Note>)>) {
        self.remove_seq(seq.token);
        self.draw_seq(seq, notes_buffer);
    }
    fn regen_seq_at(&mut self, a: usize, notes_buffer: &mut Vec<(usize, Vec<Note>)>) {
        self.remove_seq(a);
        self.draw_seq_at(a, notes_buffer);
        // let s = &mut self.sequences[a];
        // self.regen_seq(s, rng, notes_buffer);
    }
}
