//! Player locomotion as the original: run speed (character speed, acceleration and stamina), the per-frame run
//! counter and stamina drain, and the motion each standing or running player plays (ready stance, turned
//! stances toward the ball, runs by direction relative to the player's forward, dash, tired variants).

use crate::libm::acosf;
use crate::ps2::{add, div, madd, msub, mul};

/// A character's movement stats (TParam.csv: SPE, Agili, STA).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// SPE / 10.
    pub speed: f32,
    /// Frames of running until full speed (Agili; ×1.5 on court surfaces 2 and 3).
    pub agility: i32,
    /// Stamina at the start of a match (STA).
    pub stamina: i32,
}

impl Stats {
    /// From TParam.csv's SPE, Agili and STA cells; `surface` is the court's surface kind.
    pub fn new(spe: i32, agility: i32, stamina: i32, surface: u8) -> Self {
        // the game's integer ×150/100 for the slow surfaces
        let agility = if surface.wrapping_sub(2) < 2 { agility * 150 / 100 } else { agility };
        Stats { speed: spe as f32 / 10.0, agility, stamina }
    }
}

/// Base run speed, metres per frame.
pub const RUN: f32 = f32::from_bits(0x3d92_d8b5); // 0.0717024
/// Stamina below this plays the tired motions (+8) and slows the run by 3 % per missing point.
pub const TIRED: i32 = 10;
/// Frames of running per stamina point (while the rally is live, more than one player).
pub const STAMINA_FRAMES: i32 = 60;

/// Speed of a running player, metres per frame: `run` frames into the run, with `stamina` left. `size` is the
/// player's size percentage (100 in normal play).
pub fn run_speed(s: &Stats, run: i32, stamina: i32, size: i32) -> f32 {
    let frac = div(run.min(s.agility) as f32, s.agility as f32);
    let base = mul(RUN, madd(1.0, f32::from_bits(0x3e99_999a), frac)); // 1 + 0.3·frac
    let scale = if stamina < TIRED {
        div(mul(msub(1.0, f32::from_bits(0x3cf5_c28f), (TIRED - stamina) as f32), size as f32), 100.0)
    } else {
        div(size as f32, 100.0)
    };
    mul(mul(base, s.speed), scale)
}

/// Velocity for a unit run direction (game space), as the game scales it.
pub fn run_velocity(dir: [f32; 3], speed: f32) -> [f32; 3] {
    dir.map(|d| mul(d, speed))
}

/// The stamina drain while running: every `STAMINA_FRAMES` frames one point, during the rally (phase 3) of a
/// game with more than one player; clamped to [`floor`, the character's stamina]. Returns (stamina, counter).
pub fn drain(s: &Stats, stamina: i32, counter: i32, players: i32, rally: bool, floor: i32) -> (i32, i32) {
    let c = counter + 1;
    if c < STAMINA_FRAMES {
        return (stamina, c);
    }
    if players < 2 || !rally {
        return (stamina, 0);
    }
    let st = stamina - 1;
    (if st < floor { floor } else { st.min(s.stamina) }, 0)
}

/// Signed angle (radians) from the player's forward (`forward` = ±1 along z) to `dir`, mirrored by `hand`
/// (−1 left-handed). Left-handers' direction is nudged 0.01 sideways first (their model is mirrored).
pub fn angle(dir: [f32; 3], forward: f32, hand: f32) -> f32 {
    let (mut x, z) = (dir[0], dir[2]);
    if hand < 0.0 {
        x = madd(add(0.0, x), f32::from_bits(0x3c23_d70a), if z < 0.0 { -1.0 } else { 1.0 });
    }
    let dot = madd(0.0, forward, z);
    let a = if dot.abs() < f32::from_bits(0x3f7f_ff58) {
        let a = acosf(dot);
        if mul(forward, x) < 0.0 { mul(a, -1.0) } else { a }
    } else if dot <= 0.0 {
        f32::from_bits(0x4049_0fdb)
    } else {
        0.0
    };
    mul(a, hand)
}

const QUARTER: f32 = f32::from_bits(0x3f49_0fdb); // π/4
const THREE_QUARTERS: f32 = f32::from_bits(0x4016_cbe4); // 3π/4

