//! B449: the lobby, v4 (ours, the approved board "Concept v4: Lobby"); M121: one screen with the character select, as
//! the M120 mockup has it. The select's columns (`roster::chars`: this machine's players' rosters, the other seats'
//! compact cards) are the lobby's seats, on the online select (`Screen::Chars`, this machine's players picking) and on
//! the lobby (`Screen::Lobby`, the cursor on the lobby's own items) alike: going between them only moves the cursor.
//! Along the top the title, Copy link, the host's Public / Private, the code pill (CODE and the join code, the relay's
//! chip; ✕ copies the link as Copy link does) and the Singles / Doubles button (the host's □ or ✕ switches). On the
//! lobby a seat's card takes the cursor (the host's X on another's takes the player off; a carried card says where it
//! goes). Under the columns the scoreboard strip: Court (✕ opens the court select), Umpire (✕ opens the umpire
//! select), Sets, Games and Mods (the host's ✕ steps them on; a tap on their ‹ › halves steps them back or on). Along
//! the bottom Leave, the chat box (the last lines heard; ✕ types) and the host's big Start ("n of N ready · who is
//! still picking") or a guest's Ready / Not ready. What the buttons do is only in the legend strip.

use super::super::frame::{self, Cell, Thumb};
use super::super::Press;
use super::super::roster::{self, PX, PY};
use super::super::{Draw, Menu, Screen, WHITE};
use super::{Act, Item, OK, SETTINGS, shared};
use crate::net::Room;
use bevy::prelude::default;

/// The strip's settings in its order: court, umpire, sets, games, mods; then the top bar's Singles / Doubles.
pub(in super::super) const SIDE: [Item; SETTINGS + 1] = [Item::Set(0), Item::Set(1), Item::Set(2), Item::Set(3), Item::Set(4), Item::Mode];

/// Seat `i`'s state on its plate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in super::super) enum State {
    Open,
    /// On the select (or its live place not ready yet).
    Picking,
    /// Picked, not readied (a guest's Ready).
    Waiting,
    Ready,
}

/// Seat `i`'s state: a computer's is ready; a person's picking while on the select, else ready once readied (the host:
/// once picked; it starts the match).
pub(in super::super) fn state(r: &Room, i: usize) -> State {
    let Some(s) = r.seats.get(i).and_then(Option::as_ref) else { return State::Open };
    if !s.human() {
        return State::Ready;
    }
    if r.picking.contains(&s.peer) || r.live_of(i).is_some_and(|l| !l.ready) {
        State::Picking
    } else if s.peer == "host" || r.ready.contains(&s.peer) {
        State::Ready
    } else {
        State::Waiting
    }
}

/// The ready seats and all of them, for Start's "n of N ready".
pub(in super::super) fn ready_count(r: &Room) -> (usize, usize) {
    ((0..r.seats.len()).filter(|&i| state(r, i) == State::Ready).count(), r.seats.len())
}

/// The title's room in the top bar (mockup px), left of Copy link.
const TITLE_W: f32 = 230.0;

impl Menu {
    /// M121: the lobby's rows under the select's columns (the strip; Leave, the chat box, Start), across the columns'
    /// width; its `stage` is the columns' view.
    pub(in super::super) fn lobby_foot(&self) -> frame::Geo {
        let [l, t, r, b] = self.grid.view;
        let band = super::band_h(self, r - l - 2.0 * roster::MARGIN * PX);
        frame::foot([l + roster::MARGIN * PX, t + band, r - roster::MARGIN * PX, b])
    }

    /// M121: the top bar's buttons, right to left: Singles / Doubles, the code pill's place (the pill right-aligned
    /// in it, as wide as its words), the host's Public (a browser's LAN host's Paste), Copy link. Fixed widths, so the
    /// arrows and the taps agree with what's drawn. `[code, copy, public, mode]`.
    pub(in super::super) fn lobby_bar(&self) -> [[f32; 4]; 4] {
        let [l, t, r, _] = self.grid.view;
        let (y, h) = (t + 16.0 * PY, 30.0 * PY);
        let mode = [r - roster::MARGIN * PX - 140.0 * PX, y, 140.0 * PX, h];
        let (copy_w, public_w) = (104.0 * PX, if self.net.hosting { 96.0 * PX } else { 0.0 });
        let room = mode[0] - 12.0 * PX - (l + 32.0 + TITLE_W * PX) - copy_w - public_w - 16.0 * PX;
        let code_w = room.clamp(150.0 * PX, 330.0 * PX);
        let code = [mode[0] - 12.0 * PX - code_w, y, code_w, h];
        let public = [code[0] - 8.0 * PX - public_w, y, public_w, h];
        let copy = [public[0] - if public_w > 0.0 { 8.0 * PX } else { 0.0 } - copy_w, y, copy_w, h];
        [code, copy, public, mode]
    }

