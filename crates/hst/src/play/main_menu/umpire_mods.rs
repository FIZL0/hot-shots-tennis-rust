//! M8f3: umpire mods on the confirm screen's umpire select, after the disc five (`crate::mods_umpire`). A pick's
//! value past Random (`UMPIRES`) is mod `value - UMPIRES - 1` of `Menu::umpire_mods`; the match gets its donor
//! (`--umpire`, the timing) and the mod (`--umpire-mod DIR`), an online room the donor and the mod's id.

use super::{Menu, Text, UMPIRES};
use crate::mods_umpire::Umpire;

/// The umpire mods' values on the select's strip, after the disc five.
pub(super) fn values(m: &Menu) -> impl Iterator<Item = usize> {
    UMPIRES + 1..UMPIRES + 1 + m.umpire_mods.len()
}

impl Menu {
    /// The umpire mod that umpire value `u` picks, if it's one.
    pub(super) fn umpire_mod_of(&self, u: usize) -> Option<&Umpire> {
        self.umpire_mods.get(u.checked_sub(UMPIRES + 1)?)
    }

    /// The disc umpire calling as `u` (a mod's donor; Random stays Random).
    pub(super) fn umpire_donor(&self, u: usize) -> usize {
        self.umpire_mod_of(u).map_or(u, |x| x.donor as usize)
    }

    /// Umpire `u`'s name: the disc's (`message0` 669 + 2u, "Umpire " dropped), a mod's own, or Random.
    pub(super) fn umpire_name(&self, text: &Text, u: usize) -> String {
        match self.umpire_mod_of(u) {
            Some(x) => x.name.clone(),
            None if u < UMPIRES => text.msg(669 + 2 * u).trim_start_matches("Umpire ").to_string(),
            None => "Random".into(),
        }
    }

    /// The picked umpire's mod flags for the match (`--umpire-mod DIR`), if it's a mod.
    pub(super) fn umpire_mod_flags(&self) -> Vec<String> {
        self.umpire_mod_of(self.umpire as usize).map(|x| vec!["--umpire-mod".to_string(), x.dir.display().to_string()]).unwrap_or_default()
    }

    /// A pick past the installed mods (one taken away since): Random.
    pub(super) fn umpire_in_range(&mut self) {
        if self.umpire as usize > UMPIRES + self.umpire_mods.len() {
            self.umpire = UMPIRES as u8;
        }
    }
}

/// Preview Voice's bank for umpire `u`: the disc's, or a mod's calls over its donor's.
pub(super) fn preview_bank(iso: &str, m: &Menu, u: u8) -> Option<crate::audio::SoundBank> {
    let mut iso = hst_data::iso::Iso::open(iso).ok()?;
    let disc = crate::audio::umpire_bank(&mut iso, m.umpire_donor(u as usize) as u8, 0)?;
    Some(match m.umpire_mod_of(u as usize) {
        Some(x) => crate::mods_umpire::bank(&disc, x),
        None => disc,
    })
}

#[cfg(test)]
mod tests {
    use super::super::*;

    fn press(f: impl FnOnce(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    /// M8f3: the umpire select's strip runs Random, the disc five, then the umpire mods; → past Anna picks the
    /// first mod, which shows by its name, plays as its donor (`--umpire`) with `--umpire-mod DIR`, isn't saved as
    /// the next run's pick, and goes back to Random when it's no longer installed.
    #[test]
    fn umpire_mods_follow_the_disc_five() {
        let ump = |n: u8, donor| crate::mods_umpire::Umpire { dir: format!("mods/gag_ump{n:02}").into(), id: format!("gag_ump{n:02}"), name: format!("Umpire {n:02}"), source: "Hot Shots Tennis: Get a Grip".into(), donor, voice: "voice/".into() };
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], court: 0, umpire: 4, umpire_mods: vec![ump(0, 2), ump(1, 0)], ..default() };
        m.sel = 4;
        assert_eq!(strip_order(&m), [UMPIRES, 0, 1, 2, 3, 4, UMPIRES + 1, UMPIRES + 2]);
        m.step(Dev::Keys, press(|p| &mut p.ok));
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_eq!(m.umpire as usize, UMPIRES + 1);
        // △: Preview Voice on a mod too
        assert!(m.step(Dev::Keys, press(|p| &mut p.card)).iter().any(|e| matches!(e, Effect::Hear(6, ..))));
        let saved = m.settings.6;
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!((m.umpire as usize, m.settings.6), (UMPIRES + 1, saved), "kept, not saved");
        let t = Text { msg: Vec::new(), glyphs: default(), grades: [[0; 5]; 14], style: [0; 14], hands: [1.0; 14], ranks: Vec::new(), mods: Vec::new() };
        assert_eq!((m.umpire_name(&t, UMPIRES + 2), m.umpire_name(&t, UMPIRES), m.umpire_donor(UMPIRES + 1)), ("Umpire 01".into(), "Random".into(), 2));
        let a = m.args(&[]);
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        assert_eq!((flag("--umpire"), flag("--umpire-mod")), ("2".into(), "mods/gag_ump00".into()));
        // a disc pick: no mod flag
        m.umpire = 3;
        assert!(!m.args(&[]).contains(&"--umpire-mod".to_string()));
        m.umpire = UMPIRES as u8 + 2;
        m.umpire_mods.pop();
        m.umpire_in_range();
        assert_eq!(m.umpire as usize, UMPIRES);
    }
}
