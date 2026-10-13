//! The game's random sources: newlib's `rand()` and its MT19937s.
//!
//! The match keeps four generators: the shared one (the match object's: weather schedule, placement, mis-hits, wild
//! aims, shouts, voice banks, effects), the AI's (seeded 1 when the game starts, reseeded from a shared draw as each
//! AI object is made), the court's (spectators, gallery, emitters) and the sound manager's (hit keys, ball bounces,
//! stroke bits, hit-spark rolls, dive rings). A new point reseeds the shared and the court generator from `rand()` (in
//! that order); the match start, a change of ends and a new point after a point's end then reseed the sound manager's.

use crate::ps2;
use hst_data::{ani::{self, Anim}, iso::Iso, xb::Archive};

/// The C library's `rand()`: a 64-bit LCG whose state is 1 at boot (the game never reseeds it); each call returns
/// bits 32..62 of the stepped state.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mt(#[serde(with = "crate::big")] pub [u32; 624], pub usize);


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

/// The match seed: `rand()`'s state as the setup finds it and the shared generator's seed. Every generator follows
/// from these two (the AI's from shared draws, the court's and the sound manager's from `rand()`), so online the host
/// picks it and the peers take it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seed {
    pub rand: Rand,
    pub shared: u32,
}

impl Seed {
    /// The menus drew `skip` `rand()`s before the setup, which seeds the shared generator with the next one.
    pub fn after(skip: u32) -> Seed {
        let mut rand = Rand::default();
        (0..skip).for_each(|_| _ = rand.next());
        let shared = rand.next();
        Seed { rand, shared }
    }

    /// The shared generator's seed given directly, `rand()` as at boot.
    pub fn shared(shared: u32) -> Seed {
        Seed { rand: Rand::default(), shared }
    }
}

/// The match's generators.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Rngs {
    pub rand: Rand,
    pub shared: Mt,
    pub ai: Mt,
    pub court: Mt,
    pub sound: Mt,
    /// The sun's lens flare's rays, made by `setup_rand` and stepped by `flare_tick`.
    pub flare: Flare,
}

/// The lens flare's 24 rays: each one's brightness (0.25..1 once stepped) and angle on screen (radians, x right,
/// y down: ray `i` points near (i + 1)·15°).
// ponytail: plain f32, the flare only has to look right; its draws are exact
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Flare {
    pub bright: [f32; 24],
    pub angle: [f32; 24],
}

impl Rngs {
    /// After the shared generator's setup draws (the weather schedule), with `rand()` where the seed left it.
    pub fn new(rand: Rand, shared: Mt) -> Rngs {
        Rngs { rand, shared, ai: Mt::new(1), court: Mt::new(1), sound: Mt::new(1), flare: Flare::default() }
    }

    /// An AI object is made: the AI generator restarts from a shared draw.
    pub fn new_ai(&mut self) {
        self.ai = Mt::new(self.shared.next());
    }

    /// A new point: the shared generator, then the court's, take fresh `rand()` seeds.
    // ponytail: a replay keeps the point's saved seed; no replays here
    pub fn new_point(&mut self) {
        self.new_point_clouds(0);
    }

    /// A new point whose weather change remakes `clouds` clouds (`weather::clouds_remade`): their draws come between
    /// the shared and the court generator's seeds. Returns them (`clouds::spawn`).
    pub fn new_point_clouds(&mut self, clouds: usize) -> Vec<u32> {
        self.shared = Mt::new(self.rand.next());
        let draws = self.clouds(clouds);
        self.court = Mt::new(self.rand.next());
        draws
    }

    /// `n` clouds are made: 5 `rand()` draws each (`clouds::spawn`).
    pub fn clouds(&mut self, n: usize) -> Vec<u32> {
        (0..5 * n).map(|_| self.rand.next()).collect()
    }

    /// The sound manager's reseed (the match start, change ends, and after `new_point` when a point has just ended):
    /// its generator takes a fresh `rand()` seed.
    pub fn change_ends(&mut self) {
        self.sound = Mt::new(self.rand.next());
    }

