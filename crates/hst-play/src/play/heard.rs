//! Sounds out of the tick (M2d): a rollback re-runs ticks, and a re-run tick must not play its sounds again.
//!
//! Every match tick has a number (`Heard::tick`, in the snapshot, so a restore winds it back). A one-shot sound (a
//! hit, whoosh, shout, bounce, cheer, jingle, umpire word, creature) plays only the first time its tick, what it is
//! and how many alike came before it that tick are seen (`Played::first`); a re-run that comes out differently
//! plays its new sounds then. What follows the sim from tick to tick (the flight whistle, the flyover, the BGM's
//! level) is only set on ticks heard for the first time (`Played::fresh`), from the sim's state then.
//!
//! The effects (impacts, sparks, ribbons, bounce marks, landing markers) and balloons need nothing here: their state
//! is in the snapshot and `Update` draws the last tick's.

use bevy::prelude::*;
use std::collections::{BTreeSet, HashMap};
use std::fmt::Debug;

/// The sounds' tick state, restored with the match (`snapshot`).
#[derive(Resource, Clone, Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct Heard {
    /// Match ticks so far.
    pub tick: u32,
    /// The flight whistle as the sim last saw it: `Game::whistle`'s count and the tick that count started.
    pub whistle: (u32, u32),
    /// Ticks left of the umpire's word.
    pub voice_left: u32,
}

/// What has been played: kept through a restore (never in the snapshot).
#[derive(Resource, Default)]
pub struct Played {
    /// The latest tick heard, and whether this tick is it.
    high: u32,
    pub fresh: bool,
    keys: BTreeSet<(u32, u64, u32)>,
    alike: HashMap<u64, u32>,
    /// The flight whistle sounding: the tick it started (`Heard::whistle`) and its play id.
    pub whistle: (Option<u32>, u64),
    /// The umpire's word and court 2's flyover sounding (play ids).
    pub umpire: u64,
    pub flyover: Option<u64>,
}

/// How many ticks back a re-run is still recognised.
const KEEP: u32 = 120;

impl Played {
    /// Tick `t` starts.
    pub fn hear(&mut self, t: u32) {
        self.fresh = t > self.high;
        self.alike.clear();
        if self.fresh {
            self.high = t;
            self.keys = self.keys.split_off(&(t.saturating_sub(KEEP), 0, 0));
        }
    }

    /// `what` is due this tick (`t`): true only the first time this tick plays it.
    pub fn first(&mut self, t: u32, what: impl Debug) -> bool {
        let h = super::snapshot::fnv(&format!("{what:?}"));
        let n = self.alike.entry(h).or_default();
        *n += 1;
        self.keys.insert((t, h, *n))
    }
}

/// The tick's number, first thing in the tick.
pub fn count(mut heard: ResMut<Heard>, mut played: ResMut<Played>) {
    heard.tick += 1;
    played.hear(heard.tick);
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Heard>().init_resource::<Played>().add_systems(FixedUpdate, count.before(super::remember));
}

#[test]
fn a_rerun_tick_plays_nothing_again() {
    let mut p = Played::default();
    let ticks = |p: &mut Played, from: u32| (from..from + 8).map(|t| { p.hear(t); [p.first(t, ("hit", t)), p.first(t, "cheer"), p.first(t, "cheer")] }).collect::<Vec<_>>();
    assert!(ticks(&mut p, 1).iter().flatten().all(|&f| f));
    assert!(ticks(&mut p, 1).iter().flatten().all(|&f| !f));
    // a re-run that came out differently plays what's new
    p.hear(3);
    assert!(p.first(3, "bounce") && !p.first(3, ("hit", 3)) && !p.fresh);
    p.hear(9);
    assert!(p.fresh && p.first(9, "cheer"));
}

impl super::snapshot::Wire for Heard {}
