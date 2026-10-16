//! The original's serve: walking along the baseline, the toss, the contact search when the swing is pressed,
//! the aim inside the service box (and how a mistimed strong toss throws it off), and the timing balloon shown
//! over a hitter's head (strokes too).
#![deny(clippy::float_arithmetic)]

use crate::lines::{BOX_MIDDLE, SERVICE_LINE, SINGLES_SIDELINE};
use crate::pose::{Clip, M4, Path, Skeleton, node_world};
use crate::ps2::{F, f32s};
use crate::swing::{PathPoint, grade};
use crate::shot::{self, Bounds, Table, class};
use crate::world::{HALF_PI, IDENTITY};
use std::f32::consts::SQRT_2;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    let up = (F(TOSS_GRAVITY) * 2.0 * (F(hand[1]) - apex[1]).abs()).sqrt();
    let t = up / TOSS_GRAVITY;
    f32s([(F(apex[0]) - hand[0]) / t, -up, (F(apex[2]) - hand[2]) / t])
}

/// Where the hand releases the ball and where the toss peaks, for a server with matrix `player` (rows, the
/// spot in row 3; mirrored for a left-hander): the hand's point turned into place, and the apex at the window's
/// top, drifted from the hand toward a point 0.7 m up the racket by 1 / (1 + √(drop to the ideal height / rise)).
pub fn toss_points(d: &ServeData, toss: Toss, player: &M4) -> ([f32; 3], [f32; 3]) {
    use crate::vu0::transform;
    let (hand, racket) = if toss == Toss::Under { (d.hand_under, &d.racket_under) } else { (d.hand_over, &d.racket_over) };
    let h = transform(player, hand);
    let q = transform(&crate::pose::vmul(racket, player), [0.0, 0.7, 0.0, 1.0]);
    let [top, ideal, _] = d.window(toss);
    let y = (-F(top)).0;
    let k = (1.0 + ((-F(ideal) - y) / (F(h[1]) - y)).sqrt()).0;
    ([h[0], h[1], h[2]], [(h[0] + (F(q[0]) - h[0]) / k).0, y, (h[2] + (F(q[2]) - h[2]) / k).0])
}

/// A character's toss hand and racket (`ServeData`), sampled once from its motions with the player at the
/// origin: `HAND_BALL` on the left hand at the toss's frame 45 (`toss`: motion 0x23 overhand, 0x24 underhand)
/// and the `Racket` node's matrix at the swing's contact frame (`swing`: 0x25, 0x26).
pub fn toss_setup(sk: &Skeleton, toss: &Clip, swing: &Clip) -> ([f32; 4], M4) {
    const ID: M4 = IDENTITY;
    let node = |n: &str| sk.names.iter().position(|x| x == n).unwrap_or_else(|| panic!("no {n} node"));
    let hand = crate::vu0::transform(&node_world(sk, &toss.locals(sk, TOSS_RELEASE as f32), node("Bip01LFinger21"), &ID), HAND_BALL);
    (hand, node_world(sk, &swing.locals(sk, SWEET_FRAME as f32), node("Racket"), &ID))
}

/// A serve's aim pulled inside the receiving service box, as the original before the scatter: the box's
/// sidelines (singles, the centre line on the server's own half; `server_x` the serve spot's x) and service line,
/// less a margin of 0.15 m across and 0.1 m along, each scaled by the share of the contact→aim direction along it
/// (unscaled when the serve is `bent`: the character bends it and the special condition holds).
pub fn inside(server_x: f32, hit: [f32; 3], aim: [f32; 3], bent: bool) -> [f32; 3] {
    let (mut lo, mut hi) = ((-F(SINGLES_SIDELINE)).0, SINGLES_SIDELINE);
    if server_x < 0.0 { lo = 0.0 } else { hi = 0.0 }
    let (mut mx, mut mz) = (0.15, 0.1);
    if !bent && (aim[0] < 0.0 && lo < hit[0]) || (0.0 < aim[0] && hit[0] < hi) {
        let (dz, dx) = ((F(aim[2]) - hit[2]).0, (F(aim[0]) - hit[0]).0);
        let inv = (1.0 / (F(dz) * dz).madd(dx, dx).sqrt()).0;
        mx = (F(mx) * ((F(dx) * inv).0.abs())).0;
        mz = (F(mz) * ((F(dz) * inv).0.abs())).0;
    }
    let (x0, x1, z1) = ((F(lo) + mx).0, (F(hi) - mx).0, (F(SERVICE_LINE) - mz).0);
    let x = if aim[0] < x0 { x0 } else if x1 < aim[0] { x1 } else { aim[0] };
    let z = if aim[2] < -F(z1) { (-F(z1)).0 } else if z1 < aim[2] { z1 } else { aim[2] };
    [(aim[0] + (F(x) - aim[0])).0, aim[1], (aim[2] + (F(z) - aim[2])).0]
}

