//! The ball hitting a player who isn't swinging, and the sound word it pops up at the ball: CONK for a slow ball,
//! SMACK for a fast one (`e_poko.tm2` / `e_bashi.tm2` beside the timing balloons in `AZUMA/C_EFF/EFFCT.XB0`).
//!
//! The game tests every player standing or moving (not swinging) each rally frame, outside replays. The first hit
//! of a point ends it (`judge::Judge::check`'s `body`) and pops up the one word of the point; it rises and swells
//! for 20 frames, then stays at full opacity until the players react to the point (practice: 30 frames, then a
//! 5-frame fade).
#![deny(clippy::float_arithmetic)]

use crate::ball::{EFFECT_GRASS, EFFECT_NET};
use crate::ps2::F;

type M4 = [[f32; 4]; 4];

/// The head's ball-sized target: how far back along the head bone's x row it sits, and its radius (m).
const HEAD_BACK: f32 = 0.15;
const HEAD_RADIUS: f32 = 0.25;
/// CONK up to this speed (km/h), SMACK above.
const CONK_KMH: f32 = 50.0;
/// For its first 20 frames the pop-up rises `RISE` / `HOLD` m a frame (0.4 m in all) and swells: by 0.15 a frame for
/// 10, then back by 0.05.
const RISE: f32 = 0.6;
const SWELL_FRAMES: i32 = 20;

/// The ball at `ball` touches the player: within 0.25 m of a point 0.15 m back along `Bip01Head`'s x row from
/// its origin (`head`, the bone's world matrix), or within `r` (the character's collision size,
/// `player::ReachStats::collision`) of the segment from `Bip01Spine` (`spine`) to `Bip01Neck` (`neck`), measured
/// square to it.
pub fn hit(ball: [f32; 3], head: &M4, spine: [f32; 3], neck: [f32; 3], r: f32) -> bool {
    let c = |k: usize| F(head[3][k]) + 0.0 - F(head[0][k]) * HEAD_BACK - ball[k];
    let (x, y, z) = (c(0), c(1), c(2));
    if (z * z).madd(x, x).madd(y, y).sqrt() < HEAD_RADIUS {
        return true;
    }
    let d: [F; 3] = std::array::from_fn(|k| F(neck[k]) - spine[k]);
    let e: [F; 3] = std::array::from_fn(|k| F(ball[k]) - spine[k]);
    let along = (d[2] * e[2]).madd(d[0], e[0]).madd(d[1], e[1]);
    let len = (d[2] * d[2]).madd(d[0], d[0]).madd(d[1], d[1]);
    if along < 0.0 || len < along {
        return false;
    }
    let far = (e[2] * e[2]).madd(e[0], e[0]).madd(e[1], e[1]) + 0.0 + 0.0;
    far - along * along / len <= F(r) * r
}

/// The words, as their textures.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Word {
    Conk,
    Smack,
}

impl Word {
    pub fn texture(self) -> &'static str {
        match self {
            Word::Conk => "e_poko.tm2",
            Word::Smack => "e_bashi.tm2",
        }
    }
}

/// The pop-up's row of the game's balloon table: size factor, near and far depth factors; frames held (practice)
/// and of the fade out (it pops in at full opacity).
const SIZE: [f32; 3] = [0.45, 0.16, 0.3];
const HOLD: i32 = 30;
const FADE_OUT: i32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Popup {
    pub word: Word,
    /// Game space; it rises from the ball where it hit.
    pub pos: [f32; 3],
    /// Swell (×0.15 a frame to 1.5, back to 1 by frame 20).
    pub scale: f32,
    /// 0..128.
    pub alpha: f32,
    frames: i32,
    rise: f32,
    /// Fading out, with its frames left.
    fade: Option<i32>,
    timer: i32,
}

impl Popup {
    /// At the ball (`pos`) going `kmh` (`sound::kmh`): CONK up to 50 km/h, SMACK above. It shows from its first
    /// `tick`, the frame it is made.
    pub fn new(pos: [f32; 3], kmh: f32) -> Self {
        Popup {
            word: if kmh <= CONK_KMH { Word::Conk } else { Word::Smack },
            pos,
            scale: 0.0,
            alpha: 128.0,
            frames: 0,
            rise: (-(F(RISE) / HOLD as f32)).0,
            fade: None,
            timer: HOLD,
        }
    }

