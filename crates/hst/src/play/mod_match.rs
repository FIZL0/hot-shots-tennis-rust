//! A standard mod (`crate::mods`) in a match slot: the donor's motions, trajectory tables, shot records, serve
//! and timing tables (the setup loads them by the donor's number), with the mod's own TParam.csv row (stats, aim,
//! reach and heights, smash window, power, timing stats, collision size), `ai_row` for the computer's
//! AIParam.csv row and second-toss pick, `hand`, and its voice bank (the donor's when it ships no wavs).

use std::sync::Arc;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use hst_data::iso::Iso;

use super::Game;
use crate::audio::SoundBank;
use crate::character::CharacterData;
use crate::mods::{self, MatchMod, Mod};

/// The mod's costume `outfit` (wrapping round its costume list).
pub(super) fn load(
    iso: &mut Iso,
    m: &Mod,
    outfit: usize,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
) -> Arc<CharacterData> {
    Arc::new(mods::load(iso, m, outfit % m.costumes.len(), meshes, materials, images, bindposes).unwrap_or_else(|e| panic!("mod {}: {e}", m.id)))
}

/// Player `i`'s TParam.csv row: the mod's in its slot, else character `c`'s.
pub(super) fn row(iso: &mut Iso, mm: Option<&MatchMod>, i: usize, c: usize) -> Vec<String> {
    match mm.filter(|m| m.slot == i) {
        Some(m) => mods::tparam(iso, &m.m).unwrap_or_else(|e| panic!("mod {}: {e}", m.m.id)),
        None => super::tparam(iso, c),
    }
}

/// Player `i`, set up as the mod's donor, takes the mod's row, AI row, hand and voices.
pub(super) fn apply(g: &mut Game, i: usize, iso: &mut Iso, m: &Mod, outfit: usize, voices: &mut [Option<Arc<SoundBank>>]) {
    let row = mods::tparam(iso, m).unwrap_or_else(|e| panic!("mod {}: {e}", m.id));
    let cells = |k: usize| -> Vec<f32> { row[k].split('/').map(|v| v.trim().parse().expect("TParam number")).collect() };
    let doubles = g.players.len() == 4;
    let p = &mut g.players[i];
    p.hand = m.hand;
    p.stats = super::stats_of(&row);
    p.body.stamina = p.stats.stamina;
    // ponytail: the AI row's costume level is the slot's `--outfits` number, as a disc character's
    p.ai = super::ai_params(iso, m.ai_row, outfit, doubles);
    p.ai_second = super::ai_second_toss(iso, m.ai_row);
    g.aim_stats[i] = super::aim_of(&row);
    g.finish.power[i] = [13, 14].map(|k| row[k].parse().expect("TParam power"));
    let smash = cells(64);
    g.smash_heights[i] = [smash[0] / 100.0, smash[1] / 100.0];
    g.reaches[i] = super::reach_of(&g.reach, &row, m.hand);
    g.timing[i].restat(&row);
    if let Some(bank) = mods::voice(m) {
        voices[i] = Some(Arc::new(bank));
    }
}

/// A bot singles match with a rerigged mod (local `context/mods/test_pc00`, see the M1a journal) on court: the mod's
/// player gets its row, hand and AI and plays a rally (needs the ISO at the repository root; skipped without it).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Args, GameSpace, character};
    use std::path::Path;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
    const MOD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/mods/test_pc00");

    #[test]
    fn bot_match_with_a_mod_plays_a_rally() {
        if !Path::new(ISO).is_file() || !Path::new(MOD).is_dir() {
            return eprintln!("no ISO or context/mods/test_pc00, skipped");
        }
        // SAFETY: only `Pads::slot_of` reads it; every player is a bot
        unsafe { std::env::set_var("HST_AUTOPLAY", "1") };
        let m = mods::read(Path::new(MOD)).unwrap();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false });
        app.insert_resource(MatchMod { slot: 1, m: m.clone() });
        app.init_resource::<super::super::Pads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, super::super::setup);
        use super::super::*;
        app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        let mut iso = Iso::open(ISO).unwrap();
        let row = mods::tparam(&mut iso, &m).unwrap();
        {
            let g = app.world().resource::<Game>();
            let p = &g.players[1];
            assert_eq!((g.chars[1], p.hand, p.stats.speed), (0, -1.0, stats_of(&row).speed));
            assert_eq!(g.reaches[1].reach, loco::ReachStats::from_tparam(&row.join(",")).reach);
            assert_ne!(g.reaches[1].reach, character_reach(&g.reach, &mut iso, 0, -1.0).reach, "the override didn't reach the contact search");
            assert_eq!(g.data[1].joints.len(), mods::load(&mut iso, &m, 0, &mut default(), &mut default(), &mut default(), &mut default()).unwrap().joints.len());
        }
        // a rally: three shots or more in one point, the mod striking one of them
        let (mut best, mut mod_hit) = (0, false);
        for frame in 0..60 * 120 {
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            best = best.max(g.shots);
            mod_hit |= g.last_hitter == 1 && g.shots >= 2;
            if best >= 3 && mod_hit {
                eprintln!("frame {frame}: a {best}-shot rally, the mod hit");
                break;
            }
        }
        assert!(best >= 3 && mod_hit, "longest rally {best} shots, mod returned a ball: {mod_hit}");
    }
}