/// The held ball on the walk and in the toss until release: this point of the left hand's `Bip01 LFinger21`.
pub const HAND_BALL: [f32; 4] = [-0.05, 0.03, -0.05, 1.0];

/// The ball in the server's stance before the toss (game space): the point of the stance's ball track
/// (`*_serve_ad00_ball`) at the motion's sampled time, turned by the server's rows and moved to their spot.
pub fn stance_ball(point: [f32; 4], rows: [[f32; 4]; 3], pos: [f32; 3]) -> [f32; 3] {
    let v = crate::vu0::transform(&[rows[0], rows[1], rows[2], [0.0, 0.0, 0.0, 1.0]], point);
    [(F(v[0]) + pos[0]).0, (F(v[1]) + pos[1]).0, (F(v[2]) + pos[2]).0]
}

/// A character's serve timing table (grades, depth bias) for one toss, as the original builds it at setup: as the
/// rally's (`swing::timing`) from the toss's own counts, but when its leading no-hit frames differ in number from the
/// rally table's (`stroke`), its first 8 frames are the rally table's.
pub fn timing(after: [i32; 3], before: [i32; 3], stroke: &(Vec<u8>, Vec<i32>)) -> (Vec<u8>, Vec<i32>) {
    let (mut g, mut b) = crate::swing::timing(after, before);
    let lead = |g: &[u8]| g.iter().take_while(|&&x| x == 0).count();
    if lead(&g) != lead(&stroke.0) {
        g[..8].copy_from_slice(&stroke.0[..8]);
        b[..8].copy_from_slice(&stroke.1[..8]);
    }
    (g, b)
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
        let h = -F(p.pos[1]);
        if grades[k] != 0 && low <= h && h <= top {
            let off = (h - ideal).abs().0;
            if best.is_none_or(|(_, b)| off < b) {
                best = Some((k, off));
            }
        }
    }
    best.map(|(k, _)| k)
}

/// The AI's serve contact pick: over the path before the first bounce, the frame closest to the ideal height
/// within the window while the ball falls, or while it still rises for a quick serve (never on an underhand toss);
/// earliest on ties, no timing table. The AI presses once the frames to it, plus its serve error, are few enough.
pub fn ai_search(d: &ServeData, toss: Toss, quick: bool, path: &[PathPoint]) -> Option<usize> {
    let quick = quick && toss != Toss::Under;
    // heights are −y; ponytail: the vertical speed is the next frame's rise (the path carries no velocity)
    let path = path.windows(2).map(|w| ((-F(w[0].pos[1])).0, (F(w[0].pos[1]) - w[1].pos[1]).0, w[0].bounces));
    ai_pick(d.window(toss), quick, path)
}

