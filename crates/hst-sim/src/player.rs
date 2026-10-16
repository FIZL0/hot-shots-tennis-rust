//! Player locomotion as the original: run speed (character speed, acceleration and stamina), the per-frame run
//! counter and stamina drain, and the motion each standing or running player plays (ready stance, turned
//! stances toward the ball, runs by direction relative to the player's forward, dash, tired variants).
#![deny(clippy::float_arithmetic)]

use crate::libm::acosf;
use crate::motion::id;
use crate::ps2::{F, chop, f, f32s, fs};
use crate::rally::phase;
use crate::shot::branch;
use crate::weather;
use std::f32::consts::{FRAC_PI_4, PI, TAU};

/// Character numbers (`pc00`…`pc13`, the disc's name sheet order) the sim treats specially.
pub mod character {
    pub const JJ: i32 = 5;
    pub const BIG_CHIEF: i32 = 7;
    pub const WILL: i32 = 11;
    pub const SUZUKI: i32 = 13;
}

/// A character's movement stats (TParam.csv: SPE, Agili, STA, and the stamina a dive / backhand / smash costs).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    /// SPE / 10.
    pub speed: f32,
    /// Frames of running until full speed (Agili; ×1.5 in the rain).
    pub agility: i32,
    /// Stamina at the start of every point (STA).
    pub stamina: i32,
    /// Stamina spent by a dive, a backhand ground stroke and a smash.
    pub dive: i32,
    pub backhand: i32,
    pub smash: i32,
}

impl Stats {
    /// From TParam.csv's SPE, Agili, STA and dive/backhand/smash cost cells; `weather` is the court's weather
    /// state (`crate::weather`, from the match's weather table).
    pub fn new(spe: i32, agility: i32, stamina: i32, [dive, backhand, smash]: [i32; 3], weather: u8) -> Self {
        // the game's integer ×150/100 in rain and heavy rain
        let agility = if (weather::RAIN..=weather::HEAVY_RAIN).contains(&weather) { agility * 150 / 100 } else { agility };
        // the FPU's int-to-float chops too (only shows past 2^24)
        Stats { speed: (F(chop(spe as f64)) / 10.0).0, agility, stamina, dive, backhand, smash }
    }
}

/// A character's reach and contact heights (m) and body-shot adjustments, as the game's TParam.csv parser leaves
/// them in the character record (and copies to the player).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReachStats {
    /// Body ADJ, Vbdy ADJ, Rizing ADJ (percent; Back ADJ and Run CON aren't read).
    pub body_adj: i32,
    pub vbody_adj: i32,
    pub rising_adj: i32,
    /// SM LOW POW, Strk HI POW.
    pub smash_low_pow: i32,
    pub stroke_high_pow: i32,
    /// Serve near-miss scatter factor.
    pub serve_scatter: f32,
    /// Height of the reach centre.
    pub base: f32,
    /// Horizontal reach: the reach "clog" plus the reach cell.
    pub reach: f32,
    /// Lowest underhand (scoop) contact.
    pub under_min: f32,
    /// Ideal contact height: ground strokes, volleys.
    pub stroke_height: f32,
    pub volley_height: f32,
    /// Smash, overhand serve and underhand serve windows: highest, ideal, lowest.
    pub smash: [f32; 3],
    pub serve: [f32; 3],
    pub under_serve: [f32; 3],
    /// Distance a far dive starts at, and the dive's limit.
    pub dive_start: f32,
    pub dive_limit: f32,
    /// Sideways body-collision size (the second half of the collision cell).
    pub collision: f32,
    /// Serv, Strk, Voley, Lob and Lob2 POW.
    pub power: [i32; 5],
    /// Frames of each timing grade 2, 3, 4 after the sweet frame (the "rabbit" cell) and before it ("turtle").
    pub after: [i32; 3],
    pub before: [i32; 3],
    /// Contact height error (cm) a ground stroke and a volley shrug off, and past which it counts double.
    pub stroke_miss: [i32; 2],
    pub volley_miss: [i32; 2],
    /// Shot-mode pressure of a body shot, a backhand ground stroke and a mistimed volley (Body/Back/V DWN).
    pub body_down: i32,
    pub back_down: i32,
    pub volley_down: i32,
    /// LOW POW, V LOW POW.
    pub low_power: [i32; 2],
}

impl ReachStats {
    /// From a character's TParam.csv row. The game splits it with strtok (empty cells vanish, so the disc's empty
    /// Special POW cell shifts every later column down one token) and '/' inside a cell; centimetre cells are
    /// int ÷ 100, the scatter factor and the reach centre go through its own decimal reader.
    pub fn from_tparam(row: &str) -> Self {
        let t: Vec<&str> = row.split(',').filter(|c| !c.is_empty()).map(str::trim).collect();
        let part = |k: usize, i: usize| t[k].split('/').nth(i).unwrap_or("");
        let int = |s: &str| atoi(s);
        let m = |s: &str| (F(atoi(s) as f32) / 100.0).0;
        let three = |k: usize| [0, 1, 2].map(|i| m(part(k, i)));
        ReachStats {
            body_adj: int(t[22]),
            vbody_adj: int(t[23]),
            rising_adj: int(t[25]),
            smash_low_pow: int(t[51]),
            stroke_high_pow: int(t[52]),
            serve_scatter: atof(t[53]),
            base: atof(t[54]),
            reach: (F(m(t[55])) + m(t[56])).0,
            dive_start: m(t[57]),
            dive_limit: m(t[58]),
            collision: m(part(60, 1)),
            under_min: m(t[61]),
            stroke_height: m(part(62, 0)),
            volley_height: m(part(62, 1)),
            smash: three(63),
            serve: three(64),
            under_serve: three(65),
            power: [11, 12, 13, 14, 15].map(|k| int(t[k])),
            after: [0, 1, 2].map(|i| int(part(27, i))),
            before: [0, 1, 2].map(|i| int(part(28, i))),
            stroke_miss: [int(t[33]), int(t[34])],
            volley_miss: [int(t[36]), int(t[37])],
            body_down: int(t[44]),
            back_down: int(t[45]),
            volley_down: int(t[47]),
            low_power: [int(t[49]), int(t[50])],
        }
    }
}

