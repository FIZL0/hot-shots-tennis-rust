//! The doubles AI's rally routines: the receive (waiting for a ball coming its way, running to the contact spot,
//! timing the swing), with the path copy, the landing and bounce picks, the dive check, the smash search, the
//! ready check and the stroke-frame pickers it calls; and the NET and BASE rally routines (waiting in the formation
//! between shots, the shot records the partners leave each other, the volley search).
#![deny(clippy::float_arithmetic)]

use crate::ai::{formation, moving, style, AiParams, Mind, PathBall, Runner, Searcher, NET_GAP};
use crate::aim::{contact, plan, Look, Pair};
use crate::instant::FIRST_CPU;
use crate::judge::Call;
use crate::lines::{BASELINE, DOUBLES_SIDELINE, SINGLES_SIDELINE};
use crate::score::{AD_COURT, DEUCE_COURT};
use crate::position::{zone_in, Cue, Formation, Return, Team};
use crate::player::{self, ReachStats, Stats};
use crate::ps2::F;
use crate::rng::{coin, r15};
use crate::swing::{DIVE_CONE, STRETCH};

/// The most path entries the AI copies.
pub const PATH_MAX: usize = 180;
/// The seen flags the tiered search marks (one per path entry).
pub const SEEN: usize = PATH_MAX;
/// How many ball-lost frames the player logs.
pub const LOST_LOG: usize = 50;

/// The match phase as the routines read it (`World::phase`; the mover's `player::Scene::phase` too).
pub mod phase {
    pub const CHANGE_ENDS: u8 = 1;
    pub const SERVE: u8 = 2;
    pub const RALLY: u8 = 3;
    pub const POINT_OVER: u8 = 4;
}

/// The match's messages to the AI side, in the original's numbering (the app's `doubles_ai::heard`).
pub mod msg {
    pub const CHANGE_ENDS: u8 = 0xc;
    pub const SERVE: u8 = 0xe;
    /// A player struck the ball.
    pub const STRIKE: u8 = 0x14;
    /// A shot was hit (after the strike's contact).
    pub const HIT: u8 = 0x15;
    /// A new ball path began.
    pub const NEW_PATH: u8 = 0x16;
    /// The players react to the point.
    pub const REACTION: u8 = 0x19;
}

/// The receive's state and the NET/BASE routines' substate (`Rally::state`, `Rally::sub`).
pub mod step {
    /// Waiting for a ball its way and searching the path for a contact (walking its guess or chasing meanwhile).
    pub const WAIT: u8 = 0;
    /// Running to the stand spot.
    pub const RUN: u8 = 1;
    /// There, waiting for the frame to press the button.
    pub const READY: u8 = 2;
    /// Pressed: the stroke plays out.
    pub const SWING: u8 = 3;
}

/// The serve guess as the routines keep it (`Rally::guess`): `ai::Guess` as a byte.
pub mod guess {
    pub const NONE: u8 = 0;
    pub const WIDE: u8 = 1;
    pub const OTHER: u8 = 2;
}

/// A shot record's frames to contact when it has none (`Shot::n`): cleared by a message, nothing found, hitting.
pub mod record {
    pub const CLEARED: i32 = -1;
    pub const NONE: i32 = -2;
    pub const HITTING: i32 = -3;
}

/// The controller the app gives every player the routines run (a computer player's, `FIRST_CPU` on).
pub const CPU: i32 = 0x21;
/// `Body::lost` once the AI has lost the ball.
pub const LOST: i32 = 2;
/// The voice state from which the partner no longer answers (`Body::mate_voice`).
const VOICE_BUSY: u8 = 2;
/// The button press goes in once the frames to the swing plus the lead fall under this.
const PRESS_AT: i32 = 9;
/// The ready check's answer when the ball is in reach without moving.
const IN_REACH: i32 = 2;
/// The swing frames when the stroke-frame pick finds nothing.
const SWING_DEFAULT: i32 = 8;
/// The path entry it heads for when nothing better is found.
const FALLBACK_ENTRY: i32 = 11;
/// A difference under this (m) counts as none in a walk.
const WALK_SNAP: f32 = f32::from_bits(0x3a83_126f); // 0.001
/// The ball's height (y down) the net check measures against: a net ball above this on the hitter's half isn't read.
const NET_BALL: f32 = f32::from_bits(0xbf68_f5c3); // -0.91
/// Gravity's share in one step of the landing's fly-on (about 9.8 / 60², the game's bits).
const GRAVITY_STEP: f32 = f32::from_bits(0x3b32_6750);
/// The dive check's height band, as multiples of the stroke height.
const DIVE_LOW: f32 = 0.2;
const DIVE_HIGH: f32 = 1.8;
/// How far a bounce pick lands ahead of the landing (m): while receiving, and in the rally routines.
const LANDING_AHEAD: f32 = 0.5;
const RALLY_LANDING_AHEAD: f32 = 0.3;
/// The middle of the doubles half (half the doubles width): where a player holds while its partner plays.
const HALF_MIDDLE: f32 = 2.7425;
/// The singles net dash keeps within ⅔ of the singles half width (2.743…, the game's bits).
const DASH_WIDTH: f32 = f32::from_bits(0x402f_92c5);

/// One entry of the predicted ball path as the AI copies it: position, velocity (4 words each) and bounces.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Ball {
    pub pos: [f32; 4],
    pub vel: [f32; 4],
    pub bounces: i32,
}

/// The AI's copy of the predicted path, shared by both AIs: the frame it was taken and its entries.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PathCopy {
    pub stamp: i32,
    pub balls: Vec<Ball>,
}

impl Ball {
    /// Flight `f`'s path entry (stored y up), `base` bounces before the path.
    pub fn of(f: &crate::ball::Flight, base: i32) -> Self {
        let ([x, y, z], [u, v, s]) = (f.ball.pos, f.ball.vel);
        Ball { pos: [x, (-F(y)).0, z, 0.0], vel: [u, (-F(v)).0, s, 0.0], bounces: base + f.contacts }
    }
}

/// What a path not stepped ahead is stepped from: the ball off the net, its shot, the court and the bounces before it.
/// A held ball's path makes no line call, so nothing but its entries comes of stepping it, and the app leaves that to
/// the copy, on the frames one is taken (P38).
#[derive(Clone, Copy, Debug)]
pub struct PathFrom {
    pub flight: crate::ball::Flight,
    pub shot: crate::ball::Shot,
    pub court: usize,
    pub base: i32,
}

impl PathFrom {
    /// The PATH_MAX entries from it.
    pub fn step(&self) -> Vec<Ball> {
        let mut f = self.flight;
        (0..PATH_MAX)
            .map(|_| {
                let b = Ball::of(&f, self.base);
                f.step(&self.shot, &crate::ball::COURTS[self.court]);
                b
            })
            .collect()
    }
}

/// The match as the rally routines read it.
#[derive(Clone, Debug, Default)]
pub struct World {
    pub frame: i32,
    /// Match phase (`phase::*`).
    pub phase: u8,
    pub players: i32,
    /// Court side: 0 deuce, 1 ad.
    pub court: i32,
    pub receiver: i32,
    /// The last hitter's slot (−1 none) and the shot count.
    pub hitter: i32,
    pub shots: i32,
    /// The ball's position (y down, unlike the path) and its bounces and court contacts (a net touch counts only as
    /// a bounce, so they differ once the ball has touched the net).
    pub ball: [f32; 4],
    pub hits: [i32; 2],
    pub gravity: f32,
    pub drag: f32,
    pub floor: i32,
    /// The predicted path (its live window) the AI copies from.
    pub path: Vec<Ball>,
    /// Or (when set) what to step it from, if a copy is taken.
    pub path_from: Option<PathFrom>,
    /// The players' shot records (by slot), the path's line call (`judge::Call` as a byte) and its line gap.
    pub records: [Shot; 4],
    pub path_mode: u8,
    pub path_gap: f32,
}

/// A player's shot record: the frame it was written, the frames to its contact (relative to `stamp`; `record::*`
/// when it has none), the contact kind (`aim::contact::*`), where the player stood and where it runs to.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Shot {
    pub stamp: i32,
    pub n: i32,
    pub kind: i32,
    pub pos: [f32; 4],
    pub vec: [f32; 4],
}

impl World {
    /// The record of `slot`, its frames made absolute.
    fn shot(&self, slot: i32) -> Shot {
        let mut s = self.records[slot as usize];
        if s.n >= 0 {
            s.n += s.stamp;
        }
        s
    }

    fn put(&mut self, slot: i32, n: i32, kind: i32, pos: [f32; 4], vec: [f32; 4]) {
        self.records[slot as usize] = Shot { stamp: self.frame, n, kind, pos, vec };
    }

    /// A record without a contact (`n` one of `record::*`): kind 0, nowhere to run.
    fn put_none(&mut self, slot: i32, n: i32, pos: [f32; 4]) {
        self.put(slot, n, 0, pos, [0.0; 4]);
    }
}

/// The AI's player as the rally routines read it.
#[derive(Clone, Debug, Default)]
pub struct Body {
    pub pos: [f32; 4],
    pub facing: [f32; 4],
    /// Run velocity x, z.
    pub vel: [f32; 2],
    pub side: f32,
    /// Hand (±1).
    pub hand: f32,
    pub team: i32,
    pub size: i32,
    pub stats: Stats,
    pub stamina: i32,
    pub tick: i32,
    pub run: i32,
    /// Move state (`ai::moving::*`).
    pub moving: u8,
    /// Depth stat; the smash stand offset (x, z).
    pub depth: f32,
    pub smash_off: [f32; 2],
    /// Controller (under `FIRST_CPU` a human pad), doubles formation (`ai::formation::*`).
    pub control: i32,
    pub formation: u8,
    /// Swing countdown (−1 idle); stroke busy (0 once the stroke is over).
    pub swing: i32,
    pub stroke: u8,
    /// Ball-lost state (`LOST` once lost) and the frames it was lost on.
    pub lost: i32,
    pub lost_log: Vec<i32>,
    pub reach: ReachStats,
    /// TParam's hand (`ai::strong::*`).
    pub strong: u8,
    /// The partner's and the opponents' positions; the partner's voice state; the hit mark.
    pub mate: [f32; 4],
    pub opp: [[f32; 4]; 2],
    pub mate_voice: u8,
    pub mark: u8,
    /// Singles: its shot's target (the net dash's stand spot after a hit); the opponent plays left-handed.
    pub target: [f32; 4],
    pub opp_lefty: bool,
}

