//! B396: the lobby, v3 (ours, the approved board "Proposal: Lobby v3"). Along the top the title's tab, the match in a
//! line ("Doubles · 1 set, 4 games") and who's hosting. The seats are equal cards, TEAM 1 · VS · TEAM 2 (singles: one
//! each side of VS); a card has its seat tag, YOU and a white focus ring on this machine's seats, a status chip (Ready ✓,
//! Picking… dashed, COM · Lv N), the host's ✕ that takes the seat's player off, the 3D preview with the stand-in note
//! over its foot for a mod this machine lacks, the name, the "machine · ping" line, the costume and hand chips, the
//! play style and the pink stats card; a computer's or open seat's card has a dashed rim. A taken seat's card follows
//! its player's place on the select live (`Live`: character, costume, hand, Random, ready). Right of the teams (under
//! them in a portrait window) the side panel: the court's picture and name (the host's OK changes it), Mode / Sets /
//! Games / Mods two by two and Umpire across (the host's ‹ › change them), the invite box (QR, the join code, Copy
//! link, the host's Public). Along the bottom Leave lobby, Chat, the help line (the chosen seat's name and style over
//! what its buttons do) and, under the side panel, the host's Start ▶ ("n of N ready · who is still picking") or a
//! guest's Ready / Not ready.
//!
//! The frame's geometry and the card's parts are shared with the confirm screen (B397) in `frame.rs` (B407).

use super::super::frame::{self, BAR_H, CARD, CARD_MAX_W, Chip, GAP, LABEL_H, LEAF, NAVY, RING, SEL_FILL, SIDE_W, SLATE, SOFT, STYLE_GOLD, VS, WELL, dotted, style_of};
use super::super::{Draw, INK, Menu, Tex, WHITE, Who};
use super::{Act, Item, OK, SETTINGS, shared, spinner, ticket_only};
use crate::net::Room;
use bevy::prelude::default;

/// The ✕'s pink.
const REMOVE: [f32; 3] = [107.0, 25.5, 54.0];
/// Stacked teams (a tall window): a row's tallest, and its least for them to stack at all.
const ROW_MAX_H: f32 = 400.0;
const ROW_MIN_H: f32 = 240.0;
/// The side panel's parts: the court card, the invite box's least height.
const COURT_H: f32 = 88.0;
const INVITE_MIN_H: f32 = 56.0;
/// The side panel's settings in the order ↑↓ walk them: court, Mode, Sets, Games, Mods, Umpire.
pub(in super::super) const SIDE: [Item; SETTINGS + 1] = [Item::Set(0), Item::Mode, Item::Set(2), Item::Set(3), Item::Set(4), Item::Set(1)];

/// Where the lobby's parts go (640×448 units, `Grid::reach`).
#[derive(Debug)]
pub(in super::super) struct Geo {
    /// A card a seat, in seat order; the team labels' rows (doubles) and the VS disc between the teams.
    pub cards: Vec<[f32; 4]>,
    pub labels: Vec<[f32; 4]>,
    pub vs: [f32; 4],
    /// The side panel: the court card, the pills (Mode, Sets, Games, Mods two by two, Umpire across), the invite box
    /// with its QR and code (the "Code" item), Copy link and Public.
    #[cfg_attr(not(test), allow(dead_code))]
    pub side: [f32; 4],
    pub court: [f32; 4],
    pub pills: [[f32; 4]; 5],
    pub invite: [f32; 4],
    pub code: [f32; 4],
    pub copy: [f32; 4],
    pub public: [f32; 4],
    /// The bottom bar: Leave lobby, Chat, the help line, Start (a guest's Ready) under the side panel.
    pub leave: [f32; 4],
    pub chat: [f32; 4],
    pub help: [f32; 4],
    pub start: [f32; 4],
}

/// Seat `i`'s state on its card.
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

