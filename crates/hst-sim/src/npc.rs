//! Background figures: which of a court's creature records (layout category 23) come to life at court load, as
//! what, and where they stand. A creature linked to an anchor record (category 14) stands at the anchor's position
//! minus its own (FPU subtractions); every figure's matrix is a turn by its yaw at that position.

use crate::{ps2, sound};
use crate::world::{self, M4};
use hst_data::exe::{EmitterRow, NpcEntry, TriggerRow};
use hst_data::layout::{self, Entry, Placement};

/// What a creature record becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A trigger creature of this type (0..53); only made with an anchor.
    Trigger(u8),
    /// Walking spectator 0..5 (npc00..npc05).
    Walker(u8),
    /// The umpire's chair (made when more than one player).
    Umpire,
    /// One of court 5's own creatures (0..5).
    Court5(u8),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Npc {
    /// Index among the court's category-23 records, in file order.
    pub record: usize,
    pub kind: Kind,
    pub world: M4,
    pub scale: f32,
}

/// Where each category-23 record stands, in file order (links resolved against the anchors' positions).
pub fn positions(plants: &[Placement]) -> Vec<[f32; 3]> {
    let of = |cat: u8, i: u16| plants.iter().filter(|p| p.category == cat).nth(i as usize).map(|p| p.pos);
    plants
        .iter()
        .filter(|p| p.category == 23)
        .map(|p| match p.link.and_then(|(c, i)| of(c, i)) {
            Some(a) => std::array::from_fn(|k| ps2::sub(a[k], p.pos[k])),
            None => p.pos,
        })
        .collect()
}

/// The figures made at court load. `entries`/`plants` are the court's layout, `roster`/`walkers` its rows of
/// the game's tables (`exe::Game::npc_roster` / `walkers`), `players` the number of players on court.
pub fn spawn(entries: &[Entry], plants: &[Placement], roster: &[NpcEntry], walkers: &[[u8; 3]; 6], players: u32) -> Vec<Npc> {
    let pos = positions(plants);
    plants
        .iter()
        .filter(|p| p.category == 23)
        .enumerate()
        .filter_map(|(record, p)| {
            let stem = &layout::resolve(entries, p, 0)?.stem;
            let kind = match roster.iter().find(|e| e.name.eq_ignore_ascii_case(stem))?.kind {
                // ponytail: the anchor object exists for every category-14 link in the courts checked
                t @ 0..=53 => (p.link?.0 == 14).then_some(Kind::Trigger(t))?,
                t @ 54..=59 => (players < 3 || walkers[t as usize - 54][2] != 0).then_some(Kind::Walker(t - 54))?,
                60 => (players >= 2).then_some(Kind::Umpire)?,
                t => Kind::Court5(t - 61),
            };
            let mut world = world::mat_mul(&world::IDENTITY, &world::rot_y(p.yaw));
            world[3] = [pos[record][0], pos[record][1], pos[record][2], 1.0];
            Some(Npc { record, kind, world, scale: p.scale })
        })
        .collect()
}

/// A walking spectator standing in place: its animation (the game's controller: `frame` shown, `next` the one
/// after, advancing by `speed`), whether it is reacting to a point, and the stagger that restarts the idle loop
/// one walker per tick after a new point. Animations: 0 idle, 1 its variant, 2 (a quiet reaction), 3/5 loop
/// (5 the cheer), 4 shown frozen.
/// ponytail: dodging the ball and players (wander mode's walk, collision, ground height) and facing the winner
/// while reacting move the walker; no recording has a walker moving, so they are not ported (P14e).
#[derive(Clone, Debug, PartialEq)]
pub struct Walker {
    pub slot: u32,
    /// 0 idle, 1 a point to react to (taken on the next tick), 2 reacted.
    pub mode: u8,
    /// Ticks since a new point while restarting (−1 when done): the idle loop restarts when it reaches `slot`.
    pub counter: i32,
    pub anim: u32,
    pub frame: f32,
    pub next: f32,
    pub speed: f32,
    /// Advances every tick (cleared by animation 4).
    pub advancing: bool,
    /// Length of each animation, in frames.
    pub lens: [f32; 7],
}