    /// The lobby's frame: the foot, the top bar's buttons, a plate a seat (its column or compact card on the select).
    pub(in super::super) fn lobby_geo(&self) -> frame::Geo {
        let mut g = self.lobby_foot();
        let c = self.columns();
        g.plates = (0..c.n).map(|p| c.parts(p).card).collect();
        [g.code, g.copy, g.public, g.mode] = self.lobby_bar();
        g.pill = g.code;
        g
    }

    /// Seat `i` is this machine's player's.
    pub(in super::super) fn mine(&self, i: usize) -> bool {
        let me = if self.net.me.is_empty() && self.net.hosting { "host" } else { self.net.me.as_str() };
        self.net.room.as_ref().is_some_and(|r| r.seats.get(i).is_some_and(|s| s.as_ref().is_some_and(|s| s.peer == me)))
    }

    /// Where `items` are drawn.
    pub(in super::super) fn lobby_rects_geo(&self) -> Vec<[f32; 4]> {
        let g = self.lobby_geo();
        self.items()
            .iter()
            .map(|it| match *it {
                Item::Seat(i) => g.plates.get(i).copied().unwrap_or_default(),
                Item::Remove(i) => g.plates.get(i).map_or_else(Default::default, |&c| roster::remove_rect(c)),
                Item::Set(i) => g.cells[i.min(4)],
                Item::Mode => g.mode,
                Item::Button("Start" | "Ready") => g.start,
                Item::Button("Code") => g.code,
                Item::Button("Copy Link") => g.copy,
                Item::Button("Public" | "Paste") => g.public,
                Item::Button("Chat") => g.chat,
                Item::Button(_) => g.back,
            })
            .collect()
    }
}

/// M121: the lobby around the select's columns (`roster::chars` draws them on both screens, then this): the top bar's
/// buttons; on the lobby each seat's focus, the host's ✕ and a carried card; the strip; Leave, the chat box and Start.
/// While this machine's players pick (`Screen::Chars`) none of it has the cursor, and a tap reaches the settings, the
/// code and Leave (`Act::Lobby`), not the seats, Start or Ready (the players ready on the select first).
pub(in super::super) fn over(d: &mut Draw, m: &Menu) {
    let lobby = m.screen == Screen::Lobby;
    let items = m.items();
    let g = m.lobby_geo();
    let at = |it: Item| items.iter().position(|x| *x == it);
    let on = |it: Item| lobby && !m.lobby_asking() && at(it).is_some_and(|k| k == m.sel);
    let hit = |d: &mut Draw, it: Item, rect: [f32; 4], k: Press| {
        let Some(i) = at(it) else { return };
        if lobby {
            d.hit(rect, Act::Row(i, k));
        } else if !matches!(it, Item::Seat(_) | Item::Remove(_) | Item::Button("Start" | "Ready")) {
            d.hit(rect, Act::Lobby(i, k));
        }
    };
    let rm = m.net.room.as_ref();
    // B548: the room this machine would host only before one has come (`room` builds every pick's wire)
    let own;
    let r = match rm {
        Some(r) => r,
        None => {
            own = m.room();
            &own
        }
    };
    top_bar(d, m, r, &g, &on, &hit);
    // the seats: on the lobby the cursor, the host's ✕, a carried card over the select's own columns
    let s = g.k * g.s;
    for (i, &rect) in g.plates.iter().enumerate().filter(|_| lobby) {
        let Some(r) = rm else { break };
        hit(d, Item::Seat(i), rect, OK);
        if on(Item::Seat(i)) {
            frame::focus(d, rect, 16.0 * PX, 4.0 * PX, 3.0 * PX);
        }
        carried(d, m, r, i, rect, s);
        if at(Item::Remove(i)).is_some() {
            let xr = roster::remove_rect(rect);
            frame::ringed(d, xr, xr[2] / 2.0, 2.0 * PX, frame::NAVY, Some(roster::seat(i).ring));
            let xs = 14.0 * PY;
            d.centred("X", xr[0] + xr[2] / 2.0, xr[1] + (xr[3] - xs) / 2.0, xs, frame::NAVY);
            if on(Item::Remove(i)) {
                frame::focus(d, xr, xr[2] / 2.0, 3.0 * PX, 3.0 * PX);
            }
            hit(d, Item::Remove(i), [xr[0] - 2.0, xr[1] - 2.0, xr[2] + 4.0, xr[3] + 4.0], OK);
        }
    }
    strip(d, m, rm, &g, &on, &hit);
    start_button(d, m, rm, &g, &items, &on);
}

