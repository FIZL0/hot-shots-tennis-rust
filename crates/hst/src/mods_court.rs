//! Court mods (M8f6; the mods repo's `standard/HST-COURT-UMPIRE-STANDARD.md` §2): a `mod.json` of `kind` `court`
//! names a disc `layout` (0..=10, the court select's order: `--stage layout + 1`) and ships `court.glb` (and
//! optionally `sky.glb`, `gimmick.glb`, `sound/bgm.wav`, `sound/ambience_<n>.wav`). The match plays on the layout
//! court: its collision, lines, net, judging, umpire chair, serve placement, cameras and court generator are the
//! disc's, so replays, rollback and online stay bit-exact whatever is installed; only what is seen and heard
//! changes (`court_mod`). The court select lists them after the disc eleven (`play/main_menu/court_mods.rs`);
//! `--court-mod DIR` plays one.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

use crate::mods::Kind;

/// The disc courts (`layout` 0..=10).
pub const DISC: u8 = 11;

/// The manifest's `light` (game space, Y down; colours linear 0..1).
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    /// Toward the sun.
    pub sun: Vec3,
    pub sun_colour: Vec3,
    pub ambient: Vec3,
    /// A second light (toward it) and its colour; zero when absent.
    pub fill: Vec3,
    pub fill_colour: Vec3,
    pub fog: Option<Fog>,
}

/// The manifest's `light.fog`: the colour, how much fog at `near` and at `far` m (linear between).
#[derive(Debug, Clone, PartialEq)]
pub struct Fog {
    pub colour: Vec3,
    pub near: f32,
    pub far: f32,
    pub amount: [f32; 2],
}

/// A looping sound file and its loop (`[start, end]` in samples, `[0, 0]` the whole file).
#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    pub path: PathBuf,
    pub at: [u64; 2],
}

/// A court mod's manifest, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Court {
    pub dir: PathBuf,
    pub id: String,
    pub name: String,
    /// The game it comes from (as `mods::Mod::source`); empty when unset.
    pub source: String,
    /// The disc court (0..=10) it plays as.
    pub layout: u8,
    /// `None`: the layout court's light.
    pub light: Option<Light>,
    pub sky: bool,
    pub gimmick: bool,
    pub bgm: Option<Loop>,
    pub ambience: Vec<Loop>,
    /// The sun's flare sprite (`lens_flare`); `None`: no flare.
    pub lens_flare: Option<PathBuf>,
}

/// The court mod drawn in this match (`--court-mod DIR`).
#[derive(Resource, Clone, Debug)]
pub struct MatchCourt(pub Court);

