use std::fmt;
use std::num::NonZeroU16;

use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

use crate::time_freq::Time;

const MIN_VALUE: u16 = 1;
const MAX_VALUE: u16 = 128;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TimeQuantum {
    numer: NonZeroU16,
    denom: NonZeroU16,
}

impl TimeQuantum {
    pub fn new(numer: NonZeroU16, denom: NonZeroU16) -> Self {
        Self { numer, denom }
    }

    pub fn from_raw(numer: u16, denom: u16) -> Result<Self, TimeQuantumError> {
        let numer = NonZeroU16::new(numer).ok_or(TimeQuantumError::ZeroNumerator)?;
        let denom = NonZeroU16::new(denom).ok_or(TimeQuantumError::ZeroDenominator)?;
        if numer.get() > MAX_VALUE || denom.get() > MAX_VALUE {
            return Err(TimeQuantumError::OutOfRange);
        }
        Ok(Self { numer, denom })
    }

    pub fn numerator(&self) -> u16 {
        self.numer.get()
    }

    pub fn denominator(&self) -> u16 {
        self.denom.get()
    }

    pub fn set_numerator(&mut self, numer: u16) -> Result<(), TimeQuantumError> {
        *self = Self::from_raw(numer, self.denominator())?;
        Ok(())
    }

    pub fn set_denominator(&mut self, denom: u16) -> Result<(), TimeQuantumError> {
        *self = Self::from_raw(self.numerator(), denom)?;
        Ok(())
    }

    pub fn ratio(&self) -> f64 {
        self.numerator() as f64 / self.denominator() as f64
    }

    pub fn step_duration(&self) -> Time {
        Time(self.ratio())
    }

    pub fn clamp_component(value: u16) -> u16 {
        value.clamp(MIN_VALUE, MAX_VALUE)
    }
}

impl Default for TimeQuantum {
    fn default() -> Self {
        TimeQuantum::from_raw(MIN_VALUE, 8).expect("default time quantum must be valid")
    }
}

impl Serialize for TimeQuantum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (self.numerator(), self.denominator()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TimeQuantum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (numer, denom) = <(u16, u16)>::deserialize(deserializer)?;
        TimeQuantum::from_raw(numer, denom).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TimeQuantumError {
    ZeroNumerator,
    ZeroDenominator,
    OutOfRange,
}

impl fmt::Display for TimeQuantumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeQuantumError::ZeroNumerator => write!(f, "time quantum numerator must be > 0"),
            TimeQuantumError::ZeroDenominator => write!(f, "time quantum denominator must be > 0"),
            TimeQuantumError::OutOfRange => write!(
                f,
                "time quantum components must be within [{}, {}]",
                MIN_VALUE, MAX_VALUE
            ),
        }
    }
}

impl std::error::Error for TimeQuantumError {}