/// Doubles in a tall window: TEAM 1's two cards over the VS disc over TEAM 2's, a label row over each team, the
/// cards no wider than `CARD_MAX_W` and no taller than `ROW_MAX_H`, the lot centred in `[x, y, w, h]`.
fn teams_stacked(n: usize, [x, y, w, h]: [f32; 4]) -> (Vec<[f32; 4]>, Vec<[f32; 4]>, [f32; 4]) {
    let cw = ((w - GAP) / 2.0).min(CARD_MAX_W);
    let ch = ((h - 2.0 * LABEL_H - VS - 2.0 * GAP) / 2.0).min(ROW_MAX_H);
    let team_w = 2.0 * cw + GAP;
    let (x0, y0) = (x + (w - team_w) / 2.0, y + (h - 2.0 * (LABEL_H + ch) - VS - 2.0 * GAP) / 2.0);
    let team_y = [y0, y0 + LABEL_H + ch + VS + 2.0 * GAP];
    let cards = (0..n).map(|i| [x0 + (i % 2) as f32 * (cw + GAP), team_y[(i / 2).min(1)] + LABEL_H, cw, ch]).collect();
    let labels = team_y.iter().map(|&ty| [x0, ty, team_w, LABEL_H]).collect();
    (cards, labels, [x + (w - VS) / 2.0, y0 + LABEL_H + ch + GAP, VS, VS])
}

/// The invite box's parts: the QR and code (the "Code" item), Copy link and Public along its foot (a guest: Copy link
/// across).
fn invite_parts([x, y, w, h]: [f32; 4], host: bool) -> ([f32; 4], [f32; 4], [f32; 4]) {
    let by = y + h - 24.0;
    let bw = if host { (w - 16.0 - 5.0) / 2.0 } else { w - 16.0 };
    ([x + 4.0, y + 4.0, w - 8.0, by - y - 6.0], [x + 8.0, by, bw, 17.0], [x + 8.0 + bw + 5.0, by, bw, 17.0])
}

impl Menu {
    pub(in super::super) fn lobby_geo(&self) -> Geo {
        let [l, t, r, b] = self.grid.reach();
        let n = self.net.room.as_ref().map_or(if self.doubles { 4 } else { 2 }, |r| r.seats.len());
        // under the title's tab, over the bottom bar
        let (top, foot) = (t + 46.0, b - BAR_H);
        let body = foot - 8.0 - top;
        let w = r - l;
        let portrait = body > w * 1.05;
        let sw = SIDE_W.min(w * 0.4);
        let side_h = 168.0;
        // doubles in a tall window: the teams one over the other
        let stacked = portrait && n == 4 && (body - side_h - 10.0 - 2.0 * LABEL_H - VS - 2.0 * GAP) / 2.0 >= ROW_MIN_H;
        let (room, side) = if stacked {
            let th = (body - side_h - 10.0).min(2.0 * (LABEL_H + ROW_MAX_H) + VS + 2.0 * GAP);
            let y = top + (body - th - 10.0 - side_h) / 2.0;
            ([l, y, w, th], [l, y + th + 10.0, w, side_h])
        } else if portrait {
            // the side panel under the teams, across; the cards no taller than they look right, the lot centred
            let th = frame::portrait_teams_h(n, w, body, side_h);
            let y = top + ((body - th - 10.0 - side_h) / 2.0).max(0.0);
            ([l, y, w, th], [l, y + th + 10.0, w, side_h])
        } else {
            ([l, top, w - sw - 10.0, body], [r - sw, top, sw, body])
        };
        let (cards, labels, vs) = if stacked { teams_stacked(n, room) } else { frame::teams(n, room) };
        let (court, pills, invite) = frame::side_parts(side, portrait, COURT_H, INVITE_MIN_H);
        let (code, copy, public) = invite_parts(invite, self.net.hosting);
        let start = [r - sw, foot, sw, BAR_H];
        let leave = [l, foot, 100.0, BAR_H];
        let chat_on = self.items().contains(&Item::Button("Chat"));
        let chat = [l + 106.0, foot, if chat_on { 76.0 } else { 0.0 }, BAR_H];
        let hx = if chat_on { chat[0] + chat[2] + 6.0 } else { l + 106.0 };
        let help = [hx, foot, start[0] - 8.0 - hx, BAR_H];
        Geo { cards, labels, vs, side, court, pills, invite, code, copy, public, leave, chat, help, start }
    }