/// C atoi: optional sign, then digits up to the first other character.
fn atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let (neg, s) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let n = s.bytes().take_while(u8::is_ascii_digit).fold(0i32, |n, d| n.wrapping_mul(10).wrapping_add((d - b'0') as i32));
    if neg { n.wrapping_neg() } else { n }
}

/// The game's decimal reader: fraction digits after the '.' at 0.1, 0.01… (each weight the last ÷ 10), then the
/// integer digits right to left at 1, 10…, each added as acc + digit·weight. No sign.
fn atof(s: &str) -> f32 {
    let b = s.as_bytes();
    let dot = b.iter().position(|&c| c == b'.');
    let mut v = F::ZERO;
    if let Some(d) = dot {
        let mut w = f(0.1);
        for &c in b[d + 1..].iter().take_while(|c| c.is_ascii_digit()) {
            v = (v + 0.0).madd((c - b'0') as f32, w);
            w /= 10.0;
        }
    }
    let mut w = F::ONE;
    for &c in b[..dot.unwrap_or(b.len())].iter().rev().take_while(|c| c.is_ascii_digit()) {
        v = (v + 0.0).madd((c - b'0') as f32, w);
        w *= 10.0;
    }
    v.0
}

/// Stamina after a stroke's contact (during the rally of a game with more than one player): ground strokes
/// cost the backhand value on the backhand side, dives and smashes theirs, volleys nothing.
pub fn stroke_stamina(s: &Stats, stamina: i32, branch: u8, forehand: bool, floor: i32) -> i32 {
    let cost = match branch {
        branch::GROUND if !forehand => s.backhand,
        branch::DIVE => s.dive,
        branch::SMASH => s.smash,
        _ => return stamina,
    };
    let st = stamina - cost;
    if st < floor { floor } else { st.min(s.stamina) }
}

/// Court bounds of the mover: |x| ≤ 8.685 and 1.5..17.885 deep on the player's own half.
const SIDE: f32 = 8.685;
const BACK: f32 = 17.885;
const NET: f32 = 1.5;

fn on_court(x: f32, z: f32) -> bool {
    x.abs() <= SIDE && z.abs() <= BACK && !(z.abs() < NET)
}

/// Push `cur` out to 1 m from the doubles partner at `mate` (`None`: no push). Returns the pushed position, or
/// with `fallback` the nearest on-court point of four around the partner (else `restore`) when the push leaves
/// the court; `None` when the push leaves the court and there is no fallback.
fn push(cur: [f32; 3], mate: [f32; 3], restore: [f32; 3], fallback: bool) -> Option<[f32; 3]> {
    let (m, r) = (fs(mate), fs(restore));
    let (dx, dz) = (F(cur[0]) - m[0], F(cur[2]) - m[2]);
    let d2 = (dz * dz).madd(dx, dx);
    if !(d2 < 1.0) {
        return Some(cur);
    }
    let inv = 1.0 / d2.sqrt();
    let out = f32s([m[0] + dx * inv, m[1] + 0.0, m[2] + dz * inv]);
    if on_court(out[0], out[2]) {
        return Some(out);
    }
    if !fallback {
        return None;
    }
    let (sx, sz) = (F::ONE.msub(dz, dz).sqrt(), F::ONE.msub(dx, dx).sqrt());
    let cands = [(m[0] - sx, r[2]), (m[0] + sx, r[2]), (r[0], m[2] - sz), (r[0], m[2] + sz)].map(|(x, z)| (x.0, z.0));
    let mut best: Option<((f32, f32), F)> = None;
    for c in cands.into_iter().filter(|c| on_court(c.0, c.1)) {
        let (ex, ez) = (c.0 - r[0], c.1 - r[2]);
        let d = (ez * ez).madd(ex, ex);
        if best.is_none_or(|(_, b)| d < b) {
            best = Some((c, d));
        }
    }
    Some(best.map_or(restore, |((x, z), _)| [x, restore[1], z]))
}

