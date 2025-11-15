use core::fmt;
use core::iter::Sum;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Time(pub f64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Freq(pub f64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Beat(pub f64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Tempo(pub f64);

impl Time {
    pub const fn new(seconds: f64) -> Self {
        Self(seconds)
    }
    pub const fn as_secs(self) -> f64 {
        self.0
    }
    pub const fn as_secs_mut(&mut self) -> &mut f64 {
        &mut self.0
    }

    pub fn to_freq(self) -> Option<Freq> {
        if self.0 == 0.0 {
            None
        } else {
            Some(Freq(1.0 / self.0))
        }
    }
}

impl Freq {
    pub const fn new(hz: f64) -> Self {
        Self(hz)
    }
    pub const fn as_hz(self) -> f64 {
        self.0
    }

    pub fn period(self) -> Option<Time> {
        if self.0 == 0.0 {
            None
        } else {
            Some(Time(1.0 / self.0))
        }
    }

    pub fn phase(self, t: Time) -> f64 {
        core::f64::consts::TAU * self.0 * t.0
    }
    pub fn wrap_phase(self, t: Time) -> f64 {
        let phi = self.phase(t);
        let two_pi = core::f64::consts::TAU;
        let r = phi % two_pi;
        if r < 0.0 {
            r + two_pi
        } else {
            r
        }
    }
}

impl Beat {
    pub const fn new(beats: f64) -> Self {
        Self(beats)
    }
    pub const fn as_beats(self) -> f64 {
        self.0
    }
    pub fn max(self, other: Beat) -> Beat {
        if self > other {
            self
        } else {
            other
        }
    }
    pub fn min(self, other: Beat) -> Beat {
        if self < other {
            self
        } else {
            other
        }
    }
}

impl Tempo {
    pub fn new(bpm: f64) -> Self {
        Self(bpm.max(f64::MIN_POSITIVE))
    }
    pub fn beats_per_minute(self) -> f64 {
        self.0
    }
    pub fn seconds_per_beat(self) -> f64 {
        60.0 / self.0.max(f64::MIN_POSITIVE)
    }
    pub fn beats_to_time(self, beats: Beat) -> Time {
        Time(beats.as_beats() * self.seconds_per_beat())
    }
    pub fn time_to_beats(self, time: Time) -> Beat {
        Beat(time.as_secs() / self.seconds_per_beat())
    }
    pub fn set_bpm(&mut self, bpm: f64) {
        self.0 = bpm.max(f64::MIN_POSITIVE);
    }
}

impl Default for Tempo {
    fn default() -> Self {
        Tempo(60.0)
    }
}

// Time ± Time
impl Add for Time {
    type Output = Time;
    fn add(self, rhs: Time) -> Time {
        Time(self.0 + rhs.0)
    }
}
impl Sub for Time {
    type Output = Time;
    fn sub(self, rhs: Time) -> Time {
        Time(self.0 - rhs.0)
    }
}
impl AddAssign for Time {
    fn add_assign(&mut self, rhs: Time) {
        self.0 += rhs.0;
    }
}
impl SubAssign for Time {
    fn sub_assign(&mut self, rhs: Time) {
        self.0 -= rhs.0;
    }
}

// Beat ± Beat
impl Add for Beat {
    type Output = Beat;
    fn add(self, rhs: Beat) -> Beat {
        Beat(self.0 + rhs.0)
    }
}
impl Sub for Beat {
    type Output = Beat;
    fn sub(self, rhs: Beat) -> Beat {
        Beat(self.0 - rhs.0)
    }
}
impl AddAssign for Beat {
    fn add_assign(&mut self, rhs: Beat) {
        self.0 += rhs.0;
    }
}
impl SubAssign for Beat {
    fn sub_assign(&mut self, rhs: Beat) {
        self.0 -= rhs.0;
    }
}

// Freq ± Freq
impl Add for Freq {
    type Output = Freq;
    fn add(self, rhs: Freq) -> Freq {
        Freq(self.0 + rhs.0)
    }
}
impl Sub for Freq {
    type Output = Freq;
    fn sub(self, rhs: Freq) -> Freq {
        Freq(self.0 - rhs.0)
    }
}
impl AddAssign for Freq {
    fn add_assign(&mut self, rhs: Freq) {
        self.0 += rhs.0;
    }
}
impl SubAssign for Freq {
    fn sub_assign(&mut self, rhs: Freq) {
        self.0 -= rhs.0;
    }
}

// Scalar ops
impl Mul<f64> for Time {
    type Output = Time;
    fn mul(self, s: f64) -> Time {
        Time(self.0 * s)
    }
}
impl Div<f64> for Time {
    type Output = Time;
    fn div(self, s: f64) -> Time {
        Time(self.0 / s)
    }
}
impl MulAssign<f64> for Time {
    fn mul_assign(&mut self, s: f64) {
        self.0 *= s;
    }
}
impl DivAssign<f64> for Time {
    fn div_assign(&mut self, s: f64) {
        self.0 /= s;
    }
}

impl Mul<f64> for Beat {
    type Output = Beat;
    fn mul(self, s: f64) -> Beat {
        Beat(self.0 * s)
    }
}
impl Div<f64> for Beat {
    type Output = Beat;
    fn div(self, s: f64) -> Beat {
        Beat(self.0 / s)
    }
}
impl MulAssign<f64> for Beat {
    fn mul_assign(&mut self, s: f64) {
        self.0 *= s;
    }
}
impl DivAssign<f64> for Beat {
    fn div_assign(&mut self, s: f64) {
        self.0 /= s;
    }
}

impl Mul<f64> for Freq {
    type Output = Freq;
    fn mul(self, s: f64) -> Freq {
        Freq(self.0 * s)
    }
}
impl Div<f64> for Freq {
    type Output = Freq;
    fn div(self, s: f64) -> Freq {
        Freq(self.0 / s)
    }
}
impl MulAssign<f64> for Freq {
    fn mul_assign(&mut self, s: f64) {
        self.0 *= s;
    }
}
impl DivAssign<f64> for Freq {
    fn div_assign(&mut self, s: f64) {
        self.0 /= s;
    }
}

// Ratios
impl Div<Time> for Time {
    type Output = f64;
    fn div(self, rhs: Time) -> f64 {
        self.0 / rhs.0
    }
}
impl Div<Freq> for Freq {
    type Output = f64;
    fn div(self, rhs: Freq) -> f64 {
        self.0 / rhs.0
    }
}
impl Div<Beat> for Beat {
    type Output = f64;
    fn div(self, rhs: Beat) -> f64 {
        self.0 / rhs.0
    }
}

// Cross products
impl Mul<Freq> for Time {
    type Output = f64;
    fn mul(self, rhs: Freq) -> f64 {
        self.0 * rhs.0
    }
}
impl Mul<Time> for Freq {
    type Output = f64;
    fn mul(self, rhs: Time) -> f64 {
        self.0 * rhs.0
    }
}
impl Mul<Tempo> for Beat {
    type Output = Time;
    fn mul(self, tempo: Tempo) -> Time {
        tempo.beats_to_time(self)
    }
}
impl Mul<Beat> for Tempo {
    type Output = Time;
    fn mul(self, beat: Beat) -> Time {
        self.beats_to_time(beat)
    }
}

// Summation
impl Sum for Time {
    fn sum<I: Iterator<Item = Time>>(iter: I) -> Time {
        Time(iter.map(|t| t.0).sum())
    }
}
impl Sum for Freq {
    fn sum<I: Iterator<Item = Freq>>(iter: I) -> Freq {
        Freq(iter.map(|f| f.0).sum())
    }
}
impl Sum for Beat {
    fn sum<I: Iterator<Item = Beat>>(iter: I) -> Beat {
        Beat(iter.map(|b| b.0).sum())
    }
}

// Display
impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} s", self.0)
    }
}
impl fmt::Display for Freq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz", self.0)
    }
}
impl fmt::Display for Beat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} beats", self.0)
    }
}
impl fmt::Display for Tempo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} BPM", self.0)
    }
}

