use itertools::Itertools;

struct RingBuff<const N: usize> {
    data: [f32; N],
    head: usize,
}

impl<const N: usize> Default for RingBuff<N> {
    fn default() -> Self {
        Self {
            data: [0.0; N],
            head: 0,
        }
    }
}

impl<const N: usize> RingBuff<N> {
    fn backward(&self, n: usize) -> f32 {
        self.data[(self.head + (N - n)) % N]
    }
    fn push(&mut self, value: f32) {
        self.head = (self.head + 1) % N;
        self.data[self.head] = value;
    }
}

pub struct Reverb {
    delays: Vec<(f32, usize)>, // WARNING: for simplicity, we work directily with the samples so it is sample_rate dependant.
    buffer: RingBuff<44100>,
    dry_factor: f32,
}

impl Reverb {
    pub fn new(dry_factor: f32, wet_factor: f32, delays: &[usize]) -> Self {
        let a = wet_factor / delays.len() as f32;

        Self {
            delays: delays.iter().map(|&d| (a, d)).collect_vec(),
            buffer: RingBuff::default(),
            dry_factor,
        }
    }

    pub fn process(&mut self, dry: f32) -> f32 {
        let mut output = self.dry_factor * dry;

        for &(a, d) in &self.delays {
            output += a * self.buffer.backward(d);
        }

        // Apply smoothing with internal memory
        // output = 0.5 * output + 0.5 * self.buffer.backward(1);
        self.buffer.push(output);

        output
    }
}
