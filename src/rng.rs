//! TRNG wrapper — same pattern as Minesweeper / Decision Engine.

extern crate alloc;

pub struct Rng {
    trng: trng::Trng,
}

impl Rng {
    pub fn new(xns: &xous_names::XousNames) -> Self {
        Self {
            trng: trng::Trng::new(xns).expect("can't connect to TRNG"),
        }
    }

    pub fn u32(&self) -> u32 {
        self.trng.get_u32().unwrap_or(0)
    }

    /// Random number in range [0, max) with rejection sampling.
    pub fn range(&self, max: u32) -> u32 {
        if max <= 1 {
            return 0;
        }
        let threshold = u32::MAX - (u32::MAX % max);
        loop {
            let val = self.u32();
            if val < threshold {
                return val % max;
            }
        }
    }

    /// Random number in range [min, max] inclusive.
    pub fn range_inclusive(&self, min: u32, max: u32) -> u32 {
        if max <= min {
            return min;
        }
        min + self.range(max - min + 1)
    }
}