impl Walker {
    fn set_frame(&mut self, t: f32) {
        let len = self.lens[self.anim as usize];
        let mut t = t;
        if matches!(self.anim, 3 | 5) && len != 0.0 {
            while len <= t {
                t = ps2::sub(t, len);
            }
            while t < 0.0 {
                t = ps2::add(t, len);
            }
        } else if len < t {
            t = len;
        } else if t < 0.0 {
            t = 0.0;
        }
        (self.next, self.frame) = (t, t);
    }

    fn advance(&mut self) {
        self.set_frame(self.next);
        self.next = ps2::add(self.next, self.speed);
    }

    fn set_anim(&mut self, anim: u32, players: u32) {
        self.anim = anim;
        self.set_frame(0.0);
        // the idle loops play at double speed on alternate ticks with four players
        self.speed = if players >= 3 && anim < 2 { 2.0 } else { 1.0 };
        self.advancing = anim != 4;
        if anim == 4 {
            self.advance();
        }
    }

    /// A random frame in the `k`th of `n` equal parts of `[0, span)`.
    fn random_frame(&mut self, part: f32, roll: &mut impl FnMut() -> u32) {
        let k = self.slot as f32;
        let r = ps2::mul(2.3283064e-10, ps2::utof(roll()));
        self.set_frame(ps2::lerp(ps2::mul(k, part), ps2::mul(k + 1.0, part), r));
    }

    /// A point was decided (the game skips faults and lets).
    pub fn react(&mut self) {
        self.mode = 1;
    }

    /// A new point: stop reacting; with `stagger` (a doubles point after the first) restart the idle loops one walker
    /// per tick.
    pub fn new_point(&mut self, stagger: bool) {
        self.mode = 0;
        if stagger {
            self.counter = 0;
        }
    }

    /// One tick. `cheer`: this walker cheers rather than reacting quietly (the gallery's pick); `tick` the
    /// gallery's tick counter (with four players, walkers 0–1 advance their idle loop on even ticks, the rest on
    /// odd); `roll` the game's random number generator.
    /// ponytail: singles reactions and the game mode where the favoured side decides (cheer or turn away) are not
    /// recorded; singles always cheers.
    pub fn step(&mut self, players: u32, cheer: bool, tick: i32, roll: &mut impl FnMut() -> u32) {
        if self.counter >= 0 {
            if self.counter == self.slot as i32 {
                self.set_anim(0, players);
                self.random_frame(ps2::div(self.lens[0], 6.0), roll);
            }
            self.counter += 1;
            if (self.slot as i32) < self.counter {
                self.counter = -1;
            }
        }
        if self.mode == 1 {
            if players == 1 {
                self.set_anim(5, players);
            } else {
                self.set_anim(if cheer { 5 } else { 2 }, players);
                self.random_frame(5.0, roll);
            }
            self.mode = 2;
        }
        let ended = self.lens[self.anim as usize] <= self.frame;
        match self.anim {
            1..=4 if ended => self.set_anim(0, players),
            0 if ended => {
                if (roll() >> 16 & 0x7fff) % 100 + 1 < 21 {
                    self.set_anim(1, players);
                } else {
                    self.set_frame(0.0);
                }
            }
            _ => {}
        }
        let go = if players >= 3 && self.anim < 2 { tick % 2 == (self.slot >= 2) as i32 } else { self.advancing };
        if go {
            self.advance();
        }
    }
}

/// Which of `count` walkers cheer a point (the gallery's pick): all of them below four, else three distinct ones
/// drawn at random (a draw that repeats an earlier pick is drawn again).
pub fn cheerers(count: usize, roll: &mut impl FnMut() -> u32) -> [bool; 6] {
    let mut out = [false; 6];
    if count < 4 {
        out[..count].fill(true);
        return out;
    }
    let mut picks = [usize::MAX; 3];
    let mut k = 0;
    while k < 3 {
        let p = (roll() >> 16 & 0x7fff) as usize % count;
        if !picks[..k].contains(&p) {
            picks[k] = p;
            k += 1;
        }
    }
    picks.iter().for_each(|&p| out[p] = true);
    out
}