    /// Seat `i` is this machine's player's.
    pub(super) fn mine(&self, i: usize) -> bool {
        let me = if self.net.me.is_empty() && self.net.hosting { "host" } else { self.net.me.as_str() };
        self.net.room.as_ref().is_some_and(|r| r.seats.get(i).is_some_and(|s| s.as_ref().is_some_and(|s| s.peer == me)))
    }

    /// Where `items` are drawn.
    pub(in super::super) fn lobby_rects_geo(&self) -> Vec<[f32; 4]> {
        let g = self.lobby_geo();
        self.items()
            .iter()
            .map(|it| match *it {
                Item::Seat(i) => g.cards.get(i).copied().unwrap_or_default(),
                Item::Remove(i) => g.cards.get(i).map_or_else(Default::default, |&c| remove_rect(c)),
                Item::Set(0) => g.court,
                Item::Set(1) => g.pills[4],
                Item::Mode => g.pills[0],
                Item::Set(i) => g.pills[(i - 1).min(3)],
                Item::Button("Start" | "Ready") => g.start,
                Item::Button("Code") => g.code,
                Item::Button("Copy Link") => g.copy,
                Item::Button("Public") => g.public,
                Item::Button("Chat") => g.chat,
                Item::Button(_) => g.leave,
            })
            .collect()
    }
}

/// A card's ✕ (its header's right end).
fn remove_rect([x, y, w, _]: [f32; 4]) -> [f32; 4] {
    [x + w - 22.0, y + 5.0, 16.0, 16.0]
}

/// The lobby screen; `help` is what the chosen item's buttons do.
pub(in super::super) fn draw(d: &mut Draw, m: &Menu, hand_x: f32, help: &str) {
    d.frame(6, 8);
    let room = m.net.room.as_ref();
    let items = m.items();
    let rects = m.lobby_rects();
    let g = m.lobby_geo();
    let at = |it: Item| items.iter().position(|x| *x == it);
    let on = |it: Item| m.net.ask.is_none() && !m.net.qr_big && at(it).is_some_and(|k| k == m.sel);
    let [l, t, r, _] = m.grid.reach();
    if let Some(rm) = room {
        // the top line: the match beside the title's tab, who hosts at the right
        let sets = super::super::SETS.get(rm.sets).copied().unwrap_or(1);
        let summary = format!("{} · {sets} set{}, {} games", if rm.doubles { "Doubles" } else { "Singles" }, if sets > 1 { "s" } else { "" }, rm.games);
        let tab = d.width("Lobby", 30.0) + 90.0;
        let sx = l - 12.0 + tab;
        d.outlined(|d| dotted(d, &summary, sx, t + 12.0, 15.0, (r - sx) * 0.55, STYLE_GOLD, false));
        let machines = rm.seats.iter().flatten().filter(|s| s.human()).map(|s| s.peer.as_str()).collect::<std::collections::BTreeSet<_>>().len();
        let plural = if machines == 1 { "" } else { "s" };
        let who = if m.net.hosting { format!("Hosting · {machines} machine{plural}") } else { format!("{}'s lobby · {machines} machine{plural}", super::name_of(rm, "host")) };
        let ww = (r - sx) * 0.4;
        let wd = d.width(&who, 11.0).min(ww);
        dotted(d, &who, r - wd, t + 15.0, 11.0, ww, LEAF, false);
        // the team labels and the VS disc
        frame::team_labels(d, &g.labels);
        frame::vs_disc(d, g.vs);
        for i in 0..rm.seats.len() {
            let Some(&rect) = g.cards.get(i) else { continue };
            if let Some(k) = at(Item::Seat(i)) {
                d.hit(rect, Act::Row(k, OK));
            }
            card(d, m, rm, i, rect, on(Item::Seat(i)));
            // the host's ✕: takes a guest out, a computer off
            if let Some(k) = at(Item::Remove(i)) {
                let [x, y, w, h] = rects[k];
                let ring = if on(Item::Remove(i)) { super::super::GOLD } else { WHITE };
                d.pill_at([x, y, w, h], [INK, ring, REMOVE], 128.0);
                d.centred("X", x + w / 2.0, y + 3.0, 11.0, WHITE);
                d.hit([x - 2.0, y - 2.0, w + 4.0, h + 4.0], Act::Row(k, OK));
            }
        }
        side_panel(d, m, rm, &g, &items, &on);
        start_button(d, m, rm, &items, &rects, &on);
    }
    for (b, rect, words) in [("Leave", g.leave, "Leave lobby"), ("Chat", g.chat, "Chat")] {
        let Some(k) = at(Item::Button(b)) else { continue };
        let sel = on(Item::Button(b));
        d.pill_at(rect, [INK, if sel { super::super::GOLD } else { WHITE }, if sel { SEL_FILL } else { SLATE }], 128.0);
        d.label(words, rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0 - 7.5, 15.0, rect[2] - 16.0, if sel { INK } else { WHITE });
        d.hit(rect, Act::Row(k, OK));
    }
    if let Some(rm) = room {
        let head = seat_head(m, d.text, rm, items.get(m.sel).copied());
        help_line(d, g.help, &head, help);
    }
    // the hand on the chosen item's left end (a card's: over its top corner, the card can sit on the window's edge)
    if let Some([x, y, _, h]) = rects.get(m.sel).filter(|_| m.net.ask.is_none() && !m.net.qr_big) {
        let (x, y) = if matches!(items.get(m.sel), Some(Item::Seat(_))) { (x - 8.0, y - 10.0) } else { (x - 22.0, y + h / 2.0 - 9.0) };
        d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x + hand_x, y, 36.0, 18.0], WHITE, 128.0);
    }
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