/// The top bar's buttons right of the title: Copy link (dim until there's a link), the host's Public / Private (a
/// browser's LAN lobby's Paste), the code pill (`online::corner`: CODE, the code greyed until it's live, the relay's
/// chip) and Singles / Doubles (a guest's Swap team: B522's □, dimmed when it has no seat to move to).
fn top_bar(d: &mut Draw, m: &Menu, r: &Room, g: &frame::Geo, on: &dyn Fn(Item) -> bool, hit: &dyn Fn(&mut Draw, Item, [f32; 4], Press)) {
    let lobby = m.screen == Screen::Lobby;
    let guest = !m.net.hosting;
    let at = |it: Item| m.items().contains(&it);
    // Singles / Doubles: the host's □ (or a tap) switches; on the select a tap is the □ (B375, B392); a guest's tap is
    // its □ on either screen (Swap team)
    let glyph = match guest {
        true => frame::page_glyph(m, if lobby { m.lobby_team() } else { (0..4).find(|&p| m.seats[p].is_some_and(|dev| m.online_team(dev).is_some())) }),
        false => frame::page_glyph(m, (0..4).find(|&p| m.seats[p].is_some())).filter(|_| m.can_switch()),
    };
    frame::bar(d, g, "", r.doubles, guest, glyph, guest || !lobby, on(Item::Mode));
    if lobby && !guest {
        hit(d, Item::Mode, g.mode, OK);
    }
    if guest && lobby && m.lobby_team().is_none() {
        frame::rounded(d, g.mode, g.mode[3] / 2.0, frame::NAVY, 64.0);
    }
    // the code pill, right-aligned in its place
    let [cx, cy, cw, ch] = g.code;
    let left = super::corner(d, m, cx + cw, cy, ch, cx);
    if on(Item::Button("Code")) {
        frame::focus(d, [left, cy, cx + cw - left, ch], ch / 2.0, 3.0 * PX, 3.0 * PX);
    }
    hit(d, Item::Button("Code"), [left, cy, cx + cw - left, ch], OK);
    let dim = shared(&m.net).is_none();
    let side = if m.lan_host() { (Item::Button("Paste"), "Paste", frame::BUTTON) } else { (Item::Button("Public"), if r.private { "Private" } else { "Public" }, if r.private { frame::BUTTON } else { frame::GREEN }) };
    for (it, rect, words, fill) in [(Item::Button("Copy Link"), g.copy, "Copy link", frame::BUTTON), (side.0, g.public, side.1, side.2)] {
        let [bx, by, bw, bh] = rect;
        if bw <= 0.0 || !at(it) && !(it == Item::Button("Copy Link") && m.online()) {
            continue;
        }
        frame::ringed(d, rect, bh / 2.0, 2.0 * PX, WHITE, Some(fill));
        if on(it) {
            frame::focus(d, rect, bh / 2.0, 3.0 * PX, 3.0 * PX);
        }
        let size = 14.0 * PY;
        let rgb = if dim && it == Item::Button("Copy Link") { [70.0; 3] } else { WHITE };
        d.label(words, bx + bw / 2.0, by + (bh - size) / 2.0, size, bw - 12.0 * PX, rgb);
        hit(d, it, rect, OK);
    }
}

/// B523: the host carrying a card: it stays dimmed where it was, "Moving"; the card it would go onto says what the
/// drop does.
fn carried(d: &mut Draw, m: &Menu, r: &Room, i: usize, rect: [f32; 4], s: f32) {
    let Some(from) = m.carrying() else { return };
    let words = match m.target_seat() {
        _ if i == from => "Moving",
        Some(t) if t == i && !r.can_swap(from, i) => "Can't swap",
        Some(t) if t == i && r.seats.get(i).is_some_and(Option::is_none) => "Move here",
        Some(t) if t == i => "Swap",
        _ => return,
    };
    let [x, y, w, h] = rect;
    if i == from {
        frame::rounded(d, rect, 14.0 * s * roster::PX, frame::NAVY, 56.0);
    }
    let size = 16.0 * s * roster::PY;
    let (pw, ph) = ((d.width(words, size) + 28.0 * s * roster::PX).min(w - 8.0), size + 12.0 * s * roster::PY);
    let (px, py) = (x + (w - pw) / 2.0, y + h * 0.42 - ph / 2.0);
    frame::rounded(d, [px, py, pw, ph], ph / 2.0, if i == from { frame::NAVY } else { frame::GOLD }, 128.0);
    d.label(words, x + w / 2.0, py + (ph - size) / 2.0, size, pw - 8.0, if i == from { frame::GOLD } else { frame::NAVY });
}