/// Run motion (3 forward, 4 back, 5 left, 6 right, 7 dash) for direction angle `a`, with hysteresis on the run
/// motion already playing (`current`, tired variants folded). `dash`: run counter past half the agility.
pub fn run_motion(current: i32, a: f32, dash: bool) -> i32 {
    let (q, tq) = (QUARTER, THREE_QUARTERS);
    let side = |a: f32| if a < 0.0 { 5 } else { 6 };
    let m = match current {
        5 => {
            if q < a && a <= tq {
                6
            } else if f32::from_bits(0xbf3b_a866) <= a && a < q {
                3
            } else if a < f32::from_bits(0xbf3b_a866) && f32::from_bits(0xc01a_25c1) < a {
                5
            } else {
                4
            }
        }
        6 => {
            if a < -q && -tq <= a {
                5
            } else if -q <= a && a <= f32::from_bits(0x3f3b_a866) {
                3
            } else if f32::from_bits(0x3f3b_a866) < a && a < f32::from_bits(0x401a_25c1) {
                6
            } else {
                4
            }
        }
        4 => {
            if a.abs() <= q {
                3
            } else if a.abs() <= f32::from_bits(0x4013_7207) {
                side(a)
            } else {
                4
            }
        }
        3 => {
            if !(a.abs() < tq) {
                4
            } else if a.abs() < f32::from_bits(0x3f56_7750) {
                3
            } else {
                side(a)
            }
        }
        _ => {
            if !(a.abs() < tq) {
                4
            } else if a.abs() < q {
                3
            } else {
                side(a)
            }
        }
    };
    if dash { 7 } else { m }
}

/// Whether the run counter has passed half the agility (the dash).
pub fn dashing(s: &Stats, run: i32) -> bool {
    s.agility / 2 < run
}

/// Standing motion for facing angle `a`: a quarter turn or more steps (run motions), otherwise `stance()`.
pub fn stand_motion(a: f32, stance: impl FnOnce() -> i32) -> i32 {
    if !(a.abs() < THREE_QUARTERS) {
        4
    } else if a.abs() < QUARTER {
        stance()
    } else if a < 0.0 {
        5
    } else {
        6
    }
}

/// Ready stance: 0 square, 1 / 2 turned to the forehand / backhand side of the ball's line.
pub struct StanceInput {
    pub players: i32,
    /// Match phase (gm+0x55): 3 rally, 4 point over.
    pub phase: u8,
    /// Index of the player who hit the ball last (−1 none).
    pub last_hitter: i32,
    pub team: i32,
    /// Base motion playing now (tired variants folded).
    pub current: i32,
    /// Singles: the player watches the ball (else square).
    pub watching: bool,
    pub pos: [f32; 3],
    pub ball: [f32; 3],
    /// The ball's travel direction.
    pub ball_dir: [f32; 3],
    pub hand: f32,
}

pub fn stance(s: &StanceInput) -> i32 {
    if s.players == 1 && s.phase != 3 {
        return 0;
    }
    if !(s.players < 2 || (s.last_hitter >= 0 && (s.last_hitter & 1) != (s.team & 1))) {
        return 0;
    }
    if s.phase == 4 && (1..=2).contains(&s.current) {
        return s.current;
    }
    if s.players == 1 && !s.watching {
        return 0;
    }
    let (dx, dz) = (crate::ps2::sub(s.pos[0], s.ball[0]), crate::ps2::sub(s.pos[2], s.ball[2]));
    let cross = msub(mul(dz, s.ball_dir[0]), dx, s.ball_dir[2]);
    let left = s.hand < 0.0;
    if (cross > 0.0) != left { 2 } else { 1 }
}

/// The motion number the game stores: motions below 8 have tired variants (+8) when stamina is below 10.
pub fn with_tiredness(motion: i32, stamina: i32) -> i32 {
    if (0..8).contains(&motion) && stamina < TIRED { motion + 8 } else { motion }
}

/// Tired variants folded onto their base motion (8..15 → 0..7).
pub fn base_motion(motion: i32) -> i32 {
    if (8..16).contains(&motion) { motion - 8 } else { motion }
}
