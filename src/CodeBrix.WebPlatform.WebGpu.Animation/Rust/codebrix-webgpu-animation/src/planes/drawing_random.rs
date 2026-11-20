//! `DrawingRandom`: a seeded random sequence - SplitMix64 seeding xoshiro256** (C#
//! `DrawingRandom`, bit for bit: the same seed gives the same numbers in both).

/// A seeded random sequence (C# `DrawingRandom`): xoshiro256** seeded by four SplitMix64 steps
/// of the seed. The same seed always gives the same sequence - on every machine, in C# and here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawingRandom {
    seed: i64,
    s0: u64,
    s1: u64,
    s2: u64,
    s3: u64,
}

impl DrawingRandom {
    /// A sequence from a seed (C# `DrawingRandom(long seed)`).
    pub fn new(seed: i64) -> DrawingRandom {
        let mut z = seed as u64;
        let s0 = split_mix(&mut z);
        let s1 = split_mix(&mut z);
        let s2 = split_mix(&mut z);
        let s3 = split_mix(&mut z);
        DrawingRandom { seed, s0, s1, s2, s3 }
    }

    /// A sequence for a key of a seed - independent sequences for many things from one seed (C#
    /// `ForKey`).
    pub fn for_key(seed: i64, key: i64) -> DrawingRandom {
        DrawingRandom::new(hash(seed, key) as i64)
    }

    /// The seed (C# `Seed`).
    pub fn seed(&self) -> i64 {
        self.seed
    }

    /// The next 64 random bits (C# `NextUInt64`).
    pub fn next_uint64(&mut self) -> u64 {
        let result = self.s1.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s1 << 17;
        self.s2 ^= self.s0;
        self.s3 ^= self.s1;
        self.s1 ^= self.s2;
        self.s0 ^= self.s3;
        self.s2 ^= t;
        self.s3 = self.s3.rotate_left(45);
        result
    }

    /// A number in 0..1 (1 excluded), 53 random bits (C# `NextDouble`).
    pub fn next_double(&mut self) -> f64 {
        (self.next_uint64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    /// A number in 0..1 (1 excluded), 24 random bits (C# `NextSingle`).
    pub fn next_single(&mut self) -> f32 {
        (self.next_uint64() >> 40) as f32 * (1.0 / 16777216.0)
    }

    /// A number in low..high (C# `Uniform`).
    pub fn uniform(&mut self, low: f64, high: f64) -> f64 {
        low + ((high - low) * self.next_double())
    }

    /// A normally distributed number (Box-Muller; C# `Normal(mean = 0, deviation = 1)`). It calls
    /// the math library's ln and cos, so it is not promised bit for bit across machines.
    pub fn normal(&mut self, mean: f64, deviation: f64) -> f64 {
        let u = 1.0 - self.next_double();
        let v = self.next_double();
        mean + (deviation * (-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos())
    }

    /// A whole number in 0..max_exclusive, unbiased (Lemire's method; C# `NextInt(int)`). The
    /// error says what to do with a maximum below 1.
    pub fn next_int(&mut self, max_exclusive: i32) -> Result<i32, String> {
        if max_exclusive < 1 {
            return Err("The exclusive maximum must be at least 1.".to_string());
        }
        let range = max_exclusive as u32;
        let mut m = ((self.next_uint64() >> 32) as u32 as u64) * range as u64;
        let mut low = m as u32;
        if low < range {
            let threshold = 0u32.wrapping_sub(range) % range;
            while low < threshold {
                m = ((self.next_uint64() >> 32) as u32 as u64) * range as u64;
                low = m as u32;
            }
        }
        Ok((m >> 32) as i32)
    }

    /// A whole number in min_inclusive..max_exclusive (C# `NextInt(int, int)`).
    pub fn next_int_between(&mut self, min_inclusive: i32, max_exclusive: i32) -> Result<i32, String> {
        let span = max_exclusive as i64 - min_inclusive as i64;
        if span < 1 || span > i32::MAX as i64 {
            return Err("The range must hold at least one number and at most 2^31 - 1.".to_string());
        }
        Ok(min_inclusive + self.next_int(span as i32)?)
    }
}

/// The SplitMix64 finaliser (C#'s internal `Mix`).
pub(crate) fn mix(z: u64) -> u64 {
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// C#'s internal `Hash(seed, key)`.
pub(crate) fn hash(seed: i64, key: i64) -> u64 {
    mix(mix(seed as u64).wrapping_add(key as u64))
}

fn split_mix(z: &mut u64) -> u64 {
    *z = z.wrapping_add(0x9E3779B97F4A7C15);
    mix(*z)
}