/// The strip: Court, Umpire, Sets, Games, Mods (the host's ‹ › on the values that step). The host's own choices before
/// the lobby is open; a guest's "..." until its line-up comes.
fn strip(d: &mut Draw, m: &Menu, r: Option<&Room>, g: &frame::Geo, on: &dyn Fn(Item) -> bool, hit: &dyn Fn(&mut Draw, Item, [f32; 4], Press)) {
    let text = d.text;
    let host = m.net.hosting;
    let own = (r.is_none() && host).then(|| m.room());
    let Some(r) = r.or(own.as_ref()) else {
        let cell = |label| Cell { label, value: "...".into(), arrows: false, sel: false, thumb: Thumb::None };
        return frame::strip(d, m, g, &[cell("COURT"), cell("UMPIRE"), cell("SETS"), cell("GAMES"), cell("MODS")]);
    };
    // the host's own choice (Random stays Random), a guest's the lobby's
    let (court, sets, games, umpire) = if host { (m.court, m.sets, m.games, m.umpire as usize) } else { (m.room_court_value(r), r.sets, r.games, r.umpire as usize) };
    let umpire_name = match r.umpire_mod.as_ref().filter(|_| !host && !r.vanilla) {
        // M8f3: the host's umpire mod by its name (a guest without it hears the donor)
        Some((_, name)) => name.clone(),
        None => m.umpire_name(text, umpire),
    };
    let cells = [
        Cell { label: "COURT", value: m.room_court_name(text, r, host), arrows: false, sel: on(Item::Set(0)), thumb: Thumb::Court(court) },
        Cell { label: "UMPIRE", value: umpire_name, arrows: false, sel: on(Item::Set(1)), thumb: Thumb::Umpire(umpire) },
        Cell { label: "SETS", value: super::super::SETS.get(sets).copied().unwrap_or(1).to_string(), arrows: host, sel: on(Item::Set(2)), thumb: Thumb::None },
        Cell { label: "GAMES", value: games.to_string(), arrows: host, sel: on(Item::Set(3)), thumb: Thumb::None },
        Cell { label: "MODS", value: if r.vanilla { "None" } else { "Allowed" }.to_string(), arrows: host, sel: on(Item::Set(4)), thumb: Thumb::None },
    ];
    frame::strip(d, m, g, &cells);
    for (i, cell) in g.cells.iter().enumerate() {
        let [x, y, w, h] = *cell;
        hit(d, Item::Set(i), *cell, OK);
        if i >= 2 {
            // a tap on the ‹ › halves: back or on
            hit(d, Item::Set(i), [x, y, w * 0.35, h], Press { ok: true, left: true, ..default() });
            hit(d, Item::Set(i), [x + w * 0.65, y, w * 0.35, h], Press { ok: true, right: true, ..default() });
        }
    }
}