/// The help line's bold first line: the chosen seat's name and play style ("Random" hides it).
fn seat_head(m: &Menu, text: &super::super::Text, r: &Room, it: Option<Item>) -> String {
    let i = match it {
        Some(Item::Seat(i) | Item::Remove(i)) => i,
        _ => return String::new(),
    };
    let Some(s) = r.seats.get(i).and_then(Option::as_ref) else { return format!("{}P · Open seat", i + 1) };
    let (name, who, random) = seat_pick(m, r, i, s);
    if random {
        return name;
    }
    format!("{name} · {}", super::super::STYLE_NAMES[style_of(text, who).min(3)])
}

/// The help line on its light box: the chosen seat's head over what its buttons do, or the words alone on up to two
/// lines.
fn help_line(d: &mut Draw, [x, y, w, h]: [f32; 4], head: &str, s: &str) {
    d.box3([x, y, w, h], 10.0, [None, None, Some(WHITE)], 108.0);
    if w < 30.0 {
        return;
    }
    let size = 10.0;
    let lines = super::super::wrap(s, ((w - 18.0) / (size * 0.5)) as usize);
    if head.is_empty() {
        let lines = if lines.len() > 2 { vec![lines[0].clone(), lines[1..].join(" ")] } else { lines };
        let top = y + (h - 12.0 * lines.len() as f32) / 2.0;
        for (k, l) in lines.iter().enumerate() {
            dotted(d, l, x + 9.0, top + 12.0 * k as f32, size, w - 18.0, NAVY, false);
        }
        return;
    }
    dotted(d, head, x + 9.0, y + 4.0, 13.0, w - 18.0, NAVY, false);
    dotted(d, &lines.join(" "), x + 9.0, y + 21.0, size, w - 18.0, NAVY, false);
}

