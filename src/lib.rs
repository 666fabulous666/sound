pub mod app;
pub mod engine;
pub mod stream;

pub const DEFAULT_LOOP_LEN: f64 = 8.0;
pub const NOTE_LINGER_TIME: f64 = 12.0;
pub const F0: f64 = 440.0;
pub const REVERB_BUFFER_LEN: usize = 100_000;
pub const SCHEDULER_STEP: f64 = 1e-2;
pub const SCHEDULER_WAKE_EARLY: f64 = 1.0;
