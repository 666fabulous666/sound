use serde::{Deserialize, Serialize};

use crate::time_freq::{Freq, Time};

/// A time-dependent parameter that combines relaxation and sinusoidal modulation.
///
/// This provides a unified way to control parameters that vary over time:
/// - Relaxation: exponential transition from start to end value
/// - LFO: sinusoidal modulation that can be synced to global clock or note time
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct TimeVarying {
    /// Base value (multiplied by relaxation factor)
    pub base: f64,
    /// Relaxation parameters (start -> end transition)
    pub relaxation: Relaxation,
    /// Low-frequency oscillator for sinusoidal modulation
    pub lfo: Lfo,
}

/// Exponential relaxation from a start value to an end value.
///
/// The formula is: end + (start - end) * exp(-rate * normalized_time)
/// where normalized_time is in [0, 1] representing the note duration.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Relaxation {
    /// Starting multiplier for the base value
    pub start: f64,
    /// Ending multiplier for the base value
    pub end: f64,
    /// Decay rate (higher = faster transition)
    pub rate: f64,
}

/// Low-frequency oscillator for sinusoidal modulation.
///
/// Adds a sinusoidal component: magnitude * sin(2π * frequency * time)
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Lfo {
    /// Amplitude of the oscillation
    pub magnitude: f64,
    /// Frequency of the oscillation
    pub frequency: Freq,
    /// If true, use global clock time; if false, use note time
    pub sync_with_clock: bool,
}

impl Default for TimeVarying {
    fn default() -> Self {
        Self {
            base: 1.0,
            relaxation: Relaxation::default(),
            lfo: Lfo::default(),
        }
    }
}

impl Default for Relaxation {
    fn default() -> Self {
        Self {
            start: 1.0,
            end: 1.0,
            rate: 1.0,
        }
    }
}

impl Default for Lfo {
    fn default() -> Self {
        Self {
            magnitude: 0.0,
            frequency: Freq(0.5),
            sync_with_clock: false,
        }
    }
}

impl Relaxation {
    /// Compute the relaxation factor at a given normalized time [0, 1].
    pub fn factor(&self, normalized_time: f64) -> f64 {
        let alpha = normalized_time.clamp(0.0, 1.0);
        let exp = (-self.rate.max(0.0) * alpha).exp();
        self.end + (self.start - self.end) * exp
    }
}

impl Lfo {
    /// Compute the LFO contribution at a given reference time.
    pub fn contribution(&self, reference_time: Time) -> f64 {
        if self.magnitude == 0.0 {
            0.0
        } else {
            self.magnitude * (self.frequency.phase(reference_time)).sin()
        }
    }
}

impl TimeVarying {
    /// Evaluate the time-varying parameter at a specific point in time.
    ///
    /// # Arguments
    /// * `note_time` - Time since the note started
    /// * `global_time` - Global clock time
    /// * `duration` - Total note duration
    ///
    /// # Returns
    /// The computed value: base * relaxation_factor + lfo_contribution
    pub fn evaluate(&self, note_time: Time, global_time: Time, duration: Time) -> f64 {
        let normalized = if duration.as_secs() <= 0.0 {
            0.0
        } else {
            (note_time / duration).clamp(0.0, 1.0)
        };

        let relax_factor = self.relaxation.factor(normalized);

        let lfo_time = if self.lfo.sync_with_clock {
            global_time
        } else {
            note_time
        };

        let mut result = self.base * relax_factor + self.lfo.contribution(lfo_time);
        if !result.is_finite() {
            result = 0.0;
        }
        result.max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relaxation_constant() {
        let relax = Relaxation {
            start: 1.0,
            end: 1.0,
            rate: 1.0,
        };
        assert_eq!(relax.factor(0.0), 1.0);
        assert_eq!(relax.factor(0.5), 1.0);
        assert_eq!(relax.factor(1.0), 1.0);
    }

    #[test]
    fn test_relaxation_decay() {
        let relax = Relaxation {
            start: 2.0,
            end: 1.0,
            rate: 5.0,
        };
        // At t=0, should be close to start
        assert!((relax.factor(0.0) - 2.0).abs() < 0.01);
        // At t=1, should be close to end
        assert!((relax.factor(1.0) - 1.0).abs() < 0.01);
        // At t=0.5, should be between start and end
        let mid = relax.factor(0.5);
        assert!(mid > 1.0 && mid < 2.0);
    }

    #[test]
    fn test_lfo_zero_magnitude() {
        let lfo = Lfo {
            magnitude: 0.0,
            frequency: Freq(1.0),
            sync_with_clock: false,
        };
        assert_eq!(lfo.contribution(Time(0.0)), 0.0);
        assert_eq!(lfo.contribution(Time(1.0)), 0.0);
    }

    #[test]
    fn test_time_varying_evaluate() {
        let tv = TimeVarying {
            base: 2.0,
            relaxation: Relaxation {
                start: 1.0,
                end: 1.0,
                rate: 0.0,
            },
            lfo: Lfo {
                magnitude: 0.0,
                frequency: Freq(1.0),
                sync_with_clock: false,
            },
        };
        // With constant relaxation (1.0) and no LFO, result should be base * 1.0 = 2.0
        assert_eq!(
            tv.evaluate(Time(0.0), Time(0.0), Time(1.0)),
            2.0
        );
    }
}