/// The original's serve contact pick over the predicted path, as (height, vertical speed, bounces) per frame, up
/// positive: the frame nearest the window's ideal height (`[top, ideal, low]`) inside it, rising (quick serve,
/// speed ≥ 0) or falling (≤ 0), the first on a tie; nothing from the first bounce on.
pub fn ai_pick(window: [f32; 3], quick: bool, path: impl IntoIterator<Item = (f32, f32, i32)>) -> Option<usize> {
    let [top, ideal, low] = window;
    let mut best: Option<(usize, f32)> = None;
    for (k, (h, vy, bounces)) in path.into_iter().enumerate() {
        if bounces > 0 {
            break;
        }
        if (if quick { vy >= 0.0 } else { vy <= 0.0 }) && low <= h && h <= top {
            let off = (F(h) - ideal).0.abs();
            if best.is_none_or(|(_, b)| off < b) {
                best = Some((k, off));
            }
        }
    }
    best.map(|(k, _)| k)
}

/// Where the serve is aimed: the diagonal service box's centre plus the stick (reaching its sidelines and
/// service line) through the rally aim's core (`shot::aim_from`, branch 0: never shorter than 3 m, no wider than
/// the character's serve angle from where the server stands). A mistimed weak or underhand toss (offset beyond
/// ±1) shrinks the area; a mistimed strong toss (timing grade 3 or 4) throws the aim deep (late) or short (early)
/// and sideways, which is where faults come from. `side` 0 deuce / 1 ad, `stick` on the court (x, z).
/// Returns the aim and the swing's error off it (see `scatter`). `rand` flips the shared generator's coins, drawn
/// only where the game draws them: the aim's centred nudge (5 or 10, then its sign) when aimed within 1 m of the
/// centre with the stick level, then the mistimed strong toss's sideways error when the stick is centred.
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
    mut rand: impl FnMut() -> bool,
) -> ([f32; 3], Miss) {
    let strong = toss == Toss::Strong;
    let sweet = strong || offset.abs() < 2;
    let (width, depth) = if sweet { (BOX_MIDDLE, 3.4) } else { ((F(BOX_MIDDLE) - 0.5).0, (F(3.4) - 1.0).0) };
    let half = (F(depth) / 2.0).0;
    let x = (F(BOX_MIDDLE) * facing).0;
    let base = [if side == 0 { (F(x) * -1.0).0 } else { x }, ((F(3.0) + half) * facing).0];
    let h = shot::Hitter { end: facing, from: [server[0], server[2]], ..Default::default() };
    let s = shot::AimStats { con: [0, 0, d.max_angle as i32], ..Default::default() };
    let t = shot::aim_from(&h, &s, stick, doubles, false, None, base, width, half);
    // aimed close to the centre line with the stick level: a random nudge sideways
    let nudge = if t[0].abs() <= 1.0 && stick[0] == 0.0 {
        let size = if rand() { 5 } else { 10 };
        if rand() { size } else { -size }
    } else {
        0
    };
    let mut miss = Miss { side: 0.0, depth: 0.0, nudge };
    if strong && (grade == grade::OFF || grade == grade::SLOW) {
        let sign = if offset < 1 { -1 } else { 1 };
        miss.depth = (F::ZERO + 0.0).madd(0.01, (sign * d.miss[0]) as f32).0;
        let full = (F(stick[1]) * stick[1]).madd(stick[0], stick[0]).sqrt().0;
        if full > 0.0 {
            let inv = (F(1.0) / full).0;
            let along = |acc: f32, s: f32| (F::ZERO + acc).madd(F(s) * inv * facing * (d.miss[1] as f32), 0.01).0;
            miss.side = along(0.0, stick[0]);
            miss.depth = along(miss.depth, stick[1]);
        } else {
            miss.side = (F::ZERO + 0.0).madd(0.01, ((if rand() { -1 } else { 1 }) * d.miss[2]) as f32).0;
        }
        miss.side = (F(miss.side) * 0.6666667).0;
        miss.depth = (F(miss.depth) * 0.6666667).0;
    }
    (t, miss)
}