/// The rally AI state (doubles, or singles with `singles`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rally {
    pub level: u8,
    /// Beside a human partner (> 0).
    pub mate: i32,
    /// The path window: first entry, entries wanted, singles, entries copied.
    pub first: i32,
    pub window: i32,
    pub singles: bool,
    pub len: i32,
    /// The receive's state (`step::*`).
    pub state: u8,
    /// The contact found: stand spot, ball, run frames, index (from `first`).
    pub stand: [f32; 4],
    pub ball: [f32; 4],
    pub frames: i32,
    pub index: i32,
    /// A new ball to search for; the contact kind (`aim::contact::*`).
    pub fresh: bool,
    pub kind: u8,
    pub stick: [f32; 4],
    pub hold: bool,
    /// The shot's plan (`aim::plan::*`).
    pub plan: u8,
    /// Frames to the swing, and the lead it presses the button by (per kind: stroke, volley, smash).
    pub swing: i32,
    pub lead: i32,
    pub leads: [i32; 3],
    pub dive: bool,
    /// The guess (`guess::*`) and its walk's frames left.
    pub guess: u8,
    pub wait: i32,
    pub body: bool,
    pub low: bool,
    pub push: bool,
    /// The spot the guess walk started from (x, z).
    pub from: [f32; 2],
    /// NET/BASE: the substate (`step::*`), the volley level pick, the centre rate and radius.
    pub sub: u8,
    pub volley_level: u8,
    pub rate: i32,
    pub radius: f32,
    /// Chase the bounce while nothing's found; held back (a line call coming); the formation spot (x, z of 4).
    pub chase: bool,
    pub held: bool,
    pub spot: [f32; 4],
    /// The path entry the next search starts from (−1 the window's start).
    pub next: i32,
    /// Following the partner's shot; holding the middle; its team hit (with the frames since).
    pub follow: bool,
    pub middle: bool,
    pub ours: bool,
    pub tick: i32,
    pub voice: bool,
    /// The voice line it called on giving way (the key bit), for the caller to play; not the game's.
    pub said: Option<bool>,
    pub lean: i32,
    pub back: Return,
    pub lane: u8,
    pub front: bool,
    pub forward: bool,
    /// The ALL-style net pick (re-picked on each return to substate 0).
    pub mind: Mind,
    /// Giving way to the partner.
    pub defer: bool,
    /// The caller's stack quad the bounce picks start from (the game leaves it uninitialised).
    pub stash: [f32; 4],
    /// Singles: the net reach (the dash's depth), the net dash, the opponent's last shot's spot, the return level
    /// pick (0..=2, the row's `return_level` mix).
    pub reach: f32,
    pub dash: bool,
    pub last: [f32; 4],
    pub return_level: u8,
    /// Singles NET/BASE: the aim's short-ball flag, the high-ball level pick, the opponent's last shot kind.
    pub short: bool,
    pub high_level: u8,
    pub opp_kind: i32,
}

/// What a call leaves for the player: the run target or stick, and the button.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Out {
    pub stick: [f32; 4],
    pub button: Option<u32>,
}

fn abs(v: f32) -> f32 {
    if v < 0.0 { (-F(v)).0 } else { v }
}

fn sign(v: f32) -> i32 {
    if v < 0.0 { -1 } else { 1 }
}

fn draw(roll: &mut dyn FnMut() -> u32, n: u32) -> u32 {
    r15(roll()) % n
}

fn chance(roll: &mut dyn FnMut() -> u32, p: i32) -> bool {
    (draw(roll, 100) as i32) < p
}

fn zero(v: &[f32; 4]) -> bool {
    v.iter().all(|c| c.to_bits() == 0)
}

/// The plan's button (`aim::button`).
pub fn button(plan: u8) -> u32 {
    crate::aim::button(plan) as u32
}

impl Body {
    /// One frame's run speed as the AI's moves estimate it.
    fn step(&self, w: &World) -> f32 {
        let (run, st) = match self.moving {
            moving::STANDING | moving::STOPPED => (0, self.stamina),
            moving::RUNNING => (
                self.run + 1,
                player::drain(&self.stats, self.stamina, self.tick, w.players, w.phase == phase::RALLY, w.floor).0,
            ),
            _ => (self.run, self.stamina),
        };
        player::run_speed(&self.stats, run, st, self.size)
    }

    /// The walk toward `out`: leaves the unit direction there (zero when there, or within ⅔ of a step unless
    /// `keep`); true when it arrives.
    fn walk(&self, w: &World, out: &mut [f32; 4], keep: bool) -> bool {
        let mut d: [f32; 4] = std::array::from_fn(|k| (F(out[k]) - self.pos[k]).0);
        if abs(d[0]) < WALK_SNAP {
            d[0] = 0.0;
        }
        if abs(d[2]) < WALK_SNAP {
            d[2] = 0.0;
        }
        (d[1], d[3]) = (0.0, 0.0);
        let mut len = (F(d[2]) * d[2]).madd(d[0], d[0]).sqrt().0;
        if len <= 0.0 {
            *out = [0.0; 4];
            return true;
        }
        let inv = (F(1.0) / len).0;
        *out = d.map(|c| (F(c) * inv).0);
        let step = self.step(w);
        if !keep && len <= (F(2.0) * step / 3.0).0 {
            *out = [0.0; 4];
        } else {
            len = (F(len) - step).0;
        }
        len <= 0.0 || zero(out)
    }

    /// One frame's move along `dir` as the stick would give it.
    fn nudge(&self, w: &World, dir: [f32; 4], me: &mut [f32; 4]) {
        let b = player::bot_stick([dir[0], dir[2]]);
        let [x, z] = player::stick_dir(b, 3);
        let step = self.step(w);
        me[0] = (me[0] + F(x) * step).0;
        me[2] = (me[2] + F(z) * step).0;
    }

    /// The ball-lost mark (a computer player's, in the rally).
    fn lose(&mut self, w: &World) {
        if self.control < FIRST_CPU {
            return;
        }
        if w.phase == phase::RALLY && self.lost_log.len() < LOST_LOG {
            self.lost_log.push(w.frame);
        }
        self.lost = LOST;
    }
}

/// A stroke-frame picker's limits.
struct Pick {
    /// Stop past this many bounces; skip balls nearer the net than `net`, with fewer than `min` bounces.
    stop: i32,
    net: f32,
    min: i32,
    low: f32,
    high: f32,
    /// Distance band; `flat` measures it on the ground.
    near: f32,
    far: f32,
    flat: bool,
    /// The depth the score stands back by.
    depth: f32,
}

impl Rally {
    fn path(&self, c: &PathCopy) -> Vec<PathBall> {
        c.balls.iter().map(|b| PathBall { pos: [b.pos[0], b.pos[1], b.pos[2]], bounces: b.bounces }).collect()
    }

    /// Copies the predicted path (once a frame, shared) unless the ball isn't coming its way (no hitter, its own team's
    /// hit, the serve before its return for a non-receiver, a serve that touched the net, a ball that touched the net
    /// and is below its top on the hitter's half); the entries copied. With none it gives the ball up, chase too.
    pub fn copy_path(&mut self, b: &Body, w: &World, c: &mut PathCopy, seen: &mut [bool], check: bool) -> i32 {
        seen.fill(false);
        if check {
            let h = w.hitter;
            if h < 0 {
                self.len = 0;
                return 0;
            }
            let ok = (h & 1) != (b.team & 1)
                && (self.singles || w.shots > 1 || w.receiver == b.team)
                && (w.shots != 1 || w.hits[0] == w.hits[1]);
            if !ok
                || (w.hits[0] != w.hits[1]
                    && !(w.ball[1] <= NET_BALL)
                    && sign(b.side) == sign(w.ball[2]))
            {
                self.len = 0;
                return 0;
            }
        }
        if c.stamp == w.frame {
            self.len = c.balls.len() as i32;
            return self.len;
        }
        let stepped = w.path_from.map(|p| p.step());
        let path = stepped.as_ref().unwrap_or(&w.path);
        let n = (self.window.max(0) as usize).min(path.len());
        c.balls = path[..n].to_vec();
        c.stamp = w.frame;
        self.len = n as i32;
        self.len
    }

    fn landing_short(&self, c: &PathCopy) -> bool {
        self.len != 0 && c.balls[self.len as usize - 1].bounces < 2
    }

    /// The path entry `n`.
    fn entry(&self, c: &PathCopy, n: i32) -> Option<[f32; 4]> {
        (n < self.len).then(|| c.balls[n as usize].pos)
    }

    /// The last entry; true when it ends on the other half.
    fn last(&self, b: &Body, c: &PathCopy, out: &mut [f32; 4]) -> bool {
        if self.len == 0 {
            return false;
        }
        *out = c.balls[self.len as usize - 1].pos;
        sign(b.side) != sign(out[2])
    }

    /// The first bounce's descent after it rises; true when it is on the other half.
    fn after_bounce(&self, b: &Body, c: &PathCopy, out: &mut [f32; 4]) -> bool {
        let mut up = false;
        for e in &c.balls[..self.len.max(0) as usize] {
            if !up && e.bounces == 1 && !(e.vel[1] <= 0.0) {
                up = true;
            }
            if up && e.bounces == 1 && e.vel[1] < 0.0 {
                *out = e.pos;
                return sign(b.side) != sign(out[2]);
            }
        }
        false
    }

