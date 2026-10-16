//! B449: the lobby, v4 (ours, the approved board "Concept v4: Lobby"; the frame and the plates are shared with the
//! confirm screen, `frame.rs`). Along the top the title, the code pill (CODE and the join code, the QR big on ✕; Copy
//! link; the host's Public / Private) and the Singles / Doubles button (the host's □ or ✕ switches). The middle is
//! the court: a team each side of the net (doubles: TEAM 1, TEAM 2; singles has no side names, B490: each client
//! keeps its own player on the near side), a plate a seat with its seat-coloured
//! header (tag, who has it: "You · Keyboard", "Mika · 35 ms" (the ping gold over 150 ms), Computer, Open seat; the
//! host's X that takes the player off), the 3D model in its well (the stand-in note for a mod this machine lacks),
//! the character, its style and hand and one chip: Ready, Picking..., Not ready, COM · rank or Open. A taken seat's
//! plate follows its player's place on the select live (`Live`). Under it the scoreboard strip: Court (✕ opens the
//! court select), Umpire (✕ opens the umpire select), Sets, Games and Mods (the host's ✕ steps them on; a tap on
//! their ‹ › halves steps them back or on). Along the bottom Leave, the chat box (the last lines heard; ✕ types) and
//! the host's big Start ("n of N ready · who is still picking") or a guest's Ready / Not ready. What the buttons do
//! is only in the legend strip.

use super::super::frame::{self, Cell, Chip, Plate, Spec, Thumb};
use super::super::{Draw, INK, Menu, Tex, WHITE, Who};
use super::{Act, Item, OK, SETTINGS, shared, spinner, ticket_only};
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

impl Menu {
    /// The frame for this lobby: its seats, the code pill (the host's Public in it), the chat box if chat's on.
    pub(in super::super) fn lobby_geo(&self) -> frame::Geo {
        let n = self.net.room.as_ref().map_or(if self.doubles { 4 } else { 2 }, |r| r.seats.len());
        let chat = self.items().contains(&Item::Button("Chat"));
        frame::layout(self.grid.reach(), Spec { n, code: true, public: self.net.hosting, chat })
    }

    /// Seat `i` is this machine's player's.
    pub(super) fn mine(&self, i: usize) -> bool {
        let me = if self.net.me.is_empty() && self.net.hosting { "host" } else { self.net.me.as_str() };
        self.net.room.as_ref().is_some_and(|r| r.seats.get(i).is_some_and(|s| s.as_ref().is_some_and(|s| s.peer == me)))
    }

    /// Where `items` are drawn.
    pub(in super::super) fn lobby_rects_geo(&self) -> Vec<[f32; 4]> {
        let g = self.lobby_geo();
        let s = g.k * g.s;
        self.items()
            .iter()
            .map(|it| match *it {
                Item::Seat(i) => g.plates.get(i).copied().unwrap_or_default(),
                Item::Remove(i) => g.plates.get(i).map_or_else(Default::default, |&c| frame::remove_rect(c, s)),
                Item::Set(i) => g.cells[i.min(4)],
                Item::Mode => g.mode,
                Item::Button("Start" | "Ready") => g.start,
                Item::Button("Code") => g.code,
                Item::Button("Copy Link") => g.copy,
                Item::Button("Public") => g.public,
                Item::Button("Chat") => g.chat,
                Item::Button(_) => g.back,
            })
            .collect()
    }
}