/// Reads and checks `dir/mod.json` as a court mod.
pub fn read(dir: &Path) -> Result<Court, String> {
    let at = dir.join("mod.json");
    let err = |m: String| format!("{}: {m}", at.display());
    let text = crate::modfs::read_to_string(&at).map_err(|e| err(e.to_string()))?;
    let j: serde_json::Value = serde_json::from_str(&text).map_err(|e| err(format!("not JSON: {e}")))?;
    if Kind::of(&j) != Kind::Court {
        return Err(err(format!("`kind` is {}: not a court mod", Kind::of(&j))));
    }
    if j["standard"].as_u64() != Some(1) {
        return Err(err(format!("`standard` is {}, this game reads standard 1", j["standard"])));
    }
    // shown as plain text, length-capped (M16)
    let text = |k: &str| j[k].as_str().filter(|t| (1..=64).contains(&t.chars().count())).map(str::to_string).ok_or_else(|| err(format!("`{k}` must be a string of 1 to 64 characters")));
    let layout = j["layout"].as_u64().filter(|&n| n < DISC as u64).ok_or_else(|| err(format!("`layout` must be a number 0..={}", DISC - 1)))? as u8;
    if !crate::modfs::is_file(&dir.join("court.glb")) {
        return Err(err("no `court.glb` beside it".into()));
    }
    let vec3 = |v: &serde_json::Value, k: &str| -> Result<Option<Vec3>, String> {
        match &v[k] {
            serde_json::Value::Null => Ok(None),
            a => a.as_array().filter(|a| a.len() == 3).and_then(|a| Some(Vec3::new(a[0].as_f64()? as f32, a[1].as_f64()? as f32, a[2].as_f64()? as f32))).filter(|v| v.is_finite()).map(Some).ok_or_else(|| err(format!("`light.{k}` must be three numbers"))),
        }
    };
    let light = match &j["light"] {
        serde_json::Value::Null => None,
        l => {
            let sun = vec3(l, "sun")?.ok_or_else(|| err("`light.sun` is missing".into()))?.normalize_or(Vec3::Y);
            let fog = match &l["fog"] {
                serde_json::Value::Null => None,
                f => {
                    let num = |k: &str| f[k].as_f64().map(|x| x as f32).filter(|x| x.is_finite()).ok_or_else(|| err(format!("`light.fog.{k}` must be a number")));
                    let amount = f["amount"].as_array().filter(|a| a.len() == 2).and_then(|a| Some([a[0].as_f64()? as f32, a[1].as_f64()? as f32])).unwrap_or([0.0, 1.0]).map(|a| a.clamp(0.0, 1.0));
                    let (near, far) = (num("near")?, num("far")?);
                    if far <= near {
                        return Err(err("`light.fog.far` must be past `near`".into()));
                    }
                    Some(Fog { colour: vec3(f, "colour")?.unwrap_or(Vec3::ONE), near, far, amount })
                }
            };
            Some(Light {
                sun,
                sun_colour: vec3(l, "sun_colour")?.unwrap_or(Vec3::ONE),
                ambient: vec3(l, "ambient")?.unwrap_or(Vec3::splat(0.5)),
                fill: vec3(l, "fill")?.map_or(Vec3::ZERO, |f| f.normalize_or_zero()),
                fill_colour: vec3(l, "fill_colour")?.unwrap_or(Vec3::ZERO),
                fog,
            })
        }
    };
    let span = |v: &serde_json::Value| v.as_array().filter(|a| a.len() == 2).and_then(|a| Some([a[0].as_u64()?, a[1].as_u64()?])).unwrap_or([0, 0]);
    let bgm = ["sound/bgm.wav", "sound/bgm.ogg"].into_iter().map(|f| dir.join(f)).find(|p| crate::modfs::is_file(p)).map(|path| Loop { path, at: span(&j["bgm_loop"]) });
    let mut ambience = Vec::new();
    for n in 0.. {
        let path = dir.join(format!("sound/ambience_{n}.wav"));
        if !crate::modfs::is_file(&path) {
            break;
        }
        ambience.push(Loop { path, at: span(&j["ambience_loop"][n]) });
    }
    Ok(Court {
        dir: dir.to_path_buf(),
        id: text("id")?,
        name: text("name")?,
        source: j["source"].as_str().unwrap_or("").split(" (").next().unwrap_or("").trim().chars().take(64).collect(),
        layout,
        light,
        sky: crate::modfs::is_file(&dir.join("sky.glb")),
        gimmick: crate::modfs::is_file(&dir.join("gimmick.glb")),
        bgm,
        ambience,
        lens_flare: j["lens_flare"].as_str().filter(|f| crate::modfs::plain(f)).map(|f| dir.join(f)).filter(|p| crate::modfs::is_file(p)),
    })
}

/// Every court mod under `root` (`mods::dirs`' order); a broken one is left out with its reason logged.
pub fn list(root: &Path) -> Vec<Court> {
    crate::mods::dirs(root).iter().filter(|d| Kind::of_dir(d) == Kind::Court).filter_map(|d| read(d).map_err(|e| warn!("{e}")).ok()).collect()
}

