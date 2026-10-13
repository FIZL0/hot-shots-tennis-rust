//! The instant replay's pacing: it skips (runs unseen and unheard) to just before the point's highlighted shot, the
//! second-to-last, then slows the match down around it, each tick followed by in-between frames the draws
//! interpolate (`Time<Fixed>::overstep_fraction`), and plays the rest at full speed. Only the clock changes: the
//! ticks the replay runs are the same, so the match and its generators are where they would be without it.
//!
//! The game's director (seen on slot 5's match point, research/journal/2026-10-12-p0b4d-instant-replay/4-PACE-FINAL)
//! ramps its slow-motion block across ticks: from ×a to ×b over n ticks, tick k shows
//! int(lo + (hi − lo)·(k/n)^curve) in-between frames, lo/hi = a − 1, b − 1 plus 0.999999 on the upper end (both
//! ends when slowing down).

use super::Game;
use bevy::prelude::*;
use std::time::Duration;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Pace>()
        .add_systems(FixedUpdate, (log.after(super::simulate), hush.before(super::play_sounds), pace.after(super::instant_replay::run)));
}

#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Pace {
    /// The point's shots: the tick each left the racket.
    shots: Vec<i32>,
    /// The last point's.
    last: Vec<i32>,
    /// A replay is paced: its shots.
    replay: Option<Vec<i32>>,
}

/// The highlighted shot's slow-motion ramps, from its tick: (first tick, from ×, to ×, ticks, curve). The lead-in
/// is the director's default for a shot just ahead (its 62-frame budget over the 2 ticks left, eased by t²), the
/// shot's own the 2 → 6 → 1 pair; the cut 15 ticks after the shot ends it.
// ponytail: the director picks a row per recorded shot's type (9 rows in the game program); this is the row the
// one recorded match point used (a rally shot), so smashes and specials pace like it — port the row pick to vary it
const RAMPS: [(i32, i32, i32, i32, f32); 3] = [(-2, 31, 1, 2, 2.0), (0, 2, 6, 3, 1.0), (3, 6, 1, 13, 1.0)];
const CUT: i32 = 15;
/// The picture comes back this many ticks before the highlighted shot.
const SHOWN: i32 = 3;

/// The tick the replay's picture comes back at: just ahead of the highlighted shot, never before the one ahead of it.
fn shown(shots: &[i32]) -> i32 {
    match shots {
        [.., before, shot, _] => (shot - SHOWN).max(before + 1),
        [shot, _] => shot - SHOWN,
        _ => 0,
    }
}

/// The in-between frames after replay tick `tick` (0 none).
fn between(shots: &[i32], tick: i32) -> u32 {
    let Some(&shot) = shots.len().checked_sub(2).and_then(|i| shots.get(i)) else { return 0 };
    let k = tick - shot;
    if k >= CUT {
        return 0;
    }
    let Some(&(at, a, b, n, curve)) = RAMPS.iter().rev().find(|r| k >= r.0) else { return 0 };
    if k - at >= n {
        return 0;
    }
    let (a, b) = ((a - 1) as f32, (b - 1) as f32);
    let (lo, hi) = if a < b { (a, b + 0.999999) } else { (a + 0.999999, b + 0.999999) };
    (lo + (hi - lo) * hst_sim::musl::powf((k - at) as f32 / n as f32, curve)) as u32
}

/// Each shot leaving the racket (the rally's count going up), outside a replay; the count's reset files the point.
fn log(g: Res<Game>, mut p: ResMut<Pace>) {
    if g.instant.replaying {
        return;
    }
    if (g.shots as usize) < p.shots.len() {
        p.last = std::mem::take(&mut p.shots);
    }
    if g.shots as usize > p.shots.len() {
        p.shots.push(g.instant.ticks);
    }
}

/// While the replay skips, nothing is heard.
// ponytail: drops the tick's queued sounds, crowd and umpire words; the ball's whistle stops on its own
fn hush(mut g: ResMut<Game>, p: Res<Pace>) {
    if p.replay.as_ref().is_some_and(|s| g.instant.replaying && g.instant.ticks < shown(s)) {
        g.sounds.clear();
        g.cheers.clear();
        g.umpire.voice = None;
    }
}

