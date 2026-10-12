//! A standard mod (`crate::mods`) in a match slot: the donor's motions, trajectory tables, shot records, serve
//! and timing tables (the setup loads them by the donor's number), with the mod's own TParam.csv row (stats, aim,
//! reach and heights, smash window, power, timing stats, collision size), `ai_row` for the computer's
//! AIParam.csv row and second-toss pick, `hand`, and its voice bank (the donor's for every program it ships no wav for).

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
    match mm.and_then(|m| m.get(i)) {
        Some(m) => mods::tparam(iso, m).unwrap_or_else(|e| panic!("mod {}: {e}", m.id)),
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
    if let Some(mut bank) = mods::voice(m) {
        // programs the mod has no wav for play the donor's disc takes (standard §5)
        if let Some(donor) = &voices[i] {
            (bank.hd, bank.bd) = (donor.hd.clone(), donor.bd.clone());
        }
        voices[i] = Some(Arc::new(bank));
    }
}

/// A bot singles match with a rerigged mod (local `context/mods/test_pc00`, see the M1a journal) on court: the mod's
/// player gets its row, hand and AI and plays a rally (needs the ISO at the repository root; skipped without it).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Args, GameSpace};
    use std::path::Path;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");
    const MOD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/mods/test_pc00");

    #[test]
    fn bot_match_with_a_mod_plays_a_rally() {
        if !Path::new(ISO).is_file() || !Path::new(MOD).is_dir() {
            return eprintln!("no ISO or context/mods/test_pc00, skipped");
        }
        let m = mods::read(Path::new(MOD)).unwrap();
        let mut app = bot_match(&m, 1);
        use super::super::*;
        let mut iso = Iso::open(ISO).unwrap();
        let row = mods::tparam(&mut iso, &m).unwrap();
        {
            let g = app.world().resource::<Game>();
            let p = &g.players[1];
            assert_eq!((g.chars[1], p.hand, p.stats.speed), (0, -1.0, stats_of(&row).speed));
            assert_eq!(g.reaches[1].reach, loco::ReachStats::from_tparam(&row.join(",")).reach);
            assert_ne!(g.reaches[1].reach, character_reach(&g.reach, &mut iso, 0, -1.0).reach, "the override didn't reach the contact search");
            assert_eq!(g.data[1].joints.len(), mods::load(&mut iso, &m, 0, &mut Assets::<Mesh>::default(), &mut Assets::<StandardMaterial>::default(), &mut Assets::<Image>::default(), &mut Assets::<SkinnedMeshInverseBindposes>::default()).unwrap().joints.len());
        }
        rally(&mut app, &m);
    }

    /// A bot singles match, Carol against mod `m` (set up as disc character `char`), after its first update.
    fn bot_match(m: &Mod, char: usize) -> App {
        use super::super::*;
        // SAFETY: only `Pads::slot_of` reads it; every player is a bot
        unsafe { std::env::set_var("HST_AUTOPLAY", "1") };
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, char], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(MatchMod(vec![None, Some(m.clone())]));
        app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
        app.update();
        app
    }

    /// Plays until a rally of three shots or more in one point with the mod (player 1) striking one of them.
    fn rally(app: &mut App, m: &Mod) {
        let (mut best, mut mod_hit) = (0, false);
        for frame in 0..60 * 120 {
            app.world_mut().run_schedule(FixedUpdate);
            let g = app.world().resource::<Game>();
            best = best.max(g.shots);
            mod_hit |= g.last_hitter == 1 && g.shots >= 2;
            if best >= 3 && mod_hit {
                eprintln!("{}: frame {frame}, a {best}-shot rally, the mod hit", m.id);
                return;
            }
        }
        panic!("{}: longest rally {best} shots, mod returned a ball: {mod_hit}", m.id);
    }

    /// M1g: the five chair umpires, packaged by the mods repo (`tools/rerig/hst_umpires.py` -> `mods/hst-mods`), are
    /// in the custom roster (`mods::list` of `mods/`, readable TParam row, as the select builds it), load as a model
    /// and each plays a rally (skipped without the ISO or the umpire mods).
    #[test]
    fn umpires_are_on_the_roster_and_play_a_rally() {
        let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods"));
        let ump: Vec<Mod> = mods::list(root).into_iter().filter(|m| m.id.starts_with("hst_umpire_")).collect();
        if !Path::new(ISO).is_file() || ump.is_empty() {
            return eprintln!("no ISO or umpire mods under mods/, skipped");
        }
        let ids: Vec<&str> = ump.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["hst_umpire_00_chika", "hst_umpire_01_suzuki", "hst_umpire_02_lily", "hst_umpire_03_robottennis", "hst_umpire_04_anna"]);
        let mut iso = Iso::open(ISO).unwrap();
        for m in &ump {
            mods::tparam(&mut iso, m).unwrap_or_else(|e| panic!("{}: no TParam row, left off the select: {e}", m.id));
            let data = mods::load(&mut iso, m, 0, &mut Assets::<Mesh>::default(), &mut Assets::<StandardMaterial>::default(), &mut Assets::<Image>::default(), &mut Assets::<SkinnedMeshInverseBindposes>::default());
            assert!(data.is_ok_and(|d| !d.joints.is_empty()), "{}: model doesn't load", m.id);
            rally(&mut bot_match(m, m.donor), m);
        }
    }

    /// A packaged Get a Grip mod (`mods/hst-mods`, the mods repo's `out/mods`) in a match: the stats the game reads
    /// are its `params.override` cells (Get a Grip's stats ranked onto TParam), its AI row is `ai_row`.
    #[test]
    fn get_a_grip_stats_reach_the_match() {
        const GAG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods/getagrip_pc00_emi");
        if !Path::new(ISO).is_file() || !Path::new(GAG).is_dir() {
            return eprintln!("no ISO or mods/hst-mods/getagrip_pc00_emi, skipped");
        }
        let m = mods::read(Path::new(GAG)).unwrap();
        let o = |k: &str| m.overrides.iter().find(|(c, _)| c == k).unwrap_or_else(|| panic!("no {k} override")).1.parse::<i32>().unwrap();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![1, m.donor], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(MatchMod(vec![None, Some(m.clone())]));
        app.init_resource::<super::super::Pads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_systems(Startup, super::super::setup);
        app.update();
        let mut iso = Iso::open(ISO).unwrap();
        let g = app.world().resource::<super::super::Game>();
        let p = &g.players[1];
        eprintln!("{}: speed {} agility {} stamina {}, power {:?}, aim {:?}, reach {}", m.id, p.stats.speed, p.stats.agility, p.stats.stamina, g.finish.power[1], g.aim_stats[1].con, g.reaches[1].reach);
        assert_eq!((p.stats.speed, p.stats.agility, p.stats.stamina), (o("SPE") as f32 / 10.0, o("Agili"), o("STA")));
        assert_eq!(g.finish.power[1], [o("Strk POW"), o("Voley POW")]);
        assert_eq!(g.aim_stats[1].con, [o("Strk CON"), o("Voley CON"), o("Serv CON")]);
        assert_eq!(g.reaches[1].reach, hst_sim::ps2::add(hst_sim::ps2::div(o("リーチ(cm)") as f32, 100.0), hst_sim::ps2::div(30.0, 100.0)));
        assert_eq!(p.ai, super::super::ai_params(&mut iso, m.ai_row, 0, false));
        assert_ne!(m.ai_row, m.donor, "Emi's AI row should follow her play style, not her body donor");
    }
}
