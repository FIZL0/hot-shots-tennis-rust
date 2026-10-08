//! The ball hitting a player who isn't swinging, and the sound word it pops up at the ball: CONK for a slow ball,
//! SMACK for a fast one (`e_poko.tm2` / `e_bashi.tm2` beside the timing balloons in `AZUMA/C_EFF/EFFCT.XB0`).
//!
//! The game tests every player standing or moving (not swinging) each rally frame, outside replays. The first hit
//! of a point ends it (`judge::Judge::check`'s `body`) and pops up the one word of the point; it rises and swells
//! for 20 frames, then stays at full opacity until the players react to the point (practice: 30 frames, then a
//! 5-frame fade).
use crate::ps2::{add, div, madd, mul, sub};

type M4 = [[f32; 4]; 4];

/// The ball at `ball` touches the player: within 0.25 m of a point 0.15 m back along `Bip01Head`'s x row from
/// its origin (`head`, the bone's world matrix), or within `r` (the character's collision size,
/// `player::ReachStats::collision`) of the segment from `Bip01Spine` (`spine`) to `Bip01Neck` (`neck`), measured
/// square to it.
pub fn hit(ball: [f32; 3], head: &M4, spine: [f32; 3], neck: [f32; 3], r: f32) -> bool {
    let c = |k: usize| sub(sub(add(head[3][k], 0.0), mul(head[0][k], 0.15)), ball[k]);
    let (x, y, z) = (c(0), c(1), c(2));
    if crate::ps2::sqrt(madd(madd(mul(z, z), x, x), y, y)) < 0.25 {
        return true;
    }
    let d: [f32; 3] = std::array::from_fn(|k| sub(neck[k], spine[k]));
    let e: [f32; 3] = std::array::from_fn(|k| sub(ball[k], spine[k]));
    let along = madd(madd(mul(d[2], e[2]), d[0], e[0]), d[1], e[1]);
    let len = madd(madd(mul(d[2], d[2]), d[0], d[0]), d[1], d[1]);
    if along < 0.0 || len < along {
        return false;
    }
    let far = add(add(madd(madd(mul(e[2], e[2]), e[0], e[0]), e[1], e[1]), 0.0), 0.0);
    sub(far, div(mul(along, along), len)) <= mul(r, r)
}

/// The words, as their textures.
#[derive(Clone, Copy, Debug, PartialEq)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
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
            word: if kmh <= 50.0 { Word::Conk } else { Word::Smack },
            pos,
            scale: 0.0,
            alpha: 128.0,
            frames: 0,
            rise: -div(0.6, HOLD as f32),
            fade: None,
            timer: HOLD,
        }
    }

    /// One frame. `practice`: fewer than two players (only then does it time out). `false`: it's gone.
    pub fn tick(&mut self, practice: bool) -> bool {
        if self.frames < 20 {
            self.pos[1] = add(self.pos[1], self.rise);
        }
        self.scale = match self.frames {
            ..10 => add(self.scale, 0.15),
            ..20 => sub(self.scale, 0.05),
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
        let f = rows[2];
        let at = [
            sub(add(self.pos[0], 0.0), mul(f[0], 0.5)),
            sub(add(sub(self.pos[1], div(0.5, 2.0)), 0.0), mul(f[1], 0.5)),
            sub(add(self.pos[2], 0.0), mul(f[2], 0.5)),
        ];
        let d = depth(at);
        let [a, near, far] = SIZE;
        let size = mul(mul(a, mul(mul(near, d), tan).max(1.0)), mul(mul(far, d), tan).min(1.0));
        (at, mul(size, mul(self.scale, 1.2)), mul(size, mul(self.scale, 0.6)))
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
}