/// End of each tick: the next tick's clock. Skipping, the fixed loop runs on within the frame; slowed, the next tick
/// comes after its in-between frames.
fn pace(g: Res<Game>, mut p: ResMut<Pace>, mut virt: ResMut<Time<Virtual>>, mut fixed: ResMut<Time<Fixed>>) {
    let p = &mut *p;
    if !g.instant.replaying {
        if p.replay.take().is_some() {
            virt.set_relative_speed(1.0);
            virt.set_max_delta(Duration::from_millis(250));
        }
        return;
    }
    let shots = p.replay.get_or_insert_with(|| p.last.clone());
    let tick = g.instant.ticks;
    if tick + 1 < shown(shots) {
        // ponytail: a frame's worth of real time runs a thousand ticks; the skip is a few hundred
        virt.set_max_delta(Duration::from_secs(1));
        virt.set_relative_speed(1000.0);
        return;
    }
    if tick + 1 == shown(shots) {
        // the skip's last tick: the rest of the frame's time is thrown away, the picture shows from here
        virt.set_max_delta(Duration::from_millis(250));
        let over = fixed.overstep();
        fixed.discard_overstep(over);
    }
    virt.set_relative_speed(1.0 / (between(shots, tick) + 1) as f32);
}

/// The skip, then the slow motion observed on slot 5's match point (the original's ticks there: shown from 313,
/// shot 316; frames per tick from its slow-motion block).
#[test]
fn the_match_point_paces_as_the_original() {
    let shots = [196, 293, 316, 360];
    assert_eq!(shown(&shots), 313);
    let frames: Vec<u32> = (313..=332).map(|t| between(&shots, t) + 1).collect();
    assert_eq!(frames, [1, 31, 24, 2, 3, 5, 6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 3, 2, 1, 1]);
}

/// A bot match through the app's frame loop, one frame a fixed tick: the match point's replay runs its unseen ticks
/// within a frame or two, then each tick takes `between` + 1 frames (±1 for the clock's rounding). Needs the ISO.
#[test]
fn a_replay_skips_then_slows_in_frames() {
    use super::instant_replay::tests::{ISO, app};
    if !std::path::Path::new(ISO).is_file() {
        return eprintln!("no ISO, skipped");
    }
    let mut app = app(false);
    app.add_plugins(plugin);
    app.insert_resource(Time::<Fixed>::from_hz(60.0));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)));
    let mut frames: Vec<(i32, u32)> = vec![];
    for _ in 0..60 * 60 * 40 {
        app.update();
        let g = app.world().resource::<Game>();
        if !g.instant.replaying {
            // an ace's replay has no skip to pace: wait for a rally's
            if !frames.is_empty() && app.world().resource::<Pace>().last.len() >= 2 {
                break;
            }
            frames.clear();
            continue;
        }
        match frames.last_mut() {
            Some((t, n)) if *t == g.instant.ticks => *n += 1,
            _ => frames.push((g.instant.ticks, 1)),
        }
    }
    let shots = app.world().resource::<Pace>().last.clone();
    eprintln!("shots {shots:?}, frames {frames:?}");
    let from = shown(&shots);
    assert!(shots.len() >= 2 && from > 20, "no skip to pace");
    // the skip: every hidden tick in the first two frames
    let at = frames.iter().position(|f| f.0 >= from - 1).expect("never shown");
    assert!(frames[..at].len() < 3 && frames[at].0 == from - 1, "skip {:?}", &frames[..=at]);
    for &(t, n) in &frames[at + 1..frames.len() - 1] {
        let want = between(&shots, t) + 1;
        assert!(n.abs_diff(want) <= 1, "tick {t}: {n} frames, want {want}");
    }
    assert!(frames.iter().map(|f| f.1).sum::<u32>() > 100, "no slow motion");
}

impl super::snapshot::Wire for Pace {}
