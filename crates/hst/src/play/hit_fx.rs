//! B148's guard: in a bot match every hit starts its effects (the impact, the sparks, the flight ribbon) and every
//! return past the serve puts the red landing marker down, so a regression can't silently drop them again.

use super::*;
use crate::{Args, GameSpace};

const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

#[test]
fn hits_start_their_effects() {
    if !std::path::Path::new(ISO).is_file() {
        return eprintln!("no ISO, skipped");
    }
    // SAFETY: only `Pads::slot_of` reads it; every player is a bot
    unsafe { std::env::set_var("HST_AUTOPLAY", "1") };
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
    app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![0, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
    app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
    app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
    app.add_systems(Startup, setup);
    app.add_systems(FixedUpdate, (remember, effects::tick, control, simulate, start_effects, motions, character::tick, played_out, held_ball).chain());
    app.update();
    let (mut hits, mut reds, mut tagged) = (0, 0, false);
    for _ in 0..60 * 60 {
        let before = app.world().resource::<Game>().shots;
        app.world_mut().run_schedule(FixedUpdate);
        let w = app.world();
        let g = w.resource::<Game>();
        tagged |= markers::shown(g);
        if g.shots > before {
            hits += 1;
            assert!(w.resource::<effects::Impacts>().playing(), "hit {hits}: no impact");
            assert!(w.resource::<effects::HitSparks>().live(), "hit {hits}: no sparks");
            assert!(w.resource::<effects::BallFlight>().live(), "hit {hits}: no ribbon");
            if g.shots > 1 {
                assert!(w.resource::<effects::LandingMarks>().red_at.is_some(), "hit {hits}: no red landing marker");
                reds += 1;
            }
        }
    }
    assert!(hits > 5 && reds > 0, "{hits} hits, {reds} red markers");
    assert!(tagged, "the player markers never showed");
}

/// A mode builds without the render app (`mode.rs`), so a material plugin in `play::plugin` draws nothing in a
/// match started from the menu: B137's `Fx` there hid every effect but in `--play`. The match must leave its
/// materials to the base app.
#[test]
fn the_match_registers_no_materials() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.add_plugins(plugin);
    assert!(!app.world().contains_resource::<Assets<effects::Fx>>(), "play::plugin added the effects' material plugin");
}