/// The lobby screen; `status` is what's happening (the wait for a mod's answer, a guest being taken out), shown in the
/// chat box.
pub(in super::super) fn draw(d: &mut Draw, m: &Menu, status: Option<&str>) {
    frame::ground(d);
    let items = m.items();
    let g = m.lobby_geo();
    let at = |it: Item| items.iter().position(|x| *x == it);
    let on = |it: Item| !m.lobby_asking() && !m.net.qr_big && at(it).is_some_and(|k| k == m.sel);
    let Some(rm) = m.net.room.as_ref() else { return };
    // the top bar: the title, the code pill, Singles / Doubles (the host's □ or a tap)
    frame::bar(d, &g, "Lobby", rm.doubles, false, on(Item::Mode));
    if let Some(k) = at(Item::Mode) {
        d.hit(g.mode, Act::Row(k, OK));
    } else {
        // only the host switches: a guest's is dimmed
        frame::rounded(d, g.mode, g.mode[3] / 2.0, frame::NAVY, 64.0);
    }
    code_pill(d, m, rm, &g, &on);
    for (b, rect) in [("Code", g.code), ("Copy Link", g.copy), ("Public", g.public)] {
        if let Some(k) = at(Item::Button(b)) {
            d.hit(rect, Act::Row(k, OK));
        }
    }
    // the court and the plates
    frame::stage(d, &g, if rm.doubles { ["TEAM 1", "TEAM 2"] } else { ["", ""] });
    let s = g.k * g.s;
    for i in 0..rm.seats.len() {
        let Some(&rect) = g.plates.get(i) else { continue };
        if let Some(k) = at(Item::Seat(i)) {
            d.hit(rect, Act::Row(k, OK));
        }
        plate(d, m, rm, i, rect, s, on(Item::Seat(i)), at(Item::Remove(i)).is_some(), on(Item::Remove(i)));
        if let Some(k) = at(Item::Remove(i)) {
            let [x, y, w, h] = frame::remove_rect(rect, s);
            d.hit([x - 2.0, y - 2.0, w + 4.0, h + 4.0], Act::Row(k, OK));
        }
    }
    strip(d, m, rm, &g, &items, &on);
    // the bottom row: Leave, the chat box, Start / Ready
    frame::back(d, &g, "Leave", on(Item::Button("Leave")));
    if let Some(k) = at(Item::Button("Leave")) {
        d.hit(g.back, Act::Row(k, OK));
    }
    if let Some(k) = at(Item::Button("Chat")) {
        crate::chat::BOXED.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut lines = crate::chat::RECENT.lock().map(|l| l.clone()).unwrap_or_default();
        if let Some(s) = status {
            lines.push((String::new(), s.to_string()));
        }
        frame::chat(d, &g, &lines, on(Item::Button("Chat")));
        d.hit(g.chat, Act::Row(k, OK));
    }
    start_button(d, m, rm, &g, &items, &on);
    if m.net.qr_big && shared(&m.net).is_some() {
        // the QR big over everything; any press closes it
        d.popup(Some(super::super::pointer::press(|k| k.back = true)));
        let [l, t, r, b] = m.grid.reach();
        let s = (b - t - 80.0).min(r - l - 40.0).min(300.0);
        let (x, y) = ((l + r - s) / 2.0, (t + b - s) / 2.0 - 10.0);
        d.box3([x - 14.0, y - 14.0, s + 28.0, s + 52.0], 14.0, [Some(INK), Some(WHITE), Some(WHITE)], 128.0);
        d.q(Tex::Qr, [0.0, 0.0, 1.0, 1.0], [x, y, s, s], WHITE, 128.0);
        let code = if ticket_only(&m.net) { "Scan to join" } else { m.net.short.as_str() };
        d.label(code, x + s / 2.0, y + s + 6.0, 24.0, s, INK);
        d.hit([x - 14.0, y - 14.0, s + 28.0, s + 52.0], super::super::pointer::press(|k| k.back = true));
    }
}

/// The code pill: CODE and the join code (a spinner until there is one), Copy link (dim until there's a link), the
/// host's Public / Private.
fn code_pill(d: &mut Draw, m: &Menu, r: &Room, g: &frame::Geo, on: &dyn Fn(Item) -> bool) {
    let [x, y, _, h] = g.pill;
    frame::ringed(d, g.pill, h / 2.0, g.x(2.0), frame::RULE, Some(frame::PANEL));
    if on(Item::Button("Code")) {
        frame::focus(d, g.code, h / 2.0, g.x(3.0), g.x(3.0));
    }
    let cx = x + g.x(14.0);
    let ls = g.y(12.0);
    let label = if ticket_only(&m.net) { "LINK" } else if r.private { "PRIVATE" } else { "CODE" };
    super::super::left(d, label, cx, y + (h - ls) / 2.0, ls, g.x(46.0), frame::CAPTION);
    let vx = cx + g.x(46.0 + 8.0);
    let vw = g.copy[0] - g.x(8.0) - vx;
    if shared(&m.net).is_some() || !m.net.short.is_empty() {
        let code = if ticket_only(&m.net) { "QR" } else if m.net.short.is_empty() { "...." } else { m.net.short.as_str() };
        let size = g.y(24.0);
        super::super::left(d, code, vx, y + (h - size) / 2.0, size, vw, frame::GOLD);
    } else {
        spinner(d, m.net.spin, vx + g.x(12.0), y + h / 2.0, g.y(9.0));
    }
    let dim = shared(&m.net).is_none();
    let public = !r.private;
    let buttons = [(Item::Button("Copy Link"), g.copy, "Copy link", frame::BUTTON), (Item::Button("Public"), g.public, if public { "Public" } else { "Private" }, if public { frame::GREEN } else { frame::BUTTON })];
    for (it, rect, words, fill) in buttons {
        let [bx, by, bw, bh] = rect;
        if bw <= 0.0 {
            continue;
        }
        frame::rounded(d, rect, bh / 2.0, fill, 128.0);
        if on(it) {
            frame::focus(d, rect, bh / 2.0, g.x(3.0), g.x(3.0));
        }
        let size = g.y(13.0);
        let rgb = if dim && it == Item::Button("Copy Link") { [70.0; 3] } else { WHITE };
        d.label(words, bx + bw / 2.0, by + (bh - size) / 2.0, size, bw - g.x(12.0), rgb);
    }
}

