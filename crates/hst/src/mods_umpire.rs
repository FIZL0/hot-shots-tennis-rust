//! Umpire mods (M8f3; the mods repo's `standard/HST-COURT-UMPIRE-STANDARD.md` §3): a `mod.json` of `kind`
//! `umpire` names a disc `donor` (0..4, the `--umpire` order) and ships `voice/<program>_<key>.wav` calls. The match
//! plays as with the donor in the chair (`--umpire <donor>`), her word lengths timing every call, so replays,
//! rollback and online stay bit-exact whatever is installed; only what is heard changes: the mod's clip in place of
//! the donor's take, the donor's for a call the mod doesn't ship. The umpire select lists them after the disc five
//! (`play/main_menu/umpire_mods.rs`); `--umpire-mod DIR` plays one; online the room carries its id and a peer
//! without it hears the donor (`net::Room::umpire_mod`).

use std::path::{Path, PathBuf};

use bevy::prelude::*;

use crate::audio::SoundBank;
use crate::mods::Kind;

/// The disc umpires (`--umpire` 0..=4).
pub const DISC: u8 = 5;

/// An umpire mod's manifest, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Umpire {
    pub dir: PathBuf,
    pub id: String,
    pub name: String,
    /// The game it comes from (as `mods::Mod::source`); empty when unset.
    pub source: String,
    /// The disc umpire (0..=4) whose timing, fallback calls and model it uses.
    pub donor: u8,
    /// The voice folder, relative to `dir`.
    pub voice: String,
}

/// The umpire mod playing in this match (`--umpire-mod DIR`, or the online room's id found under `mods/`).
#[derive(Resource, Clone, Debug)]
pub struct MatchUmpire(pub Umpire);

/// Reads and checks `dir/mod.json` as an umpire mod.
pub fn read(dir: &Path) -> Result<Umpire, String> {
    let at = dir.join("mod.json");
    let err = |m: String| format!("{}: {m}", at.display());
    let text = crate::modfs::read_to_string(&at).map_err(|e| err(e.to_string()))?;
    let j: serde_json::Value = serde_json::from_str(&text).map_err(|e| err(format!("not JSON: {e}")))?;
    if Kind::of(&j) != Kind::Umpire {
        return Err(err(format!("`kind` is {}: not an umpire mod", Kind::of(&j))));
    }
    if j["standard"].as_u64() != Some(1) {
        return Err(err(format!("`standard` is {}, this game reads standard 1", j["standard"])));
    }
    // shown as plain text, length-capped (M16)
    let text = |k: &str| j[k].as_str().filter(|t| (1..=64).contains(&t.chars().count())).map(str::to_string).ok_or_else(|| err(format!("`{k}` must be a string of 1 to 64 characters")));
    let donor = j["donor"].as_u64().filter(|&n| n < DISC as u64).ok_or_else(|| err(format!("`donor` must be a number 0..={}", DISC - 1)))? as u8;
    let voice = j["voice"].as_str().unwrap_or("voice/").to_string();
    if !crate::modfs::plain(voice.strip_suffix('/').unwrap_or(&voice)) {
        return Err(err(format!("`voice` `{voice}` must be a path inside the mod folder")));
    }
    Ok(Umpire {
        dir: dir.to_path_buf(),
        id: text("id")?,
        name: text("name")?,
        source: j["source"].as_str().unwrap_or("").split(" (").next().unwrap_or("").trim().chars().take(64).collect(),
        donor,
        voice,
    })
}

/// Every umpire mod under `root` (`mods::dirs`' order); a broken one is left out with its reason logged.
pub fn list(root: &Path) -> Vec<Umpire> {
    crate::mods::dirs(root).iter().filter(|d| Kind::of_dir(d) == Kind::Umpire).filter_map(|d| read(d).map_err(|e| warn!("{e}")).ok()).collect()
}

/// The installed umpire mod with `id` (the first in `list` order), if any.
pub fn find(root: &Path, id: &str) -> Option<Umpire> {
    list(root).into_iter().find(|u| u.id == id)
}

