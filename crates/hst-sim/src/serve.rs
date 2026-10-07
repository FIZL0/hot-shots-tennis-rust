//! The original's serve: walking along the baseline, the toss, the contact search when the swing is pressed,
//! the aim inside the service box (and how a mistimed strong toss throws it off), and the timing balloon shown
//! over a hitter's head (strokes too).

use crate::swing::PathPoint;
use crate::shot::{self, Bounds, Table};

/// Toss gravity (the ball's gravity factor × g per frame²).
const TOSS_GRAVITY: f32 = 0.9 * 0.0027222224;
/// Baseline walk per frame, and how close to the centre mark / how far out the server may stand.
pub const WALK: f32 = 0.033333;
pub const WALK_MIN: f32 = 0.7;
pub const WALK_MAX_SINGLES: f32 = 3.4149997;
pub const WALK_MAX_DOUBLES: f32 = 4.7850003;
/// Frames from the toss press until the ball leaves the hand (toss animation frame 45).
pub const TOSS_RELEASE: u32 = 45;
/// The contact frame of every swing (grades and the swing's animation speed are relative to it).
pub const SWEET_FRAME: i32 = 8;

/// The three tosses: strong (overhand, the riskier serve), weak (overhand, forgiving), underhand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toss {
    Strong,
    Weak,
    Under,
}

/// A character's serve heights (TParam.csv) and timing tables.
#[derive(Clone, Debug)]
pub struct ServeData {
    /// Overhand window: highest, ideal, lowest contact height (m).
    pub over: [f32; 3],
    /// Underhand window.
    pub under: [f32; 3],
    /// Timing grade by frames from the swing press (strong toss / weak and underhand); length = horizon.
    pub strong_grades: Vec<u8>,
    pub weak_grades: Vec<u8>,
    /// Hand at the toss and the toss's apex drift, in the player's frame (x toward the racket side mirrored
    /// by facing, y down, z forward), from the toss animation.
    pub hand_over: [f32; 3],
    pub hand_under: [f32; 3],
    pub apex_drift_over: [f32; 2],
    pub apex_drift_under: [f32; 2],
    /// Strong-toss mistiming error (cm): depth, along the stick, random sideways (per skill level).
    pub miss: [i32; 3],
    /// Widest serve angle from straight ahead (degrees).
    pub max_angle: f32,
}

impl ServeData {
    pub fn window(&self, toss: Toss) -> [f32; 3] {
        if toss == Toss::Under { self.under } else { self.over }
    }
    pub fn grades(&self, toss: Toss) -> &[u8] {
        if toss == Toss::Strong { &self.strong_grades } else { &self.weak_grades }
    }
}

/// The ball's launch velocity for a toss from `hand` peaking at `apex` (game space): straight up at
/// √(2gh), drifting to the apex's x/z by the time it gets there.
pub fn toss_velocity(hand: [f32; 3], apex: [f32; 3]) -> [f32; 3] {
    let up = (TOSS_GRAVITY * 2.0 * (hand[1] - apex[1]).abs()).sqrt();
    let t = up / TOSS_GRAVITY;
    [(apex[0] - hand[0]) / t, -up, (apex[2] - hand[2]) / t]
}

/// Where the hand releases the ball and where the toss peaks, for a server at `pos` facing `facing`.
pub fn toss_points(d: &ServeData, toss: Toss, pos: [f32; 3], facing: f32) -> ([f32; 3], [f32; 3]) {
    let (hand, drift) = if toss == Toss::Under { (d.hand_under, d.apex_drift_under) } else { (d.hand_over, d.apex_drift_over) };
    let at = |x: f32, y: f32, z: f32| [pos[0] + x * facing, y, pos[2] + z * facing];
    let h = at(hand[0], hand[1], hand[2]);
    let top = -d.window(toss)[0];
    (h, at(hand[0] + drift[0], top, hand[2] + drift[1]))
}

/// The held ball on the walk and in the toss until release: this point of the left hand's `Bip01 LFinger21`.
pub const HAND_BALL: [f32; 4] = [-0.05, 0.03, -0.05, 1.0];

/// The ball in the server's stance before the toss (game space): the point of the stance's ball track
/// (`*_serve_ad00_ball`) at the motion's sampled time, turned by the server's rows and moved to their spot.
pub fn stance_ball(point: [f32; 4], rows: [[f32; 4]; 3], pos: [f32; 3]) -> [f32; 3] {
    use crate::ps2::add;
    let v = crate::vu0::transform(&[rows[0], rows[1], rows[2], [0.0, 0.0, 0.0, 1.0]], point);
    [add(v[0], pos[0]), add(v[1], pos[1]), add(v[2], pos[2])]
}

