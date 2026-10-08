//! The original's serve: walking along the baseline, the toss, the contact search when the swing is pressed,
//! the aim inside the service box (and how a mistimed strong toss throws it off), and the timing balloon shown
//! over a hitter's head (strokes too).

use crate::pose::{Clip, M4, Skeleton, node_world};
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
    /// The ball in the hand at the toss's release and the racket's matrix at contact, in the player's own frame
    /// (overhand / underhand; see `toss_setup`).
    pub hand_over: [f32; 4],
    pub hand_under: [f32; 4],
    pub racket_over: M4,
    pub racket_under: M4,
    /// Strong-toss mistiming error (cm): depth, along the stick, random sideways (per skill level).
    pub miss: [i32; 3],
    /// Depth bias (tenths of a metre, + deep) by frames from the swing press, beside the timing tables.
    pub strong_bias: Vec<i32>,
    pub weak_bias: Vec<i32>,
    /// How far (cm) the contact may stray from the ideal height before it costs depth, and how far beyond that
    /// the cost doubles (TParam Strk GH / Strk NH).
    pub reach: [i32; 2],
    /// A strong toss's short error is multiplied by this (TParam column 54).
    pub short_miss: f32,
    /// Serve power and low-ball power (TParam Serv POW, LOW POW): together they decide whether the weak toss
    /// serves from the `dw1` tables (see `dw1`).
    pub power: i32,
    pub low_power: i32,
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
    pub fn bias(&self, toss: Toss) -> &[i32] {
        if toss == Toss::Strong { &self.strong_bias } else { &self.weak_bias }
    }
}

/// The ball's launch velocity for a toss from `hand` peaking at `apex` (game space): straight up at
/// √(2gh), drifting to the apex's x/z by the time it gets there.
pub fn toss_velocity(hand: [f32; 3], apex: [f32; 3]) -> [f32; 3] {
    let up = (TOSS_GRAVITY * 2.0 * (hand[1] - apex[1]).abs()).sqrt();
    let t = up / TOSS_GRAVITY;
    [(apex[0] - hand[0]) / t, -up, (apex[2] - hand[2]) / t]
}

/// Where the hand releases the ball and where the toss peaks, for a server with matrix `player` (rows, the
/// spot in row 3; mirrored for a left-hander): the hand's point turned into place, and the apex at the window's
/// top, drifted from the hand toward a point 0.7 m up the racket by 1 / (1 + √(drop to the ideal height / rise)).
pub fn toss_points(d: &ServeData, toss: Toss, player: &M4) -> ([f32; 3], [f32; 3]) {
    use crate::ps2::{add, div, sqrt, sub};
    use crate::vu0::transform;
    let (hand, racket) = if toss == Toss::Under { (d.hand_under, &d.racket_under) } else { (d.hand_over, &d.racket_over) };
    let h = transform(player, hand);
    let q = transform(&crate::pose::vmul(racket, player), [0.0, 0.7, 0.0, 1.0]);
    let [top, ideal, _] = d.window(toss);
    let y = -top;
    let k = add(1.0, sqrt(div(sub(-ideal, y), sub(h[1], y))));
    ([h[0], h[1], h[2]], [add(h[0], div(sub(q[0], h[0]), k)), y, add(h[2], div(sub(q[2], h[2]), k))])
}

/// A character's toss hand and racket (`ServeData`), sampled once from its motions with the player at the
/// origin: `HAND_BALL` on the left hand at the toss's frame 45 (`toss`: motion 0x23 overhand, 0x24 underhand)
/// and the `Racket` node's matrix at the swing's contact frame (`swing`: 0x25, 0x26).
pub fn toss_setup(sk: &Skeleton, toss: &Clip, swing: &Clip) -> ([f32; 4], M4) {
    const ID: M4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    let node = |n: &str| sk.names.iter().position(|x| x == n).unwrap_or_else(|| panic!("no {n} node"));
    let hand = crate::vu0::transform(&node_world(sk, &toss.locals(sk, TOSS_RELEASE as f32), node("Bip01LFinger21"), &ID), HAND_BALL);
    (hand, node_world(sk, &swing.locals(sk, SWEET_FRAME as f32), node("Racket"), &ID))
}

