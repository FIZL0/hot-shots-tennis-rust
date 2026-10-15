//! M8f6: court mods on the court select, after the disc eleven (`crate::mods_court`). A pick's value past Random
//! (`COURTS`) is mod `value - COURTS - 1` of `Menu::court_mods`; the match plays on its layout court (`--stage`,
//! `--court`: collision, judging, the chair) and draws and sounds the mod (`--court-mod DIR`). Online (M8f8) the room
//! carries the layout as its court and the mod's id and name (`Room::court_mod`): a peer with it installed draws it
//! (`--court-mod-id`), one without plays the layout's disc court. A vanilla lobby's strip leaves them out.

use super::{COURTS, Menu, Text};
use crate::mods_court::Court;

/// The court mods' values on the select's strip, after the disc eleven (none in a vanilla lobby).
pub(super) fn values(m: &Menu) -> impl Iterator<Item = usize> {
    let vanilla = m.online() && m.net.room.as_ref().is_some_and(|r| r.vanilla);
    let n = if vanilla { 0 } else { m.court_mods.len() };
    COURTS + 1..COURTS + 1 + n
}

impl Menu {
    /// The court mod that court value `c` picks, if it's one.
    pub(super) fn court_mod_of(&self, c: usize) -> Option<&Court> {
        self.court_mods.get(c.checked_sub(COURTS + 1)?)
    }

    /// The disc court `c` plays on (a mod's layout; Random stays Random).
    pub(super) fn court_layout(&self, c: usize) -> usize {
        self.court_mod_of(c).map_or(c, |x| x.layout as usize)
    }

    /// Court `c`'s name in two lines: the disc's (`message0` 276 / 277 + 2c), a mod's own over its game, or Random.
    pub(super) fn court_lines(&self, text: &Text, c: usize) -> [String; 2] {
        match self.court_mod_of(c) {
            Some(x) => [x.name.clone(), source(x)],
            None if c < COURTS => [text.msg(276 + 2 * c).to_string(), text.msg(277 + 2 * c).to_string()],
            None => ["Random".into(), String::new()],
        }
    }

    /// Court `c`'s full name ("Random Court" for Random).
    pub(super) fn court_title(&self, text: &Text, c: usize) -> String {
        match self.court_mod_of(c) {
            Some(x) => x.name.clone(),
            None if c < COURTS => self.court_lines(text, c).join(" "),
            None => "Random Court".into(),
        }
    }

    /// Court `c`'s short name (the first line; "Random Court" for Random).
    pub(super) fn court_short(&self, text: &Text, c: usize) -> String {
        if c == COURTS { "Random Court".into() } else { self.court_lines(text, c)[0].clone() }
    }

    /// The name card's line under a court mod's name: its game and the disc court it plays as.
    pub(super) fn court_mod_sub(&self, text: &Text, c: usize) -> Option<String> {
        let x = self.court_mod_of(c)?;
        Some(format!("{}, plays as {}", source(x), self.court_title(text, x.layout as usize)))
    }

    /// The picked court's mod flags for the match (`--court-mod DIR`), if it's a mod.
    pub(super) fn court_mod_flags(&self) -> Vec<String> {
        self.court_mod_of(self.court).map(|x| vec!["--court-mod".to_string(), x.dir.display().to_string()]).unwrap_or_default()
    }

    /// M8f8: the court card's name for online room `r`: the host's own pick, a guest's the room's mod when it's
    /// installed here (its id), else the layout's disc court.
    pub(super) fn room_court_name(&self, text: &Text, r: &crate::net::Room, hosting: bool) -> String {
        if hosting {
            return if self.court == COURTS { "Random".into() } else { self.court_lines(text, self.court)[0].clone() };
        }
        match r.court_mod.as_ref().filter(|_| !r.vanilla).and_then(|(id, _)| self.court_mods.iter().find(|x| x.id == *id)) {
            Some(x) => x.name.clone(),
            None if r.court < COURTS => text.msg(276 + 2 * r.court).to_string(),
            None => "Random".into(),
        }
    }

    /// A pick past the installed mods (one taken away since): Random.
    pub(super) fn court_in_range(&mut self) {
        if self.court > COURTS + self.court_mods.len() {
            self.court = COURTS;
        }
    }
}

/// A mod's game, short ("Get a Grip" from "Hot Shots Tennis: Get a Grip"), or "Mod".
fn source(x: &Court) -> String {
    x.source.split(':').next_back().map(str::trim).filter(|s| !s.is_empty()).unwrap_or("Mod").to_string()
}

#[cfg(test)]
mod tests {
    use super::super::*;

