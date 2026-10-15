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

use super::super::{Draw, INK, Menu, Tex, WHITE, Who};
use super::{Act, Item, OK, SETTINGS, shared, spinner, ticket_only};
use crate::net::Room;
use bevy::prelude::default;

/// Seat colours (1P red, 2P yellow, 3P blue, 4P green), the board's: ring, card, well, soft words.
const RING: [[f32; 3]; 4] = [[127.5, 53.5, 53.5], [127.5, 105.0, 31.5], [46.0, 100.0, 127.5], [63.5, 108.5, 77.0]];
const CARD: [[f32; 3]; 4] = [[37.0, 14.0, 14.0], [30.5, 24.0, 5.5], [9.0, 22.5, 34.0], [10.5, 20.5, 14.5]];
const WELL: [[f32; 3]; 4] = [[47.0, 15.5, 15.5], [39.0, 30.0, 6.0], [10.5, 29.0, 43.5], [12.0, 24.0, 17.0]];
const SOFT: [[f32; 3]; 4] = [[127.5, 104.0, 104.0], [127.5, 120.0, 92.0], [102.5, 119.0, 127.5], [100.5, 121.0, 106.0]];
/// The play style's gold, the stats card's pink and its rules, each grade's colour (F, E, D, C, B, A: grade 0..5).
const STYLE_GOLD: [f32; 3] = [127.5, 105.0, 31.5];
const PINK: [f32; 3] = [122.5, 69.0, 96.0];
const RULE: [f32; 3] = [127.5, 97.0, 111.5];
const GRADE: [[f32; 3]; 6] = [[67.0, 71.0, 75.0], [78.0, 27.0, 90.5], [14.0, 63.0, 107.0], [23.5, 79.0, 34.0], [107.0, 25.5, 54.0], [120.0, 70.0, 0.0]];
const STATS: [&str; 5] = ["Serve", "Stroke", "Volley", "Impact", "Footwork"];
/// The board's navy (VS, the invite box, words on light boxes), the dark green over the wall, the grey pills, the
/// ready chip's green, the ✕'s pink, the light boxes' cream and their soft caption.
const NAVY: [f32; 3] = [14.5, 21.5, 34.5];
const LEAF: [f32; 3] = [15.5, 29.0, 11.5];
const LEAF_RULE: [f32; 3] = [55.5, 87.0, 45.0];
const SLATE: [f32; 3] = [54.0, 55.5, 71.0];
const READY: [f32; 3] = [23.5, 84.0, 48.0];
const REMOVE: [f32; 3] = [107.0, 25.5, 54.0];
const PAPER: [f32; 3] = [125.5, 123.5, 117.0];
const CAPTION: [f32; 3] = [53.5, 55.0, 71.5];
/// The board at 1280×720 is 0.6 of it in 640×448 units: the side panel's width (300), the bottom bar's height (64),
/// the VS disc (56), the gap between the cards and the parts.
const SIDE_W: f32 = 180.0;
const BAR_H: f32 = 38.0;
const VS: f32 = 34.0;
const GAP: f32 = 7.0;
/// A card's widest (a very wide window: the teams keep to the middle), its portrait height per width, the team
/// label's row.
const CARD_MAX_W: f32 = 210.0;
const LABEL_H: f32 = 14.0;
/// Stacked teams (a tall window): a row's tallest, and its least for them to stack at all.
const ROW_MAX_H: f32 = 400.0;
const ROW_MIN_H: f32 = 240.0;
/// The side panel's parts: the court card, the pills' height and the gap between them.
const COURT_H: f32 = 88.0;
const PILL_H: f32 = 28.0;
const PILL_GAP: f32 = 4.0;
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

