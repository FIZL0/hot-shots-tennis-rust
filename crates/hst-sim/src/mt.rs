//! The AI's random generator: a plain MT19937, seeded by each AI object as it is made (with a draw from the
//! game's main generator) and drawn by every AI routine.

#[derive(Clone)]
pub struct Mt {
    state: [u32; 624],
    index: usize,
}

impl Mt {
    pub fn new(seed: u32) -> Mt {
        let mut state = [0; 624];
        state[0] = seed;
        for k in 1..624 {
            state[k] = (k as u32).wrapping_add((state[k - 1] ^ state[k - 1] >> 30).wrapping_mul(0x6c07_8965));
        }
        Mt { state, index: 624 }
    }

    pub fn next(&mut self) -> u32 {
        if self.index >= 624 {
            for k in 0..624 {
                let y = (self.state[k] & 0x8000_0000) | (self.state[(k + 1) % 624] & 0x7fff_ffff);
                self.state[k] = self.state[(k + 397) % 624] ^ (y >> 1) ^ if y & 1 != 0 { 0x9908_b0df } else { 0 };
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }
}

impl Default for Mt {
    fn default() -> Mt {
        Mt::new(1)
    }
}

#[test]
fn reference_outputs() {
    // MT19937's published first outputs for seed 5489
    let mut m = Mt::new(5489);
    assert_eq!([m.next(), m.next()], [3_499_211_612, 581_869_302]);
}