/// The strong toss's mistiming error (cm: depth, along the stick, random sideways) for a character (0..13): the
/// game's skill level 0 (characters 0..2, 5), 2 (8..11) or 1 picks a row of `rows` (`hst_data::exe::Game::serve_miss`).
pub fn miss_of(rows: [[i32; 3]; 3], character: usize) -> [i32; 3] {
    rows[match character {
        0..=2 | 5 => 0,
        8..=11 => 2,
        _ => 1,
    }]
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
/// 10 cm past `reach[1]` more. A clean hit (`grade::clean`) has none; ±1 m at most.
pub fn depth_error(d: &ServeData, toss: Toss, k: usize, grade: u8, height: f32) -> i32 {
    if grade::clean(grade) {
        return 0;
    }
    let off = (F(height) - d.window(toss)[1]).0;
    let cm = (F(off.abs()) * 100.0).0 as i32 / 10 * 10;
    let [from, step] = d.reach;
    let steps = match cm - from {
        n if n < 0 => 0,
        n if n < step => n / 10,
        n => step / 10 + (n - step) / 10 * 2,
    };
    let e = (F(steps as f32) * 6.0).0 as i32 * if off > 0.0 { 1 } else { -1 };
    (d.bias(toss)[k] + e).clamp(-10, 10)
}

/// Whether a weak toss serves from the `dw1` tables: its −0.5 variant weight, scaled by the share of the serve
/// power the low-ball power covers, must stay below −0.2.
pub fn dw1(d: &ServeData) -> bool {
    d.power >= 1 && -0.5 * (F((d.power - (d.power - d.low_power).max(0)) as f32) / d.power as f32) < -0.2
}

/// Where the serve lands off its aim (add it to the aim; see `launch`): the swing's `miss` with the depth
/// `error` of `depth_error` (a strong toss's short error × `short_miss`), × 1.5, sideways and along the line
/// from the contact `hit` to the `aim`.
pub fn scatter(d: &ServeData, toss: Toss, miss: Miss, error: i32, hit: [f32; 3], aim: [f32; 3]) -> [f32; 3] {
    let side = (F(miss.nudge.clamp(-10, 10) as f32) / 10.0 / 2.0 + miss.side).0;
    let mut depth = (F(error as f32) / 10.0 + miss.depth).0;
    if toss == Toss::Strong && depth < 0.0 {
        depth = (F(depth) * d.short_miss).0;
    }
    scatter_along(side, depth, hit, aim)
}

/// `side` and `depth` (m) × 1.5 across and along the line from `hit` to `aim`, as the original's launch sets
/// out every timing scatter.
pub fn scatter_along(side: f32, depth: f32, hit: [f32; 3], aim: [f32; 3]) -> [f32; 3] {
    use crate::vu0::{cross, normalize, transform};
    if side == 0.0 && depth == 0.0 {
        return [0.0; 3];
    }
    let (dx, dz) = ((F(aim[0]) - hit[0]).0, (F(aim[2]) - hit[2]).0);
    let inv = (1.0 / (F(dx) * dx).madd(dz, dz).sqrt()).0;
    let up = normalize([0.0, 1.0, 0.0, 0.0]);
    let across = normalize(cross(up, [(F(dx) * inv).0, 0.0, (F(dz) * inv).0, 0.0]));
    let ahead = normalize(cross(across, up));
    let v = transform(&[across, up, ahead, [0.0, 0.0, 0.0, 1.0]], [(F(1.5) * side).0, 0.0, (F(1.5) * depth).0, 0.0]);
    [v[0], v[1], v[2]]
}

/// A serve's launch, as the original: the trajectory table is looked up as if from the contact point less the
/// scatter toward the aim, and the ball flies to aim + scatter. `turn` is the shot record's spin and side angle
/// (radians; only slice serves have a side angle) and whether the server is left-handed (see
/// `shot::launch_turned`), and `bend` the special serve's (`shot::effect`).
pub fn launch(table: &Table, underhand: bool, radius: f32, hit: [f32; 3], aim: [f32; 3], scatter: [f32; 3], turn: (f32, f32, bool), bend: f32) -> shot::Launch {
    let l = shot::lookup(table, &Bounds::serve(underhand, radius), [(F(hit[0]) - scatter[0]).0, hit[1], (F(hit[2]) - scatter[2]).0], aim);
    let target = [(F(aim[0]) + scatter[0]).0, aim[1], (F(aim[2]) + scatter[2]).0, 1.0];
    let (spin, side, lefty) = turn;
    shot::launch_turned(class::SERVE, [hit[0], hit[1], hit[2], 1.0], target, l.elevation, l.speed, spin, side, bend, lefty, l.frames)
}

/// The balloon over a hitter's head.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    Some((F(a as f32) / 128.0).0)
}