    fn press(f: impl FnOnce(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    /// M8f6: the court select's strip runs Random, the disc eleven, then the court mods; → past the last disc
    /// court picks the first mod, which shows by its name, plays on its layout court (`--stage`, `--court`) with
    /// `--court-mod DIR`, and goes back to Random when it's no longer installed.
    #[test]
    fn court_mods_follow_the_disc_eleven() {
        let court = |n: u8, layout| crate::mods_court::Court {
            dir: format!("mods/gag_c{n:02}").into(),
            id: format!("gag_c{n:02}"),
            name: format!("Court {n:02}"),
            source: "Hot Shots Tennis: Get a Grip".into(),
            layout,
            light: None,
            sky: false,
            gimmick: false,
            bgm: None,
            ambience: Vec::new(),
            lens_flare: None,
        };
        let last = *STRIP.last().unwrap();
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], court: last, court_mods: vec![court(1, 5), court(3, 9)], ..default() };
        m.sel = 1;
        let mut want = vec![COURTS];
        want.extend(STRIP);
        want.extend([COURTS + 1, COURTS + 2]);
        assert_eq!(strip_order(&m), want);
        m.step(Dev::Keys, press(|p| &mut p.ok));
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_eq!(m.court, COURTS + 1);
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!((m.court, m.picking), (COURTS + 1, None));
        let t = Text { msg: Vec::new(), glyphs: default(), grades: [[0; 5]; 14], style: [0; 14], hands: [1.0; 14], ranks: Vec::new(), mods: Vec::new() };
        assert_eq!(m.court_lines(&t, COURTS + 2), ["Court 03".to_string(), "Get a Grip".into()]);
        assert_eq!((m.court_title(&t, COURTS + 1), m.court_title(&t, COURTS), m.court_layout(COURTS + 2)), ("Court 01".into(), "Random Court".into(), 9));
        let a = m.args(&[]);
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        assert_eq!((flag("--stage"), flag("--court"), flag("--court-mod")), ("6".into(), "6".into(), "mods/gag_c01".into()));
        // a disc pick: no mod flag
        m.court = 3;
        assert!(!m.args(&[]).contains(&"--court-mod".to_string()));
        m.court = COURTS + 2;
        m.court_mods.pop();
        m.court_in_range();
        assert_eq!(m.court, COURTS);
    }

    /// M8f8: online, the host's strip has the court mods (none in a vanilla lobby); its Start sends the mod's layout
    /// as the room's court and the mod's id and name alongside; a guest's court card names the mod only when it has it
    /// installed, else the layout's disc court it will play on.
    #[test]
    fn online_rooms_carry_the_court_mod() {
        use super::super::online::{Fx, Go};
        use crate::net::{Cmd, Room};
        let court = |n: u8, layout| crate::mods_court::Court { dir: format!("mods/gag_c{n:02}").into(), id: format!("gag_c{n:02}"), name: format!("Court {n:02}"), source: "Get a Grip".into(), layout, light: None, sky: false, gimmick: false, bgm: None, ambience: Vec::new(), lens_flare: None };
        let mut m = Menu { screen: Screen::Lobby, court: COURTS + 2, court_mods: vec![court(1, 5), court(3, 9)], ..default() };
        m.sel = 1;
        m.net.go = Some(Go::Host);
        m.net.hosting = true;
        m.net.room = Some(Room::default());
        assert_eq!(strip_order(&m).len(), 1 + STRIP.len() + 2, "the mods online");
        let mut fx = Vec::new();
        m.online_host(&mut fx);
        assert!(fx.iter().any(|e| matches!(e, Effect::Online(Fx::Net(Cmd::Venue(9, ..))))), "{fx:?}");
        assert!(fx.contains(&Effect::Online(Fx::Net(Cmd::CourtMod(Some(("gag_c03".into(), "Court 03".into())))))), "{fx:?}");
        m.court = 3;
        let mut fx = Vec::new();
        m.online_host(&mut fx);
        assert!(fx.contains(&Effect::Online(Fx::Net(Cmd::CourtMod(None)))), "a disc court: no mod");
        m.net.room.as_mut().unwrap().vanilla = true;
        m.sel = 1;
        assert_eq!(strip_order(&m).len(), 1 + STRIP.len(), "a vanilla lobby: none");
        // a guest: the room's mod by name if it's installed here, else its layout's disc name
        let t = Text { msg: (0..300).map(|i| format!("msg{i}")).collect(), glyphs: default(), grades: [[0; 5]; 14], style: [0; 14], hands: [1.0; 14], ranks: Vec::new(), mods: Vec::new() };
        let mut r = Room { court: 9, court_mod: Some(("gag_c03".into(), "Court 03".into())), ..default() };
        assert_eq!(m.room_court_name(&t, &r, false), "Court 03");
        r.court_mod = Some(("gag_c99".into(), "Court 99".into()));
        assert_eq!(m.room_court_name(&t, &r, false), "msg294", "not installed: the layout");
        r.court_mod = Some(("gag_c03".into(), "Court 03".into()));
        r.vanilla = true;
        assert_eq!(m.room_court_name(&t, &r, false), "msg294", "vanilla: the layout");
        m.court = COURTS + 1;
        assert_eq!(m.room_court_name(&t, &r, true), "Court 01", "the host's own pick");
    }

    /// M8f6: the court select with a hundred court mods (long names) at every aspect: the strip scrolls (at most
    /// `STRIP_SHOWN` slots, the pick among them) and the screen draws within the menu's quad pool.
    #[test]
    fn a_hundred_court_mods_fit_the_strip() {
        let t = pointer::tests::text_with_mods(0);
        let court = |n: usize| crate::mods_court::Court { dir: format!("mods/c{n}").into(), id: format!("c{n}"), name: "A Rather Long Court Name Here".into(), source: "Some Game: Its Subtitle".into(), layout: (n % 11) as u8, light: None, sky: false, gimmick: false, bgm: None, ambience: Vec::new(), lens_flare: None };
        for a in [4.0, 21.0 / 9.0, 16.0 / 9.0, 4.0 / 3.0, 9.0 / 16.0, 0.4] {
            for pick in [COURTS, COURTS + 1, COURTS + 50, COURTS + 100] {
                let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], court: pick, court_mods: (0..100).map(court).collect(), grid: Grid::fit(a), ..default() };
                (m.sel, m.picking) = (1, Some(0));
                let rects = strip_rects(&m).0;
                assert_eq!(rects.iter().filter(|r| r[2] > 0.0).count(), STRIP_SHOWN);
                assert!(rects[strip_at(&m)][2] > 0.0, "{a} {pick}: the pick is off the strip");
                let n = drawn(&m, &t, &Bindings::default(), &[], 0, 0.0).out.len();
                assert!(n < POOL * 9 / 10, "{a} {pick}: {n} quads");
            }
        }
    }
}
