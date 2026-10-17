//! B558: the Settings pages from the character select and the online lobby (remaster-only by request), as the pause
//! menu opens them over a match (`pause.rs`): the legend's Settings key (O, a pad's R3) opens the first tab, Back on a
//! tab returns to the same screen and row, every pick, seat and the connection kept (`Menu::over` holds where it was).
//!
//! Online the lobby keeps running underneath: `online::poll` sees the screen and row they were opened on (`under`, `back`),
//! so its events, picks and the host's countdown go on as on the select, and a match that starts, a kick or a closed
//! lobby takes the menu with it (the pages close). The countdown shows on the pages while it runs.

use super::{Dev, Effect, Menu, Press, Screen};

/// A Settings tab (`options.rs`).
pub(super) fn page(s: Screen) -> bool {
    matches!(s, Screen::Display | Screen::Graphics | Screen::Advanced | Screen::Sound | Screen::Controls | Screen::ModSettings | Screen::Mobile)
}

/// Screens running a lobby underneath (online), the ones the pages open over.
fn lobby(s: Screen) -> bool {
    matches!(s, Screen::Chars | Screen::Lobby | Screen::Confirm)
}

impl Menu {
    /// Whether `dev`'s Settings key opens the pages here: on the select (no name picker or chat typing) and the
    /// online lobby (no prompt over it, no card carried).
    pub(super) fn settings_here(&self, dev: Dev) -> bool {
        let typing = crate::chat::OPEN.load(std::sync::atomic::Ordering::Relaxed) || (0..self.players()).any(|q| self.naming(q));
        let chars = self.screen == Screen::Chars && (self.driven(dev).is_some() || self.online());
        let lobby = self.screen == Screen::Lobby && self.net.room.is_some() && !self.lobby_asking() && self.carrying().is_none();
        (chars || lobby) && !typing && !self.mod_asking() && !self.takeover_asking()
    }

    /// The Settings key: the pages open on the first tab, where this screen was kept for Back.
    pub(super) fn settings_step(&mut self, dev: Dev, k: Press, out: &mut Vec<Effect>) -> bool {
        if !k.settings {
            return false;
        }
        if !self.settings_here(dev) {
            return false;
        }
        self.over = Some((self.screen, self.sel));
        (self.screen, self.sel, self.editing, self.listing) = (Screen::Display, 0, None, None);
        out.push(Effect::Sound(2));
        true
    }

    /// Back on a Settings tab: the screen and row they were opened on, if any (else the main menu's Settings row).
    pub(super) fn settings_back(&mut self) -> bool {
        let Some((s, sel)) = self.over.take() else { return false };
        (self.screen, self.sel) = (s, sel);
        true
    }
}

/// Before `online::poll`'s work: the lobby's screen and row back under the pages; the tab and row they were on.
pub(super) fn under(menu: &mut Menu) -> Option<(Screen, usize)> {
    if !page(menu.screen) {
        // a stale one (the screen was changed under the pages): nothing to go back to
        menu.over = None;
        return None;
    }
    let (s, sel) = menu.over.take()?;
    let held = (menu.screen, menu.sel);
    (menu.screen, menu.sel) = (s, sel);
    Some(held)
}

/// After it: the pages again, over wherever the lobby is now (the select, its lobby or the confirm); a lobby that
/// went on without them (the match starting, the online menu after a kick or a closed lobby) closes them.
pub(super) fn back(menu: &mut Menu, held: Option<(Screen, usize)>) {
    let Some((pg, row)) = held else { return };
    if lobby(menu.screen) && !menu.net.started {
        menu.over = Some((menu.screen, menu.sel));
        (menu.screen, menu.sel) = (pg, row);
    }
}

/// While the host's countdown runs, the words the pages' footnote shows instead (the lobby's Ready / Start says it).
pub(super) fn countdown(m: &Menu) -> Option<String> {
    let n = m.counting();
    (m.over.is_some() && n > 0 && page(m.screen)).then(|| format!("The match is starting ({n})"))
}

