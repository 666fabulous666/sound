use serde::Deserialize;
pub mod basics;

#[derive(Deserialize, Clone, Copy)]
pub enum WaveType {
    Sine,
    Square,
    Triangle,
    Sawtooth,
    DistOrg,
    Custom2,
    Droplet,
    DropletOct,
    HiHat,
    Kick,
    Snare,
    Ride,
    Mute,
    Xylo,
}