    /// Where the ball first lands (flying the last entry on if it hasn't), `half` a metre on, a reach to the
    /// nearer side.
    fn landing(&self, b: &Body, w: &World, c: &PathCopy, half: f32, out: &mut [f32; 4]) -> bool {
        let len = self.len.max(0) as usize;
        if c.balls[0].bounces > 0 {
            return false;
        }
        let me = b.pos;
        let horiz = |v: &[f32; 4]| (F(v[2]) * v[2]).madd(v[0], v[0]).sqrt().0;
        let mut v;
        if let Some(e) = c.balls[..len].iter().find(|e| e.bounces > 0) {
            *out = e.pos;
            v = e.vel;
            let h = horiz(&v);
            if !(h <= 0.0) {
                let inv = (F(1.0) / h).0;
                v = v.map(|c| (F(c) * inv).0);
            }
        } else {
            let e = c.balls[len - 1];
            *out = e.pos;
            v = e.vel;
            while !(out[1] <= 0.0) {
                let s = (F(v[1]) * v[1]).madd(v[0], v[0]).madd(v[2], v[2]).sqrt().0;
                v = v.map(|c| (F::ZERO + c).msub(F(c) * s, w.drag).0);
                v[1] = (F::ZERO + v[1]).msub(GRAVITY_STEP, w.gravity).0;
                for k in 0..4 {
                    out[k] = (F(out[k]) + v[k]).0;
                }
            }
            let h = horiz(&v);
            if !(h <= 0.0) {
                let inv = (F(1.0) / h).0;
                v = v.map(|c| (F(c) * inv).0);
            }
        }
        let p: [f32; 4] = std::array::from_fn(|k| (F::ZERO + out[k]).madd(v[k], half).0);
        let rs = (F(b.reach.reach) * b.side).0;
        let dz = (F(p[2]) - me[2]).0;
        let (l, r) = ((F(p[0]) - rs).0, (F(p[0]) + rs).0);
        let (dl, dr) = ((F(l) - me[0]).0, (F(r) - me[0]).0);
        let d1 = (F(dz) * dz).madd(dl, dl).0;
        let d2 = (F(dz) * dz).madd(dr, dr).0;
        *out = p;
        out[0] = if d1 <= d2 { l } else { r };
        true
    }

    /// The dive check: false (dive) when a ball 5..16 frames on, on the other half, ahead of the player within
    /// dive height, is just out of its run.
    fn no_dive(&self, b: &Body, c: &PathCopy) -> bool {
        if b.moving != moving::RUNNING {
            return true;
        }
        let n = (self.len as usize).min(16);
        let speed = (F(b.vel[1]) * b.vel[1]).madd(b.vel[0], b.vel[0]).sqrt().0;
        let run = (F(1.25) * speed).0;
        let r = &b.reach;
        let (low, high) = ((F(DIVE_LOW) * r.stroke_height).0, (F(DIVE_HIGH) * r.stroke_height).0);
        let far = (F(STRETCH) * r.reach).0;
        for t in 5..n {
            let e = c.balls[t].pos;
            if abs(e[2]) < 0.5 || sign(b.side) == sign(e[2]) || !(low <= e[1] && e[1] <= high) {
                continue;
            }
            let mut d: [f32; 4] = std::array::from_fn(|k| (F(e[k]) - b.pos[k]).0);
            d[1] = 0.0;
            let len = (F(d[2]) * d[2]).madd(d[0], d[0]).sqrt().0;
            let inv = (F(1.0) / len).0;
            let n = [(F(d[0]) * inv).0, (F(0.0) * inv).0, (F(d[2]) * inv).0];
            let f = b.facing;
            let dot = (F(f[1]) * n[1]).madd(f[0], n[0]).madd(f[2], n[2]).0;
            if dot < DIVE_CONE {
                continue;
            }
            let lim = (2.0 * (F::ZERO + far).madd(run, t as f32)).0;
            if !(lim < len) {
                return false;
            }
        }
        true
    }

    /// The smash search: the ball nearest the ideal smash height, falling, in the smash window and in time.
    fn smash_search(&self, b: &Body, w: &World, c: &PathCopy, from: i32, min: i32) -> Option<([f32; 4], [f32; 4], i32, i32)> {
        let r = &b.reach;
        let [high, ideal, low] = r.smash;
        let runner = self.runner(b, w);
        let off_x = |x: f32| (x - b.hand * (F(b.smash_off[0]) * b.side) / 2.0).0;
        let off_z = |z: f32| (z - F(b.smash_off[1]) * b.side / 2.0).0;
        let mut best: Option<(usize, f32, i32)> = None;
        for at in (if from < 0 { self.first } else { from }).max(0) as usize..self.len.max(0) as usize {
            let e = c.balls[at];
            if e.bounces >= 2 {
                break;
            }
            let [x, y, z, _] = e.pos;
            if e.bounces < min || sign(b.side) == sign(z) || abs(z) < NET_GAP || !(e.vel[1] <= 0.0) {
                continue;
            }
            if !(low <= y && y <= high) {
                continue;
            }
            let (sx, sz) = (off_x(x), off_z(z));
            if !(abs(sz) < BASELINE) {
                continue;
            }
            let score = abs((F(y) - ideal).0);
            let frames = runner.frames_to([sx, sz]);
            if (at as i32 - self.first) < frames {
                continue;
            }
            if best.is_none_or(|(_, s, _)| score < s) {
                best = Some((at, score, frames));
            }
        }
        let (at, _, frames) = best?;
        let ball = c.balls[at].pos;
        let mut stand = ball;
        stand[0] = off_x(stand[0]);
        stand[2] = off_z(stand[2]);
        Some((stand, ball, frames, at as i32 - self.first))
    }

    fn runner(&self, b: &Body, w: &World) -> Runner {
        Runner {
            stats: b.stats,
            size: b.size,
            side: b.side,
            pos: [b.pos[0], b.pos[2]],
            stamina: b.stamina,
            tick: b.tick,
            run: b.run,
            moving: b.moving,
            players: w.players,
            rally: w.phase == phase::RALLY,
            floor: w.floor,
        }
    }

    /// A stroke-frame pick: the entry nearest where the player stands back by `depth`; (absolute index, past 8).
    fn pick(&self, b: &Body, c: &PathCopy, p: Pick) -> Option<(i32, bool)> {
        let mut me = b.pos;
        me[1] = b.reach.base;
        let mut best: Option<(usize, f32)> = None;
        for at in self.first.max(0) as usize..self.len.max(0) as usize {
            let e = c.balls[at];
            if e.bounces > p.stop {
                break;
            }
            let [x, y, z, _] = e.pos;
            if abs(z) < p.net || sign(b.side) == sign(z) || e.bounces < p.min || !(p.low <= y && y <= p.high) {
                continue;
            }
            let (dx, dy, dz) = ((F(x) - me[0]).0, (F(y) - me[1]).0, (F(z) - me[2]).0);
            let d = if p.flat {
                (F(dz) * dz).madd(dx, dx).sqrt().0
            } else {
                (F(dy) * dy).madd(dx, dx).madd(dz, dz).sqrt().0
            };
            if !(p.near <= d && d <= p.far) {
                continue;
            }
            let score = abs(((F::ZERO + me[2]).madd(p.depth, b.side) - z).0);
            if best.is_none_or(|(_, s)| score < s) {
                best = Some((at, score));
            }
        }
        best.map(|(at, _)| (at as i32, at >= 9))
    }

    /// The pick for the contact kind: (swing frames).
    fn swing_frame(&self, b: &Body, c: &PathCopy) -> Option<i32> {
        self.kind_frame(b, c, self.kind)
    }

    /// The pick for contact kind `kind`.
    fn kind_frame(&self, b: &Body, c: &PathCopy, kind: u8) -> Option<i32> {
        let r = &b.reach;
        let m = |a: f32, b: f32| (F(a) * b).0;
        let far = m(STRETCH, r.reach);
        let base = Pick { stop: 1, net: 0.5, min: 0, low: 0.0, high: 0.0, near: 0.0, far, flat: false, depth: b.depth };
        let got = match kind {
            contact::SMASH => self.pick(b, c, Pick { min: 0, low: r.smash[2], high: r.smash[0], far: r.reach, flat: true, depth: b.smash_off[1], ..base }),
            contact::VOLLEY => self.pick(b, c, Pick {
                stop: 0,
                net: if self.singles { 0.5 } else { NET_GAP },
                min: i32::MIN,
                high: m(STRETCH, r.volley_height),
                ..base
            }),
            contact::REACH => self.pick(b, c, Pick { min: 1, low: r.stroke_height, high: r.volley_height, ..base }),
            contact::TIERED => (0..4).find_map(|tier| self.tier_frame(b, c, tier)),
            _ => None,
        };
        got.map(|(at, _)| at)
    }

    /// The tiered kind's pick at one tier (0 the tightest band round the stroke height and reach, 3 the widest).
    fn tier_frame(&self, b: &Body, c: &PathCopy, tier: u8) -> Option<(i32, bool)> {
        let r = &b.reach;
        let m = |a: f32, b: f32| (F(a) * b).0;
        let (h, rr) = (r.stroke_height, r.reach);
        let (lo, hi, near, far) = match tier {
            0 => (m(0.9, h), m(1.1, h), m(0.9, rr), m(1.1, rr)),
            1 => (m(0.7, h), m(1.3, h), m(0.7, rr), m(1.3, rr)),
            2 => (m(0.3, h), m(1.7, h), m(0.3, rr), m(1.3, rr)),
            _ => (0.0, m(3.0, h), 0.0, m(1.3, rr)),
        };
        let base = Pick { stop: 1, net: 0.5, min: 0, low: lo, high: hi, near, far, flat: false, depth: b.depth };
        self.pick(b, c, base)
    }

