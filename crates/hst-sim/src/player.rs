//! Player locomotion as the original: run speed (character speed, acceleration and stamina), the per-frame run
//! counter and stamina drain, and the motion each standing or running player plays (ready stance, turned
//! stances toward the ball, runs by direction relative to the player's forward, dash, tired variants).

use crate::libm::acosf;
use crate::ps2::{add, div, madd, msub, mul, sqrt, sub};

/// A character's movement stats (TParam.csv: SPE, Agili, STA, and the stamina a dive / backhand / smash costs).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// SPE / 10.
    pub speed: f32,
    /// Frames of running until full speed (Agili; ×1.5 in weather 2 and 3).
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
    /// state (court object +0x135, from the match's weather table).
    pub fn new(spe: i32, agility: i32, stamina: i32, [dive, backhand, smash]: [i32; 3], weather: u8) -> Self {
        // the game's integer ×150/100 in weather 2 and 3
        let agility = if weather.wrapping_sub(2) < 2 { agility * 150 / 100 } else { agility };
        Stats { speed: spe as f32 / 10.0, agility, stamina, dive, backhand, smash }
    }
}

/// A character's reach and contact heights (m) and body-shot adjustments, as the game's TParam.csv parser leaves
/// them in the character record (copied to player +0x1310.. and +0x13a0..+0x13ec).
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
}