/// The game's mover: keep 1 m from the doubles partner (`mate`, its position as of this player's update), step
/// by `delta`, clamp to the court bounds (`forward` = ±1 the way the player faces), and keep 1 m from the partner
/// again. `short`: the half-court singles mode (z within 6.4 of the net).
pub fn mover(pos: [f32; 3], delta: [f32; 3], forward: f32, mate: Option<[f32; 3]>, short: bool) -> [f32; 3] {
    let mut cur = pos;
    if let Some(m) = mate {
        cur = push(cur, m, pos, true).unwrap_or(pos);
    }
    let saved = cur;
    cur = [0, 1, 2].map(|i| (F(cur[i]) + delta[i]).0);
    if !(cur[0].abs() <= SIDE) {
        cur[0] = (F(SIDE) * if cur[0] < 0.0 { -1.0 } else { 1.0 }).0;
    }
    if !(cur[2].abs() <= BACK) {
        cur[2] = (-F(BACK) * forward).0;
    }
    if cur[2].abs() < NET {
        cur[2] = (-F(NET) * forward).0;
    }
    if short {
        let lim = F(crate::lines::SERVICE_LINE);
        if forward < 0.0 {
            if !(cur[2] <= lim) {
                cur[2] = lim.0;
            }
        } else if cur[2] < -lim {
            cur[2] = (-lim).0;
        }
    }
    if let Some(m) = mate {
        cur = push(cur, m, saved, false).unwrap_or(saved);
    }
    [cur[0], pos[1], cur[2]]
}

/// Base run speed, metres per frame.
pub const RUN: f32 = 0.0717024;
/// The run's acceleration: up to this much faster at full speed (agility frames in).
const RUN_GAIN: f32 = 0.3;
/// A tired player's run loses this much per stamina point below `TIRED`.
const TIRED_LOSS: f32 = 0.03;
/// Stamina below this plays the tired motions (+8) and slows the run by 3 % per missing point.
pub const TIRED: i32 = 10;
/// Frames of running per stamina point (while the rally is live, more than one player).
pub const STAMINA_FRAMES: i32 = 60;

/// Speed of a running player, metres per frame: `run` frames into the run, with `stamina` left. `size` is the
/// player's size percentage (100 in normal play).
pub fn run_speed(s: &Stats, run: i32, stamina: i32, size: i32) -> f32 {
    let frac = F(run.min(s.agility) as f32) / s.agility as f32;
    let base = RUN * F::ONE.madd(RUN_GAIN, frac);
    let scale = if stamina < TIRED {
        F::ONE.msub(TIRED_LOSS, (TIRED - stamina) as f32) * size as f32 / 100.0
    } else {
        F(size as f32) / 100.0
    };
    (base * s.speed * scale).0
}

/// Velocity for a unit run direction (game space), as the game scales it.
pub fn run_velocity(dir: [f32; 3], speed: f32) -> [f32; 3] {
    dir.map(|d| (F(d) * speed).0)
}

/// The stamina drain while running: every `STAMINA_FRAMES` frames one point, during the rally of a
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
    let (mut x, z) = (F(dir[0]), dir[2]);
    if hand < 0.0 {
        x = (F::ZERO + x).madd(LEFTY_NUDGE, if z < 0.0 { -1.0 } else { 1.0 });
    }
    let dot = F::ZERO.madd(forward, z);
    let a = if dot.abs() < ALIGNED {
        let a = F(acosf(dot.0));
        if F(forward) * x < 0.0 { a * -1.0 } else { a }
    } else if dot <= 0.0 {
        F(PI)
    } else {
        F::ZERO
    };
    (a * hand).0
}

/// Left-handers' run direction is nudged this far sideways (their model is mirrored).
const LEFTY_NUDGE: f32 = 0.01;
/// |cos| at or past this counts as straight ahead or behind (no acos).
const ALIGNED: f32 = 0.99999;

const QUARTER: f32 = FRAC_PI_4;
const THREE_QUARTERS: f32 = 2.3561945; // 3π/4
/// The run motions' hysteresis: a side run holds to 42° ahead and 138° behind, a back run turns sideways only
/// inside 132°, a forward run holds to 48°.
const SIDE_AHEAD: f32 = 0.7330383; // 42°
const NEG_SIDE_AHEAD: f32 = -SIDE_AHEAD;
const SIDE_BEHIND: f32 = 2.4085543; // 138°
const NEG_SIDE_BEHIND: f32 = -SIDE_BEHIND;
const BACK_TO_SIDE: f32 = 2.3038347; // 132°
const FORWARD_HOLD: f32 = 0.83775806; // 48°

/// Run motion (forward, back, left, right, dash) for direction angle `a`, with hysteresis on the run motion
/// already playing (`current`, tired variants folded). `dash`: run counter past half the agility.
pub fn run_motion(current: i32, a: f32, dash: bool) -> i32 {
    let (q, tq) = (f(QUARTER), f(THREE_QUARTERS));
    let side = |a: f32| if a < 0.0 { id::RUN_LEFT } else { id::RUN_RIGHT };
    let m = match current {
        id::RUN_LEFT => {
            if q < a && a <= tq {
                id::RUN_RIGHT
            } else if NEG_SIDE_AHEAD <= a && a < q {
                id::RUN_FORWARD
            } else if a < NEG_SIDE_AHEAD && NEG_SIDE_BEHIND < a {
                id::RUN_LEFT
            } else {
                id::RUN_BACK
            }
        }
        id::RUN_RIGHT => {
            if a < -q && -tq <= a {
                id::RUN_LEFT
            } else if -q <= a && a <= SIDE_AHEAD {
                id::RUN_FORWARD
            } else if SIDE_AHEAD < a && a < SIDE_BEHIND {
                id::RUN_RIGHT
            } else {
                id::RUN_BACK
            }
        }
        id::RUN_BACK => {
            if a.abs() <= q {
                id::RUN_FORWARD
            } else if a.abs() <= BACK_TO_SIDE {
                side(a)
            } else {
                id::RUN_BACK
            }
        }
        id::RUN_FORWARD => {
            if !(a.abs() < tq) {
                id::RUN_BACK
            } else if a.abs() < FORWARD_HOLD {
                id::RUN_FORWARD
            } else {
                side(a)
            }
        }
        _ => {
            if !(a.abs() < tq) {
                id::RUN_BACK
            } else if a.abs() < q {
                id::RUN_FORWARD
            } else {
                side(a)
            }
        }
    };
    if dash { id::DASH } else { m }
}