/// The strip: Court, Umpire, Sets, Games, Mods (the host's ‹ › on the values that step).
fn strip(d: &mut Draw, m: &Menu, r: &Room, g: &frame::Geo, items: &[Item], on: &dyn Fn(Item) -> bool) {
    let text = d.text;
    let at = |it: Item| items.iter().position(|x| *x == it);
    let host = m.net.hosting;
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
        let Some(k) = at(Item::Set(i)) else { continue };
        let [x, y, w, h] = *cell;
        d.hit(*cell, Act::Row(k, OK));
        if i >= 2 {
            // a tap on the ‹ › halves: back or on
            d.hit([x, y, w * 0.35, h], Act::Row(k, super::Press { ok: true, left: true, ..default() }));
            d.hit([x + w * 0.65, y, w * 0.35, h], Act::Row(k, super::Press { ok: true, right: true, ..default() }));
        }
    }
}

/// The host's Start ("n of N ready · who is still picking") or a guest's Ready / Not ready.
fn start_button(d: &mut Draw, m: &Menu, r: &Room, g: &frame::Geo, items: &[Item], on: &dyn Fn(Item) -> bool) {
    let Some(k) = items.iter().position(|it| matches!(it, Item::Button("Start" | "Ready"))) else { return };
    let (n, of) = ready_count(r);
    let picking = (0..r.seats.len()).find(|&i| state(r, i) == State::Picking).and_then(|i| r.seats[i].as_ref().map(|s| super::seat_name(m, r, i, &s.peer)));
    let ready = match (&picking, r.open()) {
        (Some(p), _) => format!("{n} of {of} ready · {p} is still picking"),
        (None, 0) if n == of => "Everyone ready".to_string(),
        (None, 0) => format!("{n} of {of} ready"),
        (None, open) => format!("{n} of {of} ready · +{open} COM"),
    };
    let (big, small, play) = match items[k] {
        _ if m.counting() > 0 => (format!("Cancel ({})", m.counting()), "The match is starting".to_string(), false),
        Item::Button("Start") => ("Start".into(), ready, true),
        _ if m.net.prepared => ("Not ready".into(), format!("{n} of {of} ready · waiting for the host"), false),
        _ => ("Ready".into(), format!("{n} of {of} ready · waiting for the host"), false),
    };
    frame::start(d, g, &big, &small, play, on(items[k]));
    d.hit(g.start, Act::Row(k, OK));
}

/// Seat `i`'s plate name, its character and whether it shows as Random.
fn seat_pick(m: &Menu, r: &Room, i: usize, s: &crate::net::Seat) -> (String, Who, bool) {
    let pl = &m.players[i];
    let live = r.live_of(i).filter(|_| s.human());
    // Random: the plate they're on now (live), else the pick they readied
    let random = pl.random || live.map_or(s.pick.random, |l| l.random || (l.ready && s.pick.random));
    let (who, _) = m.pick(i);
    // the name: the mod's own (B234) for one this machine lacks, Random hidden until the match (B217)
    let name = match (live, &s.pick.r#mod) {
        _ if random => "Random".to_string(),
        (Some(l), _) if !l.name.is_empty() => l.name.clone(),
        (Some(_), _) | (None, None) => match who {
            Who::Mod(j) => m.mods[j].name.clone(),
            Who::Disc(c) => super::super::CHAR_NAMES[c.min(super::super::CHARS - 1)].to_string(),
        },
        (None, Some(w)) => w.name.clone(),
    };
    (name, who, random)
}