    /// The match setup's `rand()` calls after the seed, up to the sound manager's second reseed (which
    /// `change_ends` makes): the sun's lens flare is made (24 rays × a brightness and an angle jitter), the sound
    /// manager's first reseed, `clouds` clouds are placed (`clouds`; the court's count for the first game's weather in
    /// singles, none in doubles), and the effects seed is drawn. Returns the effects seed (the effects' LCG restarts
    /// from it at every new point) and the clouds' draws.
    pub fn setup_rand(&mut self, clouds: usize) -> (u32, Vec<u32>) {
        // each ray: a brightness of `% 100` hundredths, then its angle: the 4096ths of a turn up to it (4096 / 24 = 170
        // each) jittered by `% 85 − 42`
        for i in 0..24 {
            self.flare.bright[i] = (self.rand.next() % 100) as f32 / 100.0;
            let n = (i as i32 + 1) * 4096 / 24 + (self.rand.next() % 85) as i32 - 42;
            self.flare.angle[i] = (std::f64::consts::PI * n as f64 / 2048.0) as f32;
        }
        self.change_ends();
        let draws = self.clouds(clouds);
        (self.rand.next(), draws)
    }

    /// The match intro's game ticks (before the first point, each a lens flare tick in clear or cloudy weather). The
    /// intro phase ends once the court's camera flyover (`camera`: `cam_cNN_00.dat`, 40-byte records whose first
    /// field is the move's frames) has run and then the "Start the match!" banner's animation (`banner`) has played
    /// out: the flyover takes its records' frames (a record of 0 frames after the first cuts to a new segment, 2
    /// frames), the banner its last key's frame, and the two hand-offs a tick each. Its timer (3 ticks) and the
    /// streamed sound are done long before. Same in singles and doubles.
    pub fn intro_ticks_of(camera: &[u8], banner: &Anim) -> usize {
        let flyover: usize = camera
            .chunks_exact(40)
            .take(20)
            .enumerate()
            .map(|(i, r)| (i16::from_le_bytes([r[0], r[1]]).max(0) as usize).max(2 * (i > 0) as usize))
            .sum();
        flyover + (banner.end_tick() / banner.ticks_per_frame) as usize + 2
    }

    /// `intro_ticks_of` for court `stage` from the disc (`COURT/NN/CMN.XB` and INPANE's `i_gamestart_00.ANI`).
    pub fn intro_ticks_on_disc(iso: &mut Iso, stage: u32) -> Option<usize> {
        let read = |iso: &mut Iso, xb: &str, name: &str| {
            let d = iso.read(xb).ok()?;
            let arc = Archive::parse(&d).ok()?;
            arc.read(arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(name))?).ok()
        };
        let camera = read(iso, &format!("COURT/{stage:02}/CMN.XB"), &format!("cam_c{stage:02}_00.dat"))?;
        let banner = read(iso, "AZUMA/INPANE/INPANE.XB0", "i_gamestart_00.ani")?;
        Some(Self::intro_ticks_of(&camera, &ani::parse(&banner).ok()?))
    }

    /// `intro_ticks_on_disc` for the retail disc's court `stage` (1–11), for callers without the disc at hand.
    // ponytail: the retail courts' lengths, checked against the disc by tests/rng.rs intro_ticks_from_the_disc
    pub fn intro_ticks(stage: u32) -> usize {
        match stage {
            1 => 364,
            5 => 446,
            11 => 354,
            _ => 394,
        }
    }

    /// One tick of the sun's lens flare while the weather shows it (clear or cloudy): each of its 24 rays' brightness
    /// steps by a `rand() % 11 − 5` hundredth, kept in 0.25..=1.
    pub fn flare_tick(&mut self) {
        for b in &mut self.flare.bright {
            *b = (*b + ((self.rand.next() % 11) as f32 - 5.0) / 100.0).clamp(0.25, 1.0);
        }
    }

    /// The match setup's shared draws after the players' (each player's voice-bank pick, AI reseed and placement):
    /// the hit-spark table (25 sparks × 4, lost to the sound manager's own roll), then the gallery bank (0..3) among
    /// those `used` doesn't hold, which it then holds (all four used: all freed first).
    pub fn setup_gallery(&mut self, used: &mut [bool; 4]) -> usize {
        for _ in 0..100 {
            self.shared.next();
        }
        if used.iter().all(|&u| u) {
            *used = [false; 4];
        }
        let free: Vec<usize> = (0..4).filter(|&k| !used[k]).collect();
        let pick = free[self.shared.r15() as usize % free.len()];
        used[pick] = true;
        pick
    }
}

