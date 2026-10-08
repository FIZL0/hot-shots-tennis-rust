//! The game's random sources: newlib's `rand()` and its MT19937s.
//!
//! The match keeps four generators: the shared one (the match object's: weather schedule, placement, mis-hits, wild
//! aims, shouts, voice banks, effects), the AI's (seeded 1 when the game starts, reseeded from a shared draw as each
//! AI object is made), the court's (spectators, gallery, emitters) and the sound manager's (hit keys, ball bounces,
//! stroke bits, hit-spark rolls, dive rings). A new point reseeds the shared and the court generator from `rand()` (in
//! that order); the match start, a change of ends and a new point after a point's end then reseed the sound manager's.

use crate::ps2;

/// The C library's `rand()`: a 64-bit LCG whose state is 1 at boot (the game never reseeds it); each call returns
/// bits 32..62 of the stepped state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rand(pub u64);

impl Default for Rand {
    fn default() -> Self {
        Rand(1)
    }
}

impl Rand {
    pub fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(1);
        (self.0 >> 32) as u32 & 0x7fff_ffff
    }
}

/// An MT19937: 624 words and the next word's index (624: regenerate first).
#[derive(Clone, Debug, PartialEq)]
pub struct Mt(pub [u32; 624], pub usize);

impl Mt {
    pub fn new(seed: u32) -> Mt {
        let mut m = [0u32; 624];
        m[0] = seed;
        for i in 1..624 {
            m[i] = (i as u32).wrapping_add((m[i - 1] ^ (m[i - 1] >> 30)).wrapping_mul(0x6c07_8965));
        }
        Mt(m, 624)
    }

    /// The generator as the game keeps it in RAM: the words at +4, the index at +0x9c4 (0x9d0 bytes, little-endian).
    pub fn from_ram(b: &[u8]) -> Mt {
        let w = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        Mt(std::array::from_fn(|k| w(4 + 4 * k)), w(0x9c4) as usize)
    }

    pub fn next(&mut self) -> u32 {
        if self.1 >= 624 {
            for k in 0..624 {
                let y = (self.0[k] & 0x8000_0000) | (self.0[(k + 1) % 624] & 0x7fff_ffff);
                self.0[k] = self.0[(k + 397) % 624] ^ (y >> 1) ^ if y & 1 != 0 { 0x9908_b0df } else { 0 };
            }
            self.1 = 0;
        }
        let mut y = self.0[self.1];
        self.1 += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// The game's usual draw: bits 16..30 (`% n` and `% 100 < chance` are taken from it).
    pub fn r15(&mut self) -> u32 {
        self.next() >> 16 & 0x7fff
    }

    /// A coin: bit 16.
    pub fn bit(&mut self) -> bool {
        self.next() >> 16 & 1 != 0
    }

    /// A uniform in [0, 1): the word as an unsigned float times 2⁻³².
    pub fn unit(&mut self) -> f32 {
        ps2::mul(ps2::utof(self.next()), 2.328_306_4e-10)
    }
}

/// The match's generators.
#[derive(Clone, Debug)]
pub struct Rngs {
    pub rand: Rand,
    pub shared: Mt,
    pub ai: Mt,
    pub court: Mt,
    pub sound: Mt,
}

impl Rngs {
    /// After the shared generator's setup draws (the weather schedule), with `rand()` where the seed left it.
    pub fn new(rand: Rand, shared: Mt) -> Rngs {
        Rngs { rand, shared, ai: Mt::new(1), court: Mt::new(1), sound: Mt::new(1) }
    }

    /// An AI object is made: the AI generator restarts from a shared draw.
    pub fn new_ai(&mut self) {
        self.ai = Mt::new(self.shared.next());
    }

    /// A new point: the shared generator, then the court's, take fresh `rand()` seeds.
    // ponytail: a replay keeps the point's saved seed; no replays here
    pub fn new_point(&mut self) {
        self.shared = Mt::new(self.rand.next());
        self.court = Mt::new(self.rand.next());
    }

    /// The sound manager's reseed (the match start, change ends, and after `new_point` when a point has just ended):
    /// its generator takes a fresh `rand()` seed.
    pub fn change_ends(&mut self) {
        self.sound = Mt::new(self.rand.next());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_outputs() {
        // MT19937's published outputs for init_genrand(5489)
        let mut m = Mt::new(5489);
        assert_eq!([m.next(), m.next(), m.next()], [3_499_211_612, 581_869_302, 3_890_346_734]);
        let mut m = Mt::new(5489);
        assert_eq!((0..10000).map(|_| m.next()).last(), Some(4_123_659_995));
    }
}