/// The match's slot 5 bank for umpire mod `u`: its donor's disc bank `disc` with the mod's calls over it
/// (`SoundBank::takes`: heard in place of the disc's, the disc's lengths still timing the match).
pub fn bank(disc: &SoundBank, u: &Umpire) -> SoundBank {
    let (bank, n) = SoundBank { hd: disc.hd.clone(), bd: disc.bd.clone(), wav: Default::default(), takes: Default::default(), stream: None }.with_takes(&u.dir.join(&u.voice));
    info!("umpire mod {}: {n} calls of its own over umpire {}'s", u.id, u.donor);
    bank
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hst-umpire-mod-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// An umpire mod reads (donor, name, voice folder); a broken one (donor out of range, a voice path leaving the
    /// folder, a character mod) is refused with the reason; `list` keeps only umpire mods, packs one level down,
    /// and `find` goes by id.
    #[test]
    fn reads_and_lists_umpire_mods() {
        let root = dir("list");
        let put = |sub: &str, json: &str| {
            let d = root.join(sub);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("mod.json"), json).unwrap();
            d
        };
        let ok = put("gag_ump00", r#"{"standard":1,"kind":"umpire","id":"gag_ump00","name":"Umpire 00","source":"Hot Shots Tennis: Get a Grip (PSP) gag_vc00","donor":4,"voice":"voice/"}"#);
        put("pack/gag_ump01", r#"{"standard":1,"kind":"umpire","id":"gag_ump01","name":"Umpire 01","donor":0}"#);
        put("bad_donor", r#"{"standard":1,"kind":"umpire","id":"x","name":"X","donor":5}"#);
        put("bad_voice", r#"{"standard":1,"kind":"umpire","id":"y","name":"Y","donor":1,"voice":"../up/"}"#);
        put("court", r#"{"standard":1,"kind":"court","id":"c","name":"C","layout":3}"#);
        put("player", r#"{"standard":1,"id":"p","name":"P"}"#);
        let u = read(&ok).unwrap();
        assert_eq!((u.id.as_str(), u.name.as_str(), u.source.as_str(), u.donor, u.voice.as_str()), ("gag_ump00", "Umpire 00", "Hot Shots Tennis: Get a Grip", 4, "voice/"));
        assert!(read(&root.join("bad_donor")).unwrap_err().contains("`donor`"));
        assert!(read(&root.join("bad_voice")).unwrap_err().contains("`voice`"));
        assert!(read(&root.join("player")).unwrap_err().contains("not an umpire mod"));
        let ids: Vec<_> = list(&root).into_iter().map(|u| u.id).collect();
        assert_eq!(ids, ["gag_ump00", "gag_ump01"]);
        assert_eq!(find(&root, "gag_ump01").map(|u| u.donor), Some(0));
        assert_eq!(find(&root, "nope"), None);
        // and the character roster doesn't take them
        assert!(crate::mods::list(&root).is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    /// The match's bank: a key the mod ships plays its clip (a wav voice) where the donor's disc take was; a key it
    /// lacks plays the donor's take; either way `centre_frames` (the match's timing) is the donor's (skipped without
    /// the ISO).
    #[test]
    fn calls_over_the_donor_timed_by_the_donor() {
        let Ok(mut iso) = hst_data::iso::Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else {
            return eprintln!("no ISO, skipped");
        };
        let root = dir("bank");
        std::fs::create_dir_all(root.join("voice")).unwrap();
        std::fs::write(root.join("mod.json"), r#"{"standard":1,"kind":"umpire","id":"b","name":"B","donor":2}"#).unwrap();
        let mut wav = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        wav.extend([16u32.to_le_bytes(), [1, 0, 1, 0], 22050u32.to_le_bytes(), (22050u32 * 2).to_le_bytes(), [2, 0, 16, 0]].concat());
        wav.extend(b"data");
        wav.extend((22050u32 * 4).to_le_bytes());
        wav.extend((0..22050 * 2).flat_map(|i: i32| (((i % 50) - 25) as i16 * 400).to_le_bytes()));
        for f in ["1_1.wav", "0_13.wav"] {
            std::fs::write(root.join("voice").join(f), &wav).unwrap();
        }
        let u = read(&root).unwrap();
        let disc = crate::audio::umpire_bank(&mut iso, u.donor, 0).unwrap();
        let modded = std::sync::Arc::new(bank(&disc, &u));
        let disc = std::sync::Arc::new(disc);
        std::fs::remove_dir_all(&root).ok();
        assert_eq!(modded.takes.len(), 2);
        let sound = crate::audio::Sound::test(&mut iso);
        let call = |program, key| hst_sim::sound::Play { slot: 5, program, key, volume: 0x80, speed: 1.0 };
        for (p, k) in [(1, 1), (0, 9), (0, 13), (2, 4)] {
            assert_eq!(sound.centre_frames(&modded, call(p, k)), sound.centre_frames(&disc, call(p, k)), "{p}_{k} timed by the donor");
        }
        assert!(sound.centre_frames(&disc, call(1, 1)) > 0);
        // "fault" is the mod's clip; "deuce" the donor's take
        sound.play_centre(&modded, call(1, 1));
        assert_eq!(sound.takes_sounding(), 1);
        assert_ne!(sound.play_centre(&modded, call(0, 9)), 0);
        assert_eq!(sound.takes_sounding(), 1);
    }
}