/// A serve's aim pulled inside the receiving service box, as the original before the scatter: the box's
/// sidelines (singles, the centre line on the server's own half; `server_x` the serve spot's x) and service line,
/// less a margin of 0.15 m across and 0.1 m along, each scaled by the share of the contact→aim direction along it
/// (unscaled when the serve is `bent`: the character bends it and the special condition holds).
pub fn inside(server_x: f32, hit: [f32; 3], aim: [f32; 3], bent: bool) -> [f32; 3] {
    use crate::ps2::{add, div, madd, mul, sqrt, sub};
    let (mut lo, mut hi) = (-4.115, 4.115);
    if server_x < 0.0 { lo = 0.0 } else { hi = 0.0 }
    let (mut mx, mut mz) = (0.15, 0.1);
    if !bent && (aim[0] < 0.0 && lo < hit[0]) || (0.0 < aim[0] && hit[0] < hi) {
        let (dz, dx) = (sub(aim[2], hit[2]), sub(aim[0], hit[0]));
        let inv = div(1.0, sqrt(madd(mul(dz, dz), dx, dx)));
        mx = mul(mx, mul(dx, inv).abs());
        mz = mul(mz, mul(dz, inv).abs());
    }
    let (x0, x1, z1) = (add(lo, mx), sub(hi, mx), sub(6.4, mz));
    let x = if aim[0] < x0 { x0 } else if x1 < aim[0] { x1 } else { aim[0] };
    let z = if aim[2] < -z1 { -z1 } else if z1 < aim[2] { z1 } else { aim[2] };
    [add(aim[0], sub(x, aim[0])), aim[1], add(aim[2], sub(z, aim[2]))]
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
/// Returns the aim and the swing's error off it (see `scatter`). `rand` are the game's coin flips in draw order.
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
    rand: [bool; 3],
) -> ([f32; 3], Miss) {
    use crate::ps2::{add, div, mul, madd, sqrt};
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
    // aimed close to the centre line with the stick level: a random nudge sideways
    let nudge = if t[0].abs() <= 1.0 && stick[0] == 0.0 { (if rand[1] { 1 } else { -1 }) * if rand[0] { 5 } else { 10 } } else { 0 };
    let mut miss = Miss { side: 0.0, depth: 0.0, nudge };
    if toss == Toss::Strong && (grade == 3 || grade == 4) {
        let sign = if offset < 1 { -1 } else { 1 };
        miss.depth = mul((sign * d.miss[0]) as f32, 0.01);
        let full = sqrt(madd(mul(stick[0], stick[0]), stick[1], stick[1]));
        if full > 0.0 {
            let inv = div(1.0, full);
            let along = |s: f32| mul(mul(mul(mul(s, inv), facing), d.miss[1] as f32), 0.01);
            miss.side = along(stick[0]);
            miss.depth = add(along(stick[1]), miss.depth);
        } else {
            miss.side = mul(((if rand[2] { -1 } else { 1 }) * d.miss[2]) as f32, 0.01);
        }
        miss.side = mul(miss.side, 0.6666667);
        miss.depth = mul(miss.depth, 0.6666667);
    }
    (t, miss)
}

/// A serve's error off its aim, decided at the swing: sideways and depth (m), and a sideways nudge in tenths
/// of a metre.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Miss {
    pub side: f32,
    pub depth: f32,
    pub nudge: i32,
}

/// The serve's depth error at the swing, in tenths of a metre (+ deep): the timing table's bias for the contact
/// frame `k`, and 6 for every 10 cm the contact height `height` strays from the ideal past `reach[0]`, 12 per
/// 10 cm past `reach[1]` more. A clean hit (grade 1 or 2) has none; ±1 m at most.
pub fn depth_error(d: &ServeData, toss: Toss, k: usize, grade: u8, height: f32) -> i32 {
    use crate::ps2::{mul, sub};
    if grade == 1 || grade == 2 {
        return 0;
    }
    let off = sub(height, d.window(toss)[1]);
    let cm = mul(off.abs(), 100.0) as i32 / 10 * 10;
    let [from, step] = d.reach;
    let steps = match cm - from {
        n if n < 0 => 0,
        n if n < step => n / 10,
        n => step / 10 + (n - step) / 10 * 2,
    };
    let e = mul(steps as f32, 6.0) as i32 * if off > 0.0 { 1 } else { -1 };
    (d.bias(toss)[k] + e).clamp(-10, 10)
}

