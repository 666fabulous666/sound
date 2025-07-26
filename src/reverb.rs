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

// const DELAYS: [usize; 64] = [
//     4409, 4651, 4877, 5009, 5231, 5471, 5653, 5821, 6037, 6203, 6427, 6553, 6733, 6899, 7121, 7297,
//     7507, 7681, 7873, 8011, 8231, 8423, 8609, 8803, 9001, 9203, 9391, 9587, 9733, 9941, 10103,
//     10271, 10427, 10613, 10771, 10937, 11113, 11287, 11467, 11617, 11789, 11953, 12109, 12263,
//     12421, 12577, 12713, 12889, 13043, 13217, 13367, 13523, 13691, 13841, 13999, 14153, 14321,
//     14489, 14621, 14699, 14713, 14717, 14723, 14729,
// ];
// const DELAYS: [usize; 1] = [14700];
pub struct Reverb {
    delays: Vec<(f32, usize)>, // WARNING: for simplicity, we work directily with the samples so it it sample_rate dependant.
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
            // if self.buffer.backward(d) == 0.0 {
            //     println!("0");
            // }
        }

        // // Apply smoothing with internal memory
        // output = 0.5 * output + 0.5 * self.buffer.backward(1);
        self.buffer.push(output);
        // println!("{}", self.buffer.backward(0));

        output
    }
}