/// Trigger creature types that just play their sound now and then (`exe::Game::emitter` gives the sound and gap).
pub const EMITTERS: [u8; 19] = [4, 7, 11, 12, 13, 16, 17, 23, 24, 25, 26, 35, 36, 41, 42, 45, 46, 50, 51];
/// The emitter type whose sound sweeps across the stereo field after it starts.
const SWEEPER: u8 = 36;

/// An ambient sound emitter: counts down to its next play, then draws a new gap.
#[derive(Clone, Debug, PartialEq)]
pub struct Emitter {
    pub ty: u8,
    pub row: EmitterRow,
    /// Where the sound plays from.
    pub pos: [f32; 3],
    /// Ticks to the next play (re-armed when it runs out).
    pub timer: i32,
    /// The countdown kept aside while a replay shows.
    pub saved: i32,
    /// Ticks left of the pan sweep after a play.
    pub sweep: i16,
    /// Pan in degrees: the sound's bearing when it starts (`sound::place`), then swept.
    /// ponytail: the library's listener follows the camera in camera modes 7–10 (the camera's eye pulled a third of
    /// the way toward its target, clamped to ±8.685 x / ±17.885 z), so the game's start pan differs there; only
    /// the fixed listener is ported (only type 36 uses the pan audibly)
    pub pan: f32,
    /// Sweep direction: down (true) or up.
    pub down: bool,
}

impl Emitter {
    /// Made at court load: draws the first gap.
    pub fn new(ty: u8, row: EmitterRow, pos: [f32; 3], roll: &mut impl FnMut() -> u32) -> Emitter {
        let mut e = Emitter { ty, row, pos, timer: 0, saved: 0, sweep: 0, pan: 0.0, down: false };
        e.arm(roll);
        e
    }

    fn arm(&mut self, roll: &mut impl FnMut() -> u32) {
        let r = ps2::mul(2.3283064e-10, ps2::utof(roll()));
        self.timer = ps2::mul(self.row.base as f32, ps2::msub(1.0, self.row.jitter, r)) as i32;
    }

    /// One tick; true when the sound starts (program 7, positional at `pos`, volume 0x40).
    pub fn step(&mut self, roll: &mut impl FnMut() -> u32) -> bool {
        let mut play = false;
        if self.timer != 0 {
            self.timer -= 1;
            if self.timer == 0 && self.row.sound != -1 {
                (self.pan, self.sweep, play) = (sound::place(self.pos).0 as f32, 240, true);
                self.down = roll() >> 16 & 1 != 0;
            }
        }
        if self.timer < 1 {
            self.arm(roll);
        }
        if self.ty == SWEEPER && self.sweep != 0 {
            self.sweep -= 1;
            self.pan = if self.down { ps2::sub(self.pan, 0.375) } else { ps2::add(self.pan, 0.375) };
            if 359.0 <= self.pan {
                self.pan = 0.0;
            } else if self.pan <= 0.0 {
                self.pan = 359.0;
            }
        }
        play
    }

    /// The figures' reset (a new point, a serve, a replay starting or ending). `replay`: a replay starts (false)
    /// or ends (true): the countdown is kept aside or taken back.
    pub fn reset(&mut self, replay: Option<bool>) {
        match replay {
            Some(true) => self.timer = self.saved,
            Some(false) => self.saved = self.timer,
            None => {}
        }
        self.sweep = 0;
    }
}

