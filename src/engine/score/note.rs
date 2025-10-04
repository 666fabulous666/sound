use serde::Deserialize;

use super::Interval;

use crate::time_freq::Time;

#[derive(Deserialize, Clone)]
pub struct Note {
    pub time: Time,
    pub duration: Time,
    pub interval: Interval,
    pub volume: f64,
}