/// The serve's (and smash's) arm table entry, built at player load from an overhead (smash 0x1f, serve 0x25) or the
/// underhand serve motion (0x26) posed at frame 8.
#[derive(Clone, Debug)]
pub struct ServeArm {
    /// Racket, right hand, right forearm and right upper arm locals (the upper arm's translation zeroed).
    pub racket: M4,
    pub hand: M4,
    pub forearm: M4,
    pub upper: M4,
    /// Model matrix of the upper arm's parent (translation zeroed).
    pub chain: M4,
    /// The upper arm turned so the racket points along the solve's axis (translation zeroed).
    pub aimed: M4,
    /// The shoulder (the upper arm's model translation).
    pub shoulder: [f32; 4],
    /// Shoulder to racket point.
    pub radius: f32,
    /// The swing's root path (`*_serve_t_dummy`, `*_s_n_dummy` for the smash) height at frame 8: the jump is scaled
    /// by it (0 without one: no jump scale).
    pub lift: f32,
    /// The racket point in the model at frame 8; the smash's z is the contact search's `Reach::smash_ahead`.
    pub point: [f32; 4],
}

/// The overhead arm's last turn about z (135°).
const THREE_QUARTER_PI: f32 = f32::from_bits(0x4016_cbe4);
/// The arm solve's bend passes stop with the racket point within 0.1 m (squared distance), or when the joint
/// already points at the contact (cos ≥ 0.99999); one pass bends a joint 30° at most (cos).
const CCD_DONE: f32 = f32::from_bits(0x3c23_d70a);
const CCD_ALIGNED: f32 = f32::from_bits(0x3f7f_ff58);
const CCD_MAX_BEND: f32 = f32::from_bits(0x3f5d_b3d7);

/// The entry for `clip`: the racket point turned about x into the y–z plane's −y (overhead) or +z (underhand)
/// from the shoulder, then the arm turned 135° about z (overhead) or 90° about y (underhand).
pub fn serve_arm(sk: &Skeleton, clip: &Clip, jump: Option<&Path>, under: bool) -> ServeArm {
    use crate::pose::{ARM_NODES, axis_angle, transpose3, vmul};
    use crate::vu0::transform;
    let idx = ARM_NODES.map(|n| sk.names.iter().position(|x| x == n).unwrap_or_else(|| panic!("no node {n}")));
    let local = clip.locals(sk, clip.wrap(8.0, false));
    let [racket, hand, forearm, mut upper, _] = idx.map(|n| local[n]);
    let mut chain: M4 = IDENTITY;
    let mut p = sk.parent[idx[3]];
    while let Some(i) = p {
        chain = vmul(&chain, &local[i]);
        p = sk.parent[i];
    }
    let tip = [racket, hand, forearm, upper, chain].iter().fold(crate::pose::TIP, |v, m| transform(m, v));
    let m = vmul(&upper, &chain);
    let sh = m[3];
    let d: [f32; 4] = std::array::from_fn(|k| (F(tip[k]) - sh[k]).0);
    let radius = (F(d[1]) * d[1]).madd(d[0], d[0]).madd(d[2], d[2]).sqrt().0;
    let inv = (1.0 / (F(d[2]) * d[2]).madd(d[1], d[1]).sqrt()).0;
    let (ny, nz) = ((F(d[1]) * inv).0, (F(d[2]) * inv).0);
    let (dot, cross) = if under {
        ((F(ny) * 0.0).madd(0.0, 0.0).madd(nz, 1.0).0, (F(ny) * 1.0).msub(nz, 0.0).0)
    } else {
        ((F(ny) * -1.0).madd(0.0, 0.0).madd(nz, 0.0).0, (F(ny) * 0.0).msub(nz, -1.0).0)
    };
    let mut angle = crate::libm::acosf(dot.clamp(-1.0, 1.0));
    if cross < 0.0 {
        angle = (F(angle) * -1.0).0;
    }
    let twist = if under { axis_angle(HALF_PI, [0.0, 1.0, 0.0]) } else { axis_angle(THREE_QUARTER_PI, [0.0, 0.0, 1.0]) };
    let mut aimed = vmul(&vmul(&vmul(&m, &axis_angle(angle, [1.0, 0.0, 0.0])), &twist), &transpose3(&chain));
    let zero_t = [0.0, 0.0, 0.0, 1.0];
    (aimed[3], upper[3], chain[3]) = (zero_t, zero_t, zero_t);
    ServeArm { racket, hand, forearm, upper, chain, aimed, shoulder: sh, radius, lift: jump.map_or(0.0, |p| p.at(8.0)[1]), point: tip }
}

