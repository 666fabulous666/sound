//! Synth-core: Pure audio synthesis engine
//!
//! This library provides the core audio synthesis functionality without any GUI dependencies.
//! It can be used as a standalone library for audio generation, or integrated into GUI applications.

pub mod engine;
pub mod error;
pub mod recorder;
pub mod stream;
pub mod session;
pub mod time_freq;

use crate::time_freq::{Freq, Time};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

// Core audio constants
pub const DEFAULT_LOOP_LEN: Time = Time(4.0);
pub const NOTE_LINGER_TIME: Time = Time(12.0);
pub const F0: Freq = Freq(440.0);
pub const REVERB_BUFFER_LEN: usize = 44100;
pub const SCHEDULER_WAKE_EARLY: f64 = 0.1;
pub const GENERATE_EARLY: Time = Time(1e0);

/// Unique identifier for sequences in the score tree
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Token(pub usize);

impl Deref for Token {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Generator for unique tokens
pub struct TokenGen(pub usize);

impl TokenGen {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn next(&mut self) -> Token {
        self.0 += 1;
        Token(self.0)
    }
}

impl Default for TokenGen {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for notes
#[derive(
    Debug, Serialize, Deserialize, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Default,
)]
pub struct NoteId(pub u64);

impl NoteId {
    pub fn new(value: u64) -> Self {
        NoteId(value)
    }
}

/// Generator for unique note IDs
#[derive(Default)]
pub struct NoteIdGen(pub u64);

impl NoteIdGen {
    pub fn next(&mut self) -> NoteId {
        self.0 += 1;
        NoteId(self.0)
    }
}

/// Apply a function to the absolute value and restore the sign.
///
/// # Examples
/// ```
/// use synth_core::sign_f;
/// assert_eq!(sign_f(5.0, |x| x * x), 25.0);
/// assert_eq!(sign_f(-5.0, |x| x * x), -25.0);
/// ```
pub fn sign_f<T: num_traits::Signed>(arg: T, f: impl Fn(T) -> T) -> T {
    arg.signum() * f(arg.abs())
}

/// Compute a rescaling factor for envelope normalization.
///
/// Given attack time `a` and decay time `b`, computes a normalization factor
/// to ensure consistent perceived loudness regardless of envelope shape.
///
/// # Arguments
/// * `a` - Attack time (must be positive and finite)
/// * `b` - Decay time (must be positive and finite)
///
/// # Returns
/// A finite rescaling factor, or `1.0` if the computation would produce NaN/infinity.
///
/// # Examples
/// ```
/// use synth_core::rescale_factor;
/// let factor = rescale_factor(1.0, 1.0);
/// assert!(factor.is_finite());
/// assert!(factor > 0.0);
/// ```
pub fn rescale_factor(a: f64, b: f64) -> f64 {
    // Validate inputs
    if !a.is_finite() || !b.is_finite() || a <= 0.0 || b <= 0.0 {
        return 1.0; // Safe fallback
    }

    let denom = a + b;
    let result = (a.powf(a) * b.powf(b)) / denom.powf(denom);

    // Ensure result is finite
    if result.is_finite() && result > 0.0 {
        result
    } else {
        1.0 // Safe fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rescale_factor_normal_values() {
        let factor = rescale_factor(1.0, 1.0);
        assert!(
            factor.is_finite(),
            "rescale_factor(1.0, 1.0) should be finite"
        );
        assert!(factor > 0.0, "rescale_factor should be positive");

        let factor2 = rescale_factor(0.5, 2.0);
        assert!(factor2.is_finite());
        assert!(factor2 > 0.0);
    }

    #[test]
    fn test_rescale_factor_edge_cases() {
        // Very small values
        let factor = rescale_factor(0.01, 0.01);
        assert!(factor.is_finite());
        assert!(factor > 0.0);

        // Very large values
        let factor = rescale_factor(100.0, 100.0);
        assert!(factor.is_finite());
        assert!(factor > 0.0);

        // Asymmetric values
        let factor = rescale_factor(0.01, 100.0);
        assert!(factor.is_finite());
        assert!(factor > 0.0);
    }

    #[test]
    fn test_rescale_factor_invalid_inputs() {
        // NaN inputs should return 1.0
        assert_eq!(rescale_factor(f64::NAN, 1.0), 1.0);
        assert_eq!(rescale_factor(1.0, f64::NAN), 1.0);

        // Infinity inputs should return 1.0
        assert_eq!(rescale_factor(f64::INFINITY, 1.0), 1.0);
        assert_eq!(rescale_factor(1.0, f64::INFINITY), 1.0);

        // Zero or negative inputs should return 1.0
        assert_eq!(rescale_factor(0.0, 1.0), 1.0);
        assert_eq!(rescale_factor(-1.0, 1.0), 1.0);
        assert_eq!(rescale_factor(1.0, 0.0), 1.0);
        assert_eq!(rescale_factor(1.0, -1.0), 1.0);
    }

    #[test]
    fn test_token_gen() {
        let mut gen = TokenGen::new();
        let t1 = gen.next();
        let t2 = gen.next();
        let t3 = gen.next();

        assert_ne!(t1, t2);
        assert_ne!(t2, t3);
        assert_eq!(*t1, 1);
        assert_eq!(*t2, 2);
        assert_eq!(*t3, 3);
    }

    #[test]
    fn test_sign_f() {
        assert_eq!(sign_f(5.0, |x| x * x), 25.0);
        assert_eq!(sign_f(-5.0, |x| x * x), -25.0);
        assert_eq!(sign_f(0.0, |x| x * x), 0.0);
    }
}