/// Whether the run counter has passed half the agility (the dash).
pub fn dashing(s: &Stats, run: i32) -> bool {
    s.agility / 2 < run
}

/// Standing motion for facing angle `a`: a quarter turn or more steps (run motions), otherwise `stance()`.
pub fn stand_motion(a: f32, stance: impl FnOnce() -> i32) -> i32 {
    if !(a.abs() < THREE_QUARTERS) {
        id::RUN_BACK
    } else if a.abs() < QUARTER {
        stance()
    } else if a < 0.0 {
        id::RUN_LEFT
    } else {
        id::RUN_RIGHT
    }
}

/// Ready stance: square (`id::STAND`), or turned to the forehand / backhand side of the ball's line
/// (`id::TURN_FORE`, `id::TURN_BACK`).
pub struct StanceInput {
    pub players: i32,
    /// Match phase (`rally::phase`).
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
    if s.players == 1 && s.phase != phase::RALLY {
        return id::STAND;
    }
    if !(s.players < 2 || (s.last_hitter >= 0 && (s.last_hitter & 1) != (s.team & 1))) {
        return id::STAND;
    }
    if s.phase == phase::POINT_OVER && (id::TURN_FORE..=id::TURN_BACK).contains(&s.current) {
        return s.current;
    }
    if s.players == 1 && !s.watching {
        return id::STAND;
    }
    let (dx, dz) = (F(s.pos[0]) - s.ball[0], F(s.pos[2]) - s.ball[2]);
    let cross = (dz * s.ball_dir[0]).msub(dx, s.ball_dir[2]);
    let left = s.hand < 0.0;
    if (cross > 0.0) != left { id::TURN_BACK } else { id::TURN_FORE }
}

/// The motion number the game stores: the stances and runs have tired variants (`id::TIRED` on) when stamina is
/// below `TIRED`.
pub fn with_tiredness(motion: i32, stamina: i32) -> i32 {
    if (0..id::TIRED).contains(&motion) && stamina < TIRED { motion + id::TIRED } else { motion }
}

/// Tired variants folded onto their base motion.
pub fn base_motion(motion: i32) -> i32 {
    if (id::TIRED..2 * id::TIRED).contains(&motion) { motion - id::TIRED } else { motion }
}

/// A player's facing and its turn state.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Facing {
    /// Unit direction the body faces (game space, w carried along).
    pub dir: [f32; 4],
    /// Turning round toward the run direction (the stand/run motion uses `dir` meanwhile).
    pub turned: bool,
    /// The turn runs the other way round except while dashing.
    pub reversed: bool,
    /// Which way round the turn goes (0 / 1), −1 undecided.
    pub way: i32,
    /// Target × facing after the last step.
    pub cross: [f32; 4],
}

/// cos 22.5°: the turn's step per frame.
const STEP: f32 = 0.9238795;