/// Whether a weak toss serves from the `dw1` tables: its −0.5 variant weight, scaled by the share of the serve
/// power the low-ball power covers, must stay below −0.2.
pub fn dw1(d: &ServeData) -> bool {
    d.power >= 1 && -0.5 * ((d.power - (d.power - d.low_power).max(0)) as f32 / d.power as f32) < -0.2
}

/// Where the serve lands off its aim (add it to the aim; see `launch`): the swing's `miss` with the depth
/// `error` of `depth_error` (a strong toss's short error × `short_miss`), × 1.5, sideways and along the line
/// from the contact `hit` to the `aim`.
pub fn scatter(d: &ServeData, toss: Toss, miss: Miss, error: i32, hit: [f32; 3], aim: [f32; 3]) -> [f32; 3] {
    use crate::ps2::{add, div, mul};
    let side = add(div(div(miss.nudge.clamp(-10, 10) as f32, 10.0), 2.0), miss.side);
    let mut depth = add(div(error as f32, 10.0), miss.depth);
    if toss == Toss::Strong && depth < 0.0 {
        depth = mul(depth, d.short_miss);
    }
    scatter_along(side, depth, hit, aim)
}

/// `side` and `depth` (m) × 1.5 across and along the line from `hit` to `aim`, as the original's launch sets
/// out every timing scatter.
pub fn scatter_along(side: f32, depth: f32, hit: [f32; 3], aim: [f32; 3]) -> [f32; 3] {
    use crate::ps2::{div, madd, mul, sqrt, sub};
    use crate::vu0::{cross, normalize, transform};
    if side == 0.0 && depth == 0.0 {
        return [0.0; 3];
    }
    let (dx, dz) = (sub(aim[0], hit[0]), sub(aim[2], hit[2]));
    let inv = div(1.0, sqrt(madd(mul(dx, dx), dz, dz)));
    let up = normalize([0.0, 1.0, 0.0, 0.0]);
    let across = normalize(cross(up, [mul(dx, inv), 0.0, mul(dz, inv), 0.0]));
    let ahead = normalize(cross(across, up));
    let v = transform(&[across, up, ahead, [0.0, 0.0, 0.0, 1.0]], [mul(1.5, side), 0.0, mul(1.5, depth), 0.0]);
    [v[0], v[1], v[2]]
}