/// The overhead serve's (or smash's) contact solve at the swing press: the arm turns about x to face the contact
/// point; the body steps forward or back what the arm can't reach (z beyond radius/√2 or behind the shoulder) and
/// jumps what the arm can't reach up; the forearm and upper arm bend in turn (up to 50 passes, CCD), then the hand,
/// with the body stepping sideways half the remaining x once. Returns the solve and the jump's scale on the swing's
/// root height. `contact`, `pos` are world points, `scale` the player's two size factors.
pub fn overhead_solve(a: &ServeArm, contact: [f32; 4], pos: [f32; 4], scale: [f32; 2]) -> (crate::pose::ArmSolve, f32) {
    use crate::pose::{axis_cos, transpose3};
    use crate::vu0::transform;
    let d: [f32; 4] = std::array::from_fn(|k| (F(contact[k]) - pos[k]).0);
    let s01 = (F(scale[0]) * scale[1]).0;
    let sh = a.shoulder;
    let mut v = [(F(d[0]) * s01 - sh[0]).0, (F(d[1]) - sh[1]).0, (F(d[2]) * scale[0] - sh[2]).0, (F(d[3]) - sh[3]).0];
    let mut step = [0.0f32; 4];
    let lim = (F(a.radius) / SQRT_2).0;
    if v[2] < 0.0 {
        step[2] = v[2];
    } else if !(v[2] <= lim) {
        step[2] = (F(v[2]) - lim).0;
    }
    v[2] = (F(v[2]) - step[2]).0;
    let ax = if v[0].abs() <= lim { (F(a.radius) * a.radius).msub(v[0], v[0]).sqrt().0 } else { v[0].abs() };
    let up = (F(v[1]) + ax).0;
    let jump = if up <= 0.0 { (-F(up)).0 } else { 0.0 };
    v[1] = (F(v[1]) + jump).0;
    let jump_scale = (-F(jump) / a.lift).0;
    // face the point: turn about x taking −y onto it
    let inv = (1.0 / (F(v[2]) * v[2]).madd(v[1], v[1]).sqrt()).0;
    let (ny, nz) = ((F(v[1]) * inv).0, (F(v[2]) * inv).0);
    let dot = (F(ny) * -1.0).madd(0.0, 0.0).madd(nz, 0.0).0;
    let turn = if (F(ny) * 0.0).msub(nz, -1.0).0 < 0.0 { -1.0 } else { 1.0 };
    let r0 = axis_cos(dot.clamp(-1.0, 1.0), turn, [1.0, 0.0, 0.0]);
    let v = transform(&r0, v);
    let solve = arm_ccd(a, v, transpose3(&r0), [0, 1], false);
    let mut solve = solve;
    solve.step[2] = (F(step[2]) * scale[0]).0;
    solve.step[0] = (F(solve.step[0]) * s01).0;
    (solve, jump_scale)
}

