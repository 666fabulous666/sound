//! Audio synthesis engine.
//!
//! This module contains the core audio synthesis components:
//! - `score`: Musical score representation and generation
//! - `waves`: Waveform generation functions
//! - `reverb`: Reverb audio effect
//! - `time_varying`: Time-dependent parameter modulation

pub mod reverb;
pub mod score;
pub mod time_varying;
pub mod waves;