/// The menus' flame (24 particles of `menu/2d/f_s.tm2`, behind the confirmation screen's VS, made at P1's pick): it
/// draws `rand()` on every frame it's up, so the calls before the match seed follow how long the menus are shown.
/// A particle lives 18 + `% 24` frames at an integer heading in degrees (0 = right, 270 = up) that steers toward
/// 270, by a random step in 226..315; it moves along the heading (faster the nearer it is to straight up), spins
/// (a slowing random turn), grows to full size, then shrinks and fades over its last 14 frames.
// ponytail: motion in plain f32 (the menu only has to look right); the draws, which the match seed hangs on, are exact
#[derive(Clone, Debug, PartialEq)]
pub struct MenuFlame {
    alive: [bool; 24],
    life: [u32; 24],
    heading: [i32; 24],
    /// Offset from the effect's origin and the step a frame, in 640×448 pixels (y down).
    pos: [[f32; 2]; 24],
    vel: [[f32; 2]; 24],
    /// The sprite's turn (radians) and its turn a frame (degrees).
    angle: [f32; 24],
    spin: [f32; 24],
    /// Size as a share of 64 px.
    scale: [f32; 24],
    /// What each particle drew this frame (before its step).
    shown: [Option<FlameSprite>; 24],
}

/// One particle as drawn: centre offset (px), side (px), turn (radians, clockwise), alpha (0..128), and whether it
/// takes the sheet's smoke half (particles 20..23) instead of the fire half.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlameSprite {
    pub at: [f32; 2],
    pub size: f32,
    pub angle: f32,
    pub alpha: f32,
    pub smoke: bool,
}

/// The effect's tuning (the menu overlay's block): starting speed, top-speed bonus, base speed, x squash, growth a
/// frame, full size, spin and shrink decays, then life (18 + `% 24`) and the fade (last 14 frames).
const SPEED0: f32 = 1.04;
const BOOST: f64 = 1.3;
const BASE: f32 = 1.04;
const SQUASH: f32 = 0.65;
const GROW: f32 = 0.13;
const FULL: f32 = 1.56;
const DECAY: f32 = 0.96;
const FADE: u32 = 14;
const DEG: f32 = 0.017_453_292;

impl MenuFlame {
    /// The screen's setup: each particle starts alive when `% 3 != 0`, else when a second draw's `& 7 == 0`.
    pub fn new(r: &mut Rand) -> MenuFlame {
        let mut f = MenuFlame {
            alive: [false; 24],
            life: [0; 24],
            heading: [0; 24],
            pos: [[0.0; 2]; 24],
            vel: [[0.0; 2]; 24],
            angle: [0.0; 24],
            spin: [0.0; 24],
            scale: [0.0; 24],
            shown: [None; 24],
        };
        for i in 0..24 {
            if Self::draw(r) % 3 != 0 || Self::draw(r) & 7 == 0 {
                f.spawn(r, i);
            }
        }
        f
    }

    fn draw(r: &mut Rand) -> i32 {
        (r.next() >> 8) as i32
    }

    /// Degrees to radians in -π..π.
    fn rad(deg: i32) -> f32 {
        let d = deg.rem_euclid(360);
        (if d > 180 { d - 360 } else { d }) as f32 * DEG
    }

    /// A heading (`% 360`), a life and a spin (`% 7 − 3`); off from the origin along the heading, small.
    fn spawn(&mut self, r: &mut Rand, i: usize) {
        self.alive[i] = true;
        self.heading[i] = Self::draw(r) % 360;
        let a = Self::rad(self.heading[i]);
        self.pos[i] = [0.0; 2];
        self.vel[i] = [SPEED0 * crate::libm::cosf(a), SPEED0 * crate::libm::sinf(a)];
        self.life[i] = 18 + (Self::draw(r) % 24) as u32;
        self.scale[i] = 0.1;
        self.angle[i] = a;
        self.spin[i] = (Self::draw(r) % 7 - 3) as f32;
    }