    /// The ready check: 1 when a frame's move toward the stand spot brings the ball in reach, 2 (`IN_REACH`) when
    /// it already is, 0 when neither.
    fn ready(&self, b: &Body, w: &World) -> i32 {
        let r = &b.reach;
        let far = (F(STRETCH) * r.reach).0;
        let mut me = b.pos;
        me[1] = r.base;
        let (dx, dz) = ((F(self.stand[0]) - me[0]).0, (F(self.stand[2]) - me[2]).0);
        let len = (F(dz) * dz).madd(dx, dx).sqrt().0;
        let dir = if len <= 0.0 {
            [0.0; 4]
        } else {
            let inv = (F(1.0) / len).0;
            [(F(dx) * inv).0, (F(0.0) * inv).0, (F(dz) * inv).0, (F(0.0) * inv).0]
        };
        b.nudge(w, dir, &mut me);
        for v in 1..=2 {
            let (dx, dy, dz) = ((F(self.ball[0]) - me[0]).0, (F(self.ball[1]) - me[1]).0, (F(self.ball[2]) - me[2]).0);
            let d3 = (F(dy) * dy).madd(dx, dx).madd(dz, dz).sqrt().0;
            let d2 = (F(dz) * dz).madd(dx, dx).sqrt().0;
            let ok = match self.kind {
                contact::SMASH => d2 <= r.reach,
                contact::VOLLEY => d3 <= far,
                contact::TIERED | contact::REACH => d2 <= far,
                _ => false,
            };
            if ok {
                return v;
            }
            me = b.pos;
            me[1] = r.base;
        }
        0
    }

    fn set_state(&mut self, s: u8, roll: &mut impl FnMut() -> u32) {
        self.state = s;
        match s {
            step::READY => {
                self.lead = match self.kind {
                    contact::VOLLEY => self.leads[1],
                    contact::SMASH => self.leads[2],
                    _ => self.leads[0],
                };
                if self.push {
                    self.lead = draw(roll, 3) as i32 - 1;
                }
            }
            step::WAIT => {
                self.fresh = true;
                self.push = false;
            }
            _ => {}
        }
    }