/// Rotation about −y by the angle with cosine `c` (sign of its sine `s`), as the game's axis-angle builder.
fn rot_y(c: f32, s: f32) -> [[f32; 4]; 4] {
    let (ax, ay, az) = (f(0.0) * -1.0, f(1.0) * -1.0, f(0.0) * -1.0);
    let k = F::ONE - c;
    let sn = F(s) * (F::ZERO + 1.0).msub(c, c).sqrt();
    let (xy, zx, yz) = (ax * ay, az * ax, ay * az);
    [
        f32s([(F::ZERO + c).madd(k, ax * ax), (k * xy).msub(az, sn), (ay * sn).madd(k, zx), F::ZERO]),
        f32s([(az * sn).madd(k, xy), (F::ZERO + c).madd(k, ay * ay), (k * yz).msub(ax, sn), F::ZERO]),
        f32s([(k * zx).msub(ay, sn), (ax * sn).madd(k, yz), (F::ZERO + c).madd(k, az * az), F::ZERO]),
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn clamp1(x: f32) -> f32 {
    if x < -1.0 { -1.0 } else if x <= 1.0 { x } else { 1.0 }
}

/// FPU dot product in the game's order: (a.y·b.y + a.x·b.x) + a.z·b.z.
fn dot(a: [f32; 4], b: [f32; 4]) -> f32 {
    let (a, b) = (fs(a), fs(b));
    (a[1] * b[1]).madd(a[0], b[0]).madd(a[2], b[2]).0
}

/// a × b (w = 0), each component `a.i·b.j − a.j·b.i` through the accumulator.
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let (a, b) = (fs(a), fs(b));
    f32s([(a[1] * b[2]).msub(a[2], b[1]), (a[2] * b[0]).msub(a[0], b[2]), (a[0] * b[1]).msub(a[1], b[0]), F::ZERO])
}

/// One frame of the body turning toward `target`: within 22.5° it snaps, otherwise it steps 22.5°.
/// Which way round is decided once per turn by comparing the pelvis orientation of the motion playing at the
/// frame's start (`start`, raw number; `start_base` folded) and the one chosen this frame (`now`) — `pelvis[m]`
/// is the forward row (x, z) of motion m's `Bip01Pelvis` model matrix at frame 0 — against the angle still to
/// turn, either way round.
pub fn turn(f: &mut Facing, target: [f32; 4], forward: f32, hand: f32, start: usize, start_base: i32, now: usize, pelvis: &[[f32; 2]]) {
    let d = f.dir;
    let settle = |f: &mut Facing| {
        f.reversed = false;
        f.turned = false;
        f.way = -1;
        f.cross = [0.0; 4];
    };
    if target.map(f32::to_bits) == d.map(f32::to_bits) {
        return settle(f);
    }
    if !(dot(target, d) < STEP) {
        f.dir = target;
        return settle(f);
    }
    let mut t = target;
    if hand < 0.0 {
        t[0] = (F::ZERO + t[0]).madd(LEFTY_NUDGE, if t[2] < 0.0 { -1.0 } else { 1.0 }).0;
    }
    let side = |x: F| if x < 0.0 { -1.0 } else { 1.0 };
    // (a.z·b.x − a.x·b.z) through the accumulator
    let cy = |a: [f32; 4], b: [f32; 4]| (F(a[2]) * b[0]).msub(a[0], b[2]);
    if side(cy(t, d)) != side(F(f.cross[1])) {
        f.way = -1;
    }
    if f.way == -1 {
        // the pelvis forward of motion m on the court plane, toward the player's forward
        let pfwd = |m: usize| {
            let [x, z] = pelvis[m];
            let x = F(x) * hand;
            let inv = 1.0 / (F(z) * z).madd(x, x).sqrt();
            f32s([x * inv * forward, F(0.0) * forward, z * inv * forward, F(0.0) * forward])
        };
        // the forward-facing dot: (v.y·0 + v.x·0) + v.z·forward
        let ahead = |v: [f32; 4]| clamp1((F(v[1]) * 0.0).madd(v[0], 0.0).madd(v[2], forward).0);
        let p1 = pfwd(start);
        let c = cross(t, d);
        let r = rot_y(ahead(d), side(F(c[1])));
        let q1 = crate::vu0::transform(&r, p1);
        let (q2, p2) = if start == now {
            let r = rot_y(clamp1(dot(d, t)), side(cy(d, t)));
            (crate::vu0::transform(&r, q1), p1)
        } else {
            let p2 = pfwd(now);
            let r = rot_y(ahead(t), side(cy(d, t)));
            (crate::vu0::transform(&r, p2), p2)
        };
        let mut a = F(acosf(clamp1(dot(q2, q1))));
        let mut b = F(acosf(clamp1(dot(p2, p1))));
        let mut g = F(acosf(clamp1(dot(t, d))));
        if -cy(q2, q1) < 0.0 {
            a *= -1.0;
        }
        if -cy(p2, p1) < 0.0 {
            b *= -1.0;
        }
        let mut other = -(F(TAU) - g);
        f.way = 0;
        if -F(c[1]) < 0.0 {
            f.way = 1;
            g *= -1.0;
            other *= -1.0;
        }
        let (e1, e2) = (a - (b + g), a - (b + other));
        if !(e1.abs() <= e2.abs()) {
            f.way = 1 - f.way;
        }
    }
    let mut s = if f.way != 0 { F(-1.0) } else { F::ONE };
    if start_base != id::DASH && f.reversed {
        s *= -1.0;
    }
    f.dir = crate::vu0::transform(&rot_y(STEP, s.0), d);
    f.cross = cross(t, f.dir);
}

/// The pad's stick bytes and d-pad bits.
pub mod pad {
    /// A stick axis byte at rest.
    pub const CENTRE: u8 = 0x80;
    /// The driver's deadzone around it (stick bytes in here read as `CENTRE`).
    pub const DEADZONE: std::ops::RangeInclusive<u8> = 0x51..=0xae;
    /// The game's dead square on the −127..127 axis scale (`axis`).
    pub const DEAD: i32 = 48;
    /// D-pad bits of the buttons word (active-high).
    pub const UP: u16 = 0x10;
    pub const RIGHT: u16 = 0x20;
    pub const DOWN: u16 = 0x40;
    pub const LEFT: u16 = 0x80;

    /// A stick byte on the game's −127..127 scale: bytes past the centre drop by one, so the centre itself reads
    /// +1 (a hair right/down).
    pub fn axis(b: u8) -> i32 {
        b as i32 - (b > CENTRE) as i32 - (CENTRE as i32 - 1)
    }
}

/// The pad driver's deadzone, on every stick axis byte (both sticks) before anything reads it.
pub fn pad_deadzone(b: u8) -> u8 {
    if pad::DEADZONE.contains(&b) { pad::CENTRE } else { b }
}

/// A stick axis byte as the pad's held direction (what menus read beside the d-pad): −1 toward 0x00 (left/up),
/// +1 toward 0xff, 0 inside the dead square on the same scale as `pad_dir`.
pub fn pad_held(b: u8) -> i32 {
    let i = pad::axis(b);
    if i.abs() < pad::DEAD { 0 } else { i.signum() }
}

/// A human's run direction (game x, z) from the pad: the left stick (`lx`, `ly` bytes on `pad::axis`'s scale)
/// rescaled past the dead square, or the d-pad (`buttons`, `pad::UP` …; right and up win) when both axes sit
/// within it (its edge included: the rescaled stick is exactly zero there); then ×1.2, clipped to length 1. Zero
/// outside serve, rally and point-over. The camera on the +z side flips it by π about y — not done here (`turned`).
pub fn pad_dir(buttons: u16, lx: u8, ly: u8, phase: u8) -> [f32; 2] {
    let (ix, iy) = (pad::axis(lx), pad::axis(ly));
    let (mut x, mut z) = if ix.abs() <= pad::DEAD && iy.abs() <= pad::DEAD {
        let held = |bit: u16| buttons & bit != 0;
        let (r, l, u, d) = (held(pad::RIGHT), held(pad::LEFT), held(pad::UP), held(pad::DOWN));
        let x = f(if r { 1.0 } else if l { -1.0 } else { 0.0 });
        let z = f(if u { 1.0 } else if d { -1.0 } else { -0.0 });
        let len = (f(0.0) * 0.0).madd(x, x).madd(z, z).sqrt();
        if len <= 1.0 { (x, z) } else { let n = 1.0 / len; (x * n, z * n) }
    } else {
        let (fx, fz) = (F(ix as f32) / 127.0, F(-iy as f32) / 127.0);
        let n = 1.0 / (fz * fz).madd(fx, fx).sqrt();
        let mut mag = fx.abs();
        if mag <= fz.abs() {
            mag = fz.abs();
        }
        let t = F(STICK_DEAD);
        let m = if mag <= t { F::ZERO } else { (mag - t) / STICK_LIVE };
        (fx * n * m, fz * n * m)
    };
    let k = STICK_GAIN;
    (x, z) = (x * k, z * k);
    let len = (z * z).madd(x, x).sqrt();
    if 1.0 < len {
        let n = 1.0 / len;
        (x, z) = (x * n, z * n);
    }
    // the camera matrix (identity) turns −0 into +0
    if (phase::SERVE..=phase::POINT_OVER).contains(&phase) { f32s([x + 0.0, z + 0.0]) } else { [0.0, 0.0] }
}

/// The stick's dead square and live span as fractions of full travel (48/127, 79/127), and the gain after.
const STICK_DEAD: f32 = 0.37795276;
const STICK_LIVE: f32 = 0.62204725;
const STICK_GAIN: f32 = 1.2;

/// `pad_dir` seen from a camera on the +z side: turned by π about y. The camera matrix's product leaves the same
/// values negated, but a zero comes out +0, not −0.
pub fn turned([x, z]: [f32; 2]) -> [f32; 2] {
    f32s([-f(x) + 0.0, -f(z) + 0.0])
}

/// One frame of the server before the toss: `dir` is the run direction (`pad_dir`, or a bot's), and
/// only an x-dominant one walks the baseline a fixed step that way, clamped between the centre mark's side
/// (|x| 0.7) and the sideline (singles or doubles), on the half the score's `side` (0 deuce, 1 ad) and the
/// player's `end` (±1) pick. Returns the new x and the motion: the serve stance, else `motion::serve_walk` (also
/// while held at a limit). ponytail: the mover's collision check is skipped (a baseline server never
/// reaches the court bounds or the partner).
pub fn serve_walk(x: f32, dir: [f32; 2], end: f32, side: i32, doubles: bool, hand: f32) -> (f32, i32) {
    use crate::serve::{WALK, WALK_MAX_DOUBLES, WALK_MAX_SINGLES, WALK_MIN};
    if dir[0] == 0.0 || !(dir[1].abs() < dir[0].abs()) {
        return (x, id::SERVE_STANCE);
    }
    let step = if dir[0] < 0.0 { -1.0 } else { 1.0 };
    let x = (F::ZERO + x).madd(WALK, step);
    let court = if side == 0 { 1.0 } else { -1.0 };
    let far = F(if doubles { WALK_MAX_DOUBLES } else { WALK_MAX_SINGLES }) * end * court;
    let min = F(WALK_MIN);
    let x = if 0.0 < far {
        if x < min { min } else if x <= far { x } else { far }
    } else if x < far {
        far
    } else if x <= -min {
        x
    } else {
        -min
    };
    (x.0, crate::motion::serve_walk(dir[0], end, hand))
}

/// A computer player's stick as the game passes it on: each axis of the AI's wanted direction (x, z) rounded half
/// away from zero to a byte, centre ± 127, clamped to 1..=255.
pub fn bot_stick(v: [f32; 2]) -> [u8; 2] {
    v.map(|f| {
        let sign = if f < 0.0 { -1.0 } else { 1.0 };
        (((F(0.5) * sign).madd(127.0, f).0 as i32 + pad::CENTRE as i32).clamp(1, 255)) as u8
    })
}

/// The run input a computer player's stick bytes give: (b − centre)/127 per axis, cut to unit length when longer;
/// none outside the serve, rally and point-over phases.
pub fn stick_dir(b: [u8; 2], phase: u8) -> [f32; 2] {
    if !(phase::SERVE..=phase::POINT_OVER).contains(&phase) {
        return [0.0, 0.0];
    }
    let [x, z] = b.map(|b| F((b as i32 - pad::CENTRE as i32) as f32) / 127.0);
    let len = (z * z).madd(x, x).sqrt();
    f32s(if len <= 1.0 { [x, z] } else { let n = 1.0 / len; [x * n, z * n] })
}

/// A computer player's run axis within this of its target counts as there.
const NEAR: f32 = 0.001;

/// A run within about 8° of straight ahead or back (cos) faces square.
const STRAIGHT: f32 = 0.99;

/// What a standing or running player sees of the match this frame.
pub struct Scene {
    pub players: i32,
    /// Match phase (`rally::phase`; 1 change ends).
    pub phase: u8,
    pub last_hitter: i32,
    pub team: i32,
    /// Forward (±1 along z) and hand (−1 left-handed).
    pub forward: f32,
    pub hand: f32,
    /// The doubles partner's position (as of this player's update).
    pub mate: Option<[f32; 3]>,
    pub ball: [f32; 3],
    pub ball_dir: [f32; 3],
    /// The half-court singles mode.
    pub short: bool,
}

/// A player's body in the play state: position, run, stamina, motion and facing.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Body {
    pub pos: [f32; 3],
    /// Last run velocity.
    pub vel: [f32; 3],
    /// Running (mode 1) or standing (mode 0).
    pub running: bool,
    pub run: i32,
    pub stamina: i32,
    pub stamina_tick: i32,
    /// Motion number set this frame (tired variants included).
    pub motion: i32,
    /// Where the body should face and where it faces.
    pub target: [f32; 4],
    pub face: Facing,
}

