//! The match's input, one `[Pad; 4]` per 60 Hz tick: seats 1P..4P's pads in the wire format (`hst_sim::replay::Pad`,
//! as the original's decoded pad) from an input source, and `control` reads nothing else. Sources: the local devices
//! (`read_input`'s latches in `Pads`), a replay (`Replay`, e.g. a PCSX2 recording's pads), later the network (M2g):
//! each pushes the tick's pads with `TickPads::push` before `control`. Who is human (`Pads::seats`) is fixed at the
//! match's first tick.
//!
//! Buttons are held bits; a shot or serve press is a bit that wasn't set the tick before. The face buttons are the
//! game's (✕ 0x2000 topspin, ○ 0x4000 slice, △ 0x1000 lob); the keyboard's serve-only key rides on □ (0x8000).
//! Sticks are the bytes after the pad driver's deadzone, as `pad_stick` leaves them, so a local pad round-trips
//! exactly. The right stick (ours: the original reads none) carries only the emote flick (`super::emote`), one axis
//! at its end, the other centred; a recording's right stick is dropped (`recorded`).

use super::{Pads, SlotPad, stick_byte};
use bevy::prelude::*;
use hst_sim::replay::Pad;
use std::collections::VecDeque;

const TRIANGLE: u16 = 0x1000;
const CROSS: u16 = 0x2000;
const CIRCLE: u16 = 0x4000;
const SQUARE: u16 = 0x8000;
/// Up, right, down, left (`loco::pad_dir`).
const DPAD: u16 = 0xf0;
const SHOTS: [(u16, i32); 3] = [(CROSS, 0), (CIRCLE, 1), (TRIANGLE, 3)];
pub const IDLE: Pad = Pad { buttons: 0, rx: 0x80, ry: 0x80, lx: 0x80, ly: 0x80 };

/// This tick's pads and the last tick's (for the presses).
#[derive(Resource, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TickPads {
    now: [Pad; 4],
    last: [Pad; 4],
}

impl Default for TickPads {
    fn default() -> Self {
        TickPads { now: [IDLE; 4], last: [IDLE; 4] }
    }
}

impl TickPads {
    /// The next tick's pads, seats 1P..4P.
    pub fn push(&mut self, pads: [Pad; 4]) {
        self.last = std::mem::replace(&mut self.now, pads);
    }

    /// This tick's pads, seats 1P..4P.
    pub fn now(&self) -> [Pad; 4] {
        self.now
    }

    /// Seat `s`'s pad this tick, as `control` reads it (the shot and serve presses in `shot`/`serve`).
    pub(super) fn slot(&self, s: usize) -> SlotPad {
        decode(self.now[s], self.last[s])
    }
}

impl TickPads {
    /// The buttons any seat pressed this tick (bits not held the tick before).
    pub(super) fn pressed(&self) -> u16 {
        self.now.iter().zip(&self.last).fold(0, |b, (n, l)| b | (n.buttons & !l.buttons))
    }
}

/// Pads to play back, one entry per tick, in place of the local devices; idle once it runs out.
#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct Replay(pub VecDeque<[Pad; 4]>);

/// The emote flicks' right stick bytes (rx, ry): up, down, left, right.
const FLICKS: [(u8, u8); 4] = [(0x80, 0), (0x80, 0xff), (0, 0x80), (0xff, 0x80)];

/// A PCSX2 recording's pad as match input: its right stick (which the original ignores) centred, so it never emotes.
pub fn recorded(p: Pad) -> Pad {
    Pad { rx: 0x80, ry: 0x80, ..p }
}

fn encode(s: &SlotPad) -> Pad {
    let shot = SHOTS.iter().find(|(_, k)| s.shot == Some(*k)).map_or(0, |(b, _)| *b);
    let serve = if s.serve { SQUARE } else { 0 };
    let (rx, ry) = s.emote.map_or((0x80, 0x80), |d| FLICKS[d as usize]);
    Pad { buttons: s.dpad | shot | serve, lx: stick_byte(s.stick.x), ly: stick_byte(-s.stick.y), rx, ry }
}

pub(super) fn decode(p: Pad, last: Pad) -> SlotPad {
    let axis = |b: u8| {
        let b = b as f32 - 128.0;
        b / if b < 0.0 { 128.0 } else { 127.0 }
    };
    let new = p.buttons & !last.buttons;
    let flick = |p: Pad| FLICKS.iter().position(|&f| f == (p.rx, p.ry)).map(|d| d as u8);
    SlotPad {
        stick: Vec2::new(axis(p.lx), -axis(p.ly)),
        dpad: p.buttons & DPAD,
        shot: SHOTS.iter().find(|(b, _)| new & b != 0).map(|(_, k)| *k),
        serve: new & SQUARE != 0,
        emote: flick(p).filter(|_| flick(p) != flick(last)),
    }
}

