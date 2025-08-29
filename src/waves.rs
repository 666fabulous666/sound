use serde::{Deserialize, Serialize};
// pub mod basics;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum WaveType {
    Sine,
    Square,
    Triangle,
    Sawtooth,
    // DistOrg,
    // Custom2,
    // Droplet,
    // DropletOct,
    // HiHat,
    // Kick,
    // Snare,
    // Ride,
    // Mute,
    // Xylo,
}

impl ToString for &WaveType {
    fn to_string(&self) -> String {
        match self {
            WaveType::Sine => "Sine".into(),
            WaveType::Square => "Square".into(),
            WaveType::Triangle => "Triangle".into(),
            WaveType::Sawtooth => "Sawtooth".into(),
            // WaveType::DistOrg => "DistOrg".into(),
            // WaveType::Custom2 => "Custom2".into(),
            // WaveType::Droplet => "Droplet".into(),
            // WaveType::DropletOct => "DropletOct".into(),
            // WaveType::HiHat => "HiHat".into(),
            // WaveType::Kick => "Kick".into(),
            // WaveType::Snare => "Snare".into(),
            // WaveType::Ride => "Ride".into(),
            // WaveType::Mute => "Mute".into(),
            // WaveType::Xylo => "Xylo".into(),
        }
    }
}