// Optional egui integration for GUI support
#[cfg(feature = "egui-support")]
use egui::emath::Numeric;

#[cfg(feature = "egui-support")]
impl Numeric for Time {
    const INTEGRAL: bool = false;
    const MIN: Self = Time(f64::MIN);
    const MAX: Self = Time(f64::MAX);

    #[inline]
    fn to_f64(self) -> f64 {
        self.0
    }

    #[inline]
    fn from_f64(num: f64) -> Self {
        Time(num)
    }
}

#[cfg(feature = "egui-support")]
impl Numeric for Freq {
    const INTEGRAL: bool = false;
    const MIN: Self = Freq(f64::MIN);
    const MAX: Self = Freq(f64::MAX);

    #[inline]
    fn to_f64(self) -> f64 {
        self.0
    }

    #[inline]
    fn from_f64(num: f64) -> Self {
        Freq(num)
    }
}

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

impl Eq for Time {}
impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for Time {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the raw bits of the f64 for deterministic hashing
        self.0.to_bits().hash(state);
    }
}

impl Eq for Freq {}
impl Ord for Freq {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for Freq {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the raw bits of the f64 for deterministic hashing
        self.0.to_bits().hash(state);
    }
}

use core::ops::{Rem, RemAssign};

// ---- truncating remainder (like f64 % f64) ----
impl Rem<Time> for Time {
    type Output = Time;
    #[inline]
    fn rem(self, rhs: Time) -> Time {
        Time(self.0 % rhs.0)
    }
}
impl RemAssign<Time> for Time {
    #[inline]
    fn rem_assign(&mut self, rhs: Time) {
        self.0 %= rhs.0;
    }
}