/// A trigger creature's state for the generic engine (`step`), driven by its type's [`TriggerRow`].
/// ponytail: steering (row `steer`), the turn eased through a pause, spline legs, random points about the area,
/// randomised waypoints (rows without `exact`), ground snapping and start modes 2–3 are not ported: no recorded
/// idle creature uses them (they are taken as the plain node / left alone). Add with P14c3–5's recordings.
#[derive(Clone, Debug, PartialEq)]
pub struct Trigger {
    pub ty: u8,
    /// The last message the figures were sent (6: a reset skips the pause).
    pub msg: u32,
    /// Its layout record's position (start point, circle centre) and its path's nodes.
    pub anchor: [f32; 4],
    pub path: Vec<[f32; 4]>,
    /// The path node a reset starts from.
    pub start: i16,
    /// Matrix set by a reset (its layout placement).
    pub home: M4,
    pub world: M4,
    pub active: bool,
    /// Runs its motion and animation (else only the sound countdown and callback).
    pub on: bool,
    pub moving: bool,
    /// Node counter (taken modulo the path length), the one before, and walking the path backwards.
    pub node: i16,
    pub prev: i16,
    pub reverse: bool,
    /// Ticks left of the leg (−1 for none), the leg's length, its unit, the pause between parts, the pause left.
    pub left: i16,
    pub total: i16,
    pub every: i16,
    pub pause: i16,
    pub wait: i16,
    pub vel: [f32; 4],
    pub target: [f32; 4],
    /// The leg's start and end points when they are the path's ends (where the animation slows).
    pub from: [f32; 4],
    pub to: [f32; 4],
    pub snap: i16,
    /// Turn to face the direction of travel on the next move.
    pub orient: bool,
    /// Circling clockwise (seen from above), its angle and radius.
    pub clockwise: bool,
    pub angle: f32,
    pub radius: f32,
    /// Ticks to the idle animation's restart.
    pub repeat: i16,
    /// Pausing at a loop's end.
    pub paused: bool,
    pub animating: bool,
    /// Animation speed (also its sounds' gate).
    pub speed: f32,
    pub stage: i16,
    /// Its animation controller: frame shown, the next, and the animation's length.
    pub frame: f32,
    pub next: f32,
    pub len: f32,
    /// Sound countdown (+ pan sweep, as [`Emitter`]).
    pub timer: i32,
    pub sweep: i16,
    pub pan: f32,
    pub down: bool,
    /// The type's own counters: idle (34, 44) or sound (37) counter, and 44's call countdown and voice.
    pub counter: i32,
    pub gap: i32,
    pub voice: (i32, i32),
    /// Startled (types 0–1, 5, 27–29, 31–32, 39: once, until the next motion reset).
    pub startled: bool,
    /// Type 48's scrub: 0 still, 1 forward, 2 back; and the frame it shows next.
    pub scrub: (i32, i32),
    /// Types 27–29: the ball touched it this tick (the game's message 0x14 to the match).
    pub struck: bool,
}

/// What the startled creatures watch: the players' positions then the ball's, and the per-type "startled" flags
/// all creatures of a type share (set by the first one startled, cleared by the type's reset).
#[derive(Clone, Debug)]
pub struct Near {
    pub pos: Vec<[f32; 4]>,
    pub flags: [bool; 64],
}

impl Default for Near {
    fn default() -> Near {
        Near { pos: Vec::new(), flags: [false; 64] }
    }
}

/// `n` ticks scaled by `1 − jitter·random` (no draw without jitter).
fn jitter((n, j): (i16, f32), roll: &mut impl FnMut() -> u32) -> i16 {
    if j == 0.0 {
        return n;
    }
    ps2::mul(n as f32, ps2::msub(1.0, j, ps2::mul(2.3283064e-10, ps2::utof(roll())))) as i32 as i16
}

fn dist2([x, y, z, _]: [f32; 4]) -> f32 {
    ps2::madd(ps2::madd(ps2::add(0.0, ps2::mul(y, y)), x, x), z, z)
}

fn minus(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|c| ps2::sub(a[c], b[c]))
}

impl Trigger {
    fn pos(&self) -> [f32; 4] {
        self.world[3]
    }

    /// Face along `dir`, keeping the position.
    fn face(&mut self, dir: [f32; 4]) {
        let pos = self.world[3];
        self.world = crate::effect::impact_matrix(dir, [pos[0], pos[1], pos[2]]);
        self.world[3] = pos;
    }

    fn restart_anim(&mut self) {
        (self.frame, self.next) = (0.0, 0.0);
    }

    /// Show frame `t` (held within the animation) and next.
    fn set_frame(&mut self, t: f32) {
        (self.frame, self.next) = (t.clamp(0.0, self.len), t.clamp(0.0, self.len));
    }