/// The underhand serve's solve: the same facing turn about x (onto +z), bends about y in the x–z plane and one hand
/// pass; the body doesn't move (zero step).
pub fn underhand_solve(a: &ServeArm, contact: [f32; 4], pos: [f32; 4], scale: [f32; 2]) -> crate::pose::ArmSolve {
    use crate::pose::{axis_cos, transpose3};
    use crate::vu0::transform;
    let d: [f32; 4] = std::array::from_fn(|k| (F(contact[k]) - pos[k]).0);
    let sh = a.shoulder;
    let v = [(d[0] * (F(scale[0]) * scale[1]) - sh[0]).0, (F(d[1]) - sh[1]).0, (F(d[2]) * scale[0] - sh[2]).0, (F(d[3]) - sh[3]).0];
    let inv = (1.0 / (F(v[2]) * v[2]).madd(v[1], v[1]).sqrt()).0;
    let (ny, nz) = ((F(v[1]) * inv).0, (F(v[2]) * inv).0);
    let dot = (F(ny) * 0.0).madd(0.0, 0.0).madd(nz, 1.0).0;
    let turn = if (F(ny) * 1.0).msub(nz, 0.0).0 < 0.0 { -1.0 } else { 1.0 };
    let r0 = axis_cos(dot.clamp(-1.0, 1.0), turn, [1.0, 0.0, 0.0]);
    let mut s = arm_ccd(a, transform(&r0, v), transpose3(&r0), [0, 2], true);
    s.step = [0.0; 4];
    s
}

/// Unit (a − b) in the plane of components `c`.
fn unit_in(a: [f32; 4], b: [f32; 4], c: [usize; 2]) -> [f32; 2] {
    let (x, y) = ((F(a[c[0]]) - b[c[0]]).0, (F(a[c[1]]) - b[c[1]]).0);
    let inv = (1.0 / (F(y) * y).madd(x, x).sqrt()).0;
    [(F(x) * inv).0, (F(y) * inv).0]
}