/// Seat `i`'s plate.
#[allow(clippy::too_many_arguments)]
fn plate(d: &mut Draw, m: &Menu, r: &Room, i: usize, rect: [f32; 4], s: f32, focus: bool, remove: bool, remove_focus: bool) {
    let text = d.text;
    let hide = m.lobby_asking() || m.net.qr_big;
    let seat = r.seats.get(i).and_then(Option::as_ref);
    let Some(st) = seat else {
        // an open seat: the computer plays it unless someone joins
        let pl = Plate { p: i, owner: "Open seat", slow: false, name: "Computer", sub: "Until someone joins", chip: Chip::Open, chip_words: "Open", dashed: true, remove: false, focus, remove_focus: false, random: false, hide, empty: true, mod_note: None, device: None };
        return frame::plate(d, &pl, rect, s);
    };
    let com = !st.human();
    let mine = m.mine(i);
    let (name, who, random) = seat_pick(m, r, i, st);
    // who has it: this machine's device, the player's name and ping, or the computer
    let me = if m.net.hosting { "host" } else { m.net.me.as_str() };
    let ping = match () {
        _ if com || mine => None,
        _ if st.peer == "host" => r.ping.get(me),
        _ => r.ping.get(&st.peer),
    };
    let owner = match () {
        _ if com => "Computer".to_string(),
        _ if mine => match m.seats[i.min(3)] {
            Some(super::super::Dev::Keys) => "You · Keyboard".into(),
            Some(super::super::Dev::Pad(_)) => "You · Controller".into(),
            _ => "You".into(),
        },
        _ => match ping {
            Some(ms) => format!("{} · {ms} ms", super::seat_name(m, r, i, &st.peer)),
            None => super::seat_name(m, r, i, &st.peer),
        },
    };
    let state = state(r, i);
    let (chip, chip_words) = match state {
        _ if com => {
            let rank = super::super::com_rank(&text.ranks, st.pick.char, st.pick.costume, r.doubles);
            (Chip::Com, format!("COM · {}", super::super::RANK_NAMES.get(rank).copied().unwrap_or("COM").trim_start_matches("Tennis ")))
        }
        State::Open | State::Ready => (Chip::Ready, "Ready".to_string()),
        State::Picking => (Chip::Picking, "Picking...".to_string()),
        State::Waiting => (Chip::Waiting, "Not ready".to_string()),
    };
    let sub = if random { "Picked at the start".to_string() } else { format!("{} · {}", frame::style_short(text, who), if m.hand(text, i) < 0.0 { "Left" } else { "Right" }) };
    let mod_note = m.remote_mod_note(i);
    let pl = Plate {
        p: i,
        owner: &owner,
        slow: ping.is_some_and(|ms| *ms > 150),
        name: &name,
        sub: &sub,
        chip,
        chip_words: &chip_words,
        dashed: com,
        remove,
        focus,
        remove_focus,
        random,
        hide,
        empty: false,
        mod_note,
        // M95: a person's platform and input (this machine's too, as the lobby has it)
        device: r.devices.get(&st.peer).copied().filter(|_| !com),
    };
    frame::plate(d, &pl, rect, s);
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

    /// Any aspect ratio: every item inside the view, apart (a plate's X on its own plate), the plates on the court.
    #[test]
    fn lays_out_at_any_aspect() {
        for aspect in [4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 1.0, 3.0 / 4.0, 9.0 / 16.0, 0.4] {
            for hosting in [true, false] {
                let mut m = Menu { screen: super::super::super::Screen::Lobby, grid: super::super::super::Grid::fit(aspect), ..default() };
                super::super::sample(&mut m);
                m.net.hosting = hosting;
                let g = m.lobby_geo();
                let [l, t, r, b] = m.grid.reach();
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
                assert!(g.plates.iter().all(|p| p[0] >= g.court[0] && p[0] + p[2] <= g.court[0] + g.court[2] + 0.01), "{aspect}");
            }
        }
    }
}