    /// The return's aim: the plan and the stick.
    fn aim(&mut self, row: &AiParams, b: &Body, w: &World, roll: &mut impl FnMut() -> u32) {
        let deuce = w.court == DEUCE_COURT;
        let l = self.level as usize;
        const SURE: [i32; 4] = [50, 60, 70, 80];
        const WIDE: [i32; 4] = [55, 67, 80, 95];
        let out = (100.0 * (F(abs(b.pos[0])) - DOUBLES_SIDELINE)).0;
        let zone = 'z: {
            if !(out <= 0.0) {
                let p = if out <= 50.0 { out } else { 50.0 };
                if chance(roll, (F(p) / 2.5).0 as i32) {
                    let lane = crate::position::zone_in([b.pos[0], b.pos[2]], DOUBLES_SIDELINE, None).0;
                    if chance(roll, 50) {
                        self.plan = plan::LOB;
                        let mut x = if lane != 0 { 0 } else { 2 };
                        if chance(roll, 30) {
                            x = draw(roll, 3) as u8;
                        }
                        break 'z (x, 2);
                    }
                    self.plan = plan::DROP;
                    break 'z (if lane != 0 { 2 } else { 0 }, 0);
                }
            }
            let u = draw(roll, 100) as i32;
            let v = draw(roll, 100) as i32;
            let lv = self.level as i32;
            let short = |me: &mut Rally, roll: &mut dyn FnMut() -> u32| {
                if b.formation == formation::DEFENSIVE && me.mate == 0 && chance(&mut *roll, 5 * lv) {
                    me.plan = plan::LOB;
                }
            };
            if u < SURE[l] {
                self.plan = plan::TOPSPIN;
                let x = if deuce { 2 } else { 0 };
                if v < 5 * lv + 45 {
                    (x, 0)
                } else if v < 5 * lv + 65 {
                    if chance(roll, 30) {
                        self.plan = plan::SLICE;
                    }
                    (x, 1)
                } else {
                    if chance(roll, 50) {
                        self.plan = plan::SLICE;
                    }
                    (x, 2)
                }
            } else if u < WIDE[l] {
                self.plan = plan::TOPSPIN;
                short(self, roll);
                (if deuce { 0 } else { 2 }, 2)
            } else {
                self.plan = if chance(roll, 60) { plan::TOPSPIN } else { plan::SLICE };
                if u < 60 {
                    short(self, roll);
                    (1, 2)
                } else if u < 90 {
                    (1, 1)
                } else {
                    (1, 0)
                }
            }
        };
        self.stick = row.aim(zone, b.side, roll);
    }

    /// The singles return's aim: the plan, the stick and maybe the net dash.
    fn singles_aim(&mut self, row: &AiParams, b: &Body, w: &World, roll: &mut impl FnMut() -> u32) {
        const W: f32 = SINGLES_SIDELINE;
        let deuce = w.court == DEUCE_COURT;
        let opp = b.opp[0];
        let (ol, od) = crate::position::zone_in([opp[0], opp[2]], W, Some(&mut *roll));
        let out = ((F(abs(b.pos[0])) - DOUBLES_SIDELINE) * 100.0).0;
        if 0.0 < out {
            let p = if out <= 50.0 { out } else { 50.0 };
            if chance(roll, (F(p) / 2.5).0 as i32) {
                let lane = crate::position::zone_in([b.pos[0], b.pos[2]], W, None).0;
                let (mut x, d) = if chance(roll, 50) {
                    self.plan = plan::LOB;
                    (if lane != 0 { 0 } else { 2 }, 2)
                } else {
                    self.plan = plan::DROP;
                    (if lane == 0 { 2 } else { 0 }, 0)
                };
                if chance(roll, 30) {
                    x = draw(roll, 3) as u8;
                }
                self.stick = row.aim((x, d), b.side, roll);
                return;
            }
        }
        if chance(roll, row.special_return_rate) {
            let x = match ol {
                0 => 2,
                2 => 0,
                _ if chance(roll, 50) => 0,
                _ => 2,
            };
            self.stick = row.aim((x, 2), b.side, roll);
            self.plan = plan::LOB;
            return;
        }
        // the net dash: NET 20%, ALL 10%
        let dash = |roll: &mut dyn FnMut() -> u32| match row.style {
            style::NET => chance(roll, 20),
            style::ALL => chance(roll, 10),
            _ => false,
        };
        match self.return_level {
            2 => {
                if chance(roll, 60) {
                    self.plan = plan::TOPSPIN;
                    self.stick = row.aim((if deuce { 0 } else { 2 }, 2), b.side, roll);
                    if self.level == 3 && abs(opp[2]) < 7.4 && chance(roll, [5, 10, 5, 20][row.style as usize & 3]) {
                        self.plan = plan::LOB;
                    }
                    if dash(roll) {
                        self.dash = true;
                    }
                    return;
                }
                let (x, d) = if ol == 2 || ol == 0 {
                    (1, 0)
                } else {
                    (if deuce { 2 } else { 0 }, chance(roll, 90) as u8)
                };
                self.plan = plan::TOPSPIN;
                self.stick = row.aim((x, d), b.side, roll);
                if ol == 1 && od == 2 && d == 0 && dash(roll) {
                    self.dash = true;
                }
            }
            1 => {
                let x = if chance(roll, 50) { 0 } else { 2 };
                self.plan = plan::TOPSPIN;
                self.stick = row.aim((x, 2), b.side, roll);
                if match row.style {
                    style::NET => chance(roll, 30),
                    style::ALL => chance(roll, 15),
                    _ => false,
                } {
                    self.plan = plan::SLICE;
                    self.dash = true;
                }
            }
            0 => {
                let (mut x, _) = crate::position::zone_in([self.last[0], self.last[2]], W, Some(&mut *roll));
                let d = match row.style {
                    style::NET => 1 + chance(roll, 50) as u8,
                    style::BASE => 1 + chance(roll, 90) as u8,
                    style::ALL => 1 + chance(roll, 70) as u8,
                    _ => 1,
                };
                if (x == 2 || x == 0) && chance(roll, 10) {
                    x = 1;
                } else if x == 1 && chance(roll, 50) {
                    x = if deuce { 2 } else { 0 };
                }
                self.plan = plan::TOPSPIN;
                self.stick = row.aim((x, d), b.side, roll);
                if d == 1 {
                    self.stick[2] = 0.0;
                }
            }
            _ => {}
        }
    }

    /// One frame of the doubles receive. `out` is the run target or stick (as the caller passed it), the button
    /// set when it presses; true once the stroke is over.
    #[allow(clippy::too_many_arguments)]
    pub fn receive(
        &mut self,
        row: &AiParams,
        b: &mut Body,
        w: &World,
        c: &mut PathCopy,
        seen: &mut [bool],
        out: &mut Out,
        roll: &mut impl FnMut() -> u32,
    ) -> bool {
        self.frames -= 1;
        self.index -= 1;
        self.swing -= 1;
        let phase = w.phase;
        let mut done = false;
        let mut s = self.state;
        if s == step::WAIT {
            if self.hold {
                return false;
            }
            if self.wait > 0 {
                self.wait -= 1;
                if self.guess == guess::NONE {
                    return false;
                }
                out.stick[2] = b.pos[2];
                out.stick[0] = match (self.guess, w.court) {
                    (guess::WIDE, DEUCE_COURT) => (F(DOUBLES_SIDELINE) * b.side).0,
                    (guess::OTHER, AD_COURT) => (-F(DOUBLES_SIDELINE) * b.side).0,
                    _ => 0.0,
                };
                b.walk(w, &mut out.stick, false);
                if zero(&out.stick) {
                    self.wait = 0;
                }
                return false;
            }
            if self.copy_path(b, w, c, seen, true) == 0 {
                return false;
            }
            let mut found = false;
            let mut searched = false;
            if self.fresh && !self.landing_short(c) {
                self.fresh = false;
                searched = true;
                let path = self.path(c);
                let s = Searcher {
                    row,
                    reach: &b.reach,
                    strong: b.strong,
                    singles: self.singles,
                    beside_human: self.mate > 0,
                    runner: self.runner(b, w),
                    depth: b.depth,
                    path: &path,
                    first: self.first.max(0) as usize,
                    end: self.len.max(0) as usize,
                };
                let hit = |me: &mut Rally, ct: crate::ai::Contact, kind| {
                    let e = c.balls[ct.at].pos;
                    me.stand = [ct.stand[0], e[1], ct.stand[1], e[3]];
                    me.ball = e;
                    me.frames = ct.frames;
                    me.index = ct.at as i32 - me.first;
                    me.kind = kind;
                };
                if row.style == style::BASE {
                    if let Some(ct) = s.reach_search(None, 1, true, self.body, roll) {
                        hit(self, ct, contact::REACH);
                        found = true;
                    }
                }
                if !found {
                    for tier in if self.low { 3 } else { 0 }..4 {
                        if let Some(ct) = s.tier_search(None, tier, 1, true, self.body, self.low, seen, roll) {
                            hit(self, ct, contact::TIERED);
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    if let Some((stand, ball, frames, index)) = self.smash_search(b, w, c, -1, 1) {
                        (self.stand, self.ball, self.frames, self.index, self.kind) = (stand, ball, frames, index, contact::SMASH);
                        found = true;
                    }
                }
            }
            let _ = searched;
            let mut go = false;
            if found {
                if self.guess != guess::NONE {
                    go = true;
                } else {
                    self.set_state(step::RUN, roll);
                    s = step::RUN;
                }
            } else {
                let mut t = [0.0; 4];
                let short = self.landing_short(c);
                let dive_ok = !short && self.dive && phase != phase::POINT_OVER && !self.no_dive(b, c);
                if dive_ok {
                    self.kind = contact::DIVE;
                    out.button = Some(crate::aim::button::CROSS as u32);
                    self.set_state(step::SWING, roll);
                } else if (short && self.landing(b, w, c, LANDING_AHEAD, &mut t))
                    || self.after_bounce(b, c, &mut t)
                    || self.entry(c, FALLBACK_ENTRY).map(|e| t = e).is_some()
                    || self.last(b, c, &mut t)
                {
                    out.stick = t;
                    b.walk(w, &mut out.stick, false);
                }
            }
            if s == step::WAIT {
                // the guess's end
                if self.guess == guess::NONE {
                    return false;
                }
                let f6 = if self.guess == guess::WIDE { b.side } else { (-F(b.side)).0 };
                self.guess = guess::NONE;
                if !go {
                    return false;
                }
                let (dz, dx) = ((F(self.stand[2]) - self.from[1]).0, (F(self.stand[0]) - self.from[0]).0);
                let inv = (1.0 / (F(dz) * dz).madd(dx, dx).sqrt()).0;
                let (nz, nx) = ((F(dz) * inv).0, (F(dx) * inv).0);
                let dot = (F::ZERO + 0.0).madd(nx, f6).madd(nz, 0.0).0;
                if !(dot < 0.5) {
                    self.push = true;
                } else if dot < 0.0 {
                    b.lose(w);
                    out.stick = [0.0; 4];
                    self.wait = row.guess[2];
                    self.set_state(step::WAIT, roll);
                    return false;
                }
                self.set_state(step::RUN, roll);
                s = step::RUN;
            }
        }
        if s == step::RUN {
            let go = if self.frames == 1 { self.ready(b, w) == IN_REACH } else { self.frames < 1 };
            if !go {
                out.stick = self.stand;
                b.walk(w, &mut out.stick, true);
                return false;
            }
            self.copy_path(b, w, c, seen, true);
            self.swing = self.swing_frame(b, c).unwrap_or(SWING_DEFAULT);
            self.set_state(step::READY, roll);
            s = step::READY;
        }
        if s == step::READY {
            if self.swing + self.lead < PRESS_AT {
                if self.singles {
                    self.singles_aim(row, b, w, roll);
                } else {
                    self.aim(row, b, w, roll);
                }
                out.button = Some(button(self.plan));
                self.set_state(step::SWING, roll);
            }
            return false;
        }
        if s == step::SWING {
            if b.swing == 1 {
                if self.push {
                    let k = if self.body { 1.5 } else { 2.0 };
                    self.stick = self.stick.map(|c| (F(c) * k).0);
                    let len = (F(self.stick[2]) * self.stick[2]).madd(self.stick[0], self.stick[0]).sqrt().0;
                    if !(len <= 1.0) {
                        let inv = (F(1.0) / len).0;
                        self.stick = self.stick.map(|c| (F(c) * inv).0);
                    }
                }
                out.stick = self.stick;
            }
            done = b.stroke == 0;
            if done && self.singles && self.dash {
                // singles: the net dash goes in to the shot's line at the net reach
                self.stand[0] = b.target[0];
                self.stand[2] = (-F(self.reach) * b.side).0;
            }
            if b.swing < 0 && w.hitter >= 0 && (w.hitter & 1) != (b.team & 1) && self.wait > 0 {
                self.wait -= 1;
            }
        }
        done
    }
}

/// What a search found: stand, ball, run frames, index (from `first`).
type Found = ([f32; 4], [f32; 4], i32, i32);

impl Rally {
    fn our_hit(b: &Body, w: &World) -> bool {
        w.hitter >= 0 && (w.hitter & 1) == (b.team & 1)
    }

    /// The formation re-pick; a new spot restarts the walk back.
    fn repick(&mut self, b: &Body, w: &World, cue: Cue, roll: &mut impl FnMut() -> u32) {
        let mut f = Formation { lane: self.lane, front: self.front, spot: [self.spot[0], self.spot[2]] };
        let t = Team { side: b.side, formation: b.formation, lean: self.lean };
        let moved = f.repick(&t, cue, [b.pos[0], b.pos[2]], w.ball[2], Self::our_hit(b, w));
        (self.lane, self.front, self.spot[0], self.spot[2]) = (f.lane, f.front, f.spot[0], f.spot[1]);
        if !matches!(cue, Cue::Middle) {
            self.forward = match b.formation {
                formation::STAGGERED => self.front,
                f => f == formation::ATTACKING,
            };
        }
        if moved {
            self.back = Return::new(self.rate, roll);
        }
    }

    /// Stand at its depth on the half the ball isn't in.
    fn hold_spot(&mut self, b: &Body, w: &World) {
        self.spot = b.pos;
        self.spot[0] = (-F(HALF_MIDDLE) * (sign(w.ball[0]) as f32)).0;
    }

    /// The walk back to the spot: true when it walks this frame (`out` then heads there).
    fn walk_back(&mut self, b: &Body, w: &World, out: &mut Out, roll: &mut impl FnMut() -> u32) {
        if self.back.step(self.rate, self.radius, [self.spot[0], self.spot[2]], [b.pos[0], b.pos[2]], roll) {
            out.stick = self.spot;
            b.walk(w, &mut out.stick, false);
        }
    }

    /// The dispatcher's entry to the receive state (and the strike/new-path messages while it receives).
    pub fn enter_receive(&mut self, roll: &mut impl FnMut() -> u32) {
        self.set_state(step::WAIT, roll);
    }

    /// The dispatcher's entry to the rally state (and the strike/hit messages while it rallies): its wait substate.
    pub fn enter_rally(&mut self, b: &Body, w: &mut World, roll: &mut impl FnMut() -> u32) {
        if self.singles {
            self.singles_set_sub(step::WAIT, roll);
        } else {
            self.set_sub(b, w, step::WAIT, false, roll);
        }
    }

    /// The singles hit message on an opponent's shot of kind `kind` (`ai::Seen::kind`): after a dive a NET or ALL
    /// style not yet dashing dashes to the middle of the net.
    pub fn heard_hit(&mut self, kind: i32, style: u8, b: &Body) {
        if kind == crate::shot::branch::DIVE as i32 && !self.dash && (style == style::NET || style == style::ALL) {
            self.dash = true;
            self.stand[0] = 0.0;
            self.stand[2] = (-F(self.reach) * b.side).0;
        }
    }

    fn set_sub(&mut self, b: &Body, w: &mut World, s: u8, keep: bool, roll: &mut impl FnMut() -> u32) {
        self.sub = s;
        match s {
            step::WAIT => {
                if !keep {
                    (self.fresh, self.next, self.chase) = (true, -1, true);
                    (self.middle, self.follow, self.ours) = (false, false, false);
                    self.back = Return::new(self.rate, roll);
                }
                w.put_none(b.team, record::NONE, b.pos);
                self.mind.rally(roll);
                self.defer = false;
            }
            step::RUN => w.put(b.team, self.index, self.kind as i32, b.pos, self.stand),
            step::READY => self.lead = self.kind_lead(),
            _ => {}
        }
    }

    /// The NET/BASE routines' button lead for the contact kind (none for a dive).
    fn kind_lead(&self) -> i32 {
        match self.kind {
            contact::DIVE => 0,
            contact::SMASH => self.leads[2],
            contact::VOLLEY => self.leads[1],
            contact::TIERED | contact::REACH => self.leads[0],
            _ => self.lead,
        }
    }

    /// The partner's voice line on giving way (one roll, a second when it speaks).
    fn call(&mut self, b: &Body, roll: &mut impl FnMut() -> u32) {
        if self.voice && b.mate_voice < VOICE_BUSY && chance(roll, 25) {
            self.said = Some(coin(roll()));
        }
        self.voice = false;
    }

    /// Leave the ball to the partner: its own and the partner's shot records compared.
    fn give_way(&mut self, b: &Body, mine: &Shot, theirs: &Shot) -> bool {
        if self.defer {
            return false;
        }
        let (t, m) = (theirs.n, mine.n);
        let run = |s: &Shot| {
            let (dz, dx) = ((F(s.vec[2]) - s.pos[2]).0, (F(s.vec[0]) - s.pos[0]).0);
            (F(dz) * dz).madd(dx, dx).sqrt().0
        };
        let yes = t >= 0 && {
            let (f4, f3) = (run(mine), run(theirs));
            if self.mate > 0 {
                if f4 < 2.5 && (F(1.0) + abs(b.pos[2])).0 < abs(b.mate[2]) {
                    false
                } else if f3 < 2.0 {
                    true
                } else if f4 < 2.5 {
                    false
                } else if t.wrapping_sub(m).wrapping_abs() < 30 {
                    (F(f3) + 1.0).0 < f4
                } else {
                    t < m
                }
            } else if t < m && f3 < f4 {
                true
            } else if m < t && f4 < f3 {
                false
            } else if f3 < 2.5 || f4 < 2.5 {
                f3 < f4
            } else {
                t < m
            }
        };
        if !yes && self.mate > 0 && (t == record::NONE || t >= 0) {
            self.defer = true;
        }
        yes
    }

    /// The entry after the last one with `k` bounces, when the path ends past `k` and `t` (as it comes in) lies on the
    /// other half.
    fn after(&self, b: &Body, c: &PathCopy, k: i32, t: &mut [f32; 4]) -> bool {
        let len = self.len as usize;
        if self.len < 2 || c.balls[len - 1].bounces <= k {
            return false;
        }
        let Some(a) = (0..len - 1).rev().find(|&a| c.balls[a].bounces == k) else { return false };
        let v = sign(b.side) != sign(t[2]);
        if v {
            *t = c.balls[a + 1].pos;
        }
        v
    }

    /// The partner stands nearer `t` than it does.
    fn mate_nearer(b: &Body, t: &[f32; 4]) -> bool {
        let d = |p: &[f32; 4]| {
            let (dz, dx) = ((F(t[2]) - p[2]).0, (F(t[0]) - p[0]).0);
            (F(dz) * dz).madd(dx, dx).0
        };
        d(&b.mate) < d(&b.pos)
    }

    /// Its team plays forward and the ball is on the other half, or nearer the net than it on this one.
    fn forward_ball(&self, b: &Body, w: &World) -> bool {
        let (oz, bz) = (b.pos[2], w.ball[2]);
        self.forward && (sign(oz) != sign(bz) || abs(bz) <= abs(oz))
    }

    /// The volley search: before the bounce, on the other half past `NET_GAP`, under `STRETCH` volley heights and in reach of
    /// the base height, nearest the volley height, in time; stops at the first found unless `all`.
    fn volley_search(&self, b: &Body, w: &World, c: &PathCopy, from: i32, all: bool, body: bool) -> Option<Found> {
        let r = &b.reach;
        let vh = r.volley_height;
        let top = (F(STRETCH) * vh).0;
        let r2 = (F(r.reach) * r.reach).0;
        let runner = self.runner(b, w);
        let (mx, mz) = (b.pos[0], b.pos[2]);
        let mut best: Option<(i32, f32, i32, [f32; 4])> = None;
        let mut at = if from < 0 { self.first } else { from };
        while at < self.len {
            let e = c.balls[at as usize];
            if e.bounces > 0 {
                break;
            }
            let p = e.pos;
            if sign(b.side) != sign(p[2]) && !(abs(p[2]) < NET_GAP) {
                'c: {
                    if !(0.0 <= p[1] && p[1] <= top) {
                        break 'c;
                    }
                    let dy = (F(p[1]) - r.base).0;
                    let f = (F::ZERO + r2).msub(dy, dy).0;
                    if f < 0.0 {
                        break 'c;
                    }
                    let half = if body { 0.0 } else { F(f).sqrt().0 };
                    let (l, rr) = ((F(p[0]) - half).0, (F(p[0]) + half).0);
                    let dz = (F(p[2]) - mz).0;
                    let (dl, dr) = ((F(l) - mx).0, (F(rr) - mx).0);
                    let mut s = p;
                    s[0] = if (F(dz) * dz).madd(dl, dl).0 <= (F(dz) * dz).madd(dr, dr).0 { l } else { rr };
                    let score = abs((F(p[1]) - vh).0);
                    s[2] = (p[2] - F(b.depth) * b.side / 2.0).0;
                    let frames = runner.frames_to([s[0], s[2]]);
                    if at - self.first < frames {
                        break 'c;
                    }
                    if best.is_none_or(|(_, bs, _, _)| !(bs <= score)) {
                        best = Some((at, score, frames, s));
                    }
                }
                if !all && best.is_some() {
                    break;
                }
            }
            at += 1;
        }
        let (at, _, frames, stand) = best?;
        Some((stand, c.balls[at as usize].pos, frames, at - self.first))
    }

    fn pair_aim(&mut self, row: &AiParams, b: &Body, roll: &mut impl FnMut() -> u32) {
        let xz = |p: [f32; 4]| [p[0], p[2]];
        let a = row.pair_aim(
            &Pair {
                me: xz(b.pos),
                mate: xz(b.mate),
                opp: [xz(b.opp[0]), xz(b.opp[1])],
                side: b.side,
                singles: self.singles,
                kind: self.kind,
                volley_level: self.volley_level,
                level: self.level,
                formation: b.formation,
                smash_third: self.mate != 0,
            },
            roll,
        );
        (self.stick, self.plan) = (a.stick, a.plan);
    }

    /// The searches in the NET (`net`) or BASE order: the contact kind found.
    #[allow(clippy::too_many_arguments)]
    fn search(&mut self, net: bool, row: &AiParams, b: &Body, w: &World, c: &PathCopy, seen: &mut [bool], roll: &mut impl FnMut() -> u32) -> Option<u8> {
        let take = |me: &mut Rally, (stand, ball, frames, index): Found| {
            (me.stand, me.ball, me.frames, me.index) = (stand, ball, frames, index);
        };
        if let Some(f) = self.smash_search(b, w, c, self.next, 0) {
            take(self, f);
            return Some(contact::SMASH);
        }
        let near = self.forward_ball(b, w) || (self.mate > 0 && abs(b.pos[2]) < abs(b.mate[2]));
        if net || near {
            if let Some(f) = self.volley_search(b, w, c, self.next, !net, self.body) {
                take(self, f);
                return Some(contact::VOLLEY);
            }
        }
        let path = self.path(c);
        let s = Searcher {
            row,
            reach: &b.reach,
            strong: b.strong,
            singles: self.singles,
            beside_human: self.mate > 0,
            runner: self.runner(b, w),
            depth: b.depth,
            path: &path,
            first: self.first.max(0) as usize,
            end: self.len.max(0) as usize,
        };
        let hit = |me: &mut Rally, ct: crate::ai::Contact| {
            let e = c.balls[ct.at].pos;
            me.stand = [ct.stand[0], e[1], ct.stand[1], e[3]];
            (me.ball, me.frames, me.index) = (e, ct.frames, ct.at as i32 - me.first);
        };
        let tiers = if self.low { 3 } else { 0 }..4;
        if near {
            let from = (self.next >= 0).then_some(self.next as usize);
            for tier in tiers {
                if let Some(ct) = s.tier_search(from, tier, 0, !net, self.body, self.low, seen, roll) {
                    hit(self, ct);
                    return Some(contact::TIERED);
                }
            }
        } else if self.fresh && !self.landing_short(c) {
            self.fresh = false;
            if !net {
                if let Some(ct) = s.reach_search(None, 1, true, self.body, roll) {
                    hit(self, ct);
                    return Some(contact::REACH);
                }
            }
            for min in [1, 0] {
                for tier in tiers.clone() {
                    if let Some(ct) = s.tier_search(None, tier, min, true, self.body, self.low, seen, roll) {
                        hit(self, ct);
                        return Some(contact::TIERED);
                    }
                }
            }
        }
        None
    }

    /// One frame of the doubles NET (`net`) or BASE rally routine. `out` is the run target or stick (as the caller
    /// passed it), the button set when it presses.
    #[allow(clippy::too_many_arguments)]
    pub fn rally(
        &mut self,
        net: bool,
        row: &AiParams,
        b: &mut Body,
        w: &mut World,
        c: &mut PathCopy,
        seen: &mut [bool],
        out: &mut Out,
        roll: &mut impl FnMut() -> u32,
    ) {
        self.frames -= 1;
        self.index -= 1;
        self.swing -= 1;
        let (frame, phase) = (w.frame, w.phase);
        let (slot, mate) = (b.team, b.team ^ 2);
        let mut s = self.sub;
        if s == step::WAIT {
            if self.hold || (phase == phase::POINT_OVER && w.shots == 1) || w.shots < 2 || self.held {
                return;
            }
            if self.wait > 0 {
                self.wait -= 1;
                return;
            }
            if !self.ours && Self::our_hit(b, w) {
                (self.ours, self.tick) = (true, 0);
            }
            if self.ours {
                self.tick += 1;
                if self.tick >= 30 {
                    self.tick = 0;
                    let at = [b.mate[0], b.mate[2]];
                    self.repick(b, w, if w.hitter == slot { Cue::Me(at) } else { Cue::Other(at) }, roll);
                }
                self.walk_back(b, w, out, roll);
                return;
            }
            if !self.follow {
                let p = w.shot(mate);
                if p.n >= 0 && (!self.fresh || (self.front && sign(b.pos[2]) == sign(w.ball[2]))) {
                    self.repick(b, w, Cue::Other([p.vec[0], p.vec[2]]), roll);
                    self.follow = true;
                }
            }
            if self.follow {
                let p = w.shot(mate);
                let (pz, bz, oz) = (b.mate[2], w.ball[2], b.pos[2]);
                let same = sign(pz) == sign(bz);
                let check = p.n < 0
                    || p.n < frame
                    || (p.n < frame + 8 && same && !(abs(bz) <= abs(pz)))
                    || (!(abs(bz) <= (F(2.0) + abs(pz)).0) && same);
                if check {
                    let near = || {
                        let (dz, dx) = ((F(oz) - bz).0, (F(b.pos[0]) - w.ball[0]).0);
                        (F(dz) * dz).madd(dx, dx).sqrt().0 <= 3.0
                    };
                    if !self.middle && (sign(oz) != sign(bz) || abs(bz) <= abs(oz) || near()) {
                        (self.follow, self.fresh) = (false, true);
                    } else {
                        self.hold_spot(b, w);
                    }
                }
            }
            if self.follow {
                let p = w.shot(mate);
                if p.n >= 0 {
                    self.repick(b, w, Cue::Other([p.vec[0], p.vec[2]]), roll);
                }
                self.walk_back(b, w, out, roll);
                return;
            }
            let n = self.copy_path(b, w, c, seen, true);
            if n <= 0 {
                return;
            }
            if let Some(kind) = self.search(net, row, b, w, c, seen, roll) {
                self.kind = kind;
                self.set_sub(b, w, step::RUN, false, roll);
                s = step::RUN;
            } else {
                let mut t = self.stash;
                let mut go = |me: &mut Rally, t: [f32; 4]| {
                    out.stick = t;
                    if b.walk(w, &mut out.stick, false) {
                        me.chase = false;
                    }
                };
                if self.landing_short(c) {
                    if self.landing(b, w, c, RALLY_LANDING_AHEAD, &mut t)
                        || self.after_bounce(b, c, &mut t)
                        || self.entry(c, FALLBACK_ENTRY).map(|e| t = e).is_some()
                        || self.last(b, c, &mut t)
                    {
                        let chase = self.chase;
                        go(self, t);
                        self.chase = chase;
                    }
                } else {
                    if self.after(b, c, 1, &mut t) {
                        let p = w.shot(mate);
                        if p.n >= 0 {
                            self.repick(b, w, Cue::Other([p.vec[0], p.vec[2]]), roll);
                            self.follow = true;
                            self.call(b, roll);
                            self.stash = t;
                            return;
                        }
                        if self.mate == 0 && Self::mate_nearer(b, &t) {
                            self.after(b, c, 0, &mut t);
                            if self.mate == 0 && Self::mate_nearer(b, &t) {
                                self.repick(b, w, Cue::Middle, roll);
                                (self.follow, self.middle) = (true, true);
                                self.stash = t;
                                return;
                            }
                        }
                    }
                    if self.dive && phase != phase::POINT_OVER && !self.no_dive(b, c) {
                        self.kind = contact::DIVE;
                        self.pair_aim(row, b, roll);
                        out.button = Some(button(self.plan));
                        self.set_sub(b, w, step::SWING, false, roll);
                    } else if self.chase
                        && (self.after_bounce(b, c, &mut t)
                            || self.entry(c, FALLBACK_ENTRY).map(|e| t = e).is_some()
                            || self.last(b, c, &mut t))
                    {
                        go(self, t);
                    }
                }
                self.stash = t;
                self.next = n - 1;
            }
        }
        if s == step::RUN {
            let (mine, theirs) = (w.shot(slot), w.shot(mate));
            if self.give_way(b, &mine, &theirs) {
                self.repick(b, w, Cue::Other([theirs.vec[0], theirs.vec[2]]), roll);
                self.follow = true;
                self.call(b, roll);
                self.set_sub(b, w, step::WAIT, true, roll);
                return;
            }
            let go = if self.frames == 1 { self.ready(b, w) == IN_REACH } else { self.frames < 1 };
            if !go {
                out.stick = self.stand;
                b.walk(w, &mut out.stick, true);
                return;
            }
            self.copy_path(b, w, c, seen, true);
            self.swing = self.swing_frame(b, c).unwrap_or(SWING_DEFAULT);
            self.set_sub(b, w, step::READY, false, roll);
            s = step::READY;
        }
        if s == step::READY {
            let mine = w.shot(slot);
            if mine.n >= 0 && mine.n < frame {
                w.put_none(slot, record::NONE, b.pos);
            }
            if self.swing + self.lead < PRESS_AT {
                if w.path_mode == Call::Out as u8 && !(w.path_gap < row.line_margin) {
                    self.held = true;
                    self.set_sub(b, w, step::WAIT, false, roll);
                } else {
                    self.pair_aim(row, b, roll);
                    out.button = Some(button(self.plan));
                    self.set_sub(b, w, step::SWING, false, roll);
                }
            }
            return;
        }
        if s == step::SWING {
            if b.swing == 1 {
                out.stick = self.stick;
            }
            if b.mark != 0 {
                w.put_none(slot, record::HITTING, b.pos);
            }
            if b.stroke == 0 {
                self.set_sub(b, w, step::WAIT, false, roll);
            }
            if b.swing < 0 && w.hitter >= 0 && (w.hitter & 1) != (b.team & 1) && self.wait > 0 {
                self.wait -= 1;
            }
        }
    }
}