/// A serve's launch, as the original: the trajectory table is looked up as if from the contact point less the
/// scatter toward the aim, and the ball flies to aim + scatter. `turn` is the shot record's spin and side angle
/// (radians; only slice serves have a side angle) and whether the server is left-handed (see
/// `shot::launch_turned`), and `bend` the special serve's (`shot::effect`).
pub fn launch(table: &Table, underhand: bool, radius: f32, hit: [f32; 3], aim: [f32; 3], scatter: [f32; 3], turn: (f32, f32, bool), bend: f32) -> shot::Launch {
    let l = shot::lookup(table, &Bounds::serve(underhand, radius), [hit[0] - scatter[0], hit[1], hit[2] - scatter[2]], aim);
    let target = [crate::ps2::add(aim[0], scatter[0]), aim[1], crate::ps2::add(aim[2], scatter[2]), 1.0];
    let (spin, side, lefty) = turn;
    shot::launch_turned(0, [hit[0], hit[1], hit[2], 1.0], target, l.elevation, l.speed, spin, side, bend, lefty, l.frames)
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

/// Fade in, hold, fade out (frames), and the height of the balloon's bottom edge over the neck joint (m).
pub const BALLOON_FRAMES: [u32; 3] = [3, 45, 3];
pub const BALLOON_LIFT: f32 = 0.5;
/// Half-size of the balloon (m) in the middle distance band (it shrinks near the camera, grows beyond ~38 m).
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

/// Balloon opacity 0..1 after `age` frames of the original's pop-up ageing (the first on the frame it comes up, 0
/// before): fade-in 43, 86, then 48 frames opaque, fade-out 85, 42, a last frame at 0, gone (None).
pub fn balloon_alpha(age: u32) -> Option<f32> {
    let [fade_in, hold, fade_out] = BALLOON_FRAMES.map(|x| x as i32);
    let k = age as i32 - 1;
    let a = if k < 0 {
        0
    } else if k < fade_in {
        128 - (fade_in - 1 - k) * 128 / fade_in
    } else if k <= fade_in + 1 + hold {
        128
    } else if k < fade_in + 2 + hold + fade_out {
        (fade_in + 1 + hold + fade_out - k) * 128 / fade_out
    } else {
        return None;
    };
    Some(a as f32 / 128.0)
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
            hand_over: [0.1442, -1.5461, 0.1196, 1.0],
            hand_under: [0.0156, -0.7736, 0.2711, 1.0],
            racket_over: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [-0.0206, -1.5629, 0.1513, 1.0]],
            racket_under: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, -0.5, 0.3, 1.0]],
            miss: [50, 30, 100],
            strong_bias: vec![-8, -6, -4, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 4, 6, 8, 10, 12, 14, 16],
            weak_bias: vec![-8, -6, -4, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 4, 6, 8, 10, 12, 14, 16],
            reach: [40, 20],
            short_miss: 3.0,
            power: 4,
            low_power: 4,
            max_angle: 22.0,
        }
    }

    #[test]
    fn aim_reaches_the_box_lines_and_mistiming_throws_it_out() {
        let d = data();
        let server = [3.0, 0.0, -12.25];
        // centred: the deuce box's centre; full tilt reaches its corner exactly
        assert_eq!(target(&d, Toss::Strong, 0, 1, server, 1.0, 0, false, [0.0, 0.0], [false; 3]), ([-2.0575, 0.0, 4.7], Miss::default()));
        let (corner, _) = target(&d, Toss::Strong, 0, 1, server, 1.0, 0, false, [-0.7071, 0.7071], [false; 3]);
        assert!((corner[0] + 4.115).abs() < 1e-3 && (corner[2] - 6.4).abs() < 1e-3, "{corner:?}");
        // the same aim with a late strong toss lands past the service line (fault)
        let hit = [3.0, -2.5, -12.0];
        let (aim, m) = target(&d, Toss::Strong, 5, 4, server, 1.0, 0, false, [0.0, 1.0], [false; 3]);
        let e = scatter(&d, Toss::Strong, m, depth_error(&d, Toss::Strong, 13, 4, 2.5), hit, aim);
        assert!(aim[2] + e[2] > 6.4, "{aim:?} {e:?}");
        // an early one falls short, the short error tripled
        let (aim, m) = target(&d, Toss::Strong, -5, 4, server, 1.0, 0, false, [0.0, 1.0], [false; 3]);
        let e = scatter(&d, Toss::Strong, m, depth_error(&d, Toss::Strong, 3, 4, 2.5), hit, aim);
        assert!(e[2] < -1.4, "{aim:?} {e:?}");
        // a mistimed weak toss stays inside
        let (weak, m) = target(&d, Toss::Weak, 5, 4, server, 1.0, 0, false, [0.0, 1.0], [false; 3]);
        assert!(weak[2] <= 6.4 && m == Miss::default(), "{weak:?}");
    }

    #[test]
    fn depth_error_by_contact_height() {
        let d = data();
        // 50 cm low: one step past the 40 cm threshold (−6) on the late frame's +2 bias
        assert_eq!(depth_error(&d, Toss::Strong, 13, 4, 2.0), -4);
        // 85 cm high: two steps, then two doubled; capped at 1 m
        assert_eq!(depth_error(&d, Toss::Strong, 13, 4, 3.35), 10);
        // a clean hit has none
        assert_eq!(depth_error(&d, Toss::Strong, 8, 1, 2.0), 0);
        // the weak toss's dw1 variant: low power covering under 40 % of the serve power keeps the base tables
        assert!(dw1(&d));
        assert!(!dw1(&ServeData { power: 10, low_power: 4, ..d }));
    }

    #[test]
    fn balloon_fade() {
        let a: Vec<i32> = (1..).map_while(balloon_alpha).map(|x| (x * 128.0) as i32).collect();
        assert_eq!(a.len(), 53);
        assert_eq!(a[..3], [43, 86, 128]);
        assert!(a[2..50].iter().all(|&x| x == 128));
        assert_eq!(a[50..], [85, 42, 0]);
        assert_eq!(balloon_alpha(0), Some(0.0));
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