    /// The motion reset (a new point, or a loop's end): back to the start, a direction bit with `reverse_roll`,
    /// the first leg and the pause before it.
    pub fn reset(&mut self, row: &TriggerRow, roll: &mut impl FnMut() -> u32) -> Vec<i32> {
        self.reset_near(row, &mut Near::default(), roll)
    }

    /// [`Trigger::reset`] with what the startled creatures watch (their reset clears their type's flag).
    pub fn reset_near(&mut self, row: &TriggerRow, near: &mut Near, roll: &mut impl FnMut() -> u32) -> Vec<i32> {
        // ponytail: types whose row keeps them off after their first reset (row byte +0x6c), and those whose model
        // starts animating, are not modelled
        let o = std::mem::take(self);
        *self = Trigger {
            ty: o.ty, anchor: o.anchor, path: o.path, start: o.start, node: o.start, prev: o.start, home: o.home, world: o.home, msg: o.msg,
            frame: o.frame, next: o.next, len: o.len, timer: o.timer, sweep: o.sweep, pan: o.pan, down: o.down,
            counter: o.counter, gap: o.gap, voice: o.voice, scrub: o.scrub, ..Trigger::default()
        };
        self.stage = row.stage;
        (self.active, self.on, self.moving) = (true, true, !self.path.is_empty());
        if self.moving && row.reverse_roll && roll() >> 16 & 1 != 0 {
            self.reverse = true;
        }
        self.world[3] = match row.start {
            1 => self.anchor,
            _ => self.home[3], // ponytail: start modes 2–3 (random point, on the circle)
        };
        if self.moving {
            self.waypoint(row, true, roll);
            if row.reset_wait.0 != 0 {
                self.pause = jitter(row.reset_wait, roll);
            }
            self.wait = if self.msg == 6 { 0 } else { self.pause };
        }
        self.speed = 1.0;
        self.callback(row, 4, near, roll)
    }

    /// Next waypoint: the leg's target, pause, velocity; `first` (from a reset) also turns to face it.
    fn waypoint(&mut self, row: &TriggerRow, first: bool, roll: &mut impl FnMut() -> u32) {
        self.every = jitter(row.every, roll);
        self.pause = jitter(row.pause, roll);
        let mut face = first && row.orient != 0;
        let n = self.path.len().max(1) as i16;
        if self.node % n == 0 || self.node % n == n - 1 {
            self.from = self.pos();
        }
        let target = match row.mode {
            0 => self.path[((roll() >> 16 & 0x7fff) % n as u32) as usize], // ponytail: area points (rows without `exact`)
            1..=3 | 5 => {
                self.prev = self.node;
                self.node += 1;
                let mut i = self.node % n;
                if row.mode == 5 && i == 0 {
                    self.stage = 1;
                    return;
                }
                if row.mode == 1 && i == 0 {
                    self.reset(row, roll);
                    if row.end_wait.0 != 0 {
                        self.paused = true;
                        self.pause = jitter(row.end_wait, roll);
                        self.wait = self.pause;
                    }
                    return;
                }
                if row.mode == 2 {
                    if i == 0 {
                        self.node += 1;
                        i = self.node % n;
                        if row.end_wait.0 != 0 {
                            self.paused = true;
                            self.pause = jitter(row.end_wait, roll);
                            self.wait = self.pause;
                        }
                        face = true;
                    }
                    if self.node / n & 1 != 0 {
                        i = n - 1 - i;
                    }
                }
                if self.reverse {
                    i = n - 1 - i;
                }
                self.path[i as usize]
            }
            6 => self.anchor,
            _ => self.target, // ponytail: spline legs (mode 4)
        };
        self.target = target;
        if self.node % n == 0 || self.node % n == n - 1 {
            self.to = target;
        }
        if face {
            let d = minus(target, self.pos());
            let [x, y, z, _] = d;
            let q = ps2::div(1.0, ps2::sqrt(ps2::madd(ps2::madd(ps2::mul(y, y), x, x), z, z)));
            self.face(d.map(|c| ps2::mul(c, q)));
        }
        self.left = -1;
        self.orient = row.orient != 0;
        if row.mode != 6 && row.speed != 0.0 && row.steer == 0.0 {
            let d = minus(target, self.pos());
            let s = dist2(d);
            let dist = ps2::sqrt(s);
            let unit = if self.every == 0 { 1 } else { self.every };
            let f = ps2::mul(row.speed, unit as f32);
            let ticks = unit.wrapping_mul(ps2::div(ps2::add(dist, ps2::sub(f, 1.0)), f) as i32 as i16).max(unit);
            (self.total, self.left) = (ticks, ticks);
            let (inv, step) = (ps2::div(1.0, ps2::sqrt(s)), ps2::div(dist, ticks as f32));
            self.vel = d.map(|c| ps2::mul(ps2::mul(c, inv), step));
        }
    }