/// The serve's contact search when the swing is pressed: over the path before the first bounce, the frame
/// whose height is closest to the ideal within the window (earliest on ties) where the timing table allows a
/// hit. `None` is a whiff.
pub fn search(d: &ServeData, toss: Toss, path: &[PathPoint]) -> Option<usize> {
    let [top, ideal, low] = d.window(toss);
    let grades = d.grades(toss);
    let mut best: Option<(usize, f32)> = None;
    for (k, p) in path.iter().enumerate().take(grades.len()) {
        if p.bounces > 0 {
            break;
        }
        let h = -p.pos[1];
        if grades[k] != 0 && low <= h && h <= top {
            let off = (h - ideal).abs();
            if best.is_none_or(|(_, b)| off < b) {
                best = Some((k, off));
            }
        }
    }
    best.map(|(k, _)| k)
}

/// Where the serve is aimed: the diagonal service box's centre plus the stick (reaching its sidelines and
/// service line), never shorter than 3 m or wider than the character's angle from where the server stands.
/// A mistimed weak or underhand toss shrinks the area; a mistimed strong toss (timing grade 3 or 4) throws the
/// aim deep (late) or short (early) and sideways, which is where faults come from. `side` 0 deuce / 1 ad,
/// `stick` on the court (x, z), `rand_bit` a coin flip for the sideways error when the stick is centred.
/// Returns the aim and that error (the scatter); the ball lands at their sum (see `launch`).
#[allow(clippy::too_many_arguments)]
pub fn target(
    d: &ServeData,
    toss: Toss,
    offset: i32,
    grade: u8,
    server: [f32; 3],
    facing: f32,
    side: i32,
    doubles: bool,
    stick: [f32; 2],
    rand_bit: bool,
) -> ([f32; 3], [f32; 3]) {
    let mistimed = offset.abs() > 1 && toss != Toss::Strong;
    let (range_x, depth) = if mistimed { (1.56, 2.4) } else { (2.0575, 3.4) };
    let range_z = depth / 2.0;
    let mut t = [facing * 2.0575 * if side == 0 { -1.0 } else { 1.0 }, 0.0, (depth / 2.0 + 3.0) * facing];
    // the stick (at most full tilt), its circle stretched onto the square
    let len = (stick[0] * stick[0] + stick[1] * stick[1]).sqrt().min(1.0);
    if len > 0.0 {
        let full = (stick[0] * stick[0] + stick[1] * stick[1]).sqrt();
        let (nx, nz) = (stick[0] / full, stick[1] / full);
        let m = nx.abs().max(nz.abs());
        t[0] += nx / m * len * range_x;
        t[2] += nz / m * len * range_z;
    }
    // no wider than the character's angle
    let max = (d.max_angle + if doubles { 3.0 } else { 0.0 }).max(0.0).to_radians();
    let (dx, dz) = (t[0] - server[0], t[2] - server[2]);
    let cos = (facing * dz / (dx * dx + dz * dz).sqrt()).clamp(-1.0, 1.0);
    if cos.acos() > max {
        let reach = dz.abs() * max.tan();
        t[0] = if server[0] < t[0] { server[0] + reach } else { server[0] - reach };
    }
    // never shorter than 3 m past the net (slid along the line from the server)
    let short = 3.0 - t[2].abs();
    if short > 0.0 {
        let (dx, dz) = (t[0] - server[0], (t[2] - server[2]).abs());
        t[2] = 3.0 * facing;
        if dx != 0.0 {
            t[0] += short * dx / dz;
        }
    }
    if toss == Toss::Strong && (grade == 3 || grade == 4) {
        let sign = if offset < 1 { -1.0 } else { 1.0 };
        let mut e = [0.0, 0.0, sign * d.miss[0] as f32 * 0.01];
        if len > 0.0 {
            let full = (stick[0] * stick[0] + stick[1] * stick[1]).sqrt();
            let k = facing * d.miss[1] as f32 * 0.01 / full;
            e[0] += stick[0] * k;
            e[2] += stick[1] * k;
        } else {
            e[0] += if rand_bit { -1.0 } else { 1.0 } * d.miss[2] as f32 * 0.01;
        }
        return (t, [e[0] * 2.0 / 3.0, 0.0, e[2] * 2.0 / 3.0]);
    }
    (t, [0.0; 3])
}

/// A serve's launch velocity and flight frames, as the original: the trajectory table is looked up as if
/// from the contact point less the scatter toward the aim, and the ball flies to aim + scatter.
pub fn launch(table: &Table, underhand: bool, radius: f32, hit: [f32; 3], aim: [f32; 3], scatter: [f32; 3]) -> ([f32; 3], i32) {
    let l = shot::lookup(table, &Bounds::serve(underhand, radius), [hit[0] - scatter[0], hit[1], hit[2] - scatter[2]], aim);
    let target = [aim[0] + scatter[0], aim[1], aim[2] + scatter[2]];
    (shot::launch(hit, target, l.elevation, l.speed), l.frames)
}