/// The singles NET and BASE rally routines: no partner, no shot records; the walk back between shots, the net dash
/// (in to its shot's line after a volley or smash, a baseliner after a smash) and, at the net, the spot it picks
/// after each hit.
impl Rally {
    fn singles_set_sub(&mut self, s: u8, roll: &mut impl FnMut() -> u32) {
        self.sub = s;
        match s {
            step::WAIT => {
                (self.fresh, self.next, self.chase) = (true, -1, true);
                self.back = Return::new(self.rate, roll);
                self.mind.rally(roll);
            }
            step::READY => self.lead = self.kind_lead(),
            _ => {}
        }
    }

    /// The shot's aim and plan: the NET chooser, or the BASE one (with its short-ball flag).
    fn shot_aim(&mut self, net: bool, row: &AiParams, b: &Body, roll: &mut impl FnMut() -> u32) {
        let l = Look {
            me: [b.pos[0], b.pos[2]],
            opp: [b.opp[0][0], b.opp[0][2]],
            side: b.side,
            singles: true,
            kind: self.kind,
            opp_kind: self.opp_kind,
            opp_lefty: b.opp_lefty,
            mark: [self.last[0], self.last[2]],
            contact_y: self.ball[1],
            heights: [b.reach.under_min, b.reach.stroke_height, b.reach.volley_height],
            volley_level: self.volley_level,
            high_level: self.high_level,
            level: self.level,
        };
        let a = if net { row.receive_aim(&l, roll) } else { row.rally_aim(&l, &mut self.short, roll) };
        (self.stick, self.plan) = (a.stick, a.plan);
    }