impl Body {
    /// A body standing at `pos` squarely facing `forward`, with full stamina.
    pub fn new(pos: [f32; 3], forward: f32, s: &Stats) -> Self {
        let dir = [0.0, 0.0, forward, 0.0];
        Body { pos, stamina: s.stamina, target: dir, face: Facing { dir, way: -1, ..Facing::default() }, ..Body::default() }
    }

    /// One frame of the play state for a standing or running player: `input` is the wanted direction (game x, z;
    /// zero to stand), `pelvis` the character's per-motion pelvis rows (see [`turn`]).
    pub fn step(&mut self, s: &Stats, input: [f32; 2], sc: &Scene, pelvis: &[[f32; 2]]) {
        let start = self.motion;
        let [ix, iz] = fs(input);
        let len = (iz * iz).madd(ix, ix).sqrt();
        if self.running {
            self.run += 1;
            (self.stamina, self.stamina_tick) = drain(s, self.stamina, self.stamina_tick, sc.players, sc.phase == phase::RALLY, 0);
        }
        if 0.0 < len {
            if !self.running {
                self.run = 0;
            }
            let inv = 1.0 / len;
            self.go(s, f32s([ix * inv, F::ZERO, iz * inv, F::ZERO]), sc);
        } else {
            if self.running && base_motion(self.motion) == id::RUN_BACK && self.stance(sc) == id::TURN_BACK {
                self.face.turned = true;
                self.face.reversed = true;
            }
            self.stand(sc);
        }
        let now = self.motion.clamp(0, pelvis.len() as i32 - 1) as usize;
        let from = start.clamp(0, pelvis.len() as i32 - 1) as usize;
        turn(&mut self.face, self.target, sc.forward, sc.hand, from, base_motion(start), now, pelvis);
    }