#[cfg(test)]
mod tests {
    use super::super::{legend, online};
    use super::*;
    use bevy::prelude::default;

    fn o() -> Press {
        Press { settings: true, ..Default::default() }
    }

    fn esc() -> Press {
        Press { back: true, ..Default::default() }
    }

    /// The select's O / R3 opens the pages; a row changes there; Back returns to the select on the same card and pick.
    #[test]
    fn the_select_opens_settings_and_back_returns_to_it() {
        let mut m = Menu { screen: Screen::Chars, doubles: true, seats: [Some(Dev::Keys), None, None, None], ..default() };
        m.players[0].cursor = 6;
        m.players[0].costume = 1;
        let was = m.players;
        assert!(legend::acts(&m, Dev::Keys).iter().any(|(a, l)| *a == legend::Act::Settings && l == "Settings"));
        m.step(Dev::Keys, o());
        assert_eq!((m.screen, m.sel, m.over), (Screen::Display, 0, Some((Screen::Chars, 0))));
        // L1 / R1 move between the tabs, Back still goes back to the select
        m.step(Dev::Keys, Press { next: true, ..Default::default() });
        assert_eq!(m.screen, Screen::Graphics);
        m.step(Dev::Keys, esc());
        assert_eq!((m.screen, m.over, m.players), (Screen::Chars, None, was));
        // from the main menu's Settings row, Back is the main menu's again
        m.screen = Screen::Display;
        m.step(Dev::Keys, esc());
        assert_eq!(m.screen, Screen::Main);
    }

    /// Not while a name picker is open (O types there), nor with nobody on the select.
    #[test]
    fn not_while_typing_a_name() {
        let mut m = Menu { screen: Screen::Chars, seats: [Some(Dev::Keys), None, None, None], ..default() };
        m.names.open[0] = Some(super::super::names::Picker::Typing("AB".into(), 0));
        m.step(Dev::Keys, o());
        assert_eq!(m.screen, Screen::Chars);
    }

    /// Online: the lobby's screen is back under the pages while the network's events come in, which can move it
    /// (the select to the lobby: the pages stay, Back goes there) or end it (a kick: the pages close with it).
    #[test]
    fn the_lobby_runs_under_the_pages() {
        let mut m = Menu { screen: Screen::Lobby, doubles: true, seats: [Some(Dev::Keys), None, None, None], ..default() };
        online::sample(&mut m);
        m.sel = 2;
        assert!(legend::acts(&m, Dev::Keys).iter().any(|(a, _)| *a == legend::Act::Settings));
        m.step(Dev::Keys, o());
        m.step(Dev::Keys, Press { down: true, ..Default::default() });
        assert_eq!((m.screen, m.sel), (Screen::Display, 1));
        let held = under(&mut m);
        assert_eq!((m.screen, m.sel, held), (Screen::Lobby, 2, Some((Screen::Display, 1))));
        m.screen = Screen::Chars;
        back(&mut m, held);
        assert_eq!((m.screen, m.sel, m.over), (Screen::Display, 1, Some((Screen::Chars, 2))));
        // the countdown shows on the pages
        assert_eq!(countdown(&m), None);
        m.net.room.as_mut().unwrap().count = 3;
        assert_eq!(countdown(&m).as_deref(), Some("The match is starting (3)"));
        // kicked: the online menu, the pages gone
        let held = under(&mut m);
        m.screen = Screen::Online;
        back(&mut m, held);
        assert_eq!((m.screen, m.over), (Screen::Online, None));
        // the match starting: the lobby stays up for its switch
        m.screen = Screen::Lobby;
        m.step(Dev::Keys, o());
        let held = under(&mut m);
        m.net.started = true;
        back(&mut m, held);
        assert_eq!((m.screen, m.over), (Screen::Lobby, None));
    }
}