    /// The dash's stand spot: its shot's line (within `DASH_WIDTH`) at the net reach, a third of the way back from
    /// it after a lob.
    fn dash_spot(&mut self, b: &Body) {
        let lim = DASH_WIDTH;
        let x = b.target[0];
        self.stand[0] = if x < -F(lim) { (-F(lim)).0 } else if x <= lim { x } else { lim };
        self.stand[2] = if self.plan == plan::LOB {
            (F(b.side) * -F(((F(BASELINE) - self.reach) / 3.0 + self.reach).0)).0
        } else {
            (-F(self.reach) * b.side).0
        };
    }

    /// NET, its stroke over: unless dashing, back to the middle at its own depth (no deeper than 10) after a shot
    /// down the middle or at the opponent's zone, else ⅔ of the way back from the net reach (no deeper than the
    /// opponent), dashing when either stands near the net.
    fn after_hit(&mut self, b: &Body, roll: &mut impl FnMut() -> u32) {
        if !self.dash {
            let xz = |p: [f32; 4]| [p[0], p[2]];
            let shot = zone_in(xz(b.target), SINGLES_SIDELINE, None);
            let them = zone_in(xz(b.opp[0]), SINGLES_SIDELINE, Some(&mut *roll));
            let mine = zone_in(xz(b.pos), SINGLES_SIDELINE, None);
            self.spot[0] = 0.0;
            if shot.0 == 1 || shot == them {
                self.spot[2] = if abs(b.pos[2]) <= 10.0 { b.pos[2] } else { (F(-10.0) * b.side).0 };
            } else {
                let back = (2.0 * (F(BASELINE) - self.reach) / 3.0 + self.reach).0;
                self.spot[2] = (F(b.side) * -F(back)).0;
                let oz = abs(b.opp[0][2]);
                if !(abs(self.spot[2]) <= oz) {
                    self.spot[2] = (F(b.side) * -F(oz)).0;
                }
                if mine.1 < 2 || them.1 < 2 {
                    self.dash = true;
                }
            }
        }
        if self.dash {
            self.dash_spot(b);
        }
    }

    /// The searches: smash (BASE: not below its ideal height), volley (NET), then dashing (and the ball not past
    /// it) the volley (BASE) and the tiers from the next entry, else on a fresh ball the reach search (BASE) and
    /// the tiers. The contact kind found.
    #[allow(clippy::too_many_arguments)]
    fn singles_search(&mut self, net: bool, row: &AiParams, b: &Body, w: &World, c: &PathCopy, seen: &mut [bool], roll: &mut impl FnMut() -> u32) -> Option<u8> {
        let take = |me: &mut Rally, (stand, ball, frames, index): Found| {
            (me.stand, me.ball, me.frames, me.index) = (stand, ball, frames, index);
        };
        if let Some(f) = self.smash_search(b, w, c, self.next, 0) {
            if net || !(f.1[1] < b.reach.smash[1]) {
                take(self, f);
                return Some(contact::SMASH);
            }
        }
        if net {
            if let Some(f) = self.volley_search(b, w, c, self.next, false, self.body) {
                take(self, f);
                return Some(contact::VOLLEY);
            }
        }
        if self.dash && sign(b.pos[2]) == sign(w.ball[2]) && !(abs(w.ball[2]) <= abs(b.pos[2])) {
            self.dash = false;
        }
        let path = self.path(c);
        let s = Searcher {
            row,
            reach: &b.reach,
            strong: b.strong,
            singles: self.singles,
            beside_human: self.mate > 0,
            runner: self.runner(b, w),
            depth: b.depth,
            path: &path,
            first: self.first.max(0) as usize,
            end: self.len.max(0) as usize,
        };
        let hit = |me: &mut Rally, ct: crate::ai::Contact| {
            let e = c.balls[ct.at].pos;
            me.stand = [ct.stand[0], e[1], ct.stand[1], e[3]];
            (me.ball, me.frames, me.index) = (e, ct.frames, ct.at as i32 - me.first);
        };
        let tiers = if self.low { 3 } else { 0 }..4;
        if self.dash {
            if !net {
                if let Some(f) = self.volley_search(b, w, c, self.next, true, self.body) {
                    take(self, f);
                    return Some(contact::VOLLEY);
                }
            }
            let from = (self.next >= 0).then_some(self.next as usize);
            for tier in tiers {
                if let Some(ct) = s.tier_search(from, tier, 0, !net, self.body, self.low, seen, roll) {
                    hit(self, ct);
                    return Some(contact::TIERED);
                }
            }
        } else if self.fresh && !self.landing_short(c) {
            self.fresh = false;
            if !net {
                if let Some(ct) = s.reach_search(None, 1, true, self.body, roll) {
                    hit(self, ct);
                    return Some(contact::REACH);
                }
            }
            for min in [1, 0] {
                for tier in tiers.clone() {
                    if let Some(ct) = s.tier_search(None, tier, min, true, self.body, self.low, seen, roll) {
                        hit(self, ct);
                        return Some(contact::TIERED);
                    }
                }
            }
        }
        None
    }

