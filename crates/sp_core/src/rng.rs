use std::f64::consts::PI;

/// Seeded, deterministic PRNG (xoshiro128**). All game randomness goes through this.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Rng {
    s: [u32; 4],
}

fn splitmix(seed: u64) -> [u32; 4] {
    let mut x = seed;
    let mut next = || {
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) & 0xFFFF_FFFF) as u32
    };
    [next(), next(), next(), next() | 1]
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { s: splitmix(seed) }
    }

    /// Snapshot of the generator so a saved game continues exactly where it left off.
    pub fn state(&self) -> [u32; 4] {
        self.s
    }

    pub fn from_state(s: [u32; 4]) -> Self {
        Self { s }
    }

    pub fn restore_state(&mut self, s: [u32; 4]) {
        self.s = s;
    }

    /// Derives an independent stream, e.g. per season/week/game.
    pub fn fork(&mut self, salt: u64) -> Rng {
        Rng::new((self.next_u32() as u64) ^ salt.wrapping_mul(0x9E37_79B1))
    }

    pub fn next_u32(&mut self) -> u32 {
        let [a, b, c, d] = &mut self.s;
        let result = b.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = *b << 9;
        *c ^= *a;
        *d ^= *b;
        *b ^= *c;
        *a ^= *d;
        *c ^= t;
        *d = d.rotate_left(11);
        result
    }

    /// Uniform double in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        self.next_u32() as f64 / 4_294_967_296.0
    }

    /// Uniform int in [0, max).
    pub fn next_int(&mut self, max: usize) -> usize {
        (self.next_f64() * max as f64).floor() as usize
    }

    /// Uniform int in [min, max] inclusive.
    pub fn range(&mut self, min: i32, max: i32) -> i32 {
        min + self.next_int((max - min + 1) as usize) as i32
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// Standard normal via Box-Muller.
    pub fn next_gaussian(&mut self) -> f64 {
        let u1 = 1.0 - self.next_f64();
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
    }

    pub fn gaussian(&mut self, mean: f64, sd: f64) -> f64 {
        mean + sd * self.next_gaussian()
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.next_int(items.len())]
    }

    /// Weighted pick; weights need not sum to 1.
    pub fn weighted<T: Copy>(&mut self, weights: &[(T, f64)]) -> T {
        let total: f64 = weights.iter().map(|w| w.1).sum();
        let mut r = self.next_f64() * total;
        for &(item, w) in weights {
            r -= w;
            if r < 0.0 {
                return item;
            }
        }
        weights[weights.len() - 1].0
    }

    /// Fisher-Yates shuffle in place.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.next_int(i + 1);
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_identical_sequence() {
        let (mut a, mut b) = (Rng::new(42), Rng::new(42));
        let xs: Vec<u32> = (0..50).map(|_| a.next_u32()).collect();
        let ys: Vec<u32> = (0..50).map(|_| b.next_u32()).collect();
        assert_eq!(xs, ys);
    }

    #[test]
    fn different_seeds_differ() {
        assert_ne!(Rng::new(1).next_u32(), Rng::new(2).next_u32());
    }

    #[test]
    fn next_f64_stays_in_unit_interval() {
        let mut r = Rng::new(7);
        for _ in 0..10_000 {
            let d = r.next_f64();
            assert!((0.0..1.0).contains(&d));
        }
    }

    #[test]
    fn state_round_trips() {
        let mut a = Rng::new(9);
        a.next_u32();
        let mut b = Rng::from_state(a.state());
        assert_eq!(a.next_u32(), b.next_u32());
    }

    #[test]
    fn gaussian_has_sane_moments() {
        let mut r = Rng::new(3);
        let xs: Vec<f64> = (0..20_000).map(|_| r.next_gaussian()).collect();
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64;
        assert!(mean.abs() < 0.03, "{mean}");
        assert!((var - 1.0).abs() < 0.05, "{var}");
    }
}