/// The balloon over a hitter's head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Balloon {
    /// Pressed early, bad timing.
    Bunny,
    /// Pressed late, bad timing.
    Turtle,
    /// A clean hit.
    Note,
    /// The sweet spot (within a frame of it).
    Sweet,
}

impl Balloon {
    /// Texture on the disc (`AZUMA/C_EFF/EFFCT.XB0`).
    pub fn texture(self) -> &'static str {
        match self {
            Balloon::Bunny => "fukidasi_A_03.tm2",
            Balloon::Turtle => "fukidasi_A_04.tm2",
            Balloon::Note => "fukidasi_onpu00.tm2",
            Balloon::Sweet => "fukidasi_onpu01.tm2",
        }
    }
}

/// Fade in, hold, fade out (frames), and height above the head (m).
pub const BALLOON_FRAMES: [u32; 3] = [3, 45, 3];
pub const BALLOON_LIFT: f32 = 0.5;
/// Half-size of the balloon (m) up close; it grows only beyond ~38 m from the camera.
pub const BALLOON_SIZE: f32 = 0.3;

/// Which balloon a hit shows: `grade` the timing table's value at the contact frame, `offset` the contact
/// frame minus the sweet frame, `dive` the reaching branch (never the sweet balloon).
pub fn balloon(grade: u8, offset: i32, dive: bool) -> Option<Balloon> {
    match grade {
        1 | 2 if offset.abs() < 2 && !dive => Some(Balloon::Sweet),
        1 | 2 => Some(Balloon::Note),
        3 | 4 if offset < 0 => Some(Balloon::Turtle),
        3 | 4 => Some(Balloon::Bunny),
        _ => None,
    }
}

/// Balloon opacity 0..1 at `age` frames.
pub fn balloon_alpha(age: u32) -> Option<f32> {
    let [fade_in, hold, fade_out] = BALLOON_FRAMES;
    if age < fade_in {
        Some((age + 1) as f32 / fade_in as f32)
    } else if age < fade_in + hold {
        Some(1.0)
    } else if age < fade_in + hold + fade_out {
        Some((fade_in + hold + fade_out - age) as f32 / fade_out as f32)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> ServeData {
        ServeData {
            over: [3.0, 2.5, 2.0],
            under: [1.0, 0.5, 0.0],
            strong_grades: vec![0, 0, 4, 4, 2, 2, 2, 2, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 4, 4, 4],
            weak_grades: vec![0, 0, 4, 4, 2, 2, 2, 2, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 4, 4, 4],
            hand_over: [0.144, -1.546, 0.12],
            hand_under: [0.016, -0.774, 0.271],
            apex_drift_over: [-0.133, 0.15],
            apex_drift_under: [0.184, 0.033],
            miss: [50, 30, 100],
            max_angle: 22.0,
        }
    }

    #[test]
    fn aim_reaches_the_box_lines_and_mistiming_throws_it_out() {
        let d = data();
        let server = [3.0, 0.0, -12.25];
        // centred: the deuce box's centre; full tilt reaches its corner exactly
        assert_eq!(target(&d, Toss::Strong, 0, 1, server, 1.0, 0, false, [0.0, 0.0], false), ([-2.0575, 0.0, 4.7], [0.0; 3]));
        let (corner, _) = target(&d, Toss::Strong, 0, 1, server, 1.0, 0, false, [-0.7071, 0.7071], false);
        assert!((corner[0] + 4.115).abs() < 1e-3 && (corner[2] - 6.4).abs() < 1e-3, "{corner:?}");
        // the same aim with a late strong toss lands past the service line (fault)
        let (aim, e) = target(&d, Toss::Strong, 5, 4, server, 1.0, 0, false, [0.0, 1.0], false);
        assert!(aim[2] + e[2] > 6.4, "{aim:?} {e:?}");
        // a mistimed weak toss stays inside
        let (weak, e) = target(&d, Toss::Weak, 5, 4, server, 1.0, 0, false, [0.0, 1.0], false);
        assert!(weak[2] <= 6.4 && e == [0.0; 3], "{weak:?}");
    }

    #[test]
    fn balloons_by_timing() {
        assert_eq!(balloon(1, 0, false), Some(Balloon::Sweet));
        assert_eq!(balloon(2, -1, false), Some(Balloon::Sweet));
        assert_eq!(balloon(2, 3, false), Some(Balloon::Note));
        assert_eq!(balloon(4, -5, false), Some(Balloon::Turtle));
        assert_eq!(balloon(4, 6, false), Some(Balloon::Bunny));
        assert_eq!(balloon(0, 0, false), None);
    }
}
