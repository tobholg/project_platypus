//! Deterministic randomness. Every random choice in the simulation comes from
//! an `Rng` seeded by *where and when* it happens, never from a thread-local
//! generator, so results do not depend on thread count or scheduling.

#[derive(Clone, Debug)]
pub struct Rng(u64);

#[inline]
const fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Mix any number of values into one well-distributed hash.
#[inline]
pub fn hash(parts: &[u64]) -> u64 {
    let mut h = 0x6A09_E667_F3BC_C909u64;
    for &p in parts {
        h = splitmix(h ^ p);
    }
    h
}

impl Rng {
    #[inline]
    pub fn seeded(parts: &[u64]) -> Self {
        Rng(hash(parts))
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        splitmix(self.0)
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    #[inline]
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    #[inline]
    pub fn coin(&mut self) -> bool {
        self.next_u64() >> 63 == 1
    }

    /// True with probability `p / 256`.
    #[inline]
    pub fn chance(&mut self, p: u8) -> bool {
        p != 0 && self.next_u8() < p
    }

    /// Uniform in `lo..=hi`.
    #[inline]
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u32() % (hi as u32 - lo as u32 + 1)) as u8
    }

    #[inline]
    pub fn sign(&mut self) -> i32 {
        if self.coin() { 1 } else { -1 }
    }
}