    /// One frame: with fewer than 13 particles out a dead one respawns when `& 7 == 0`; with 13 or more, when
    /// `% 3 != 0` or else a second draw's `& 7 == 0`. Each live one (a new one too) is drawn, then loses a frame of
    /// life and, still alive, turns, moves, grows or shrinks and steers.
    pub fn frame(&mut self, r: &mut Rand) {
        let dead = self.alive.iter().filter(|&&a| !a).count();
        for i in 0..24 {
            if !self.alive[i] && ((dead >= 13 && Self::draw(r) % 3 != 0) || Self::draw(r) & 7 == 0) {
                self.spawn(r, i);
            }
            self.shown[i] = None;
            if !self.alive[i] {
                continue;
            }
            let alpha = if self.life[i] < FADE { (self.life[i] * 128 / FADE) as f32 } else { 128.0 };
            self.shown[i] = Some(FlameSprite { at: self.pos[i], size: 64.0 * self.scale[i], angle: self.angle[i], alpha, smoke: i >= 20 });
            self.life[i] -= 1;
            if self.life[i] == 0 {
                self.alive[i] = false;
                continue;
            }
            self.angle[i] += self.spin[i] * DEG;
            self.spin[i] *= DECAY;
            if self.angle[i].abs() > std::f32::consts::PI {
                self.angle[i] -= std::f32::consts::TAU * self.angle[i].signum();
            }
            self.pos[i] = [self.pos[i][0] + self.vel[i][0], self.pos[i][1] + self.vel[i][1]];
            self.scale[i] = if self.life[i] < FADE { self.scale[i] * DECAY } else { (self.scale[i] + GROW).min(FULL) };
            let a = self.heading[i];
            // a random step: a sixth of the way to 270 (truncated), scaled by 100..299 %, at least 1
            let jitter = |r: &mut Rand, sixth: i32| (sixth * (Self::draw(r) % 200 + 100) / 100).max(1);
            self.heading[i] += match a {
                0..=90 => -((a + 90) >> 2),
                91..=180 => (270 - a) >> 2,
                181..=225 => (270 - a) / 6,
                226..=269 => jitter(r, (270 - a) / 6),
                270 => if Self::draw(r) & 1 == 0 { -1 } else { 1 },
                271..=315 => -jitter(r, (a - 270) / 6),
                316..=359 => -((a - 270) / 6),
                _ => 0,
            };
            self.heading[i] = self.heading[i].rem_euclid(360);
            // faster the nearer the heading is to vertical: off it by d degrees (0..90), BASE + BOOST·(1 − d/180)
            let h = self.heading[i];
            let d = match h {
                0..=89 => h + 90,
                90..=179 => 180 - h,
                180..=269 => 270 - h,
                _ => h - 270,
            };
            let speed = BASE + (BOOST * (1.0 - d as f64 / 180.0)) as f32;
            let a = Self::rad(h);
            self.vel[i] = [SQUASH * speed * crate::libm::cosf(a), speed * crate::libm::sinf(a)];
        }
    }

    /// The particles drawn on the last frame, in drawing order.
    pub fn sprites(&self) -> impl Iterator<Item = FlameSprite> + '_ {
        self.shown.iter().flatten().copied()
    }
}

/// Player `i`'s voice bank (b or a) from its draw `drawn` (`r15 % 100 >= 70`, always made) and the banks `banks` of
/// the players before it, `chars` every player's character: two of one character take different banks (the second
/// the other of the first's); three: the first two keep their draws, the third takes the other of its partner's
/// (`i ^ 2`) when the partner is one of them; four: the third takes b when the first two have a, a when they have b,
/// and the fourth a when exactly one of the first three has a, else b.
pub fn voice_bank(chars: &[u32], i: usize, banks: &[bool], drawn: bool) -> bool {
    let mine = chars[i];
    let same = chars.iter().filter(|&&c| c == mine).count();
    let prev: Vec<bool> = (0..i).filter(|&k| chars[k] == mine).map(|k| banks[k]).collect();
    match (same, prev.as_slice()) {
        (2, [b]) => !b,
        (3, [_, ..]) if i >= 2 && chars[i ^ 2] == mine => !banks[i ^ 2],
        (4, [a, b]) if a == b => !a,
        (4, [_, _, _]) => prev.iter().filter(|&&b| !b).count() != 1,
        _ => drawn,
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

    #[test]
    fn gallery_banks_across_matches() {
        // four setups play each bank once; the fifth frees them all and may pick any
        let (mut r, mut used) = (Rngs::new(Rand(1), Mt::new(7)), [false; 4]);
        let mut picks: Vec<usize> = (0..4).map(|_| r.setup_gallery(&mut used)).collect();
        picks.sort();
        assert_eq!(picks, [0, 1, 2, 3]);
        let fifth = r.setup_gallery(&mut used);
        assert_eq!(used.iter().filter(|&&u| u).count(), 1);
        assert!(used[fifth]);
    }
}