impl Rem<Freq> for Freq {
    type Output = Freq;
    #[inline]
    fn rem(self, rhs: Freq) -> Freq {
        Freq(self.0 % rhs.0)
    }
}
impl RemAssign<Freq> for Freq {
    #[inline]
    fn rem_assign(&mut self, rhs: Freq) {
        self.0 %= rhs.0;
    }
}

// ---- Euclidean remainder (always in [0, rhs) if rhs > 0) ----
impl Time {
    #[inline]
    pub fn rem_euclid(self, rhs: Time) -> Time {
        Time(self.0.rem_euclid(rhs.0))
    }
    #[inline]
    pub fn rem_euclid_assign(&mut self, rhs: Time) {
        self.0 = self.0.rem_euclid(rhs.0);
    }
}

impl Freq {
    #[inline]
    pub fn rem_euclid(self, rhs: Freq) -> Freq {
        Freq(self.0.rem_euclid(rhs.0))
    }
    #[inline]
    pub fn rem_euclid_assign(&mut self, rhs: Freq) {
        self.0 = self.0.rem_euclid(rhs.0);
    }
}

use core::ops::Neg;

impl Neg for Time {
    type Output = Time;
    #[inline]
    fn neg(self) -> Time {
        Time(-self.0)
    }
}

impl Neg for Freq {
    type Output = Freq;
    #[inline]
    fn neg(self) -> Freq {
        Freq(-self.0)
    }
}

pub trait DivByFreq {
    fn div_by(self, f: Freq) -> Time;
}
pub trait DivByTime {
    fn div_by(self, t: Time) -> Freq;
}

impl DivByFreq for f64 {
    #[inline]
    fn div_by(self, f: Freq) -> Time {
        Time(self / f.0)
    }
}
impl DivByTime for f64 {
    #[inline]
    fn div_by(self, t: Time) -> Freq {
        Freq(self / t.0)
    }
}