    /// One tick; the sounds started.
    pub fn step(&mut self, row: &TriggerRow, roll: &mut impl FnMut() -> u32) -> Vec<i32> {
        self.step_near(row, &mut Near::default(), roll)
    }

    /// [`Trigger::step`] with what the startled creatures watch.
    pub fn step_near(&mut self, row: &TriggerRow, near: &mut Near, roll: &mut impl FnMut() -> u32) -> Vec<i32> {
        if !self.active {
            return Vec::new();
        }
        let mut sounds = Vec::new();
        if self.on {
            if self.moving && self.wait == 0 {
                if self.left > 0 {
                    self.left -= 1;
                }
                if row.mode == 6 {
                    let dir = if self.clockwise { -1.0 } else { 1.0 };
                    let mut a = ps2::madd(self.angle, ps2::mul(0.017453292, row.speed), dir);
                    if world::PI < a {
                        a = ps2::sub(a, world::TWO_PI);
                    } else if a < -world::PI {
                        a = ps2::add(world::TWO_PI, a);
                    }
                    self.angle = a;
                    let m = world::mat_mul(&world::IDENTITY, &world::rot_y(a));
                    self.world[3] = std::array::from_fn(|c| ps2::madd(self.target[c], m[2][c], self.radius));
                    if self.orient {
                        let pos = self.world[3];
                        let turn = world::mat_mul(&world::IDENTITY, &world::rot_y(ps2::mul(dir, world::HALF_PI)));
                        self.world = world::mat_mul(&m, &turn);
                        self.world[3] = pos;
                    }
                } else {
                    // ponytail: steering creatures (row `steer`) move like the rest
                    self.world[3] = std::array::from_fn(|c| ps2::add(self.world[3][c], self.vel[c]));
                    if self.orient {
                        let v = self.vel;
                        self.face(if row.orient == 2 { [v[0], 0.0, v[2], 0.0] } else { v });
                        self.orient = false;
                    }
                }
                if self.snap != 0 {
                    self.snap -= 1;
                    if self.snap == 0 {
                        self.snap = row.snap as i16; // ponytail: the ground snap itself (collision) not ported
                    }
                }
                let pause = self.every != 0 && self.left % self.every == 0;
                let mut next = false;
                if pause && row.retarget && self.left != 0 {
                    next = true;
                    self.node = self.prev;
                }
                if row.speed != 0.0 && self.left == 0 {
                    next = true;
                }
                if next {
                    self.waypoint(row, false, roll);
                }
                if pause {
                    self.wait = self.pause;
                }
            } else if self.moving {
                self.wait -= 1;
                if row.steer == 0.0 && self.wait == 0 && row.wake {
                    self.repeat = 1;
                }
                if self.paused && self.wait == 0 {
                    self.paused = false;
                }
            }
            if row.near != [0.0; 2] || row.stages != 0 {
                let (mut speed, mut changed) = (self.speed, false);
                if self.moving {
                    for (p, r) in [(self.from, row.near[0]), (self.to, row.near[1])] {
                        let d = ps2::sqrt(dist2(minus(p, self.pos())));
                        if r != 0.0 && d < r {
                            (speed, changed) = (ps2::div(d, r), true);
                        }
                    }
                }
                if row.stages != 0 && self.stage < row.stages {
                    (speed, changed) = (ps2::div(self.stage as f32, row.stages as f32), true);
                }
                if !changed && self.speed != 1.0 {
                    (speed, changed) = (1.0, true);
                }
                if changed {
                    self.speed = speed;
                }
            }
            if self.animating {
                if self.repeat != 0 {
                    self.repeat -= 1;
                    if self.repeat == 0 {
                        self.restart_anim();
                    }
                }
                if row.repeat.0 >= 1 && self.repeat == 0 && self.len <= self.frame {
                    self.repeat = jitter(row.repeat, roll);
                }
                // the controller: show the next frame (held at the ends), step on by one
                self.frame = self.next.clamp(0.0, self.len);
                self.next = ps2::add(self.frame, 1.0);
            }
        }
        if self.timer != 0 {
            self.timer -= 1;
            if self.timer == 0 && row.sound != -1 {
                sounds.push(row.sound);
                (self.pan, self.sweep) = (sound::place([self.world[3][0], self.world[3][1], self.world[3][2]]).0 as f32, 240);
                self.down = roll() >> 16 & 1 != 0;
            }
        }
        sounds.extend(self.callback(row, 2, near, roll));
        sounds
    }