    /// One frame. `practice`: fewer than two players (only then does it time out). `false`: it's gone.
    pub fn tick(&mut self, practice: bool) -> bool {
        if self.frames < SWELL_FRAMES {
            self.pos[1] = (F(self.pos[1]) + self.rise).0;
        }
        self.scale = match self.frames {
            ..10 => (F(self.scale) + 0.15).0,
            10..SWELL_FRAMES => (F(self.scale) - 0.05).0,
            _ => 1.0,
        };
        self.frames += 1;
        match self.fade {
            None if practice => {
                self.timer -= 1;
                if self.timer < 0 {
                    self.fade = Some(FADE_OUT);
                }
                true
            }
            None => true,
            Some(n) => {
                let n = n - 1;
                self.fade = Some(n);
                self.alpha = ((n * 0x80) / FADE_OUT).max(0) as f32;
                n >= 0
            }
        }
    }

    /// The quad: its bottom centre (game space) and half width and half height (m). `rows` the camera's right,
    /// down and forward (`camera::View::rot`), `depth` the bottom centre's depth from the camera and `tan` the
    /// tangent of the horizontal half-angle. It stands 0.25 m above the ball and 0.5 m toward the camera; up close
    /// it shrinks, far off it grows.
    pub fn quad(&self, rows: [[f32; 3]; 3], depth: impl Fn([f32; 3]) -> f32, tan: f32) -> ([f32; 3], f32, f32) {
        quad(self.pos, SIZE, self.scale, rows, depth, tan)
    }
}

/// A word pop-up's quad (`Popup::quad`) at `pos` with its balloon-table row's size factors `size` and swell `scale`.
fn quad(pos: [f32; 3], size: [f32; 3], scale: f32, rows: [[f32; 3]; 3], depth: impl Fn([f32; 3]) -> f32, tan: f32) -> ([f32; 3], f32, f32) {
    let f = rows[2];
    let at = [
        (F(pos[0]) + 0.0 - F(f[0]) * 0.5).0,
        (F(pos[1]) - F(0.5) / 2.0 + 0.0 - F(f[1]) * 0.5).0,
        (F(pos[2]) + 0.0 - F(f[2]) * 0.5).0,
    ];
    let d = depth(at);
    let [a, near, far] = size.map(F);
    let size = a * (near * d * tan).max(1.0) * (far * d * tan).min(1.0);
    (at, (size * (F(scale) * 1.2)).0, (size * (F(scale) * 0.6)).0)
}

/// The word of what the ball bounced off (RATTLE off a fence, the net's, wood, water…): a material's bounce effect
/// kind 1..=15 (`ball::Material::effect`), as its texture beside the body-hit words.
pub fn bounce_texture(effect: u8) -> Option<&'static str> {
    const NAMES: [&str; 15] = [
        "netpole", "netup", "netdown", "yuka", "kusa", "suna", "tuchi", "metal01", "metal02", "metal03", "cloth", "wood",
        "water", "taki", "tamari",
    ];
    NAMES.get(effect.checked_sub(1)? as usize).copied()
}

/// A bounce word's balloon-table row (size factor, near and far depth factors), frames held and frames of the fade.
const BOUNCE_SIZE: [f32; 3] = [0.3, 0.18, 0.3];
const BOUNCE_HOLD: i32 = 20;
const BOUNCE_FADE: i32 = 5;

/// One bounce word: it pops in at full opacity where the ball touched, holds 20 frames (ticking from the frame it is
/// made) and fades out over 5.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BouncePop {
    pub effect: u8,
    pub pos: [f32; 3],
    /// 0..128.
    pub alpha: f32,
    timer: i32,
    fading: bool,
}

impl BouncePop {
    /// One frame. `false`: it's gone.
    pub fn tick(&mut self) -> bool {
        self.alpha = 128.0;
        self.timer -= 1;
        if !self.fading {
            if self.timer < 0 {
                (self.fading, self.timer) = (true, BOUNCE_FADE);
            }
            return true;
        }
        self.alpha = ((self.timer * 0x80) / BOUNCE_FADE).max(0) as f32;
        self.timer >= 0
    }

    pub fn quad(&self, rows: [[f32; 3]; 3], depth: impl Fn([f32; 3]) -> f32, tan: f32) -> ([f32; 3], f32, f32) {
        quad(self.pos, BOUNCE_SIZE, 1.0, rows, depth, tan)
    }
}

/// Bounce words: only for contact counts below this, at least this many frames apart, at most this many at once.
const BOUNCE_CONTACTS: i32 = 6;
const BOUNCE_GAP: i32 = 4;
const BOUNCE_MAX: usize = 10;

