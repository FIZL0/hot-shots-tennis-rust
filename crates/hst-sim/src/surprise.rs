//! The surprise pop-ups over a player's head, ported from the original's pop-up list (the one that also holds the
//! timing balloons):
//!
//! - **"!"** (`burefukidashi`): a computer player caught off guard. Raised when its AI takes a change-of-pace or
//!   fast-ball reaction (`ai::Timing::reacted`) or finds its guess wrong (`ai::Verdict::Wrong`); never for a human.
//!   One at a time per player, none while a clean-hit balloon (note or sweet) is over it; it replaces that player's
//!   timing balloon, a bad-timing balloon (bunny or turtle) isn't shown while it's up, and a clean-hit balloon takes it
//!   down.
//! - **Sweat drop** (`A_fukidasi_10`) in doubles when a point ends on an out (also after the net), a double fault or
//!   an illegal hit (a serve returned by the receiver's partner, a ball hit twice by one team): over the player who
//!   hit last (the server if nobody hit), with a "..." balloon (`A_fukidasi_11`) over their partner. Both hold until
//!   the next point is set up.
//! - **Swirl** (`e_guruguru`), the upset pop-up in singles on the same calls, over the same player: five 80-px cells
//!   side by side, the next every 3 frames, held until the next point is set up.
//!
//! All are camera-facing quads drawn like the timing balloons: same anchor over the head, the swirl a little bigger.

/// Which pop-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Popup {
    Bang,
    Sweat,
    Dots,
    Swirl,
}

impl Popup {
    /// Texture in `AZUMA/C_EFF/EFFCT.XB0`.
    pub fn texture(self) -> &'static str {
        match self {
            Popup::Bang => "burefukidashi.tm2",
            Popup::Sweat => "A_fukidasi_10.tm2",
            Popup::Dots => "A_fukidasi_11.tm2",
            Popup::Swirl => "e_guruguru.tm2",
        }
    }

    /// Fade-in, hold and fade-out frames (the per-kind table; the hold of sweat and dots never runs down).
    pub fn frames(self) -> [i32; 3] {
        match self {
            Popup::Bang => [3, 30, 3],
            Popup::Sweat | Popup::Dots | Popup::Swirl => [3, 60, 3],
        }
    }

    /// The per-kind base half-width (m) of the quad.
    pub fn size(self) -> f32 {
        if self == Popup::Swirl { 0.35 } else { 0.3 }
    }
}

/// A live pop-up: the original's stage machine (0 fade-in, 1 hold, 2 fade-out) and its alpha (0..=128, 128 opaque).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pop {
    pub popup: Popup,
    pub player: usize,
    stage: u8,
    t: i32,
    pub alpha: i32,
    /// Frames aged, and the swirl's cell (0..5, the next every third frame).
    n: u32,
    pub cell: u8,
}

impl Pop {
    /// A new pop-up; `tick` it the same frame (the list ages right after spawning, before drawing).
    pub fn new(popup: Popup, player: usize) -> Self {
        Pop { popup, player, stage: 0, t: popup.frames()[0], alpha: 128, n: 0, cell: 0 }
    }

    /// One frame of ageing; false once it's gone (the list drops it before drawing).
    pub fn tick(&mut self) -> bool {
        let [fade_in, hold, fade_out] = self.popup.frames();
        self.alpha = 128;
        if self.popup == Popup::Swirl {
            self.n += 1;
            if self.n % 3 == 0 {
                self.cell = (self.cell + 1) % 5;
            }
        }
        match self.stage {
            0 => {
                self.t -= 1;
                self.alpha = (128 - self.t * 128 / fade_in).min(128);
                if self.t < 0 {
                    (self.stage, self.t) = (1, hold);
                }
            }
            1 if self.popup == Popup::Bang => {
                self.t -= 1;
                if self.t < 0 {
                    (self.stage, self.t) = (2, fade_out);
                }
            }
            1 => {}
            _ => {
                self.t -= 1;
                self.alpha = (self.t * 128 / fade_out).max(0);
                if self.t < 0 {
                    return false;
                }
            }
        }
        true
    }
}

/// The sweat drop's player and the "..." player when a doubles point ends on `call` (judge codes: 1 out, 3 double
/// fault, 5 out after the net, 6 illegal hit). `shots` this point, `hitter` the last hitter, `server`.
pub fn sweat(players: usize, call: u8, shots: i32, hitter: i32, server: i32) -> Option<(usize, usize)> {
    if players < 4 || !matches!(call, 1 | 3 | 5 | 6) {
        return None;
    }
    let who = if shots != 0 { hitter } else { server } as usize;
    Some((who, if who < 2 { who + 2 } else { who - 2 }))
}

/// The swirl's player when a singles point ends on `call` (as [`sweat`]).
pub fn swirl(players: usize, call: u8, shots: i32, hitter: i32, server: i32) -> Option<usize> {
    (players == 2 && matches!(call, 1 | 3 | 5 | 6)).then_some(if shots != 0 { hitter } else { server } as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alphas(popup: Popup, frames: usize) -> Vec<i32> {
        let mut p = Pop::new(popup, 0);
        (0..frames).map_while(|_| p.tick().then_some(p.alpha)).collect()
    }

    /// Frame by frame as the original's list (P24 capture, slot 5): 43, 86, then 33 frames opaque, 85, 42 and a
    /// last invisible frame.
    #[test]
    fn bang_frames() {
        let a = alphas(Popup::Bang, 100);
        assert_eq!(a.len(), 38);
        assert_eq!(a[..3], [43, 86, 128]);
        assert!(a[2..35].iter().all(|&x| x == 128));
        assert_eq!(a[35..], [85, 42, 0]);
    }

    /// The sweat drop fades in and stays.
    #[test]
    fn sweat_holds() {
        let a = alphas(Popup::Sweat, 1000);
        assert_eq!(a.len(), 1000);
        assert_eq!(a[..3], [43, 86, 128]);
        assert!(a[2..].iter().all(|&x| x == 128));
    }

    #[test]
    fn sweat_who() {
        assert_eq!(sweat(4, 6, 1, 3, 0), Some((3, 1)));
        assert_eq!(sweat(4, 3, 0, -1, 2), Some((2, 0)));
        assert_eq!(sweat(4, 2, 1, 0, 0), None);
        assert_eq!(sweat(2, 1, 4, 1, 0), None);
        assert_eq!(swirl(2, 1, 4, 1, 0), Some(1));
        assert_eq!(swirl(2, 3, 0, -1, 0), Some(0));
        assert_eq!(swirl(2, 2, 4, 1, 0), None);
        assert_eq!(swirl(4, 1, 4, 1, 0), None);
    }

    /// The swirl turns a cell every third frame from the spawn frame's ageing, through five cells.
    #[test]
    fn swirl_cells() {
        let mut p = Pop::new(Popup::Swirl, 0);
        let cells: Vec<u8> = (0..16).map(|_| (p.tick(), p.cell).1).collect();
        assert_eq!(cells, [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 0, 0]);
    }
}