    /// The type's own behaviour on message `msg` (2 each tick, 4 at a reset); the sounds started.
    fn callback(&mut self, row: &TriggerRow, msg: u8, near: &mut Near, roll: &mut impl FnMut() -> u32) -> Vec<i32> {
        let mut sounds = Vec::new();
        let u = |r: u32| ps2::mul(2.3283064e-10, ps2::utof(r));
        let pos = self.pos();
        let within = |r: f32| near.pos.iter().map(|&p| ps2::sqrt(dist2(minus(p, pos))) < r).collect::<Vec<_>>();
        let ty = self.ty as usize;
        match (self.ty, msg) {
            // startled within 2.0 (the first of its type this point): off along its path with a one-shot animation
            (0 | 1 | 5 | 31 | 32 | 39, 2) if !self.startled => {
                for near_by in within(2.0) {
                    if near_by && !near.flags[ty] {
                        self.startled = true;
                        self.moving |= ty != 31;
                        if ty == 1 || ty == 31 {
                            self.timer = 1;
                        }
                        (self.on, self.animating) = (true, true);
                        self.restart_anim();
                        // ponytail: its byte +0xbc (set here) is not kept
                        near.flags[ty] = true;
                    }
                }
            }
            (0 | 1 | 5 | 31 | 32 | 39, 2) => {
                // 31 sets off at frame 35; the animation's end holds on a type's frame
                if ty == 31 && self.frame == 35.0 {
                    self.moving = true;
                }
                if self.len <= self.frame {
                    self.set_frame(match ty {
                        0 => 28.0,
                        31 => 36.0,
                        _ => 32.0,
                    });
                }
            }
            (0 | 1 | 5 | 31 | 32 | 39, 0 | 4) => {
                // ponytail: 32 stays on with its controller at speed 1 (the others at 0): the controller speed and
                // the facing angles are not kept
                (self.on, self.animating, self.moving, self.world) = (ty == 32, false, false, self.home);
                self.set_frame(0.0);
                near.flags[ty] = false;
            }
            // within 4.0: the animation once; the ball's box (±0.2) on its own tells the match (message 0x14)
            (27..=29, 2) => {
                self.struck = false;
                if self.startled {
                    return sounds;
                }
                let last = near.pos.len().wrapping_sub(1);
                for (i, near_by) in within(4.0).into_iter().enumerate() {
                    if near_by {
                        (self.startled, self.animating) = (true, true);
                        self.restart_anim();
                    }
                    let b = near.pos[i];
                    let (lo, hi) = (|v: f32| ps2::sub(v, 0.2), |v: f32| ps2::add(0.2, v));
                    if i == last && (0..3).all(|c| lo(b[c]) < hi(pos[c]) && !(hi(b[c]) <= lo(pos[c]))) {
                        (self.startled, self.struck) = (true, true);
                    }
                }
            }
            (27..=29, 4) => (self.on, self.moving, self.world, self.animating) = (true, false, self.home, false),
            // scrubs forward while someone is within 1.0, to the end; then back to 0 the next time
            (48, 2) => {
                let (mut s, mut f) = self.scrub;
                let far = || within(1.0).iter().all(|&n| !n);
                match s {
                    0 if !far() => s = if f != 0 { 2 } else { 1 },
                    1 => {
                        self.set_frame(f as f32);
                        f += 1;
                        if self.len <= f as f32 && far() {
                            s = 0;
                        }
                        f = (f as f32).min(self.len) as i32;
                    }
                    2 => {
                        self.set_frame(f as f32);
                        f -= 1;
                        if f < 1 && far() {
                            s = 0;
                        }
                        f = f.max(0); // ponytail: the game's floor is a global, 0 in every state seen
                    }
                    _ => {}
                }
                self.scrub = (s, f);
            }
            (48, 4) => {
                (self.on, self.moving, self.world, self.startled, self.animating) = (true, false, self.home, false, false);
                self.restart_anim();
                self.scrub = (0, self.frame as i32);
            }
            (34, 2) if !self.animating => {
                self.counter -= 1;
                if self.counter < 0 {
                    self.animating = true;
                    self.restart_anim();
                    sounds.push(row.sound);
                }
            }
            (34, 2) if self.len <= self.frame => {
                self.counter = ps2::madd(300.0, 300.0, u(roll())) as i32;
                self.animating = false;
            }
            (34, 4) => {
                // ponytail: the facing angles it also sets are not kept
                (self.world, self.on, self.moving, self.animating) = (self.home, true, false, false);
                self.counter = ps2::madd(300.0, 300.0, u(roll())) as i32;
            }
            (37, 2) if self.speed == 0.0 => self.counter = 0,
            (37, 2) => {
                let r = roll();
                // ponytail: with a game flag clear the first call always plays (not found set otherwise)
                if self.counter == 0 && (r >> 16 & 0x7fff) % 100 + 1 < 41 {
                    sounds.push(0x28);
                }
                if self.counter % 25 == 0 {
                    sounds.push(row.sound);
                }
                self.counter = self.counter.wrapping_add(1);
            }
            (37, 4) => self.counter = 0,
            (44, 2) if self.on => {
                // ponytail: unrecorded (deciding set only); ported from the game's code
                self.gap -= 1;
                if self.gap < 0 && self.voice.0 != -1 {
                    sounds.push(row.sound);
                    self.gap = ps2::mul(self.voice.1 as f32, ps2::msub(1.0, row.idle.1, u(roll()))) as i32;
                }
                if !self.animating {
                    self.counter -= 1;
                    if self.counter < 0 {
                        self.animating = true;
                        self.restart_anim();
                    }
                } else if self.len <= self.frame {
                    self.counter = jitter(row.idle, roll) as i32;
                    self.animating = false;
                }
            }
            _ => {}
        }
        sounds
    }
}