/// The cards of `n` seats in the teams' room `[x, y, w, h]`: two teams either side of the VS disc, a label row over
/// each in doubles; the cards no wider than `CARD_MAX_W`, the lot centred.
fn teams(n: usize, [x, y, w, h]: [f32; 4]) -> (Vec<[f32; 4]>, Vec<[f32; 4]>, [f32; 4]) {
    let per = (n / 2).max(1);
    let label = if per > 1 { LABEL_H } else { 0.0 };
    let cw = ((w - VS - 2.0 * GAP - 2.0 * (per - 1) as f32 * GAP) / (2 * per) as f32).min(CARD_MAX_W);
    let team_w = per as f32 * cw + (per - 1) as f32 * GAP;
    let x0 = x + (w - 2.0 * team_w - VS - 2.0 * GAP) / 2.0;
    let team_x = [x0, x0 + team_w + VS + 2.0 * GAP];
    let cards = (0..n).map(|i| [team_x[(i / per).min(1)] + (i % per) as f32 * (cw + GAP), y + label, cw, h - label]).collect();
    let labels = if per > 1 { team_x.iter().map(|&tx| [tx, y, team_w, LABEL_H]).collect() } else { vec![] };
    let vs = [x0 + team_w + GAP, y + label + (h - label - VS) / 2.0, VS, VS];
    (cards, labels, vs)
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

/// The side panel's parts in `[x, y, w, h]`: a column (beside the teams), or a wide strip (under them in a portrait
/// window) with the court card left and the pills over the invite box right.
fn side_parts([x, y, w, h]: [f32; 4], wide: bool) -> ([f32; 4], [[f32; 4]; 5], [f32; 4]) {
    let (court, px, py, pw) = if wide {
        let cw = (w - GAP) / 2.0;
        ([x, y, cw, h], x + cw + GAP, y, cw)
    } else {
        ([x, y, w, COURT_H], x, y + COURT_H + 6.0, w)
    };
    let half = (pw - PILL_GAP) / 2.0;
    let row = |k: usize| py + (PILL_H + PILL_GAP) * k as f32;
    let pills = [[px, row(0), half, PILL_H], [px + half + PILL_GAP, row(0), half, PILL_H], [px, row(1), half, PILL_H], [px + half + PILL_GAP, row(1), half, PILL_H], [px, row(2), pw, PILL_H]];
    let top = row(3) + 2.0;
    (court, pills, [px, top, pw, (y + h - top).max(56.0)])
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
            let per = (n / 2).max(1) as f32;
            let cw = ((w - VS - 2.0 * GAP - 2.0 * (per - 1.0) * GAP) / (2.0 * per)).min(CARD_MAX_W);
            let th = (body - side_h - 10.0).min(cw * 2.7 + LABEL_H).max(200.0);
            let y = top + ((body - th - 10.0 - side_h) / 2.0).max(0.0);
            ([l, y, w, th], [l, y + th + 10.0, w, side_h])
        } else {
            ([l, top, w - sw - 10.0, body], [r - sw, top, sw, body])
        };
        let (cards, labels, vs) = if stacked { teams_stacked(n, room) } else { teams(n, room) };
        let (court, pills, invite) = side_parts(side, portrait);
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

/// `s` from `x` at `size`, a "·" drawn as a dot (the font has none), squeezed to fit `max`; `centre` centres it on `x`.
fn dotted(d: &mut Draw, s: &str, x: f32, y: f32, size: f32, max: f32, rgb: [f32; 3], centre: bool) {
    let parts: Vec<&str> = s.split('·').collect();
    let dot = size * 0.5;
    let full: f32 = parts.iter().map(|p| d.width(p, size)).sum::<f32>() + dot * (parts.len() - 1) as f32;
    let k = (max / full.max(1.0)).min(1.0);
    let x0 = if centre { x - full * k / 2.0 } else { x };
    let start = d.out.len();
    let mut at = x0;
    for (j, p) in parts.iter().enumerate() {
        d.text(p, at, y, size, rgb);
        at += d.width(p, size);
        if j + 1 < parts.len() {
            let r = size * 0.09;
            d.q(Tex::Solid, [0.0; 4], [at + dot / 2.0 - r, y + size * 0.48 - r, 2.0 * r, 2.0 * r], rgb, 128.0);
            d.out.last_mut().unwrap().round = [r; 4];
            at += dot;
        }
    }
    // squeezed across only: the words keep their height
    for q in &mut d.out[start..] {
        q.dst[0] = x0 + (q.dst[0] - x0) * k;
        q.dst[2] *= k;
    }
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
        for (k, &[x, y, w, _]) in g.labels.iter().enumerate() {
            let words = format!("TEAM {}", k + 1);
            let tw = d.width(&words, 10.0);
            d.text(&words, x + 1.0, y + 1.0, 10.0, LEAF);
            d.q(Tex::Solid, [0.0; 4], [x + tw + 6.0, y + 6.0, (w - tw - 6.0).max(0.0), 1.5], LEAF_RULE, 128.0);
        }
        let [vx, vy, vw, _] = g.vs;
        d.pill_at([vx, vy, vw, vw], [INK, WHITE, NAVY], 128.0);
        d.centred("VS", vx + vw / 2.0, vy + vw / 2.0 - 8.0, 16.0, [127.5, 77.5, 30.5]);
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
        d.pill_at(rect, [INK, if sel { super::super::GOLD } else { WHITE }, if sel { [128.0, 108.0, 37.0] } else { SLATE }], 128.0);
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
    let [x, y, w, h] = g.court;
    let sel = on(Item::Set(0));
    d.box3([x, y, w, h], 12.0, [Some(INK), Some(if sel { super::super::GOLD } else { WHITE }), Some(PAPER)], 128.0);
    let ph = h - 34.0;
    let pic = [x + 3.0, y + 3.0, w - 6.0, ph];
    let first = d.out.len();
    super::super::court_picture(d, court, pic);
    let rr = 12.0 * 11.0 / 16.0 - (3.0 - 12.0 * 4.0 / 16.0);
    d.out[first..].iter_mut().filter(|q| q.dst == pic).for_each(|q| q.round = [rr, rr, 0.0, 0.0]);
    let name = if court == super::super::COURTS { "Random".to_string() } else { text.msg(276 + 2 * court).to_string() };
    let ny = y + 3.0 + ph + 3.0;
    d.text("COURT", x + 9.0, ny, 7.0, CAPTION);
    let change = at(Item::Set(0)).map(|_| "OK: change");
    let cw = change.map_or(0.0, |c| d.width(c, 8.0) + 6.0);
    super::super::left(d, &name, x + 9.0, ny + 9.0, 15.0, w - 18.0 - cw, NAVY);
    if let Some(c) = change {
        d.text(c, x + w - 9.0 - d.width(c, 8.0), ny + 13.0, 8.0, CAPTION);
    }
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
        let sel = on(it);
        d.pill_at(*rect, [INK, if sel { super::super::GOLD } else { WHITE }, if sel { [128.0, 108.0, 37.0] } else { SLATE }], 128.0);
        let (cap_rgb, rgb) = if sel { (INK, INK) } else { ([108.0, 109.0, 117.0], WHITE) };
        d.text(cap, x + 12.0, y + 3.5, 6.5, cap_rgb);
        // the host's ‹ ›: ←→ change it (a tap on the pill's halves too)
        let arrows = k.is_some();
        if arrows {
            let a = if sel { INK } else { [100.5, 101.5, 112.0] };
            d.text("<", x + 9.0, y + 11.0, 12.0, a);
            d.text(">", x + w - 9.0 - d.width(">", 12.0), y + 11.0, 12.0, a);
        }
        d.label(&value, x + w / 2.0, y + 11.0, 12.0, w - 34.0, rgb);
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
        d.pill_at(rect, [INK, if sel { super::super::GOLD } else { WHITE }, if sel { [128.0, 108.0, 37.0] } else { fill }], 128.0);
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

fn style_of(text: &super::super::Text, who: Who) -> usize {
    match who {
        Who::Disc(c) => text.style[c.min(13)],
        Who::Mod(j) => text.mods.get(j).map_or(0, |t| t.0),
    }
}

/// The status chip ending at `right` on the header row at `y`, at most `max` wide: Ready ✓ (green), Picking… (dashed
/// gold), Not ready (dashed soft), COM · Lv N (white), Open (dashed white); its left edge.
fn chip(d: &mut Draw, words: &str, kind: State, com: bool, right: f32, y: f32, max: f32, soft: [f32; 3]) -> f32 {
    let size = 8.5;
    let tick = kind == State::Ready && !com;
    let tw = d.width(words, size) + if tick { 10.0 } else { 0.0 };
    let w = (tw + 12.0).min(max);
    let (x, h) = (right - w, 14.0);
    let (fill, ink, ring, dashed) = match kind {
        _ if com => (Some(WHITE), NAVY, WHITE, false),
        State::Ready => (Some(READY), WHITE, WHITE, false),
        State::Picking => (None, STYLE_GOLD, STYLE_GOLD, true),
        State::Waiting => (None, soft, soft, true),
        State::Open => (None, WHITE, WHITE, true),
    };
    if dashed {
        dashes(d, [x, y, w, h], ring);
    } else {
        d.pill_at([x, y, w, h], [INK, ring, fill.unwrap_or(ring)], 128.0);
    }
    let inner = w - 12.0 - if tick { 10.0 } else { 0.0 };
    let words_w = d.width(words, size).min(inner);
    let wx = x + (w - words_w - if tick { 10.0 } else { 0.0 }) / 2.0;
    dotted(d, words, wx, y + 2.5, size, inner, ink, false);
    if tick {
        self::tick(d, wx + words_w + 3.0, y + 3.0, 8.0, ink);
    }
    x
}

/// A chip's dashed rim: dashes along a pill's straight edges, its round ends as short arcs of dashes.
fn dashes(d: &mut Draw, [x, y, w, h]: [f32; 4], rgb: [f32; 3]) {
    let r = h / 2.0;
    let (dash, gap, t) = (3.5f32, 2.5, 1.3);
    let mut i = 0.0;
    while i < w - 2.0 * r {
        let l = dash.min(w - 2.0 * r - i);
        d.q(Tex::Solid, [0.0; 4], [x + r + i, y, l, t], rgb, 128.0);
        d.q(Tex::Solid, [0.0; 4], [x + r + i, y + h - t, l, t], rgb, 128.0);
        i += dash + gap;
    }
    for (cx, a0) in [(x + r, std::f32::consts::FRAC_PI_2), (x + w - r, -std::f32::consts::FRAC_PI_2)] {
        for s in 0..3 {
            let a = a0 + (s as f32 + 0.5) * std::f32::consts::PI / 3.0;
            let (px, py) = (cx + (r - t / 2.0) * a.cos(), y + r - (r - t / 2.0) * a.sin());
            d.q(Tex::Solid, [0.0; 4], [px - dash * 0.4, py - t / 2.0, dash * 0.8, t], rgb, 128.0);
            d.out.last_mut().unwrap().angle = -(a + std::f32::consts::FRAC_PI_2);
        }
    }
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
    let (ix, iw) = (x + 6.0, w - 12.0);
    // the header: seat tag, YOU, the status chip, the host's ✕
    let tag = format!("{}P", i + 1);
    let tag_w = d.width(&tag, 10.0) + 8.0;
    d.box3([ix, y + 6.0, tag_w, 14.0], 4.0, [None, None, Some(RING[p])], 128.0);
    d.text(&tag, ix + 4.0, y + 8.0, 10.0, NAVY);
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
    chip(d, &chip_words, st, com, right, y + 6.0, (right - hx).max(24.0), SOFT[p]);
    // from the foot up: stats, style, chips, the name's two lines; the preview takes the rest
    let foot = y + h - 6.0;
    let row = 11.5;
    let stats_h = 5.0 * row + 4.0;
    let stats_y = foot - stats_h;
    let style_y = stats_y - 15.0;
    let chips_y = style_y - 15.0;
    let sub_y = chips_y - 12.0;
    let name_y = sub_y - 17.0;
    let well = [ix, y + 25.0, iw, name_y - 3.0 - (y + 25.0)];
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
    preview(d, i, random, m.net.ask.is_some() || m.net.qr_big, well, WELL[p]);
    if let Some(n) = m.remote_mod_name(i) {
        let note = format!("{n}: you don't have it, so you'll see its stand-in");
        let lines = super::super::wrap(&note, ((iw - 10.0) / 4.4).max(8.0) as usize);
        let nh = 5.0 + 9.5 * lines.len().min(3) as f32;
        let ny = well[1] + well[3] - 4.0 - nh;
        d.box3([ix + 4.0, ny, iw - 8.0, nh], 5.0, [None, None, Some(STYLE_GOLD)], 128.0);
        for (k, l) in lines.iter().take(3).enumerate() {
            super::super::left(d, l, ix + 8.0, ny + 3.0 + 9.5 * k as f32, 8.0, iw - 16.0, NAVY);
        }
    }
    d.outlined(|d| super::super::left(d, &name, ix, name_y, 16.0, iw, WHITE));
    dotted(d, &sub, ix, sub_y, 8.5, iw, if ping.is_some_and(|ms| *ms > 150) { STYLE_GOLD } else { SOFT[p] }, false);
    // the chips: costume (the shirt) and hand
    let hand = m.hand(text, i);
    let words = [
        if random { "Costume ?".to_string() } else { format!("Costume {}", costume + 1) },
        if random { "?".into() } else if hand < 0.0 { "Left".into() } else { "Right".into() },
    ];
    let cw = (iw - 4.0) / 2.0;
    for (k, words) in words.iter().enumerate() {
        let px = ix + (cw + 4.0) * k as f32;
        d.pill_at([px, chips_y, cw, 12.0], [WELL[p], WELL[p], WELL[p]], 128.0);
        let lx = if k == 0 && cw > 46.0 {
            super::super::shirt(d, [px + 5.0, chips_y + 2.5, 8.0, 7.0], RING[p], WELL[p]);
            px + 15.0
        } else {
            px + 4.0
        };
        d.label(words, (lx + px + cw - 4.0) / 2.0, chips_y + 2.0, 8.5, px + cw - 4.0 - lx, WHITE);
    }
    // the play style over the pink stats card
    d.box3([ix, style_y, iw, 12.0], 3.0, [None, None, Some(WELL[p])], 128.0);
    let style = if random { "?" } else { super::super::STYLE_NAMES[style_of(text, who).min(3)] };
    d.label(style, ix + iw / 2.0, style_y + 2.0, 9.0, iw - 8.0, STYLE_GOLD);
    stats(d, text, who, random, [ix, stats_y, iw, stats_h], row);
}

/// The 3D preview (`previews.rs`: `Tex::Preview(i)`), 4:5 and as big as `[x, y, w, h]` allows, on the seat's well;
/// the Random card's "?" tile; just the well with `hide`.
fn preview(d: &mut Draw, i: usize, random: bool, hide: bool, [x, y, w, h]: [f32; 4], well: [f32; 3]) {
    if h < 12.0 {
        return;
    }
    d.box3([x, y, w, h], 8.0, [None, None, Some(well)], 128.0);
    if random {
        let fw = (w - 8.0).min((h - 8.0) * 92.0 / 68.0);
        d.q(super::super::C02, super::super::RANDOM_TILE, [x + (w - fw) / 2.0, y + (h - fw * 68.0 / 92.0) / 2.0, fw, fw * 68.0 / 92.0], WHITE, 128.0);
        return;
    }
    if hide {
        return;
    }
    let ph = h - 4.0;
    let pw = (ph * super::super::previews::CARD_W).min(w - 4.0);
    let ph = pw / super::super::previews::CARD_W;
    d.q(Tex::Preview(i), [0.0; 4], [x + (w - pw) / 2.0, y + (h - ph) / 2.0, pw, ph], WHITE, 128.0);
}

/// The pink stats card: five rows `row` high, each a label and its grade's letter in a light circle; "?" for Random.
fn stats(d: &mut Draw, text: &super::super::Text, who: Who, random: bool, [x, y, w, h]: [f32; 4], row: f32) {
    let grades = super::super::grades_of(text, who);
    d.box3([x, y, w, h], 3.0, [None, None, Some(PINK)], 128.0);
    for (k, label) in STATS.iter().enumerate() {
        let ry = y + 2.0 + row * k as f32;
        if k > 0 {
            d.q(Tex::Solid, [0.0; 4], [x + 3.0, ry - 0.5, w - 6.0, 1.2], RULE, 128.0);
        }
        d.outlined(|d| super::super::left(d, label, x + 5.0, ry + 1.5, 8.5, w - 22.0, WHITE));
        let gs = row - 2.0;
        let gx = x + w - 3.0 - gs;
        d.pill_at([gx, ry + 1.0, gs, gs], [[126.0; 3], [126.0; 3], [126.0; 3]], 128.0);
        let g = grades[k].min(5) as usize;
        let (letter, rgb) = if random { ("?", GRADE[0]) } else { (&"FEDCBA"[g..g + 1], GRADE[g]) };
        d.label(letter, gx + gs / 2.0, ry + 1.5, 8.0, gs, rgb);
    }
}

/// A tick mark `s` high from (x, y), two strokes.
fn tick(d: &mut Draw, x: f32, y: f32, s: f32, rgb: [f32; 3]) {
    for (dst, a) in [([x, y + s * 0.55, s * 0.45, 1.6], 0.8), ([x + s * 0.25, y + s * 0.35, s * 0.8, 1.6], -1.0)] {
        d.q(Tex::Solid, [0.0; 4], dst, rgb, 128.0);
        d.out.last_mut().unwrap().angle = a;
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

    #[test]
    fn equal_cards_either_side_of_vs() {
        // doubles: two equal cards a team, the VS disc between, the label rows over them
        let (cards, labels, vs) = teams(4, [0.0, 0.0, 400.0, 300.0]);
        assert!(cards.iter().all(|c| (c[2] - cards[0][2]).abs() < 1e-3 && c[3] == 300.0 - LABEL_H));
        assert!(cards[1][0] + cards[1][2] < vs[0] && vs[0] + vs[2] < cards[2][0]);
        assert_eq!(labels.len(), 2);
        // singles: one a side, no labels; a very wide room keeps the cards narrow and centred
        let (cards, labels, vs) = teams(2, [0.0, 0.0, 1000.0, 300.0]);
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
