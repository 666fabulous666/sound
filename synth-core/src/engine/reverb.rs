use crate::engine::score::DelayTapSeconds;

struct RingBuff<const N: usize> {
    data: Vec<f64>,
    head: usize,
}

impl<const N: usize> Default for RingBuff<N> {
    fn default() -> Self {
        Self {
            data: [0.0; N].to_vec(),
            head: 0,
        }
    }
}

impl<const N: usize> RingBuff<N> {
    fn backward(&self, n: usize) -> f64 {
        if N == 0 {
            return 0.0;
        }
        let step = n.min(N.saturating_sub(1));
        self.data[(self.head + (N - step)) % N]
    }
    fn push(&mut self, value: f64) {
        self.head = (self.head + 1) % N;
        self.data[self.head] = value;
    }
}

pub struct Reverb<const B: usize> {
    // delays: Vec<f64>,
    buffer: RingBuff<B>,
    dry_factor: f64,
    wet_factor: f64,
    sample_rate: f64,
}

impl<const B: usize> Reverb<B> {
    pub fn new(dry_factor: f64, wet_factor: f64, sample_rate: f64) -> Self {
        Self {
            buffer: RingBuff::default(),
            dry_factor,
            wet_factor,
            sample_rate,
        }
    }

    pub fn process(&mut self, dry: f64, delays_seconds: &[DelayTapSeconds]) -> f64 {
        if delays_seconds.is_empty() {
            self.buffer.push(dry);
            return dry;
        }

        let wet_gain: Vec<(usize, f64)> = delays_seconds
            .iter()
            .map(|tap| {
                let wet = (tap.weight * self.wet_factor).clamp(0.0, 1.0);
                (tap.steps(self.sample_rate), wet)
            })
            .collect();

        let total_wet: f64 = wet_gain.iter().map(|(_, w)| *w).sum::<f64>().min(1.0);
        let dry_gain = (1.0 - total_wet) * self.dry_factor.clamp(0.0, 1.0);
        let mut output = dry_gain * dry;

        for (steps, wet) in wet_gain {
            output += wet * self.buffer.backward(steps);
        }

        self.buffer.push(output);

        output
    }
}