    /// The computer player's run toward `target` (x, z): the unit direction (an axis within 1 mm counts as there),
    /// zeroed when the target is within ⅔ of next frame's run step (`run_speed` with the run count and stamina
    /// as `step` will have them; `keep` runs on regardless). Returns the direction and whether it is there: no
    /// direction, or the target within one step. ponytail: modes 0/1 only (`running`); a stroke (mode 2) would
    /// count the run from 0, the drain floor is `step`'s 0.
    pub fn toward(&self, s: &Stats, target: [f32; 2], players: i32, phase: u8, keep: bool) -> ([f32; 2], bool) {
        let mut x = F(target[0]) - self.pos[0];
        let mut z = F(target[1]) - self.pos[2];
        if x.abs() < NEAR {
            x = F::ZERO;
        }
        if z.abs() < NEAR {
            z = F::ZERO;
        }
        let len = (z * z).madd(x, x).sqrt();
        if len <= 0.0 {
            return ([0.0, 0.0], true);
        }
        let inv = 1.0 / len;
        let dir = f32s([x * inv, z * inv]);
        let (run, stamina) = if self.running {
            (self.run + 1, drain(s, self.stamina, self.stamina_tick, players, phase == phase::RALLY, 0).0)
        } else {
            (0, self.stamina)
        };
        let step = run_speed(s, run, stamina, 100);
        if !keep && len <= f(2.0) * step / 3.0 {
            return ([0.0, 0.0], true);
        }
        (dir, len - step <= 0.0)
    }