impl ReachStats {
    /// From a character's TParam.csv row. The game splits it with strtok (empty cells vanish, so the disc's empty
    /// Special POW cell shifts every later column down one token) and '/' inside a cell; centimetre cells are
    /// int ÷ 100, the scatter factor and the reach centre go through its own decimal reader.
    pub fn from_tparam(row: &str) -> Self {
        let t: Vec<&str> = row.split(',').filter(|c| !c.is_empty()).map(str::trim).collect();
        let part = |k: usize, i: usize| t[k].split('/').nth(i).unwrap_or("");
        let int = |s: &str| atoi(s);
        let m = |s: &str| div(atoi(s) as f32, 100.0);
        let three = |k: usize| [0, 1, 2].map(|i| m(part(k, i)));
        ReachStats {
            body_adj: int(t[22]),
            vbody_adj: int(t[23]),
            rising_adj: int(t[25]),
            smash_low_pow: int(t[51]),
            stroke_high_pow: int(t[52]),
            serve_scatter: atof(t[53]),
            base: atof(t[54]),
            reach: add(m(t[55]), m(t[56])),
            dive_start: m(t[57]),
            dive_limit: m(t[58]),
            collision: m(part(60, 1)),
            under_min: m(t[61]),
            stroke_height: m(part(62, 0)),
            volley_height: m(part(62, 1)),
            smash: three(63),
            serve: three(64),
            under_serve: three(65),
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
    let mut v = 0.0f32;
    if let Some(d) = dot {
        let mut w = 0.1f32;
        for &c in b[d + 1..].iter().take_while(|c| c.is_ascii_digit()) {
            v = madd(add(v, 0.0), (c - b'0') as f32, w);
            w = div(w, 10.0);
        }
    }
    let mut w = 1.0f32;
    for &c in b[..dot.unwrap_or(b.len())].iter().rev().take_while(|c| c.is_ascii_digit()) {
        v = madd(add(v, 0.0), (c - b'0') as f32, w);
        w = mul(w, 10.0);
    }
    v
}

/// Stamina after a stroke's contact (during the rally of a game with more than one player): ground strokes
/// (branch 1) cost the backhand value on the backhand side, dives (3) and smashes (4) theirs, volleys nothing.
pub fn stroke_stamina(s: &Stats, stamina: i32, branch: u8, forehand: bool, floor: i32) -> i32 {
    let cost = match branch {
        1 if !forehand => s.backhand,
        3 => s.dive,
        4 => s.smash,
        _ => return stamina,
    };
    let st = stamina - cost;
    if st < floor { floor } else { st.min(s.stamina) }
}

/// Court bounds of the mover: |x| ≤ 8.685 and 1.5..17.885 deep on the player's own half.
const SIDE: f32 = f32::from_bits(0x410a_f5c3);
const BACK: f32 = f32::from_bits(0x418f_147b);
const NET: f32 = 1.5;

fn on_court(x: f32, z: f32) -> bool {
    x.abs() <= SIDE && z.abs() <= BACK && !(z.abs() < NET)
}

/// Push `cur` out to 1 m from the doubles partner at `mate` (`None`: no push). Returns the pushed position, or
/// with `fallback` the nearest on-court point of four around the partner (else `restore`) when the push leaves
/// the court; `None` when the push leaves the court and there is no fallback.
fn push(cur: [f32; 3], mate: [f32; 3], restore: [f32; 3], fallback: bool) -> Option<[f32; 3]> {
    let (dx, dz) = (sub(cur[0], mate[0]), sub(cur[2], mate[2]));
    let d2 = madd(mul(dz, dz), dx, dx);
    if !(d2 < 1.0) {
        return Some(cur);
    }
    let inv = div(1.0, sqrt(d2));
    let out = [add(mate[0], mul(dx, inv)), add(mate[1], 0.0), add(mate[2], mul(dz, inv))];
    if on_court(out[0], out[2]) {
        return Some(out);
    }
    if !fallback {
        return None;
    }
    let (sx, sz) = (sqrt(msub(1.0, dz, dz)), sqrt(msub(1.0, dx, dx)));
    let cands = [(sub(mate[0], sx), restore[2]), (add(mate[0], sx), restore[2]), (restore[0], sub(mate[2], sz)), (restore[0], add(mate[2], sz))];
    let mut best: Option<((f32, f32), f32)> = None;
    for c in cands.into_iter().filter(|c| on_court(c.0, c.1)) {
        let (ex, ez) = (sub(c.0, restore[0]), sub(c.1, restore[2]));
        let d = madd(mul(ez, ez), ex, ex);
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
    cur = [add(cur[0], delta[0]), add(cur[1], delta[1]), add(cur[2], delta[2])];
    if !(cur[0].abs() <= SIDE) {
        cur[0] = mul(SIDE, if cur[0] < 0.0 { -1.0 } else { 1.0 });
    }
    if !(cur[2].abs() <= BACK) {
        cur[2] = mul(-BACK, forward);
    }
    if cur[2].abs() < NET {
        cur[2] = mul(-NET, forward);
    }
    if short {
        let lim = f32::from_bits(0x40cc_cccd);
        if forward < 0.0 {
            if !(cur[2] <= lim) {
                cur[2] = lim;
            }
        } else if cur[2] < -lim {
            cur[2] = -lim;
        }
    }
    if let Some(m) = mate {
        cur = push(cur, m, saved, false).unwrap_or(saved);
    }
    [cur[0], pos[1], cur[2]]
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

/// A player's facing (+0x3d60) and its turn state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Facing {
    /// Unit direction the body faces (game space, w carried along).
    pub dir: [f32; 4],
    /// Turning round toward the run direction (+0x3dd1; the stand/run motion uses `dir` meanwhile).
    pub turned: bool,
    /// The turn runs the other way round except while dashing (+0x3dd0).
    pub reversed: bool,
    /// Which way round the turn goes (0 / 1), −1 undecided (+0x3dd4).
    pub way: i32,
    /// Target × facing after the last step (+0x3de0).
    pub cross: [f32; 4],
}

/// cos 22.5°: the turn's step per frame.
const STEP: f32 = f32::from_bits(0x3f6c_835e);

/// Rotation about −y by the angle with cosine `c` (sign of its sine `s`), as the game's axis-angle builder.
fn rot_y(c: f32, s: f32) -> [[f32; 4]; 4] {
    let (ax, ay, az) = (mul(0.0, -1.0), mul(1.0, -1.0), mul(0.0, -1.0));
    let k = sub(1.0, c);
    let sn = mul(s, sqrt(msub(add(0.0, 1.0), c, c)));
    let (xy, zx, yz) = (mul(ax, ay), mul(az, ax), mul(ay, az));
    [
        [madd(add(0.0, c), k, mul(ax, ax)), msub(mul(k, xy), az, sn), madd(mul(ay, sn), k, zx), 0.0],
        [madd(mul(az, sn), k, xy), madd(add(0.0, c), k, mul(ay, ay)), msub(mul(k, yz), ax, sn), 0.0],
        [msub(mul(k, zx), ay, sn), madd(mul(ax, sn), k, yz), madd(add(0.0, c), k, mul(az, az)), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn clamp1(x: f32) -> f32 {
    if x < -1.0 { -1.0 } else if x <= 1.0 { x } else { 1.0 }
}

/// FPU dot product in the game's order: (a.y·b.y + a.x·b.x) + a.z·b.z.
fn dot(a: [f32; 4], b: [f32; 4]) -> f32 {
    madd(madd(mul(a[1], b[1]), a[0], b[0]), a[2], b[2])
}

/// a × b (w = 0), each component `a.i·b.j − a.j·b.i` through the accumulator.
fn cross(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [msub(mul(a[1], b[2]), a[2], b[1]), msub(mul(a[2], b[0]), a[0], b[2]), msub(mul(a[0], b[1]), a[1], b[0]), 0.0]
}

/// One frame of the body turning toward `target` (+0x3dc0): within 22.5° it snaps, otherwise it steps 22.5°.
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
        t[0] = madd(add(0.0, t[0]), f32::from_bits(0x3c23_d70a), if t[2] < 0.0 { -1.0 } else { 1.0 });
    }
    let side = |x: f32| if x < 0.0 { -1.0 } else { 1.0 };
    if side(msub(mul(t[2], d[0]), t[0], d[2])) != side(f.cross[1]) {
        f.way = -1;
    }
    if f.way == -1 {
        // the pelvis forward of motion m on the court plane, toward the player's forward
        let pfwd = |m: usize| {
            let [x, z] = pelvis[m];
            let x = mul(x, hand);
            let inv = div(1.0, sqrt(madd(mul(z, z), x, x)));
            [mul(mul(x, inv), forward), mul(0.0, forward), mul(mul(z, inv), forward), mul(0.0, forward)]
        };
        let p1 = pfwd(start);
        let c = cross(t, d);
        let r = rot_y(clamp1(madd(madd(mul(d[1], 0.0), d[0], 0.0), d[2], forward)), side(c[1]));
        let q1 = crate::vu0::transform(&r, p1);
        let (q2, p2) = if start == now {
            let r = rot_y(clamp1(dot(d, t)), side(msub(mul(d[2], t[0]), d[0], t[2])));
            (crate::vu0::transform(&r, q1), p1)
        } else {
            let p2 = pfwd(now);
            let r = rot_y(clamp1(madd(madd(mul(t[1], 0.0), t[0], 0.0), t[2], forward)), side(msub(mul(d[2], t[0]), d[0], t[2])));
            (crate::vu0::transform(&r, p2), p2)
        };
        let mut a = acosf(clamp1(dot(q2, q1)));
        let mut b = acosf(clamp1(dot(p2, p1)));
        let mut g = acosf(clamp1(dot(t, d)));
        if -msub(mul(q2[2], q1[0]), q2[0], q1[2]) < 0.0 {
            a = mul(a, -1.0);
        }
        if -msub(mul(p2[2], p1[0]), p2[0], p1[2]) < 0.0 {
            b = mul(b, -1.0);
        }
        let mut other = -sub(f32::from_bits(0x40c9_0fdb), g);
        f.way = 0;
        if -c[1] < 0.0 {
            f.way = 1;
            g = mul(g, -1.0);
            other = mul(other, -1.0);
        }
        let (e1, e2) = (sub(a, add(b, g)), sub(a, add(b, other)));
        if !(e1.abs() <= e2.abs()) {
            f.way = 1 - f.way;
        }
    }
    let mut s = if f.way != 0 { -1.0 } else { 1.0 };
    if start_base != 7 && f.reversed {
        s = mul(s, -1.0);
    }
    f.dir = crate::vu0::transform(&rot_y(STEP, s), d);
    f.cross = cross(t, f.dir);
}

/// The pad driver's deadzone, on every stick axis byte (both sticks) before anything reads it: 0x51..=0xae → 0x80.
pub fn pad_deadzone(b: u8) -> u8 {
    if (0x51..=0xae).contains(&b) { 0x80 } else { b }
}

/// A stick axis byte as the pad's held direction (what menus read beside the d-pad): −1 toward 0x00 (left/up),
/// +1 toward 0xff, 0 inside |48| on the same −127..127 scale as `pad_dir`.
pub fn pad_held(b: u8) -> i32 {
    let i = b as i32 - (b >= 0x81) as i32 - 0x7f;
    if i.abs() < 48 { 0 } else { i.signum() }
}

/// A human's run direction (game x, z) from the pad: the left stick (`lx`, `ly` bytes, 0x80 centre; bytes ≥ 0x81 drop
/// by one, so the centre itself reads a hair right/down) rescaled past a 48/127 dead square, or the d-pad (`buttons`
/// active-high: 0x10 up, 0x20 right, 0x40 down, 0x80 left; right and up win) when both axes sit inside |48|; then
/// ×1.2, clipped to length 1. Zero outside serve, rally and point-over (phases 2–4). The camera on the +z side flips
/// it by π about y — not done here (callers negate).
pub fn pad_dir(buttons: u16, lx: u8, ly: u8, phase: u8) -> [f32; 2] {
    let axis = |b: u8| b as i32 - (b >= 0x81) as i32 - 0x7f;
    let (ix, iy) = (axis(lx), axis(ly));
    let (mut x, mut z) = if ix.abs() < 48 && iy.abs() < 48 {
        let (r, l, u, d) = (buttons & 0x20 != 0, buttons & 0x80 != 0, buttons & 0x10 != 0, buttons & 0x40 != 0);
        let x: f32 = if r { 1.0 } else if l { -1.0 } else { 0.0 };
        let z: f32 = if u { 1.0 } else if d { -1.0 } else { -0.0 };
        let len = sqrt(madd(madd(mul(0.0, 0.0), x, x), z, z));
        if len <= 1.0 { (x, z) } else { let n = div(1.0, len); (mul(x, n), mul(z, n)) }
    } else {
        let (fx, fz) = (div(ix as f32, 127.0), div(-iy as f32, 127.0));
        let n = div(1.0, sqrt(madd(mul(fz, fz), fx, fx)));
        let mut mag = fx.abs();
        if mag <= fz.abs() {
            mag = fz.abs();
        }
        let t = f32::from_bits(0x3ec1_8306);
        let m = if mag <= t { 0.0 } else { div(sub(mag, t), f32::from_bits(0x3f1f_3e7d)) };
        (mul(mul(fx, n), m), mul(mul(fz, n), m))
    };
    let k = f32::from_bits(0x3f99_999a);
    (x, z) = (mul(x, k), mul(z, k));
    let len = sqrt(madd(mul(z, z), x, x));
    if 1.0 < len {
        let n = div(1.0, len);
        (x, z) = (mul(x, n), mul(z, n));
    }
    // the camera matrix (identity) turns −0 into +0
    if (2..=4).contains(&phase) { [x + 0.0, z + 0.0] } else { [0.0, 0.0] }
}

/// One frame of the server before the toss (play state 1): `dir` is the run direction (`pad_dir`, or a bot's), and
/// only an x-dominant one walks the baseline a fixed step that way, clamped between the centre mark's side
/// (|x| 0.7) and the sideline (singles or doubles), on the half the score's `side` (0 deuce, 1 ad) and the
/// player's `end` (±1) pick. Returns the new x and the motion: 0x20 standing, else `motion::serve_walk` (also
/// while held at a limit). ponytail: the mover's collision check is skipped (a baseline server never
/// reaches the court bounds or the partner).
pub fn serve_walk(x: f32, dir: [f32; 2], end: f32, side: i32, doubles: bool, hand: f32) -> (f32, i32) {
    use crate::serve::{WALK, WALK_MAX_DOUBLES, WALK_MAX_SINGLES, WALK_MIN};
    if dir[0] == 0.0 || !(dir[1].abs() < dir[0].abs()) {
        return (x, 0x20);
    }
    let step = if dir[0] < 0.0 { -1.0 } else { 1.0 };
    let x = madd(add(0.0, x), WALK, step);
    let court = if side == 0 { 1.0 } else { -1.0 };
    let far = mul(mul(if doubles { WALK_MAX_DOUBLES } else { WALK_MAX_SINGLES }, end), court);
    let x = if 0.0 < far {
        if x < WALK_MIN { WALK_MIN } else if x <= far { x } else { far }
    } else if x < far {
        far
    } else if x <= -WALK_MIN {
        x
    } else {
        -WALK_MIN
    };
    (x, crate::motion::serve_walk(dir[0], end, hand))
}

/// A computer player's stick as the game passes it on: each axis of the AI's wanted direction (x, z) rounded half
/// away from zero to a byte, 0x80 ± 127, clamped to 1..=255.
pub fn bot_stick(v: [f32; 2]) -> [u8; 2] {
    v.map(|f| {
        let sign = if f < 0.0 { -1.0 } else { 1.0 };
        ((madd(mul(0.5, sign), 127.0, f) as i32 + 0x80).clamp(1, 255)) as u8
    })
}

/// The run input a computer player's stick bytes give: (b − 0x80)/127 per axis, cut to unit length when longer;
/// none outside the serve, rally and point-over phases (2–4).
pub fn stick_dir(b: [u8; 2], phase: u8) -> [f32; 2] {
    if !(2..=4).contains(&phase) {
        return [0.0, 0.0];
    }
    let [x, z] = b.map(|b| div((b as i32 - 0x80) as f32, 127.0));
    let len = sqrt(madd(mul(z, z), x, x));
    if len <= 1.0 { [x, z] } else { let n = div(1.0, len); [mul(x, n), mul(z, n)] }
}

const NEAR: f32 = f32::from_bits(0x3a83_126f); // 0.001

/// What a standing or running player sees of the match this frame.
pub struct Scene {
    pub players: i32,
    /// Match phase (gm+0x55): 1 change ends, 2 serve, 3 rally, 4 point over.
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Body {
    pub pos: [f32; 3],
    /// Last run velocity (+0x3e00).
    pub vel: [f32; 3],
    /// Running (mode 1) or standing (mode 0).
    pub running: bool,
    pub run: i32,
    pub stamina: i32,
    pub stamina_tick: i32,
    /// Motion number set this frame (tired variants included).
    pub motion: i32,
    /// Where the body should face (+0x3dc0) and where it faces.
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
        let len = sqrt(madd(mul(input[1], input[1]), input[0], input[0]));
        if self.running {
            self.run += 1;
            (self.stamina, self.stamina_tick) = drain(s, self.stamina, self.stamina_tick, sc.players, sc.phase == 3, 0);
        }
        if 0.0 < len {
            if !self.running {
                self.run = 0;
            }
            let inv = div(1.0, len);
            self.go(s, [mul(input[0], inv), 0.0, mul(input[1], inv), 0.0], sc);
        } else {
            if self.running && base_motion(self.motion) == 4 && self.stance(sc) == 2 {
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
        let mut x = sub(target[0], self.pos[0]);
        let mut z = sub(target[1], self.pos[2]);
        if x.abs() < NEAR {
            x = 0.0;
        }
        if z.abs() < NEAR {
            z = 0.0;
        }
        let len = sqrt(madd(mul(z, z), x, x));
        if len <= 0.0 {
            return ([0.0, 0.0], true);
        }
        let inv = div(1.0, len);
        let dir = [mul(x, inv), mul(z, inv)];
        let (run, stamina) = if self.running {
            (self.run + 1, drain(s, self.stamina, self.stamina_tick, players, phase == 3, 0).0)
        } else {
            (0, self.stamina)
        };
        let step = run_speed(s, run, stamina, 100);
        if !keep && len <= div(mul(2.0, step), 3.0) {
            return ([0.0, 0.0], true);
        }
        (dir, sub(len, step) <= 0.0)
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
        let ahead = madd(madd(mul(0.0, dir[1]), 0.0, dir[0]), sc.forward, dir[2]);
        self.target = if f32::from_bits(0x3f7d_70a4) < ahead {
            [0.0, 0.0, sc.forward, 0.0]
        } else if ahead < -f32::from_bits(0x3f7d_70a4) {
            [-0.0, -0.0, mul(sc.forward, -1.0), -0.0]
        } else {
            dir
        };
        self.vel = run_velocity([dir[0], dir[1], dir[2]], run_speed(s, self.run, self.stamina, 100));
        self.pos = mover(self.pos, self.vel, sc.forward, sc.mate, sc.short);
        let t = [self.target[0], self.target[1], self.target[2]];
        if !self.face.turned && !(angle(t, sc.forward, sc.hand).abs() < THREE_QUARTERS) && current == 2 {
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