/// A point's bounce words. They are made from the rally's start until the next point's setup (on after the point is
/// decided too, unlike the bounce sounds), by the same rules as `sound::Bounces`' material sounds: a new contact count
/// below 6, not within 4 frames of the last word, a soft obstacle's (the net's) word not again before a court
/// contact, never effect kind 5; at most 10 on screen at once (the game counts the live ones down as they fade out).
/// ponytail: the game's word for a ball rolling on a material (kusa) is left out; no capture shows one.
#[derive(Clone, Debug, Default)]
pub struct BounceWords {
    pub pops: Vec<BouncePop>,
    last: i32,
    cooldown: i32,
    soft: bool,
}

impl BounceWords {
    /// One frame: the ball's contact count `n` and its last contact (point, material). The new word ticks the frame
    /// it is made.
    pub fn frame(&mut self, n: i32, at: [f32; 3], m: &crate::ball::Material) {
        if n == 0 {
            self.last = 0;
        } else if self.last != n && n < BOUNCE_CONTACTS {
            self.last = n;
            if self.cooldown < 1 && self.pops.len() < BOUNCE_MAX {
                self.soft &= !m.court;
                if !(self.soft && m.effect == EFFECT_NET) && m.effect != EFFECT_GRASS {
                    if m.effect != 0 {
                        self.pops.push(BouncePop { effect: m.effect, pos: at, alpha: 128.0, timer: BOUNCE_HOLD, fading: false });
                        self.cooldown = BOUNCE_GAP;
                    }
                    self.soft = m.effect == EFFECT_NET;
                }
            }
        }
        self.pops.retain_mut(|p| p.tick());
        self.cooldown -= (self.cooldown != 0) as i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_and_trunk() {
        let mut head = [[0.0; 4]; 4];
        head[0] = [1.0, 0.0, 0.0, 0.0];
        head[3] = [0.15, -1.7, 0.0, 1.0];
        let (spine, neck) = ([0.0, -1.0, 0.0], [0.0, -1.5, 0.0]);
        assert!(hit([0.0, -1.9, 0.0], &head, spine, neck, 0.2));
        assert!(hit([0.19, -1.2, 0.0], &head, spine, neck, 0.2));
        assert!(!hit([0.21, -1.2, 0.0], &head, spine, neck, 0.2));
        assert!(!hit([0.0, -0.9, 0.0], &head, spine, neck, 0.2), "below the spine bone");
    }

    #[test]
    fn swells_rises_and_stays() {
        let mut p = Popup::new([0.0, -1.0, 0.0], 50.0);
        assert_eq!(p.word, Word::Conk);
        assert_eq!(Popup::new([0.0; 3], 50.1).word, Word::Smack);
        let mut scales = vec![];
        for _ in 0..200 {
            assert!(p.tick(false));
            scales.push(p.scale);
        }
        assert!((scales[9] - 1.5).abs() < 1e-5 && (scales[19] - 1.0).abs() < 1e-5 && scales[20] == 1.0);
        assert!((p.pos[1] + 1.4).abs() < 1e-5 && p.alpha == 128.0);
        let mut p = Popup::new([0.0; 3], 0.0);
        let n = (0..100).take_while(|_| p.tick(true)).count();
        assert_eq!(n, 31 + 5, "held 31 frames, then the fade's frames at 102, 76, 51, 25, 0");
    }

    #[test]
    fn bounce_words() {
        let fence = crate::ball::Material { effect: 9, ..crate::ball::COURT };
        let mut w = BounceWords::default();
        w.frame(1, [0.0; 3], &fence);
        assert_eq!(bounce_texture(9), Some("metal02"));
        let mut alphas = vec![w.pops[0].alpha];
        while let Some(p) = { w.frame(1, [0.0; 3], &fence); w.pops.first() } {
            alphas.push(p.alpha);
        }
        assert_eq!(alphas.len(), 21 + 5, "21 frames at 128, then 102, 76, 51, 25, 0");
        assert_eq!(&alphas[20..], &[128.0, 102.0, 76.0, 51.0, 25.0, 0.0][..alphas.len() - 20]);
        let mut w = BounceWords::default();
        for n in 1..4 {
            w.frame(n, [0.0; 3], &fence);
        }
        assert_eq!(w.pops.len(), 1, "4 frames between words");
        let net = crate::ball::Material { effect: 2, ..crate::ball::COURT };
        let mut w = BounceWords::default();
        w.frame(1, [0.0; 3], &crate::ball::Material { court: false, ..net });
        (0..5).for_each(|_| w.frame(1, [0.0; 3], &fence));
        w.frame(2, [0.0; 3], &crate::ball::Material { court: false, ..net });
        assert_eq!(w.pops.len(), 1, "the net's word not again before a court contact");
    }
}