/// The host's Start ("n of N ready · who is still picking") or a guest's Ready / Not ready; before the lobby is open,
/// what it waits for (no tap).
fn start_button(d: &mut Draw, m: &Menu, r: Option<&Room>, g: &frame::Geo, items: &[Item], on: &dyn Fn(Item) -> bool) {
    let big = if m.net.hosting { "Start" } else { "Ready" };
    let Some(r) = r else {
        let small = if m.net.hosting { "Opening the lobby..." } else { "Joining the lobby..." };
        return frame::start(d, g, big, small, false, false);
    };
    let Some(k) = items.iter().position(|it| matches!(it, Item::Button("Start" | "Ready"))) else { return };
    let (n, of) = ready_count(r);
    let picking = (0..r.seats.len()).find(|&i| state(r, i) == State::Picking).and_then(|i| r.seats[i].as_ref().map(|s| super::seat_name(m, r, i, &s.peer)));
    // M122: then a player whose mods' looks are still downloading (the match waits for them)
    let fetching = super::downloading(m, r);
    let ready = match (&picking, r.open()) {
        (Some(p), _) => format!("{n} of {of} ready · {p} is still picking"),
        (None, _) if let Some(f) = &fetching => format!("{n} of {of} ready · downloading: {f}"),
        (None, 0) if n == of => "Everyone ready".to_string(),
        (None, 0) => format!("{n} of {of} ready"),
        (None, open) => format!("{n} of {of} ready · +{open} COM"),
    };
    let (big, small, play) = match items[k] {
        _ if m.counting() > 0 => (format!("Cancel ({})", m.counting()), "The match is starting".to_string(), false),
        Item::Button("Start") => ("Start".into(), ready, true),
        _ if m.net.prepared && fetching.is_some() => ("Not ready".into(), ready, false),
        _ if m.net.prepared => ("Not ready".into(), format!("{n} of {of} ready · waiting for the host"), false),
        _ if fetching.is_some() => ("Ready".into(), ready, false),
        _ => ("Ready".into(), format!("{n} of {of} ready · waiting for the host"), false),
    };
    frame::start(d, g, &big, &small, play, on(items[k]));
    if m.screen == Screen::Lobby {
        d.hit(g.start, Act::Row(k, OK));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::{CPU, Live, Pick, Seat};

    fn room() -> Room {
        let seat = |peer: &str, c| Some(Seat { peer: peer.into(), pick: Pick::disc(c, 0) });
        Room { doubles: true, seats: vec![seat("host", 6), seat("g", 1), seat(CPU, 9), None], ..Default::default() }
    }

    #[test]
    fn seat_states_and_ready_count() {
        let mut r = room();
        assert_eq!((0..4).map(|i| state(&r, i)).collect::<Vec<_>>(), [State::Ready, State::Waiting, State::Ready, State::Open]);
        r.picking.insert("g".into());
        assert_eq!(state(&r, 1), State::Picking);
        r.picking.clear();
        // the live place still choosing is picking, even out of `picking`
        r.live.insert("g".into(), vec![Live { char: 4, ..Default::default() }]);
        assert_eq!(state(&r, 1), State::Picking);
        r.live.insert("g".into(), vec![Live { char: 4, ready: true, ..Default::default() }]);
        r.ready.insert("g".into());
        assert_eq!(state(&r, 1), State::Ready);
        assert_eq!(ready_count(&r), (3, 4));
    }

    /// Any aspect ratio: every item inside the view, apart (a seat's X on its own column), the columns over the strip.
    #[test]
    fn lays_out_at_any_aspect() {
        for aspect in [4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 1.0, 3.0 / 4.0, 9.0 / 16.0, 0.4] {
            for hosting in [true, false] {
                let mut m = Menu { screen: super::super::super::Screen::Lobby, grid: super::super::super::Grid::fit(aspect), ..default() };
                super::super::sample(&mut m);
                m.net.hosting = hosting;
                let g = m.lobby_geo();
                // M121: the lobby is the select's columns, across the whole view
                let [l, t, r, b] = m.grid.view;
                let inside = |c: &[f32; 4]| c[0] >= l - 0.01 && c[1] >= t - 0.01 && c[0] + c[2] <= r + 0.01 && c[1] + c[3] <= b + 0.01;
                let apart = |a: &[f32; 4], c: &[f32; 4]| a[0] + a[2] <= c[0] + 0.01 || c[0] + c[2] <= a[0] + 0.01 || a[1] + a[3] <= c[1] + 0.01 || c[1] + c[3] <= a[1] + 0.01;
                let items = m.items();
                let rects = m.lobby_rects();
                for (k, a) in rects.iter().enumerate() {
                    assert!(inside(a), "{aspect} {:?} {a:?}", items[k]);
                    for (j, c) in rects.iter().enumerate().skip(k + 1) {
                        let x = matches!((items[k], items[j]), (Item::Seat(s), Item::Remove(x)) if s == x);
                        assert!(x || apart(a, c), "{aspect}: {:?} {a:?} over {:?} {c:?}", items[k], items[j]);
                    }
                }
                // the columns over the strip, inside the stage
                assert!(g.plates.iter().all(|p| p[0] >= g.court[0] - 0.01 && p[0] + p[2] <= g.court[0] + g.court[2] + 0.01 && p[1] + p[3] <= g.strip[1] + 0.01), "{aspect} {:?} {:?}", g.plates, g.strip);
            }
        }
    }
}
