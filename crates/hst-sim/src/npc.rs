//! Background figures: which of a court's creature records (layout category 23) come to life at court load, as
//! what, and where they stand. A creature linked to an anchor record (category 14) stands at the anchor's position
//! minus its own (FPU subtractions); every figure's matrix is a turn by its yaw at that position.

use crate::{libm, ps2, sound};
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
/// ponytail: dodging the ball and players (wander mode's walk, collision, ground height) moves the walker; no
/// recording has a walker moving, so it is not ported (P14e). Its turns in place are [`walker_facing`].
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

    /// The match's start: with more than one player the cheer (animation 5) from a random frame in this walker's
    /// sixth of it, until the first point restarts the idle loops; else nothing until then.
    pub fn start(&mut self, players: u32, roll: &mut impl FnMut() -> u32) {
        self.counter = 0;
        if players > 1 {
            self.set_anim(5, players);
            self.random_frame(ps2::div(self.lens[5], 6.0), roll);
            self.counter = -1;
        }
    }

    /// A point was decided (the game skips faults and lets).
    pub fn react(&mut self) {
        self.mode = 1;
    }

    /// A new point: stop reacting; with `stagger` (any point after a decided one, or the match's first with more
    /// than one player) restart the idle loops one walker per tick.
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

/// A walker's drawn matrix: standing at `home`, turned toward (0, 0, `z`). Every serve placement (and the
/// walkers' reset) turns them to the court centre (`z` 0); a point they react to turns them to the middle of the
/// winners' half, `z` = 6.4 when the winning team's first player stands at z ≥ 0, else −6.4. Rows (up × forward,
/// up, forward, home); the layout's yaw only lasts until the first serve.
pub fn walker_facing(home: [f32; 4], z: f32) -> M4 {
    let (dx, dz) = (ps2::sub(0.0, home[0]), ps2::sub(z, home[2]));
    let inv = ps2::div(1.0, ps2::sqrt(ps2::madd(ps2::mul(dx, dx), dz, dz)));
    let f = [ps2::mul(dx, inv), 0.0, ps2::mul(dz, inv)];
    let up = [0.0, 1.0, 0.0];
    let cross = |a: usize, b: usize| ps2::msub(ps2::mul(up[a], f[b]), up[b], f[a]);
    [[cross(1, 2), cross(2, 0), cross(0, 1), 0.0], [0.0, 1.0, 0.0, 0.0], [f[0], 0.0, f[2], 0.0], home]
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

/// The gallery's cheer marks: a cheering walker registers one at its position, held for `slot` × 3 ticks; then
/// every 5th tick the mark jumps to a random offset from it (three draws). The gallery forgets them (its count goes
/// to 0, the slots keep their state) at a new point and a change of ends.
///
/// The manager keeps 6 slots for the walkers' marks and 93 more for court 5's own ([`Cheers::court5`]). On court 5
/// it steps and draws the marks 6 slots on from the ones counted: court 5's while they are set, and after the count
/// is cleared, as many of court 5's (from where they stopped) as walkers have cheered since. A walker cheering
/// while court 5's 93 are set writes over the 88th and switches the sprite to the walkers'.
#[derive(Clone, Debug, PartialEq)]
pub struct Cheers {
    pub slots: Vec<Cheer>,
    pub count: usize,
    /// The sprite drawn at each mark: 0 the walkers', 1 court 5's.
    pub kind: u8,
    /// The court is court 5 (the marks read 6 slots on).
    pub court5: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cheer {
    pub pos: [f32; 3],
    pub base: [f32; 3],
    pub delay: i32,
    pub n: i32,
}

/// The offsets a cheer mark jumps by on each axis.
const CHEER_JUMP: [f32; 6] = [0.05, 0.2, 0.1, -0.04, -0.21, -0.1];
/// The slots after the walkers' 6 that court 5's marks take.
const COURT5_MARKS: usize = 93;

impl Default for Cheers {
    fn default() -> Cheers {
        Cheers { slots: vec![Cheer::default(); 6 + COURT5_MARKS], count: 0, kind: 0, court5: false }
    }
}

impl Cheers {
    /// The marks of a match on `court`.
    pub fn new(court: u8) -> Cheers {
        Cheers { court5: court == 5, ..Cheers::default() }
    }

    /// Walker `slot` at `pos` cheers (written at the count even when the count is full).
    pub fn add(&mut self, pos: [f32; 3], slot: u32) {
        self.slots[self.count] = Cheer { pos, base: pos, delay: slot as i32 * 3, n: 0 };
        self.kind = 0;
        if self.count < 6 {
            self.count += 1;
        }
    }

    /// The match's start, a change of ends, a new point: every mark zeroed, the count 0. (A new point straight
    /// after the match's start or a change of ends only zeroes the count; nothing else of it is ever seen.)
    pub fn clear(&mut self) {
        *self = Cheers { court5: self.court5, ..Cheers::default() };
    }

    /// The match is over (more than one player): court 5 shows its own marks again when the favoured team won,
    /// any other court keeps its marks; an unfavoured win zeroes them.
    pub fn match_over(&mut self, favoured: bool, marks: &[[f32; 4]]) {
        if !favoured {
            self.clear();
        } else if self.court5 {
            self.court5(marks);
        }
    }

    /// Court 5's own marks (`exe::Game::court5_marks`) set: at a two-or-more-player match's start, when
    /// [`court5_again`] says so at a point's end, and at the match's end when the favoured team won.
    pub fn court5(&mut self, marks: &[[f32; 4]]) {
        for (c, m) in self.slots[6..].iter_mut().zip(marks) {
            let p = [m[0], m[1], m[2]];
            *c = Cheer { pos: p, base: p, delay: 3, n: 0 };
        }
        (self.kind, self.count) = (1, COURT5_MARKS);
    }

    /// The marks stepped and drawn.
    pub fn shown(&self) -> &[Cheer] {
        let at = if self.court5 { 6 } else { 0 };
        &self.slots[at..at + self.count]
    }

    /// The gallery's tick (after its cheer and reactions).
    pub fn step(&mut self, roll: &mut impl FnMut() -> u32) {
        let at = if self.court5 { 6 } else { 0 };
        for c in &mut self.slots[at..at + self.count] {
            if c.delay < 1 {
                if c.n % 5 == 0 && c.n != 0 {
                    c.pos = std::array::from_fn(|i| ps2::add(c.base[i], CHEER_JUMP[(roll() >> 16 & 0x7fff) as usize % 6]));
                }
                c.n += 1;
            }
            c.delay -= 1;
        }
    }
}

/// Court 5 shows its own marks again at a point's end: called plain (`call` 0), and a game or set won short of
/// the match's end (`game`) or the point an ace or a return winner (`shots` ≤ 2, the last hitter's team won it).
/// ponytail: the scoreboard voids an ace or return winner for one ball state of its own (not ported)
pub fn court5_again(call: u8, game: bool, shots: i32, hitter_won: bool) -> bool {
    call == 0 && (game || (1..=2).contains(&shots) && hitter_won)
}

/// One of court 5's own creatures (roster kinds 61..66, `creatureMdl/wim-0N` with `creatureAni/wim-0N.UVA`): a
/// figure whose texture is a 4×4 atlas picked by its UV-animation frame. Its column (`base`) is drawn once when
/// made; its row is the cheer level (0 seated, 1..3 standing up), which ping-pongs between 1 and 3 every other tick
/// while it cheers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Crowd {
    pub base: i32,
    pub on: bool,
    pub tick: i32,
    /// The UV-animation frame shown (set on the model at the start of the next tick).
    pub frame: i32,
    pub level: i32,
    pub up: bool,
}

impl Crowd {
    /// Made at the match's start: one draw of the shared generator for its atlas column.
    pub fn new(roll: &mut impl FnMut() -> u32) -> Crowd {
        let base = (roll() >> 16 & 3) as i32;
        Crowd { base, frame: base, ..Crowd::default() }
    }

    /// Seated again: at the serve set-up and as a point's end is left.
    pub fn reset(&mut self) {
        *self = Crowd { base: self.base, frame: self.base, ..Crowd::default() };
    }

    /// Stands up: at the start of a match of more than one player, at its end, and at [`crowd_cheers_point`].
    /// One draw for the starting level; the tick count and direction carry on.
    pub fn cheer(&mut self, roll: &mut impl FnMut() -> u32) {
        self.on = true;
        self.level = (roll() >> 16 & 0x7fff) as i32 % 3 + 1;
    }

    pub fn step(&mut self) {
        if !self.on {
            return;
        }
        if self.tick % 2 == 0 {
            match (self.up, self.level) {
                (false, l) if l < 2 => self.up = true,
                (false, _) => self.level -= 1,
                (true, l) if l < 3 => self.level += 1,
                (true, _) => self.up = false,
            }
            self.frame = self.base + self.level * 4;
        }
        self.tick += 1;
    }
}

/// Court 5's creatures cheer a point's end unless an instant replay was picked for it, and then only for a game,
/// set or match won (`event`) or a call other than a fault, let or net (`call` 1..4, 6).
pub fn crowd_cheers_point(replay: bool, event: bool, call: u8) -> bool {
    !replay && (event || !matches!(call, 1..=4 | 6))
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
/// ponytail: the turn eased through a pause, spline legs, random points about the area,
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
    /// Steering rows: the heading's pitch (about X) and yaw (about Y), eased toward the target each tick.
    pub pitch: f32,
    pub yaw: f32,
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
    /// Type 15 (court 4's passing ball): playing, its route (0 from home, 1–3 from `routes`).
    pub playing: bool,
    pub route: u32,
    pub routes: [[[f32; 4]; 2]; 3],
    /// Had its go (type 15 played, a startled type startled): a reset leaves it off until a new match unless its
    /// row rearms it.
    pub done: bool,
    /// `done` as the last serve set-up found it, with the matrix and the path's node, previous node, leg ticks left
    /// and pause left: a replay's start puts them back ([`Trigger::serve_setup`]); resets keep them.
    pub kept_done: bool,
    pub kept: (M4, i16, i16, i16, i16),
    /// The startled types' model weight, set at their reset: 1 for type 32, 0 for the others (it scales the
    /// model's sub-meshes when drawn).
    pub weight: f32,
    /// Type 38's animation restarted on its other clip (a ball faster than its row's `hit_kmh`); kept through resets.
    pub alt: bool,
    /// Type 38's two clips' lengths (its model's motions 0 and 1): a hit sets `len` to the one it plays.
    pub clip_lens: [f32; 2],
    /// Type 2: where the point's first bounce landed, if in the court with no body hit.
    pub landing: Option<[f32; 4]>,
    /// Types 19 and 33: a sound armed (at a game's end) for the next reset; type 40: it cheers for `team`.
    pub armed: bool,
    pub team: i32,
    /// Type 19's sweep loudness (as `angle` its bearing in degrees); the last sound's volume (0–128) and bearing
    /// (type 6's flyover updates its volume each tick).
    pub vol: f32,
    pub volume: i32,
    pub bearing: i32,
}

/// What the startled creatures watch: the players' positions then the ball's, and the per-type "startled" flags
/// all creatures of a type share (set by the first one startled, cleared by the type's reset).
#[derive(Clone, Debug)]
pub struct Near {
    pub pos: Vec<[f32; 4]>,
    pub flags: [bool; 64],
    /// The match is changing ends (its state 1; type 15 only rolls then) and how many play.
    pub ends: bool,
    pub players: u32,
    /// A tiebreak in the deciding set (type 44's crowd animates and calls only then).
    pub deciding: bool,
    /// The gallery's manager paused (set by type 44's reset).
    pub paused: bool,
    /// The players' model matrices then the ball's (the ball-hit obstacles test the boxes; 38 and 43 the ball's
    /// position, its row 3), and the ball's speed in km/h ([`ball_kmh`] of its last move): both as the tick began
    /// (the obstacles run before the ball and players move).
    pub boxes: Vec<M4>,
    pub kmh: f32,
    /// The point just decided: a game or set won short of the match's end, a point the game names (an ace, a
    /// return, smash or passing winner), its call (0 plain), the winning team and the shots played.
    pub game: bool,
    pub named: bool,
    pub call: u8,
    pub winner: i32,
    pub shots: i32,
    /// The teams with a human player.
    pub humans: [bool; 2],
    /// The ball's bounces so far, the first one's spot and whether it was in the court with no body hit.
    pub bounces: i32,
    pub landing: [f32; 4],
    pub in_court: bool,
    /// The camera's eye.
    pub eye: [f32; 4],
}

impl Default for Near {
    fn default() -> Near {
        Near {
            pos: Vec::new(), flags: [false; 64], ends: false, players: 0, deciding: false, paused: false, boxes: Vec::new(), kmh: 0.0,
            game: false, named: false, call: 0, winner: 0, shots: 0, humans: [false; 2], bounces: 0, landing: [0.0; 4],
            in_court: false, eye: [0.0; 4],
        }
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

/// `b` turned to `a` the short way round: the plain difference unless adding or taking a full turn is shorter.
fn turn(a: f32, b: f32) -> f32 {
    let d = ps2::sub(ps2::add(a, world::PI), ps2::add(b, world::PI));
    let (up, dn) = (ps2::add(world::TWO_PI, d), ps2::sub(d, world::TWO_PI));
    if d.abs() < up.abs() && d.abs() < dn.abs() {
        d
    } else if up.abs() < dn.abs() {
        up
    } else {
        dn
    }
}

/// Into ±π by one turn.
fn wrap(a: f32) -> f32 {
    if world::PI < a {
        ps2::sub(a, world::TWO_PI)
    } else if a < -world::PI {
        ps2::add(world::TWO_PI, a)
    } else {
        a
    }
}

/// (pitch, yaw) of direction `d`: yaw about Y from x and z, pitch from −y over the horizontal length.
fn heading([x, y, z, _]: [f32; 4]) -> (f32, f32) {
    (libm::atan2f(-y, ps2::sqrt(ps2::madd(ps2::mul(z, z), x, x))), libm::atan2f(x, z))
}

/// `d` scaled to unit length (its w too).
fn unit(d: [f32; 4]) -> [f32; 4] {
    let q = ps2::div(1.0, ps2::sqrt(dist2(d)));
    d.map(|c| ps2::mul(c, q))
}

/// [`unit`] with the length summed y, x, z on the accumulator (no leading zero).
fn unit3(d: [f32; 4]) -> [f32; 4] {
    let [x, y, z, _] = d;
    let q = ps2::div(1.0, ps2::sqrt(ps2::madd(ps2::madd(ps2::mul(y, y), x, x), z, z)));
    d.map(|c| ps2::mul(c, q))
}

/// The ball's speed in km/h from its last move (its position change over the last tick; the length summed y, x, z).
pub fn ball_kmh([x, y, z]: [f32; 3]) -> f32 {
    ps2::div(ps2::mul(3600.0, ps2::mul(60.0, ps2::sqrt(ps2::madd(ps2::madd(ps2::mul(y, y), x, x), z, z)))), 1000.0)
}

/// The box half-extents of obstacle types 14 and 21, and of any other type (38's), set by the game's code.
pub fn obstacle_extents(ty: u8) -> [f32; 3] {
    match ty {
        14 => [1.0, 2.5, 0.1],
        21 => [1.0, 0.9, 0.025],
        _ => [0.18, 0.8, 0.18],
    }
}

/// Whether box `a` (half-extents 0.2 along its matrix rows: a player or the ball) overlaps obstacle box `b`
/// (half-extents `e`): the separating-axis test over both boxes' axes and their nine cross products.
pub fn boxes_overlap(a: &M4, b: &M4, e: [f32; 3]) -> bool {
    let dot = |u: [f32; 4], l: [f32; 4]| ps2::madd(ps2::madd(ps2::mul(u[1], l[1]), u[0], l[0]), u[2], l[2]);
    let cross = |u: [f32; 4], v: [f32; 4]| {
        let c = |i: usize, j: usize| ps2::msub(ps2::mul(u[i], v[j]), u[j], v[i]);
        [c(1, 2), c(2, 0), c(0, 1), 0.0]
    };
    let t = minus(b[3], a[3]);
    let mut axes = (0..3).flat_map(|i| [a[i], b[i]]).chain((0..9).map(|k| cross(a[k / 3], b[k % 3])));
    let radius = |m: &M4, e: [f32; 3], l: [f32; 4]| ps2::add(ps2::mul(e[2], dot(m[2], l)).abs(), ps2::add(ps2::mul(e[0], dot(m[0], l)).abs(), ps2::mul(e[1], dot(m[1], l)).abs()));
    axes.all(|l| dot(l, t).abs() <= ps2::add(radius(a, [0.2; 3], l), radius(b, e, l)))
}

/// Whether the ball's box (±0.2 about `ball`) overlaps the box ±`e` about `at`.
fn ball_in(ball: [f32; 4], at: [f32; 4], e: [f32; 3]) -> bool {
    (0..3).all(|c| ps2::sub(ball[c], 0.2) < ps2::add(at[c], e[c]) && !(ps2::add(ball[c], 0.2) <= ps2::sub(at[c], e[c])))
}

/// The sound a trigger creature's key plays: the ball-hit obstacles' on program 2 (43's on 7) at the game's sound
/// effect level (0.9 · 128) scaled by 1.2 (1.35 for 38); the others' on program 7 at 0x40.
pub fn trigger_sound(ty: u8, key: i32) -> sound::Play {
    let level = |k: f32| ps2::mul(115.0, k) as i32;
    let (program, volume) = match ty {
        14 | 21 => (2, level(1.2)),
        38 => (2, level(1.35)),
        43 => (7, level(1.2)),
        _ => (7, 0x40),
    };
    sound::Play { slot: 0, program, key: key as u8, volume, speed: 1.0 }
}

impl Trigger {
    fn pos(&self) -> [f32; 4] {
        self.world[3]
    }

    /// Where it stands.
    pub fn pos3(&self) -> [f32; 3] {
        [self.world[3][0], self.world[3][1], self.world[3][2]]
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

    /// Show frame `t` and next: wrapped into the animation when it loops, else held within it.
    fn set_frame(&mut self, t: f32, row: &TriggerRow) {
        // no animation (length 0, e.g. court 7's 27–29): the controller only stores the next frame
        if self.len == 0.0 {
            self.next = t;
            return;
        }
        let t = self.held(t, row);
        (self.frame, self.next) = (t, t);
    }

    /// The controller's frame set: rows without an idle repeat gap loop their animation (the spawn sets the
    /// controller's loop flag), the others hold at the ends.
    fn held(&self, t: f32, row: &TriggerRow) -> f32 {
        let (len, mut t) = (self.len, t);
        if row.repeat.0 != 0 || len == 0.0 {
            return t.clamp(0.0, len);
        }
        while len <= t {
            t = ps2::sub(t, len);
        }
        while t < 0.0 {
            t = ps2::add(t, len);
        }
        t
    }

    /// The motion reset (a new point, or a loop's end): back to the start, a direction bit with `reverse_roll`,
    /// the first leg and the pause before it.
    pub fn reset(&mut self, row: &TriggerRow, roll: &mut impl FnMut() -> u32) -> Vec<i32> {
        self.reset_near(row, &mut Near::default(), roll)
    }

    /// [`Trigger::reset`] with what the startled creatures watch (their reset clears their type's flag).
    pub fn reset_near(&mut self, row: &TriggerRow, near: &mut Near, roll: &mut impl FnMut() -> u32) -> Vec<i32> {
        // its one go played: off until a new match unless the row rearms it
        if self.done && !row.rearm {
            self.active = false;
            return Vec::new();
        }
        // ponytail: those whose model starts animating are not modelled
        let o = std::mem::take(self);
        *self = Trigger {
            ty: o.ty, anchor: o.anchor, path: o.path, start: o.start, node: o.start, prev: o.start, home: o.home, world: o.home, msg: o.msg,
            frame: o.frame, next: o.next, len: o.len, timer: o.timer, sweep: o.sweep, pan: o.pan, down: o.down,
            counter: o.counter, gap: o.gap, voice: o.voice, scrub: o.scrub,
            playing: o.playing, route: o.route, routes: o.routes, done: o.done, kept_done: o.kept_done, kept: o.kept, weight: o.weight, alt: o.alt, clip_lens: o.clip_lens, armed: o.armed, ..Trigger::default()
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

    /// Type 44 made with `voice` (`Game::deciding_voices`, the first four made; the rest silent): its call's
    /// countdown.
    pub fn give_voice(&mut self, row: &TriggerRow, voice: (i32, i32), roll: &mut impl FnMut() -> u32) {
        self.voice = voice;
        if voice.1 != 0 {
            let u = ps2::mul(2.3283064e-10, ps2::utof(roll()));
            self.gap = ps2::mul(voice.1 as f32, ps2::msub(1.0, row.idle.1, u)) as i32;
        }
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
            if row.steer != 0.0 {
                (self.pitch, self.yaw) = heading(unit(minus(target, self.pos())));
            }
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
                let mut arrived = false;
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
                } else if row.steer != 0.0 {
                    // steering: pitch and yaw eased by `steer` toward the target, a step of `speed` along the heading
                    let (p, y) = heading(unit(minus(self.target, self.pos())));
                    self.pitch = wrap(ps2::madd(ps2::add(0.0, self.pitch), row.steer, turn(p, self.pitch)));
                    self.yaw = wrap(ps2::madd(ps2::add(0.0, self.yaw), row.steer, turn(y, self.yaw)));
                    let m = world::mat_mul(&world::mat_mul(&world::IDENTITY, &world::rot_x(self.pitch)), &world::rot_y(self.yaw));
                    let f = m[2];
                    if self.orient {
                        self.face(if row.orient == 2 { [f[0], 0.0, f[2], 0.0] } else { f });
                    }
                    self.world[3] = std::array::from_fn(|c| ps2::madd(ps2::add(0.0, self.world[3][c]), f[c], row.speed));
                    // arrived once the heading points past the target (seen from the origin, then from here)
                    // ponytail: the first point it measures from (+0x260) is never written: the origin in every recording
                    let [x, y, z, _] = unit3(minus(self.target, [0.0; 4]));
                    if !(ps2::madd(ps2::madd(ps2::mul(f[1], y), f[0], x), f[2], z) < 0.0) {
                        let [x, y, z, _] = unit3(minus(self.target, self.pos()));
                        let dot = ps2::madd(ps2::add(0.0, ps2::madd(ps2::add(0.0, ps2::mul(f[1], y)), f[0], x)), f[2], z);
                        arrived = dot <= 0.0;
                    }
                } else {
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
                let mut next = arrived;
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
                // the controller: show the next frame (looped or held at the ends), step on by one
                if self.len != 0.0 {
                    self.frame = self.held(self.next, row);
                }
                self.next = ps2::add(if self.len == 0.0 { self.next } else { self.frame }, 1.0);
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
        // the draw after the tick counts `stage` down (set to 1 at a one-shot path's end): at 0 it is gone and its
        // type's startled flag clears
        // ponytail: rows with byte +0x71 (of the moving ones only type 32) count only while in view; not ported
        if self.on && self.stage != 0 {
            self.stage -= 1;
            if self.stage == 0 {
                self.active = false;
                near.flags[self.ty as usize] = false;
            }
        }
        sounds
    }

    /// The match's messages, sent whether it is active or not: 6 the point decided, 7 the scoreboard's pause over,
    /// 8 a game's end (just before its reset); the sounds started.
    pub fn message(&mut self, row: &TriggerRow, msg: u8, near: &mut Near, roll: &mut impl FnMut() -> u32) -> Vec<i32> {
        self.callback(row, msg, near, roll)
    }

    /// The serve set-up's message (also sent when a replay starts): out of a replay it keeps `done`, the matrix and the
    /// path's place, at a replay's start it puts them back; then it resets, except type 37 and, while the match's
    /// phase (`phase`, read when the message comes: 0 the match's start, 1 a change of ends) is 0 or 1, all but
    /// type 15. The restored matrix and place only last when no reset follows.
    pub fn serve_setup(&mut self, row: &TriggerRow, replaying: bool, phase: u8, near: &mut Near, roll: &mut impl FnMut() -> u32) -> Vec<i32> {
        self.msg = 0xe;
        if replaying {
            (self.done, (self.world, self.node, self.prev, self.left, self.wait)) = (self.kept_done, self.kept);
        } else {
            (self.kept_done, self.kept) = (self.done, (self.world, self.node, self.prev, self.left, self.wait));
        }
        if self.ty != 37 && (phase > 1 || self.ty == 15) { self.reset_near(row, near, roll) } else { Vec::new() }
    }

    /// Type 2's matrix: at the landing spot, turned (about Y) to face the camera's eye.
    fn facing_eye(&self, near: &Near) -> M4 {
        let l = self.landing.unwrap_or_default();
        let (x, z) = (ps2::sub(near.eye[0], l[0]), ps2::sub(near.eye[2], l[2]));
        let q = ps2::div(1.0, ps2::sqrt(ps2::madd(ps2::mul(x, x), z, z)));
        let (f, u) = ([ps2::mul(x, q), 0.0, ps2::mul(z, q), 0.0], [0.0, 1.0, 0.0, 0.0]);
        let cross = |a: usize, b: usize| ps2::sub(ps2::mul(u[a], f[b]), ps2::mul(u[b], f[a]));
        [[cross(1, 2), cross(2, 0), cross(0, 1), 0.0], u, f, [l[0], l[1], l[2], 1.0]]
    }

    /// The type's own behaviour on message `msg` (2 each tick, 4 at a reset; 6–8 as [`Trigger::message`]); the
    /// sounds started.
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
                        (near.flags[ty], self.done) = (true, true);
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
                    }, row);
                }
            }
            (0 | 1 | 5 | 31 | 32 | 39, 0 | 4) => {
                (self.on, self.weight, self.animating, self.moving, self.world) = (ty == 32, (ty == 32) as u8 as f32, false, false, self.home);
                (self.pitch, self.yaw) = heading(self.home[2]);
                self.set_frame(0.0, row);
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
            (27..=29, 4) => {
                (self.on, self.moving, self.world, self.animating) = (true, false, self.home, false);
                (self.pitch, self.yaw) = heading(self.home[2]);
            }
            // ball-hit obstacles: a player's box (14, 21) starts the animation and sound; the ball's does at its
            // speed, and tells the match (message 0x14); once until the reset
            (14 | 21, 2) => {
                self.struck = false;
                if self.startled {
                    return sounds;
                }
                let e = obstacle_extents(self.ty);
                let Some(i) = near.boxes.iter().position(|a| boxes_overlap(a, &self.world, e)) else { return sounds };
                let ball = i + 1 == near.boxes.len();
                if !ball || row.hit_kmh <= near.kmh {
                    self.restart_anim();
                    (self.startled, self.animating) = (true, true);
                    sounds.push(if ty == 14 { 0xb } else { 0xa });
                }
                (self.startled, self.struck) = (true, ball);
            }
            // turns to face a ball of 20 km/h or more (its other animation above the row's speed)
            (38 | 43, 2) => {
                self.struck = false;
                let Some(&[.., ball]) = near.boxes.last() else { return sounds };
                let e = if ty == 38 { [0.18, 0.8, 0.18] } else { [0.15, 1.3, 0.15] };
                if self.startled || !ball_in(ball, pos, e) {
                    return sounds;
                }
                if ty == 43 && row.hit_kmh <= near.kmh {
                    self.restart_anim();
                    self.animating = true;
                    sounds.push(0x23);
                } else if ty == 38 && 20.0 <= near.kmh {
                    self.restart_anim();
                    self.alt = row.hit_kmh <= near.kmh;
                    self.len = self.clip_lens[self.alt as usize];
                    (self.startled, self.animating) = (true, true);
                    self.yaw = wrap(heading(unit3(minus(ball, pos))).1);
                    self.world = world::mat_mul(&world::mat_mul(&world::IDENTITY, &world::rot_x(self.pitch)), &world::rot_y(self.yaw));
                    self.world[3] = pos;
                    sounds.push(0xc);
                }
                (self.startled, self.struck) = (true, true);
            }
            (14 | 21 | 38 | 43, 0 | 4) => {
                // (the frame back to 0 on the clip it last played)
                (self.on, self.moving, self.world, self.startled, self.animating) = (true, false, self.home, false, false);
                (self.pitch, self.yaw) = heading(self.home[2]);
                self.set_frame(0.0, row);
            }
            // scrubs forward while someone is within 1.0, to the end; then back to 0 the next time
            (48, 2) => {
                let (mut s, mut f) = self.scrub;
                let far = || within(1.0).iter().all(|&n| !n);
                match s {
                    0 if !far() => s = if f != 0 { 2 } else { 1 },
                    1 => {
                        self.set_frame(f as f32, row);
                        f += 1;
                        if self.len <= f as f32 && far() {
                            s = 0;
                        }
                        f = (f as f32).min(self.len) as i32;
                    }
                    2 => {
                        self.set_frame(f as f32, row);
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
                (self.world, self.on, self.moving, self.animating) = (self.home, true, false, false);
                (self.pitch, self.yaw) = heading(self.home[2]);
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
            // the change of ends' resets: once a match, one in two rolls it off along a route (its animation
            // carries it); it plays on through the next reset
            (15, 4) => {
                if near.ends && 2 <= near.players && !self.done && roll() >> 16 & 1 == 0 {
                    self.route = roll() >> 16 & 3;
                    if self.route == 0 {
                        self.world = self.home;
                        (self.pitch, self.yaw) = heading(self.home[2]);
                    } else {
                        let [p0, p1] = self.routes[self.route as usize - 1];
                        let (x, z) = (ps2::sub(p1[0], p0[0]), ps2::sub(p1[2], p0[2]));
                        let q = ps2::div(1.0, ps2::sqrt(ps2::madd(ps2::mul(x, x), z, z)));
                        let (f, u) = ([ps2::mul(x, q), 0.0, ps2::mul(z, q), 0.0], [0.0, 1.0, 0.0, 0.0]);
                        let cross = |a: usize, b: usize| ps2::sub(ps2::mul(u[a], f[b]), ps2::mul(u[b], f[a]));
                        let m = [[cross(1, 2), cross(2, 0), cross(0, 1), 0.0], u, f, [0.0, 0.0, 0.0, 1.0]];
                        self.world = world::mat_mul(&m, &world::rot_y(world::PI));
                        self.world[3] = [p0[0], p0[1], p0[2], 1.0];
                    }
                    (self.speed, self.playing) = (1.0, true);
                    self.restart_anim();
                }
                (self.on, self.animating) = (self.playing, true);
                self.moving &= self.playing;
            }
            // a bounce sound at four frames; it fades out (+0x274, its alpha) over the last 10 and is gone
            (15, 2) if self.playing => {
                if [86.0, 147.0, 200.0, 240.0].contains(&self.frame) {
                    sounds.push(row.sound);
                }
                if ps2::sub(self.len, self.frame) <= 10.0 {
                    self.speed = ps2::sub(self.speed, 0.1);
                }
                if self.len <= self.frame {
                    (self.playing, self.on, self.done) = (false, false, true);
                }
            }
            // the reset: in the deciding set its idle countdown, turned to the court centre where it stands, and the
            // gallery's manager paused (`Near::paused`); else the manager resumed and its call's countdown (a voiced
            // one only)
            (44, 4) => {
                (self.on, self.animating) = (near.deciding, false);
                near.paused = near.deciding;
                if self.on {
                    self.counter = ps2::mul(row.idle.0 as f32, ps2::msub(1.0, row.idle.1, u(roll()))) as i32;
                    self.world = walker_facing(self.home[3], 0.0);
                } else if self.voice.1 != 0 {
                    self.gap = ps2::mul(self.voice.1 as f32, ps2::msub(1.0, row.idle.1, u(roll()))) as i32;
                }
            }
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
            // court 1: where the point's first bounce landed (in the court, no body hit), shown facing the camera
            // after a game or set won on a plain call with a named point
            (2, 2) => {
                if near.bounces == 1 && self.counter != 1 {
                    self.landing = near.in_court.then_some(near.landing);
                    self.counter = 1;
                } else {
                    self.counter = near.bounces;
                }
                if self.on {
                    self.world = self.facing_eye(near);
                }
            }
            (2, 6) if near.game && self.landing.is_some() && near.named && near.call == 0 => {
                self.set_frame(1.0, row);
                (self.animating, self.on, near.flags[ty]) = (true, true, true);
                self.world = self.facing_eye(near);
            }
            (2, 4) => {
                (self.counter, self.landing, self.on, self.moving, self.world, self.animating) = (0, None, false, false, self.home, false);
                self.restart_anim();
                near.flags[ty] = false;
            }
            // court 2: the flyover after a game or set won on a plain call with a named point: its sound follows it,
            // fading with |z|; it steers along its reset's leg at y −60
            (6, 7) if near.game && near.named && near.call == 0 => {
                (self.animating, self.on, near.flags[ty]) = (true, true, true);
                self.restart_anim();
                sounds.push(row.sound);
                self.volume = flyover_volume(pos[2]);
            }
            (6, 2) if self.on => {
                let (p, y) = heading(unit3(self.vel));
                self.pitch = wrap(ps2::add(self.pitch, turn(p, self.pitch)));
                self.yaw = wrap(ps2::add(self.yaw, turn(y, self.yaw)));
                self.world = world::mat_mul(&world::mat_mul(&world::IDENTITY, &world::rot_x(self.pitch)), &world::rot_y(self.yaw));
                self.world[3] = [ps2::add(pos[0], self.vel[0]), -60.0, ps2::add(pos[2], self.vel[2]), ps2::add(pos[3], self.vel[3])];
                self.volume = flyover_volume(self.world[3][2]);
            }
            (6, 4) => {
                (self.on, self.moving, self.world, self.animating, self.volume) = (false, false, self.home, false, -1);
                near.flags[ty] = false;
            }
            // court 5's and court 8's: a game's end rolls one in two to arm a sound for its reset, once a match;
            // 33 plays its sound there, 19 sweeps its own round the stereo field over 6 s
            (19 | 33, 8) => {
                if roll() >> 16 & 1 == 0 && !self.done {
                    self.armed = true;
                }
            }
            (19 | 33, 4) => {
                (self.on, self.animating, self.playing) = (false, false, false);
                if std::mem::take(&mut self.armed) {
                    if ty == 33 {
                        sounds.push(row.sound);
                        (self.volume, self.done) = (0x80, true);
                    } else {
                        (self.playing, self.angle, self.vol, self.counter, self.volume) = (true, 135.0, 0.0, 0, -1);
                    }
                }
            }
            (19, 2) if self.playing => {
                let c = self.counter;
                if 360 <= c {
                    (self.done, self.playing) = (true, false);
                } else if c % 10 == 0 {
                    // ponytail: the call's arguments read as (bearing, volume); a guess from their ranges
                    sounds.push(row.sound);
                    (self.bearing, self.volume) = (self.angle as i32, self.vol as i32);
                }
                let t = c as f32;
                if t < 180.0 {
                    self.vol = ps2::mul(ps2::sub(1.0, libm::powf(ps2::msub(1.0, t, 0.0055555557), 4.0)), 128.0);
                    self.angle = ps2::sub(self.angle, 0.5);
                } else if t == 180.0 {
                    (self.vol, self.angle) = (128.0, 45.0);
                } else {
                    self.vol = ps2::mul(ps2::sub(1.0, libm::powf(ps2::mul(ps2::sub(t, 180.0), 0.0055555557), 4.0)), 128.0);
                    self.angle = ps2::sub(self.angle, 0.5);
                    if self.angle <= 0.0 {
                        self.angle = 359.0;
                    }
                }
                self.counter += 1;
            }
            // court 6's: in the deciding set it stands (its head following the ball) and the gallery's manager pauses
            // ponytail: the head's look-at is not drawn
            (22, 4) => {
                (self.world, self.moving, self.animating, self.on) = (self.home, false, false, near.deciding);
                near.paused = self.on;
                if self.on {
                    (self.pitch, self.yaw) = heading(self.home[2]);
                }
            }
            (22, 8) => near.paused = self.on,
            (44, 8) => near.paused = near.deciding,
            // court 9's: cheers when a team with no human wins a game or set in three shots or fewer
            (40, 4) => {
                self.on = false;
                self.armed = near.humans[0] != near.humans[1];
                self.team = !near.humans[0] as i32;
                self.animating = false;
            }
            (40, 7) if self.armed && near.game && near.winner == (self.team != 1) as i32 && near.shots < 3 => {
                sounds.push(row.sound);
                self.volume = 0x80;
            }
            _ => {}
        }
        sounds
    }
}

/// The flyover's volume at `z`: full at 0, silent past ±550.
fn flyover_volume(z: f32) -> i32 {
    let a = z.abs();
    if 550.0 < a {
        return 0;
    }
    ps2::mul(ps2::sub(1.0, libm::powf(ps2::sub(1.0, ps2::sub(1.0, ps2::div(a, 550.0))), 2.0)), 128.0) as i32
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
            pitch: 0.0,
            yaw: 0.0,
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
            playing: false,
            route: 0,
            routes: [[[0.0; 4]; 2]; 3],
            done: false,
            kept_done: false,
            kept: ([[0.0; 4]; 4], 0, 0, 0, 0),
            weight: 0.0,
            alt: false,
            clip_lens: [0.0; 2],
            landing: None,
            armed: false,
            team: 0,
            vol: 0.0,
            volume: 0,
            bearing: 0,
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

    #[test]
    fn court5_marks_alias_the_walkers_slots() {
        let marks: Vec<[f32; 4]> = (0..93).map(|i| [i as f32, 0.0, 0.0, 1.0]).collect();
        let mut c = super::Cheers::new(5);
        c.court5(&marks);
        assert_eq!((c.kind, c.shown().len(), c.shown()[0].pos[0]), (1, 93, 0.0));
        // a walker's mark on the full count lands in slot 93: court 5's 88th
        c.add([-1.0; 3], 2);
        assert_eq!((c.kind, c.shown().len(), c.shown()[87].pos, c.shown()[87].delay), (0, 93, [-1.0; 3], 6));
        // after a clear, walkers write slots 0.. while court 5 still reads its own from slot 6
        c.clear();
        c.add([-2.0; 3], 0);
        assert_eq!(c.shown()[0].pos, [0.0; 3]);
        // elsewhere the walkers' own
        let mut c = super::Cheers::new(1);
        c.add([-2.0; 3], 0);
        assert_eq!(c.shown()[0].pos, [-2.0; 3]);
    }

    /// Recorded at a doubles match's start (court 4): the four walkers register marks 0–3 (delays 0, 3, 6, 9)
    /// in their order, each from its cheer, then the manager zeroes them; court 5 then sets its own.
    #[test]
    fn match_start() {
        let marks: Vec<[f32; 4]> = (0..93).map(|i| [i as f32, 1.0, 0.0, 1.0]).collect();
        for court in [4, 5] {
            let mut c = super::Cheers::new(court);
            let mut draws = 0;
            for slot in 0..4 {
                let mut w = super::Walker { slot, mode: 0, counter: 0, anim: 0, frame: 0.0, next: 0.0, speed: 1.0, advancing: true, lens: [60.0; 7] };
                w.start(4, &mut || { draws += 1; 0x8000_0000 });
                // the middle of its sixth of the cheer
                assert_eq!((w.anim, w.counter, w.frame), (5, -1, 10.0 * slot as f32 + 5.0));
                c.add([slot as f32; 3], slot);
            }
            assert_eq!((draws, c.count, c.slots[3].delay), (4, 4, 9));
            c.clear();
            if court == 5 {
                c.court5(&marks);
            }
            assert_eq!(c.shown().len(), if court == 5 { 93 } else { 0 });
        }
        // singles too; one player draws nothing
        let mut w = super::Walker { slot: 0, mode: 0, counter: 5, anim: 0, frame: 0.0, next: 0.0, speed: 1.0, advancing: true, lens: [60.0; 7] };
        w.start(1, &mut || unreachable!());
        assert_eq!((w.anim, w.counter), (0, 0));
    }

    #[test]
    fn court5_point_and_match_over() {
        use super::court5_again;
        // a game won, an ace, a return winner; not with a call, nor a winner from the third shot on
        assert!(court5_again(0, true, 7, false) && court5_again(0, false, 1, true) && court5_again(0, false, 2, true));
        assert!(!court5_again(1, true, 1, true) && !court5_again(0, false, 3, true) && !court5_again(0, false, 2, false));
        let marks = vec![[1.0, 2.0, 3.0, 1.0]; 93];
        let mut c = super::Cheers::new(5);
        c.match_over(true, &marks);
        assert_eq!((c.kind, c.count), (1, 93));
        c.match_over(false, &marks);
        assert_eq!((c.kind, c.count, c.slots[6].pos), (0, 0, [0.0; 3]));
        let mut c = super::Cheers::new(2);
        c.add([1.0; 3], 0);
        c.match_over(true, &marks);
        assert_eq!(c.count, 1);
    }
}