/// The side panel: the court card, the settings pills, the invite box.
fn side_panel(d: &mut Draw, m: &Menu, r: &Room, g: &Geo, items: &[Item], on: &dyn Fn(Item) -> bool) {
    let text = d.text;
    let at = |it: Item| items.iter().position(|x| *x == it);
    // the host's own choice (Random stays Random), a guest's the lobby's
    let (court, sets, games, umpire) = if m.net.hosting { (m.court, m.sets, m.games, m.umpire as usize) } else { (r.court, r.sets, r.games, r.umpire as usize) };
    // the court card: its picture, COURT over its name, the host's "change"
    let name = if court == super::super::COURTS { "Random".to_string() } else { text.msg(276 + 2 * court).to_string() };
    frame::court_card(d, court, &name, g.court, on(Item::Set(0)), at(Item::Set(0)).is_some());
    if let Some(k) = at(Item::Set(0)) {
        d.hit(g.court, Act::Row(k, OK));
    }
    // the pills: Mode, Sets, Games, Mods, then Umpire across
    let values = [
        (Item::Mode, "MODE", if r.doubles { "Doubles" } else { "Singles" }.to_string()),
        (Item::Set(2), "SETS", super::super::SETS.get(sets).copied().unwrap_or(1).to_string()),
        (Item::Set(3), "GAMES", games.to_string()),
        (Item::Set(4), "MODS", if r.vanilla { "None" } else { "Allowed" }.to_string()),
        (
            Item::Set(1),
            "UMPIRE",
            // M8f3: the host's umpire mod by its name (a guest without it hears the donor)
            match r.umpire_mod.as_ref().filter(|_| !m.net.hosting && !r.vanilla) {
                Some((_, name)) => name.clone(),
                None => m.umpire_name(text, umpire),
            },
        ),
    ];
    for (rect, (it, cap, value)) in g.pills.iter().zip(values) {
        let [x, y, w, h] = *rect;
        let k = at(it);
        // the host's ‹ ›: ←→ change it (a tap on the pill's halves too)
        frame::setting_pill(d, *rect, cap, &value, on(it), k.is_some());
        if let Some(k) = k {
            d.hit(*rect, Act::Row(k, OK));
            d.hit([x, y, w / 2.0, h], Act::Row(k, super::Press { left: true, ..default() }));
            d.hit([x + w / 2.0, y, w / 2.0, h], Act::Row(k, super::Press { right: true, ..default() }));
        }
    }
    // the invite box: the QR (a spinner until the code's there), JOIN CODE over the code, Copy link and Public
    let [x, y, w, h] = g.invite;
    d.box3([x, y, w, h], 12.0, [Some(INK), Some(WHITE), Some(NAVY)], 128.0);
    let [cx, cy, cw, ch] = g.code;
    if on(Item::Button("Code")) {
        d.box3([cx, cy, cw, ch], 8.0, [None, Some(super::super::GOLD), None], 128.0);
    }
    let q = (ch - 6.0).min(cw * 0.45).min(84.0);
    let (qx, qy) = (cx + 4.0, cy + (ch - q) / 2.0);
    if shared(&m.net).is_some() {
        d.box3([qx, qy, q, q], 3.0, [None, None, Some(WHITE)], 128.0);
        d.q(Tex::Qr, [0.0, 0.0, 1.0, 1.0], [qx + 2.0, qy + 2.0, q - 4.0, q - 4.0], WHITE, 128.0);
    } else {
        spinner(d, m.net.spin, qx + q / 2.0, qy + q / 2.0, (q * 0.25).min(12.0));
    }
    let tx = qx + q + 7.0;
    let tw = cx + cw - 4.0 - tx;
    let label = if ticket_only(&m.net) { "JOIN LINK" } else if r.private { "PRIVATE CODE" } else { "JOIN CODE" };
    let mid = cy + ch / 2.0;
    super::super::left(d, label, tx, mid - 15.0, 7.0, tw, [84.5, 92.0, 104.0]);
    let code = if ticket_only(&m.net) { "LINK" } else if m.net.short.is_empty() { "...." } else { m.net.short.as_str() };
    super::super::left(d, code, tx, mid - 6.0, 20.0, tw, STYLE_GOLD);
    if let Some(k) = at(Item::Button("Code")) {
        d.hit(g.code, Act::Row(k, OK));
    }
    let dim = shared(&m.net).is_none();
    let public = !r.private;
    let buttons = [
        (Item::Button("Copy Link"), g.copy, "Copy link".to_string(), [26.0, 35.0, 53.0]),
        (Item::Button("Public"), g.public, if public { "Public".into() } else { "Private".into() }, if public { [23.5, 62.5, 39.5] } else { SLATE }),
    ];
    for (it, rect, words, fill) in buttons {
        let Some(k) = at(it) else { continue };
        let sel = on(it);
        d.pill_at(rect, [INK, if sel { super::super::GOLD } else { WHITE }, if sel { SEL_FILL } else { fill }], 128.0);
        let rgb = if sel { INK } else if dim && it == Item::Button("Copy Link") { [70.0; 3] } else { WHITE };
        let [bx, by, bw, bh] = rect;
        if it == Item::Button("Public") && public {
            // the board's "● Public"
            let tw = d.width(&words, 9.0).min(bw - 18.0);
            let dx = bx + (bw - tw - 8.0) / 2.0;
            d.q(Tex::Solid, [0.0; 4], [dx, by + bh / 2.0 - 2.5, 5.0, 5.0], rgb, 128.0);
            d.out.last_mut().unwrap().round = [2.5; 4];
            d.label(&words, dx + 8.0 + tw / 2.0, by + bh / 2.0 - 4.5, 9.0, bw - 18.0, rgb);
        } else {
            d.label(&words, bx + bw / 2.0, by + bh / 2.0 - 4.5, 9.0, bw - 10.0, rgb);
        }
        d.hit(rect, Act::Row(k, OK));
    }
}