    /// One frame of the singles NET (`net`) or BASE rally routine; `out` as in [`Rally::rally`].
    #[allow(clippy::too_many_arguments)]
    pub fn singles_rally(
        &mut self,
        net: bool,
        row: &AiParams,
        b: &mut Body,
        w: &World,
        c: &mut PathCopy,
        seen: &mut [bool],
        out: &mut Out,
        roll: &mut impl FnMut() -> u32,
    ) {
        self.frames -= 1;
        self.index -= 1;
        self.swing -= 1;
        let mut s = self.sub;
        if s == step::SWING {
            if b.swing == 1 {
                out.stick = self.stick;
            }
            if b.stroke == 0 {
                if net {
                    self.after_hit(b, roll);
                }
                self.singles_set_sub(step::WAIT, roll);
            }
            if b.swing < 0 && w.hitter >= 0 && (w.hitter & 1) != (b.team & 1) && self.wait > 0 {
                self.wait -= 1;
            }
            return;
        }
        if s == step::WAIT {
            if self.hold || (w.phase == phase::POINT_OVER && w.shots == 1) || self.held {
                return;
            }
            if self.wait > 0 {
                self.wait -= 1;
                return;
            }
            if Self::our_hit(b, w) {
                if self.dash {
                    out.stick = if net { self.stand } else { [0.0, 0.0, (-F(self.reach) * b.side).0, 0.0] };
                    b.walk(w, &mut out.stick, false);
                } else if self.back.there {
                } else if self.back.going {
                    let (dz, dx) = ((F(self.spot[2]) - b.pos[2]).0, (F(self.spot[0]) - b.pos[0]).0);
                    if (F(dz) * dz).madd(dx, dx).0 <= (F(self.radius) * self.radius).0 {
                        self.back.there = true;
                    } else {
                        out.stick = self.spot;
                        b.walk(w, &mut out.stick, false);
                    }
                } else {
                    self.back.wait += 1;
                    if self.back.wait >= 60 {
                        self.back.wait = 0;
                        self.back.going = chance(roll, self.rate);
                    }
                }
                return;
            }
            let n = self.copy_path(b, w, c, seen, true);
            if n <= 0 {
                return;
            }
            let Some(kind) = self.singles_search(net, row, b, w, c, seen, roll) else {
                let mut t = [0.0; 4];
                if self.landing_short(c) {
                    if self.landing(b, w, c, RALLY_LANDING_AHEAD, &mut t)
                        || self.after_bounce(b, c, &mut t)
                        || self.entry(c, FALLBACK_ENTRY).map(|e| t = e).is_some()
                        || self.last(b, c, &mut t)
                    {
                        out.stick = t;
                        b.walk(w, &mut out.stick, false);
                    }
                } else if self.dive && w.phase != phase::POINT_OVER && !self.no_dive(b, c) {
                    self.kind = contact::DIVE;
                    self.shot_aim(net, row, b, roll);
                    out.button = Some(button(self.plan));
                    self.singles_set_sub(step::SWING, roll);
                } else if self.chase
                    && (self.after_bounce(b, c, &mut t) || self.entry(c, FALLBACK_ENTRY).map(|e| t = e).is_some() || self.last(b, c, &mut t))
                {
                    out.stick = t;
                    if b.walk(w, &mut out.stick, false) {
                        self.chase = false;
                    }
                }
                self.next = n - 1;
                return;
            };
            self.kind = kind;
            self.singles_set_sub(step::RUN, roll);
            s = step::RUN;
        }
        if s == step::RUN {
            let go = if self.frames == 1 { self.ready(b, w) == IN_REACH } else { self.frames < 1 };
            if !go {
                out.stick = self.stand;
                b.walk(w, &mut out.stick, true);
                return;
            }
            self.copy_path(b, w, c, seen, true);
            self.swing = self.swing_frame(b, c).unwrap_or(SWING_DEFAULT);
            self.singles_set_sub(step::READY, roll);
        }
        if self.swing + self.lead < PRESS_AT {
            if w.path_mode == Call::Out as u8 && !(w.path_gap < row.line_margin) {
                self.held = true;
                self.singles_set_sub(step::WAIT, roll);
                return;
            }
            self.shot_aim(net, row, b, roll);
            out.button = Some(button(self.plan));
            self.singles_set_sub(step::SWING, roll);
            if net && (self.kind == contact::VOLLEY || self.kind == contact::SMASH) {
                self.dash = true;
                self.dash_spot(b);
            } else if !net && self.kind == contact::SMASH {
                self.dash = true;
            }
        }
    }
}

/// A human player's shot record keeper (its AI object runs it while a human plays it): whether it has found its
/// contact, the path entry the next search starts from, the kind found, the tiered search's bounce floor (1 while
/// the find was made with it) and whether it may volley (it stood inside the service line when the shot came).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Human {
    pub found: bool,
    pub next: i32,
    pub kind: u8,
    pub min: i32,
    pub volley: bool,
}

impl Human {
    /// The strike, new path and point messages: search again from the path's start; `z` is the player's.
    pub fn reset(&mut self, z: f32) {
        (self.found, self.next, self.volley) = (false, 0, abs(z) < crate::lines::SERVICE_LINE);
    }
}

impl Rally {
    /// One frame of a human player's shot record, so a computer partner can weigh it (`give_way`): while the
    /// opponents' shot comes, the contact it could make — a smash, a volley (near the net) or a tiered stroke from
    /// the widest tier, searched along the path until one is found, then kept up while the stroke-frame pick still
    /// finds it. `claimed`: the path object's claim is this player's (its record is left alone).
    #[allow(clippy::too_many_arguments)]
    pub fn human_record(
        &mut self,
        h: &mut Human,
        row: &AiParams,
        b: &Body,
        w: &mut World,
        c: &mut PathCopy,
        seen: &mut [bool],
        claimed: bool,
        roll: &mut impl FnMut() -> u32,
    ) {
        let slot = b.team;
        if w.hitter < 0 || w.hitter & 1 == slot & 1 || w.shot(slot).n == record::NONE || claimed {
            return;
        }
        let n = self.copy_path(b, w, c, seen, true);
        let path = self.path(c);
        let s = Searcher {
            row,
            reach: &b.reach,
            strong: b.strong,
            singles: self.singles,
            beside_human: self.mate > 0,
            runner: self.runner(b, w),
            depth: b.depth,
            path: &path,
            first: self.first.max(0) as usize,
            end: self.len.max(0) as usize,
        };
        let first = self.first;
        let put = |w: &mut World, at: i32, kind: i32, stand: [f32; 4]| w.put(slot, at, kind, b.pos, stand);
        let tiered = |ct: crate::ai::Contact| {
            let e = c.balls[ct.at].pos;
            (ct.at as i32 - first, [ct.stand[0], e[1], ct.stand[1], e[3]])
        };
        if !h.found {
            h.min = 0;
            if h.next >= n {
                return;
            }
            h.found = true;
            if let Some((stand, _, _, at)) = self.smash_search(b, w, c, h.next, 0) {
                put(w, at, contact::SMASH as i32, stand);
                h.kind = contact::SMASH;
                return;
            }
            if h.volley {
                if let Some((stand, _, _, at)) = self.volley_search(b, w, c, h.next, true, false) {
                    put(w, at, contact::VOLLEY as i32, stand);
                    h.kind = contact::VOLLEY;
                    return;
                }
            }
            for min in [1, 0] {
                if let Some(ct) = s.tier_search(Some(h.next.max(0) as usize), 3, min, true, false, false, seen, roll) {
                    let (at, stand) = tiered(ct);
                    put(w, at, contact::TIERED as i32, stand);
                    (h.kind, h.min) = (contact::TIERED, min);
                    return;
                }
            }
            h.found = false;
            h.next = n - 1;
            if c.balls[(n - 1) as usize].bounces >= 2 {
                w.put_none(slot, record::NONE, b.pos);
            }
            return;
        }
        let lost = match h.kind {
            contact::TIERED => true,
            contact::VOLLEY | contact::SMASH => {
                let got = if h.kind == contact::SMASH { self.smash_search(b, w, c, -1, 0) } else { self.volley_search(b, w, c, -1, true, false) };
                if let Some((stand, _, _, at)) = got {
                    put(w, at, h.kind as i32, stand);
                    return;
                }
                self.kind_frame(b, c, h.kind).is_none()
            }
            _ => {
                w.put_none(slot, record::NONE, b.pos);
                return;
            }
        };
        if !lost {
            return;
        }
        loop {
            if let Some(ct) = s.tier_search(None, 3, h.min, true, false, false, seen, roll) {
                let (at, stand) = tiered(ct);
                put(w, at, contact::TIERED as i32, stand);
                h.kind = contact::TIERED;
                return;
            }
            if h.min != 1 {
                if self.tier_frame(b, c, 3).is_none() {
                    w.put_none(slot, record::NONE, b.pos);
                }
                return;
            }
            h.min = 0;
        }
    }
}
