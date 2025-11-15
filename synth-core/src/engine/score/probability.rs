use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Floating-point probability constrained to [0.0, 1.0].
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Probability(f64);

impl Probability {
    pub const fn certainty() -> Self {
        Self(1.0)
    }

    pub const fn impossibility() -> Self {
        Self(0.0)
    }

    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    pub fn as_f64(self) -> f64 {
        self.0
    }

    pub fn set(&mut self, value: f64) {
        self.0 = value.clamp(0.0, 1.0);
    }
}

impl Default for Probability {
    fn default() -> Self {
        Self::certainty()
    }
}

impl From<Probability> for f64 {
    fn from(value: Probability) -> Self {
        value.as_f64()
    }
}

impl Serialize for Probability {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.as_f64())
    }
}

impl<'de> Deserialize<'de> for Probability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        Ok(Probability::new(value))
    }
}