/// The local devices' source: the latched presses go out once and are spent.
/// ponytail: a second press of the same button within one tick of the first is lost (held bits on the wire)
fn local(mut pads: ResMut<Pads>, mut tick: ResMut<TickPads>) {
    let now = pads.slots.each_mut().map(|s| {
        let p = encode(s);
        (s.shot, s.serve, s.emote) = (None, false, None);
        p
    });
    tick.push(now);
}

/// A local slot's pad for this tick, its latched presses spent (as `local`).
pub(super) fn take(s: &mut SlotPad) -> Pad {
    let p = encode(s);
    (s.shot, s.serve, s.emote) = (None, false, None);
    p
}

fn replayed(mut r: ResMut<Replay>, mut tick: ResMut<TickPads>) {
    tick.push(r.0.pop_front().unwrap_or([IDLE; 4]));
}

/// Who is human, fixed at the match's first tick (`--pads` fixes it at startup): 1P, and 2P..4P whose pads are in.
fn fix_seats(mut pads: ResMut<Pads>) {
    if pads.seats.is_none() {
        let c = pads.connected;
        pads.seats = Some(std::array::from_fn(|s| s == 0 || c > s));
    }
}

/// The input entry, before `control`.
pub(super) fn systems() -> impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, ()> {
    (fix_seats, local.run_if(not(resource_exists::<Replay>)), replayed.run_if(resource_exists::<Replay>)).chain()
}

pub fn plugin(app: &mut App) {
    app.init_resource::<TickPads>().add_systems(FixedUpdate, systems().before(super::control));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every local pad state goes through the wire and back unchanged: the stick bit for bit, as `pad_stick` made it.
    #[test]
    fn local_pads_round_trip() {
        let mut last = IDLE;
        for i in 0..=400 {
            let v = i as f32 / 200.0 - 1.0;
            for (dpad, shot, serve) in [(0, None, false), (0x10 | 0x20, Some(0), false), (0x80, Some(1), true), (0x40, Some(3), false)] {
                let s = SlotPad { stick: super::super::pad_stick(Vec2::new(v, -v * 0.7)), dpad, shot, serve, emote: None };
                let p = encode(&s);
                let d = decode(p, IDLE);
                assert_eq!((d.stick.x.to_bits(), d.stick.y.to_bits(), d.dpad, d.shot, d.serve), (s.stick.x.to_bits(), s.stick.y.to_bits(), dpad, shot, serve), "{v}");
                // held from the tick before: no new press
                let held = decode(p, p);
                assert_eq!((held.shot, held.serve, held.dpad), (None, false, dpad));
                last = p;
            }
        }
        assert_eq!(decode(IDLE, last).shot, None);
    }
}

/// The replay harness through the app's match entry: a PCSX2 recording's 1P pads (`context/fixtures/round1.bin`)
/// drive the app's singles match as `Replay` (needs the ISO at the repository root and the fixture; skipped without).
#[cfg(test)]
mod replay_tests {
    use super::super::*;
    use super::{IDLE, Replay};
    use crate::{Args, GameSpace};
    use std::path::Path;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
    const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures/round1.bin");

    /// Player 0's position every tick with these pads on 1P (2P a bot).
    fn run(pads: &[[hst_sim::replay::Pad; 4]]) -> Vec<V3> {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { seats: Some([true, false, false, false]), autoplay: false, ..default() });
        app.insert_resource(Replay(pads.iter().copied().collect()));
        app.init_resource::<super::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, super::systems(), control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        (0..pads.len())
            .map(|_| {
                app.world_mut().run_schedule(FixedUpdate);
                app.world().resource::<Game>().players[0].pos
            })
            .collect()
    }

    #[test]
    fn a_recording_drives_the_match() {
        let (Ok(data), true) = (std::fs::read(FIXTURE), Path::new(ISO).is_file()) else {
            return eprintln!("no ISO or round1.bin, skipped");
        };
        let pads: Vec<_> = hst_sim::replay::frames(&data).iter().take(1200).map(|f| [super::recorded(f.pad(0)), IDLE, IDLE, IDLE]).collect();
        let played = run(&pads);
        let idle = run(&vec![[IDLE; 4]; pads.len()]);
        assert_eq!(played, run(&pads), "the same pads gave a different match");
        let moved = played.windows(2).filter(|w| w[0] != w[1]).count();
        assert!(played != idle && moved > 0, "the recording's pads didn't reach 1P ({moved} ticks moved)");
        eprintln!("1P moved on {moved} of {} ticks", pads.len());
    }
}