/// M8f8: the court mod with `id` under `root` (an online room's), if it's installed.
pub fn find(root: &Path, id: &str) -> Option<Court> {
    list(root).into_iter().find(|c| c.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A court mod reads (layout, light, fog, sounds and their loops); a broken one (layout out of range, no
    /// court.glb, fog ending before it starts, another kind) is refused with the reason; `list` keeps only court
    /// mods, packs one level down.
    #[test]
    fn reads_and_lists_court_mods() {
        let root = std::env::temp_dir().join(format!("hst-court-mod-{}", std::process::id()));
        let put = |sub: &str, json: &str, files: &[&str]| {
            let d = root.join(sub);
            std::fs::create_dir_all(d.join("sound")).unwrap();
            std::fs::write(d.join("mod.json"), json).unwrap();
            for f in files {
                std::fs::write(d.join(f), b"x").unwrap();
            }
            d
        };
        let ok = put(
            "gag_c01",
            r#"{"standard":1,"kind":"court","id":"gag_c01","name":"Hill","source":"Hot Shots Tennis: Get a Grip (PSP) court01","layout":5,
               "light":{"sun":[0,-2,0],"sun_colour":[0.7,0.8,0.5],"ambient":[1,1,1],"fill":[1,0,0],"fill_colour":[0,0.2,0.7],"fog":{"colour":[0.5,0.6,0.7],"near":20,"far":300,"amount":[0,0.8]}},
               "bgm_loop":[100,900],"ambience_loop":[[5,50]]}"#,
            &["court.glb", "sky.glb", "sound/bgm.wav", "sound/ambience_0.wav", "sound/ambience_1.wav"],
        );
        put("pack/plain", r#"{"standard":1,"kind":"court","id":"plain","name":"Plain","layout":0}"#, &["court.glb"]);
        put("bad_layout", r#"{"standard":1,"kind":"court","id":"x","name":"X","layout":11}"#, &["court.glb"]);
        put("no_glb", r#"{"standard":1,"kind":"court","id":"y","name":"Y","layout":2}"#, &[]);
        put("bad_fog", r#"{"standard":1,"kind":"court","id":"z","name":"Z","layout":2,"light":{"sun":[0,-1,0],"fog":{"near":50,"far":10}}}"#, &["court.glb"]);
        put("umpire", r#"{"standard":1,"kind":"umpire","id":"u","name":"U","donor":1}"#, &[]);
        let c = read(&ok).unwrap();
        assert_eq!((c.id.as_str(), c.name.as_str(), c.source.as_str(), c.layout, c.sky, c.gimmick), ("gag_c01", "Hill", "Hot Shots Tennis: Get a Grip", 5, true, false));
        let l = c.light.as_ref().unwrap();
        assert_eq!((l.sun, l.fill, l.fill_colour), (Vec3::NEG_Y, Vec3::X, Vec3::new(0.0, 0.2, 0.7)));
        assert_eq!(l.fog, Some(Fog { colour: Vec3::new(0.5, 0.6, 0.7), near: 20.0, far: 300.0, amount: [0.0, 0.8] }));
        assert_eq!(c.bgm.as_ref().map(|b| b.at), Some([100, 900]));
        assert_eq!(c.ambience.iter().map(|a| a.at).collect::<Vec<_>>(), [[5, 50], [0, 0]]);
        let plain = read(&root.join("pack/plain")).unwrap();
        assert_eq!((plain.light, plain.bgm, plain.ambience.len()), (None, None, 0));
        assert!(read(&root.join("bad_layout")).unwrap_err().contains("`layout`"));
        assert!(read(&root.join("no_glb")).unwrap_err().contains("court.glb"));
        assert!(read(&root.join("bad_fog")).unwrap_err().contains("`light.fog.far`"));
        assert!(read(&root.join("umpire")).unwrap_err().contains("not a court mod"));
        let ids: Vec<_> = list(&root).into_iter().map(|c| c.id).collect();
        assert_eq!(ids, ["gag_c01", "plain"]);
        // and neither the character roster nor the umpire select takes them
        assert!(crate::mods::list(&root).is_empty());
        assert_eq!(crate::mods_umpire::list(&root).len(), 1);
        std::fs::remove_dir_all(&root).ok();
    }
}
