//! Background figures: which of a court's creature records (layout category 23) come to life at court load, as
//! what, and where they stand. A creature linked to an anchor record (category 14) stands at the anchor's position
//! minus its own (FPU subtractions); every figure's matrix is a turn by its yaw at that position.

use crate::{ps2, sound};
use crate::world::{self, M4};
use hst_data::exe::{EmitterRow, NpcEntry};
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
