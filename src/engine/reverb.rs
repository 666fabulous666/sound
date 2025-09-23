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
        self.data[(self.head + (N - n)) % N]
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

    pub fn process(&mut self, dry: f64, delays: &[f64]) -> f64 {
        let mut output = self.dry_factor * dry;

        let a = self.wet_factor / delays.len() as f64;
        for d in delays.iter() {
            output += a * self
                .buffer
                .backward((*d / 1000.0 * self.sample_rate) as usize);
        }

        self.buffer.push(output);

        output
    }
}