impl Default for Trigger {
    fn default() -> Trigger {
        Trigger {
            ty: 0,
            msg: 0,
            anchor: [0.0; 4],
            path: Vec::new(),
            start: 0,
            home: world::IDENTITY,
            world: world::IDENTITY,
            active: false,
            on: false,
            moving: false,
            node: 0,
            prev: 0,
            reverse: false,
            left: 0,
            total: 0,
            every: 0,
            pause: 0,
            wait: 0,
            vel: [0.0; 4],
            target: [0.0; 4],
            from: [0.0; 4],
            to: [0.0; 4],
            snap: 0,
            orient: false,
            clockwise: false,
            angle: 0.0,
            radius: 0.0,
            repeat: 0,
            paused: false,
            animating: false,
            speed: 0.0,
            stage: 0,
            frame: 0.0,
            next: 0.0,
            len: 0.0,
            timer: 0,
            sweep: 0,
            pan: 0.0,
            down: false,
            counter: 0,
            gap: 0,
            voice: (-1, 0),
            startled: false,
            scrub: (0, 0),
            struck: false,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn three_distinct_cheerers() {
        assert_eq!(super::cheerers(2, &mut || unreachable!()), [true, true, false, false, false, false]);
        // draws 1, 1 (again), 4, 1, 0: picks 1, 4, 0
        let mut draws = [1u32, 1, 4, 1, 0].into_iter().map(|d| d << 16);
        assert_eq!(super::cheerers(5, &mut || draws.next().unwrap()), [true, true, false, false, true, false]);
    }
}