/// The host's Start ▶ ("n of N ready · who is still picking") or a guest's Ready / Not ready.
fn start_button(d: &mut Draw, m: &Menu, r: &Room, items: &[Item], rects: &[[f32; 4]], on: &dyn Fn(Item) -> bool) {
    let Some(k) = items.iter().position(|it| matches!(it, Item::Button("Start" | "Ready"))) else { return };
    let ([x, y, w, h], sel) = (rects[k], on(items[k]));
    let (n, of) = ready_count(r);
    let picking = (0..r.seats.len()).find(|&i| state(r, i) == State::Picking).and_then(|i| r.seats[i].as_ref().map(|s| super::seat_name(m, r, i, &s.peer)));
    let ready = match (&picking, r.open()) {
        (Some(p), _) => format!("{n} of {of} ready · {p} is still picking"),
        (None, 0) => format!("{n} of {of} ready"),
        (None, open) => format!("{n} of {of} ready · +{open} COM"),
    };
    let (big, small, play) = match items[k] {
        _ if m.counting() > 0 => (format!("Cancel ({})", m.counting()), "The match is starting".to_string(), false),
        Item::Button("Start") => ("Start".into(), ready, true),
        _ if m.net.prepared => ("Not ready".into(), format!("{n} of {of} ready · waiting for the host"), false),
        _ => ("Ready".into(), format!("{n} of {of} ready · waiting for the host"), false),
    };
    let fill = if sel { [128.0, 118.0, 60.0] } else { super::super::GOLD };
    d.pill_at([x, y, w, h], [INK, if sel { WHITE } else { NAVY }, fill], 128.0);
    let size = 19.0;
    let bw = d.width(&big, size) + if play { 16.0 } else { 0.0 };
    let bx = x + (w - bw) / 2.0;
    d.text(&big, bx, y + 3.0, size, NAVY);
    if play {
        // ▶: the font has none
        let (tx, ty, s) = (bx + bw - 11.0, y + 7.0, 11.0);
        for c in 0..6 {
            let f = c as f32 / 6.0;
            d.q(Tex::Solid, [0.0; 4], [tx + s * 0.85 * f, ty + s * f / 2.0, s * 0.85 / 6.0 + 0.3, s * (1.0 - f)], NAVY, 128.0);
        }
    }
    dotted(d, &small, x + w / 2.0, y + 23.0, 9.0, w - 26.0, NAVY, true);
    d.hit([x, y, w, h], Act::Row(k, OK));
}