/// The bend shared by both serve solves, toward `v` (already in the facing frame `t0` undoes) in the plane of
/// components `c` (x–y, turning about z; x–z, turning about y): forearm and upper arm in turn up to 50 passes
/// (each within 30°), then the hand (only if within 30°) — twice with the sideways step in between (overhead), or
/// once (underhand). Leaves the step's raw x (unscaled) in `step[0]`.
fn arm_ccd(a: &ServeArm, mut v: [f32; 4], t0: M4, c: [usize; 2], under: bool) -> crate::pose::ArmSolve {
    use crate::pose::{TIP, axis_cos, keep_row3, transpose3, vmul};
    use crate::vu0::transform;
    let axis = if under { [0.0, 1.0, 0.0] } else { [0.0, 0.0, 1.0] };
    // the plane's dot and the turn's sign (the cross product's out-of-plane component)
    let dot_of = |u: [f32; 2], w: [f32; 2]| (F(u[1]) * w[1]).madd(u[0], w[0]).madd(0.0, 0.0).0;
    let cross_of = |u: [f32; 2], w: [f32; 2]| if under { (0.0 + F(u[1]) * w[0]).msub(u[0], w[1]).0 } else { (0.0 + F(u[0]) * w[1]).msub(u[1], w[0]).0 };
    let (chain, mut hand, mut forearm, mut aimed) = (a.chain, a.hand, a.forearm, a.aimed);
    let at_hand = if under { transform(&hand, transform(&a.racket, TIP)) } else { transform(&vmul(&a.racket, &hand), TIP) };
    let tip = |forearm: &M4, aimed: &M4| transform(&chain, transform(aimed, transform(forearm, at_hand)));
    let dist = |v: [f32; 4], p: [f32; 4]| {
        let (dx, dy) = ((F(v[c[0]]) - p[c[0]]).0, (F(v[c[1]]) - p[c[1]]).0);
        (F(dy) * dy).madd(dx, dx).0
    };
    let mut p = tip(&forearm, &aimed);
    let mut elbow = true;
    for _ in 0..50 {
        if dist(v, p) <= CCD_DONE {
            break;
        }
        let (m, frame) = if elbow { (vmul(&vmul(&forearm, &aimed), &chain), vmul(&aimed, &chain)) } else { (vmul(&aimed, &chain), chain) };
        let (u, w) = (unit_in(p, m[3], c), unit_in(v, m[3], c));
        let dot = dot_of(u, w);
        if !(dot < CCD_ALIGNED) {
            break;
        }
        let cos = if dot < CCD_MAX_BEND { CCD_MAX_BEND } else { dot };
        let s = if dot < 0.0 || cross_of(u, w) < 0.0 { -1.0 } else { 1.0 };
        let bent = vmul(&vmul(&m, &axis_cos(cos, s, axis)), &transpose3(&frame));
        let j = if elbow { &mut forearm } else { &mut aimed };
        *j = keep_row3(bent, j);
        p = tip(&forearm, &aimed);
        elbow = !elbow;
    }
    let frame = vmul(&vmul(&forearm, &aimed), &chain);
    let mut step = [0.0f32; 4];
    for k in 0..2 {
        let m = vmul(&hand, &frame);
        let (u, w) = (unit_in(p, m[3], c), unit_in(v, m[3], c));
        let dot = dot_of(u, w);
        if !(dot < CCD_MAX_BEND) {
            let cr = if under { (F(u[1]) * w[0]).msub(u[0], w[1]).0 } else { (F(u[0]) * w[1]).msub(u[1], w[0]).0 };
            let bent = vmul(&vmul(&m, &axis_cos(dot.clamp(-1.0, 1.0), if cr < 0.0 { -1.0 } else { 1.0 }, axis)), &transpose3(&frame));
            hand = keep_row3(bent, &hand);
        }
        if k == 1 || under {
            break;
        }
        p = transform(&frame, transform(&hand, transform(&a.racket, TIP)));
        if v[0].abs() <= p[0].abs() {
            break;
        }
        step[0] = ((F(v[0]) - p[0]) / 2.0).0;
        v[0] = (F(v[0]) - step[0]).0;
    }
    // back out of the facing turn
    aimed = vmul(&vmul(&vmul(&aimed, &chain), &t0), &transpose3(&chain));
    let q = |pose: &M4, m: &M4| crate::quat::from_matrix(&vmul(&transpose3(pose), m));
    crate::pose::ArmSolve { step, quats: [q(&a.hand, &hand), q(&a.forearm, &forearm), q(&a.upper, &aimed), [0.0, 0.0, 0.0, 1.0]] }
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
        assert_eq!(target(&d, Toss::Strong, 0, 1, server, 1.0, 0, false, [0.0, 0.0], || false), ([-2.0575, 0.0, 4.7], Miss::default()));
        let (corner, _) = target(&d, Toss::Strong, 0, 1, server, 1.0, 0, false, [-std::f32::consts::FRAC_1_SQRT_2, std::f32::consts::FRAC_1_SQRT_2], || false);
        assert!((corner[0] + 4.115).abs() < 1e-3 && (corner[2] - 6.4).abs() < 1e-3, "{corner:?}");
        // the same aim with a late strong toss lands past the service line (fault)
        let hit = [3.0, -2.5, -12.0];
        let (aim, m) = target(&d, Toss::Strong, 5, 4, server, 1.0, 0, false, [0.0, 1.0], || false);
        let e = scatter(&d, Toss::Strong, m, depth_error(&d, Toss::Strong, 13, 4, 2.5), hit, aim);
        assert!(aim[2] + e[2] > 6.4, "{aim:?} {e:?}");
        // an early one falls short, the short error tripled
        let (aim, m) = target(&d, Toss::Strong, -5, 4, server, 1.0, 0, false, [0.0, 1.0], || false);
        let e = scatter(&d, Toss::Strong, m, depth_error(&d, Toss::Strong, 3, 4, 2.5), hit, aim);
        assert!(e[2] < -1.4, "{aim:?} {e:?}");
        // a mistimed weak toss stays inside
        let (weak, m) = target(&d, Toss::Weak, 5, 4, server, 1.0, 0, false, [0.0, 1.0], || false);
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