    fn stance(&self, sc: &Scene) -> i32 {
        stance(&StanceInput {
            players: sc.players,
            phase: sc.phase,
            last_hitter: sc.last_hitter,
            team: sc.team,
            current: base_motion(self.motion),
            watching: true,
            pos: self.pos,
            ball: sc.ball,
            ball_dir: sc.ball_dir,
            hand: sc.hand,
        })
    }

    /// Standing (mode 0): face forward, the stance or a turning step.
    fn stand(&mut self, sc: &Scene) {
        self.running = false;
        self.target = [0.0, 0.0, sc.forward, 0.0];
        let d = if self.face.turned { self.face.dir } else { self.target };
        let m = stand_motion(angle([d[0], d[1], d[2]], sc.forward, sc.hand), || self.stance(sc));
        self.motion = with_tiredness(m, self.stamina);
        self.pos = mover(self.pos, [0.0; 3], sc.forward, sc.mate, sc.short);
    }

    /// Running (mode 1) in unit direction `dir`.
    fn go(&mut self, s: &Stats, dir: [f32; 4], sc: &Scene) {
        let current = base_motion(self.motion);
        self.running = true;
        let ahead = (f(0.0) * dir[1]).madd(0.0, dir[0]).madd(sc.forward, dir[2]);
        let straight = F(STRAIGHT);
        self.target = if straight < ahead {
            [0.0, 0.0, sc.forward, 0.0]
        } else if ahead < -straight {
            [-0.0, -0.0, (F(sc.forward) * -1.0).0, -0.0]
        } else {
            dir
        };
        self.vel = run_velocity([dir[0], dir[1], dir[2]], run_speed(s, self.run, self.stamina, 100));
        self.pos = mover(self.pos, self.vel, sc.forward, sc.mate, sc.short);
        let t = [self.target[0], self.target[1], self.target[2]];
        if !self.face.turned && !(angle(t, sc.forward, sc.hand).abs() < THREE_QUARTERS) && current == id::TURN_BACK {
            self.face.turned = true;
            self.face.reversed = true;
        }
        let d = if self.face.turned { [self.face.dir[0], self.face.dir[1], self.face.dir[2]] } else { t };
        self.motion = with_tiredness(run_motion(current, angle(d, sc.forward, sc.hand), dashing(s, self.run)), self.stamina);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The named constants keep the original's bits.
    #[test]
    fn constant_bits() {
        let bits = [
            (SIDE, 0x410a_f5c3),
            (BACK, 0x418f_147b),
            (crate::lines::SERVICE_LINE, 0x40cc_cccd),
            (RUN, 0x3d92_d8b5),
            (RUN_GAIN, 0x3e99_999a),
            (TIRED_LOSS, 0x3cf5_c28f),
            (LEFTY_NUDGE, 0x3c23_d70a),
            (ALIGNED, 0x3f7f_ff58),
            (PI, 0x4049_0fdb),
            (TAU, 0x40c9_0fdb),
            (QUARTER, 0x3f49_0fdb),
            (THREE_QUARTERS, 0x4016_cbe4),
            (NEG_SIDE_AHEAD, 0xbf3b_a866),
            (NEG_SIDE_BEHIND, 0xc01a_25c1),
            (SIDE_AHEAD, 0x3f3b_a866),
            (SIDE_BEHIND, 0x401a_25c1),
            (BACK_TO_SIDE, 0x4013_7207),
            (FORWARD_HOLD, 0x3f56_7750),
            (STEP, 0x3f6c_835e),
            (STICK_DEAD, 0x3ec1_8306),
            (STICK_LIVE, 0x3f1f_3e7d),
            (STICK_GAIN, 0x3f99_999a),
            (NEAR, 0x3a83_126f),
            (STRAIGHT, 0x3f7d_70a4),
        ];
        for (k, (v, b)) in bits.into_iter().enumerate() {
            assert_eq!(v.to_bits(), b, "constant {k}");
        }
    }

    #[test]
    fn partner_push() {
        // stepping into the partner ends 1 m from it, along the line between them
        let p = mover([0.0, 0.0, -5.0], [0.0, 0.0, 0.5], 1.0, Some([0.0, 0.0, -4.0]), false);
        assert_eq!(p, [0.0, 0.0, -5.0]);
        let p = mover([0.6, 0.0, -5.0], [0.0, 0.0, 0.5], 1.0, Some([0.0, 0.0, -4.0]), false);
        assert!(((p[0] * p[0] + (p[2] + 4.0) * (p[2] + 4.0)).sqrt() - 1.0).abs() < 1e-5, "{p:?}");
        // far apart: a plain step, clamped to the court
        assert_eq!(mover([8.6, 0.0, -17.8], [0.2, 0.0, -0.2], 1.0, Some([0.0, 0.0, -4.0]), false), [8.685, 0.0, -17.885]);
    }

    #[test]
    fn pad_bytes() {
        assert_eq!([0x50, 0x51, 0x7f, 0xae, 0xaf].map(pad_deadzone), [0x50, 0x80, 0x80, 0x80, 0xaf]);
        // menus: one past the driver's deadzone is still inside |48|
        assert_eq!([0x00, 0x4f, 0x50, 0x80, 0xaf, 0xb0, 0xff].map(pad_held), [-1, -1, 0, 0, 0, 1, 1]);
    }
}