/// Seat `i`'s card name, its character and whether it shows as Random.
fn seat_pick(m: &Menu, r: &Room, i: usize, s: &crate::net::Seat) -> (String, Who, bool) {
    let pl = &m.players[i];
    let live = r.live_of(i).filter(|_| s.human());
    // Random: the card they're on now (live), else the pick they readied
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

/// Seat `i`'s card.
fn card(d: &mut Draw, m: &Menu, r: &Room, i: usize, [x, y, w, h]: [f32; 4], on: bool) {
    let text = d.text;
    let p = i.min(3);
    let seat = r.seats.get(i).and_then(Option::as_ref);
    let com = seat.is_some_and(|s| !s.human());
    let mine = m.mine(i);
    let fill = if seat.is_some() { CARD[p] } else { CARD[p].map(|c| c * 0.6) };
    // this machine's seats: the white focus ring round the card
    if mine {
        d.box3([x - 4.0, y - 4.0, w + 8.0, h + 8.0], 17.0, [None, Some(WHITE), None], 128.0);
    }
    d.box3([x, y, w, h], 13.0, [Some(INK), None, Some(fill)], 128.0);
    let rim = if on { super::super::GOLD } else { RING[p] };
    if seat.is_none() || com {
        super::super::dashed(d, [x + 1.0, y + 1.0, w - 2.0, h - 2.0], rim);
    } else {
        d.box3([x, y, w, h], 13.0, [None, Some(rim), None], 128.0);
    }
    let rows = frame::rows([x, y, w, h]);
    let (ix, iw) = (rows.ix, rows.iw);
    // the header: seat tag, YOU, the status chip, the host's ✕
    let tag_w = frame::seat_tag(d, i, ix, y + 6.0);
    let mut hx = ix + tag_w + 4.0;
    if mine {
        d.text("YOU", hx, y + 9.0, 8.5, RING[p]);
        hx += d.width("YOU", 8.5) + 4.0;
    }
    let host_x = m.net.hosting && seat.is_some() && !mine;
    let right = if host_x { x + w - 26.0 } else { x + w - 6.0 };
    let st = state(r, i);
    let level = || {
        let s = seat.map_or(&r.seats[0], |_| &r.seats[i]).as_ref();
        let rank = s.map_or(0, |s| super::super::com_rank(&text.ranks, s.pick.char, s.pick.costume, r.doubles));
        super::super::RANK_NAMES.get(rank).copied().unwrap_or("COM").trim_start_matches("Tennis ").to_string()
    };
    let chip_words = match st {
        _ if com => format!("COM · {}", level()),
        State::Open => "Open".into(),
        State::Ready => "Ready".into(),
        State::Picking => "Picking...".into(),
        State::Waiting => "Not ready".into(),
    };
    let kind = match st {
        _ if com => Chip::Com,
        State::Open => Chip::Open,
        State::Ready => Chip::Ready,
        State::Picking => Chip::Picking,
        State::Waiting => Chip::Waiting(SOFT[p]),
    };
    frame::chip(d, &chip_words, kind, right, y + 6.0, (right - hx).max(24.0));
    // from the foot up: stats, style, chips, the name's two lines; the preview takes the rest
    let (well, name_y, sub_y, chips_y) = (rows.well, rows.name_y, rows.sub_y, rows.chips_y);
    let Some(s) = seat else {
        // an open seat: the computer until someone joins
        d.box3(well, 8.0, [None, None, Some(WELL[p])], 128.0);
        let size = (well[3] - 10.0).min(iw - 10.0).clamp(10.0, 56.0);
        super::super::face_frame(d, super::super::cell_face(None), ix + iw / 2.0, well[1] + well[3] / 2.0, size, WHITE);
        d.outlined(|d| super::super::left(d, "Open seat", ix, name_y, 16.0, iw, WHITE));
        super::super::left(d, "Computer until someone joins", ix, sub_y, 8.5, iw, SOFT[p]);
        let words = if m.net.hosting { "OK adds a computer player now; left open, Start fills it." } else { "Anyone with the code can take it; left open, the computer plays it." };
        for (k, l) in super::super::wrap(words, (iw / 4.6) as usize).iter().take(6).enumerate() {
            super::super::left(d, l, ix + 2.0, chips_y + 2.0 + 11.0 * k as f32, 8.5, iw - 4.0, SOFT[p]);
        }
        return;
    };
    let (name, who, random) = seat_pick(m, r, i, s);
    let costume = m.pick(i).1;
    // the machine and its ping: this machine's device, the player's name and ping, or the computer
    let me = if m.net.hosting { "host" } else { m.net.me.as_str() };
    let ping = match () {
        _ if com || mine => None,
        _ if s.peer == "host" => r.ping.get(me),
        _ => r.ping.get(&s.peer),
    };
    let sub = match () {
        _ if com => "Computer".to_string(),
        _ if mine => match m.seats[p] {
            Some(super::super::Dev::Keys) => "Keyboard · this machine".into(),
            Some(super::super::Dev::Pad(_)) => "Controller · this machine".into(),
            _ => "This machine".into(),
        },
        _ => match ping {
            Some(ms) => format!("{} · {ms} ms", super::seat_name(m, r, i, &s.peer)),
            None => super::seat_name(m, r, i, &s.peer),
        },
    };
    // the preview, the stand-in note over its foot
    frame::preview(d, i, random, m.net.ask.is_some() || m.net.qr_big, well, WELL[p]);
    if let Some(n) = m.remote_mod_name(i) {
        frame::stand_in(d, n, &rows);
    }
    d.outlined(|d| super::super::left(d, &name, ix, name_y, 16.0, iw, WHITE));
    dotted(d, &sub, ix, sub_y, 8.5, iw, if ping.is_some_and(|ms| *ms > 150) { STYLE_GOLD } else { SOFT[p] }, false);
    // the costume and hand chips, the play style over the pink stats card
    frame::card_foot(d, text, p, &rows, who, random, costume, m.hand(text, i));
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

    #[test]
    fn equal_cards_either_side_of_vs() {
        // doubles: two equal cards a team, the VS disc between, the label rows over them
        let (cards, labels, vs) = frame::teams(4, [0.0, 0.0, 400.0, 300.0]);
        assert!(cards.iter().all(|c| (c[2] - cards[0][2]).abs() < 1e-3 && c[3] == 300.0 - LABEL_H));
        assert!(cards[1][0] + cards[1][2] < vs[0] && vs[0] + vs[2] < cards[2][0]);
        assert_eq!(labels.len(), 2);
        // singles: one a side, no labels; a very wide room keeps the cards narrow and centred
        let (cards, labels, vs) = frame::teams(2, [0.0, 0.0, 1000.0, 300.0]);
        assert_eq!((cards.len(), labels.len(), cards[0][2]), (2, 0, CARD_MAX_W));
        assert!(((cards[0][0] - 0.0) - (1000.0 - cards[1][0] - cards[1][2])).abs() < 1e-3);
        assert!((vs[0] + vs[2] / 2.0 - 500.0).abs() < 1e-3);
    }

    /// Any aspect ratio: everything inside the view, nothing overlapping, the side panel beside the teams or (portrait)
    /// under them.
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
                let mut parts: Vec<[f32; 4]> = g.cards.clone();
                parts.extend([g.side, g.leave, g.help, g.start]);
                for (k, a) in parts.iter().enumerate() {
                    assert!(inside(a), "{aspect} {k} {a:?}");
                    for c in &parts[k + 1..] {
                        assert!(apart(a, c), "{aspect}: {a:?} over {c:?}");
                    }
                }
                assert!(g.pills.iter().chain([&g.court, &g.invite]).all(|p| inside(p) && p[1] >= g.side[1] - 0.01));
                assert!(g.cards.iter().all(|c| c[2] > 80.0 && c[3] > 180.0), "{aspect}: {:?}", g.cards);
                let under = g.cards.iter().all(|c| g.side[1] >= c[1] + c[3]);
                assert_eq!(under, aspect < 1.0, "{aspect}: the side panel under the teams only in portrait");
            }
        }
    }
}
