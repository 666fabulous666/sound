use itertools::Itertools;

struct RingBuff<const N: usize> {
    data: [f64; N],
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
    fn backward(&self, n: usize) -> f64 {
        self.data[(self.head + (N - n)) % N]
    }
    fn push(&mut self, value: f64) {
        self.head = (self.head + 1) % N;
        self.data[self.head] = value;
    }
}

pub struct Reverb<const B: usize> {
    delays: Vec<(f64, usize)>, // WARNING: for simplicity, we work directily with the samples so it is sample_rate dependant.
    buffer: RingBuff<B>,
    dry_factor: f64,
}

impl<const B: usize> Reverb<B> {
    pub fn new(dry_factor: f64, wet_factor: f64, delays: &[usize]) -> Self {
        let a = wet_factor / delays.len() as f64;

        Self {
            delays: delays.iter().map(|&d| (a, d)).collect_vec(),
            buffer: RingBuff::default(),
            dry_factor,
        }
    }

    pub fn process(&mut self, dry: f64) -> f64 {
        let mut output = self.dry_factor * dry;

        for &(a, d) in &self.delays {
            output += a * self.buffer.backward(d);
        }

        self.buffer.push(output);

        output
    }
}
