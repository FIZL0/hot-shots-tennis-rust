//! B370: the lobby as seat cards (plan/B368.md § Online lobby, ours after the mockup's "Online lobby" board). The top
//! bar has the join code with its QR (OK or a tap opens it big) and the host's Start ("n of 4 ready"; a guest's Ready
//! and "Waiting for host"). The settings row sits under it. Then a card per seat: this machine's players share the
//! wide part (2fr), every other seat is one narrow card (1fr; two rows of two in a portrait window). A taken seat's
//! card follows its player's place on the select live (`Live`: character, costume, hand, Random, ready): the
//! character, costume and hand pills, the preview, the play style over the pink stats card, Ready ✓ / Picking…, the
//! stand-in note for a mod this machine lacks, the ping and the host's action. A computer's or open seat's card has a
//! dashed rim. Along the bottom: Characters, Copy link, Public, Leave and the help line.

use super::super::{Draw, INK, Menu, Tex, WHITE, Who};
use super::{Act, Item, OK, SETTINGS, shared, spinner, ticket_only};
use crate::net::Room;

/// Seat colours (1P red, 2P yellow, 3P blue, 4P green), the mockup's: ring, card, well, soft words.
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
/// The top bar's code pill and Start widths, and the gap between the cards.
const BAR_W: f32 = 170.0;
const GAP: f32 = 8.0;

/// Where the lobby's parts go (640×448 units, `Grid::reach`).
pub(in super::super) struct Geo {
    pub code: [f32; 4],
    pub start: [f32; 4],
    pub row: [[f32; 4]; SETTINGS + 1],
    /// A card a seat, in seat order.
    pub cards: Vec<[f32; 4]>,
    /// The bottom row: its buttons from the left, the help line after them.
    pub bottom: [f32; 4],
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

/// The ready seats and all of them, for Start's "n of 4 ready".
pub(in super::super) fn ready_count(r: &Room) -> (usize, usize) {
    ((0..r.seats.len()).filter(|&i| state(r, i) == State::Ready).count(), r.seats.len())
}

/// The cards' columns: this machine's seats share `2fr` (`mine`), every other seat has `1fr`; across `[x, w]`.
pub(in super::super) fn columns(mine: &[bool], x: f32, w: f32) -> Vec<(f32, f32)> {
    let k = mine.iter().filter(|m| **m).count();
    let weight = |m: bool| if m { 2.0 / k as f32 } else { 1.0 };
    let total: f32 = mine.iter().map(|&m| weight(m)).sum();
    let unit = (w - GAP * (mine.len().max(1) - 1) as f32) / total;
    let mut at = x;
    mine.iter()
        .map(|&m| {
            let c = (at, weight(m) * unit);
            at += c.1 + GAP;
            c
        })
        .collect()
}

impl Menu {
    pub(in super::super) fn lobby_geo(&self) -> Geo {
        let [l, t, r, b] = self.grid.reach();
        // the top bar right of the title's tab: the code pill, then Start
        let bw = BAR_W.min((r - l - 150.0 - GAP) / 2.0);
        let start = [r - bw, t + 20.0, bw, 36.0];
        let code = [r - 2.0 * bw - GAP, t + 20.0, bw, 36.0];
        const PARTS: [f32; SETTINGS + 1] = [1.7, 1.15, 0.8, 0.8, 1.0, 1.05];
        let unit = (r - l - 6.0 * SETTINGS as f32) / PARTS.iter().sum::<f32>();
        let mut at = l;
        let row = PARTS.map(|p| {
            let rect = [at, t + 62.0, p * unit, 36.0];
            at += p * unit + 6.0;
            rect
        });
        let (top, bottom) = (t + 104.0, b - 40.0);
        let n = self.net.room.as_ref().map_or(if self.doubles { 4 } else { 2 }, |r| r.seats.len());
        let mine: Vec<bool> = (0..n).map(|i| self.mine(i)).collect();
        // a narrow or portrait window: two rows of two
        let cards = if (r - l < 480.0 || bottom - top > r - l) && n > 2 {
            let h = (bottom - top - GAP) / 2.0;
            (0..n)
                .map(|i| {
                    let pair = [i & !1, i | 1].map(|j| mine.get(j).copied().unwrap_or(false));
                    let (x, w) = columns(&pair, l, r - l)[i & 1];
                    [x, top + (h + GAP) * (i / 2) as f32, w, h]
                })
                .collect()
        } else {
            columns(&mine, l, r - l).into_iter().map(|(x, w)| [x, top, w, bottom - top]).collect()
        };
        Geo { code, start, row, cards, bottom: [l, b - 34.0, r - l, 30.0] }
    }

    /// Seat `i` is this machine's player's.
    fn mine(&self, i: usize) -> bool {
        let me = if self.net.me.is_empty() && self.net.hosting { "host" } else { self.net.me.as_str() };
        self.net.room.as_ref().is_some_and(|r| r.seats.get(i).is_some_and(|s| s.as_ref().is_some_and(|s| s.peer == me)))
    }

    /// The bottom row's buttons: (item, label width).
    fn bottom_buttons(&self) -> Vec<(&'static str, f32)> {
        let all = [("Characters", 110.0), ("Copy Link", 92.0), ("Public", 82.0), ("Leave", 104.0)];
        let items = self.items();
        all.into_iter().filter(|(b, _)| items.contains(&Item::Button(b))).collect()
    }

    /// Where `items` are drawn.
    pub(in super::super) fn lobby_rects_geo(&self) -> Vec<[f32; 4]> {
        let g = self.lobby_geo();
        let mut x = g.bottom[0];
        let mut bottom = std::collections::HashMap::new();
        for (b, w) in self.bottom_buttons() {
            bottom.insert(b, [x, g.bottom[1], w, g.bottom[3]]);
            x += w + 6.0;
        }
        self.items()
            .iter()
            .map(|it| match *it {
                Item::Seat(i) => g.cards.get(i).copied().unwrap_or_default(),
                Item::Remove(i) => {
                    let [x, y, w, _] = g.cards.get(i).copied().unwrap_or_default();
                    [x + w - 26.0, y + 4.0, 22.0, 22.0]
                }
                Item::Mode => g.row[SETTINGS],
                Item::Set(i) => g.row[i],
                Item::Button("Start" | "Ready") => g.start,
                Item::Button("Code") => g.code,
                Item::Button(b) => bottom.get(b).copied().unwrap_or_default(),
            })
            .collect()
    }

    /// The help line's box: the bottom row after its buttons.
    fn help_rect(&self) -> [f32; 4] {
        let g = self.lobby_geo();
        let used: f32 = self.bottom_buttons().iter().map(|b| b.1 + 6.0).sum();
        [g.bottom[0] + used, g.bottom[1] - 2.0, g.bottom[2] - used, g.bottom[3] + 4.0]
    }
}

/// The lobby screen.
pub(in super::super) fn draw(d: &mut Draw, m: &Menu, hand_x: f32, help: &str) {
    let text = d.text;
    d.frame(6, 8);
    let room = m.net.room.as_ref();
    let items = m.items();
    let rects = m.lobby_rects();
    let g = m.lobby_geo();
    let at = |it: Item| items.iter().position(|x| *x == it);
    let on = |it: Item| m.net.ask.is_none() && !m.net.qr_big && at(it).is_some_and(|k| k == m.sel);
    super::settings_row(d, m, text, &items);
    if let Some(r) = room {
        for i in 0..r.seats.len() {
            let Some(&rect) = g.cards.get(i) else { continue };
            if let Some(k) = at(Item::Seat(i)) {
                d.hit(rect, Act::Row(k, OK));
            }
            card(d, m, r, i, rect, on(Item::Seat(i)));
            // B216: the host's ✕ that takes a computer player off
            if let Some(k) = at(Item::Remove(i)) {
                let [x, y, w, h] = rects[k];
                let ring = if on(Item::Remove(i)) { super::super::GOLD } else { WHITE };
                d.pill_at([x, y, w, h], [INK, ring, [120.0, 30.0, 30.0]], 128.0);
                d.centred("X", x + w / 2.0, y + 3.0, 16.0, WHITE);
                d.hit([x, y, w, h], Act::Row(k, OK));
            }
        }
        code_pill(d, m, r, g.code, at(Item::Button("Code")), on(Item::Button("Code")));
        start_button(d, m, r, &items, &rects, &on);
    }
    for (b, _) in m.bottom_buttons() {
        let Some(k) = at(Item::Button(b)) else { continue };
        let (r, sel) = (rects[k], on(Item::Button(b)));
        let label = match b {
            "Copy Link" => "Copy link",
            "Public" if room.is_some_and(|r| r.private) => "Private",
            "Leave" => "Leave lobby",
            b => b,
        };
        let dim = b == "Copy Link" && shared(&m.net).is_none();
        let fill = if sel { [128.0, 108.0, 37.0] } else if b == "Leave" { [90.0, 104.0, 80.0] } else { [62.0, 60.0, 75.0] };
        d.pill_at(r, [INK, if sel { super::super::GOLD } else { WHITE }, fill], 128.0);
        d.label(label, r[0] + r[2] / 2.0, r[1] + 6.0, 16.0, r[2] - 14.0, if dim { [70.0; 3] } else if sel || b != "Leave" { WHITE } else { INK });
        d.hit(r, Act::Row(k, OK));
    }
    if room.is_some() {
        super::super::help_box(d, m.help_rect(), help);
    }
    // the hand on the chosen item's left end
    if let Some([x, y, _, h]) = rects.get(m.sel).filter(|_| m.net.ask.is_none() && !m.net.qr_big) {
        let y = if matches!(items.get(m.sel), Some(Item::Seat(_))) { y + 10.0 } else { y + h / 2.0 - 9.0 };
        d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x - 22.0 + hand_x, y, 36.0, 18.0], WHITE, 128.0);
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

/// The code pill: the QR small (a spinner until the code's there), the label and the code big.
fn code_pill(d: &mut Draw, m: &Menu, r: &Room, [x, y, w, h]: [f32; 4], k: Option<usize>, on: bool) {
    d.pill_at([x, y, w, h], [INK, if on { super::super::GOLD } else { WHITE }, if on { [128.0, 108.0, 37.0] } else { [20.0, 24.0, 34.0] }], 128.0);
    let q = h - 8.0;
    let (qx, qy) = (x + 14.0, y + 4.0);
    if shared(&m.net).is_some() {
        d.q(Tex::Solid, [0.0; 4], [qx, qy, q, q], WHITE, 128.0);
        d.q(Tex::Qr, [0.0, 0.0, 1.0, 1.0], [qx + 1.0, qy + 1.0, q - 2.0, q - 2.0], WHITE, 128.0);
    } else {
        spinner(d, m.net.spin, qx + q / 2.0, qy + q / 2.0, 9.0);
    }
    let tx = qx + q + 6.0;
    let label = if ticket_only(&m.net) { "JOIN LINK" } else if r.private { "PRIVATE CODE" } else { "CODE" };
    d.text(label, tx, y + 3.0, 9.0, [100.0; 3]);
    let code = if ticket_only(&m.net) { "LINK" } else if m.net.short.is_empty() { "...." } else { m.net.short.as_str() };
    d.outlined(|d| super::super::left(d, code, tx, y + 11.0, 22.0, x + w - 12.0 - tx, super::super::GOLD));
    if let Some(k) = k {
        d.hit([x, y, w, h], Act::Row(k, OK));
    }
}

/// The host's Start ("n of 4 ready") or a guest's Ready (and "Waiting for host").
fn start_button(d: &mut Draw, m: &Menu, r: &Room, items: &[Item], rects: &[[f32; 4]], on: &dyn Fn(Item) -> bool) {
    let Some(k) = items.iter().position(|it| matches!(it, Item::Button("Start" | "Ready"))) else { return };
    let ([x, y, w, h], sel) = (rects[k], on(items[k]));
    let (n, of) = ready_count(r);
    let (big, small) = match items[k] {
        _ if m.counting() > 0 => (format!("Cancel ({})", m.counting()), "the match is starting".to_string()),
        Item::Button("Start") if r.open() > 0 => ("Start".into(), format!("{n} of {of} ready, +{} COM", r.open())),
        Item::Button("Start") => ("Start".into(), format!("{n} of {of} ready")),
        _ if m.net.prepared => ("Not Ready".into(), "Waiting for host".into()),
        _ => ("Ready".into(), "Waiting for host".into()),
    };
    let fill = if sel { super::super::GOLD } else { [128.0, 100.0, 20.0] };
    d.pill_at([x, y, w, h], [INK, if sel { WHITE } else { INK }, fill], 128.0);
    d.label(&big, x + w / 2.0, y + 2.0, 20.0, w - 24.0, INK);
    d.label(&small, x + w / 2.0, y + 22.0, 10.0, w - 24.0, INK);
    d.hit([x, y, w, h], Act::Row(k, OK));
}

/// Seat `i`'s card.
fn card(d: &mut Draw, m: &Menu, r: &Room, i: usize, [x, y, w, h]: [f32; 4], on: bool) {
    let text = d.text;
    let p = i.min(3);
    let seat = r.seats.get(i).and_then(Option::as_ref);
    let com = seat.is_some_and(|s| !s.human());
    let mine = m.mine(i);
    let fill = if seat.is_some() { CARD[p] } else { CARD[p].map(|c| c * 0.6) };
    d.box3([x, y, w, h], 14.0, [Some(INK), None, Some(fill)], 128.0);
    if seat.is_none() || com {
        super::super::dashed(d, [x + 1.0, y + 1.0, w - 2.0, h - 2.0], if on { super::super::GOLD } else { RING[p] });
    } else {
        d.box3([x, y, w, h], 14.0, [None, Some(if on { super::super::GOLD } else { RING[p] }), None], 128.0);
    }
    let (ix, iw) = (x + 8.0, w - 16.0);
    let tag = format!("{}P", i + 1);
    d.outlined(|d| d.text(&tag, ix, y + 6.0, 14.0, RING[p]));
    let Some(s) = seat else {
        // an open seat: the computer until someone joins
        let mid = y + h / 2.0;
        d.label("COM", x + w / 2.0, mid - 50.0, 24.0, iw, RING[p]);
        d.label("Random", x + w / 2.0, mid - 22.0, 16.0, iw, WHITE);
        super::super::face_frame(d, super::super::cell_face(None), x + w / 2.0, mid + 16.0, 40.0f32.min(iw), WHITE);
        let lines = super::super::wrap("Open seat: a computer player until someone joins with the code", (iw / 5.5) as usize);
        for (k, l) in lines.iter().take(6).enumerate() {
            d.label(l, x + w / 2.0, mid + 44.0 + 12.0 * k as f32, 10.0, iw, SOFT[p]);
        }
        if m.net.hosting {
            d.label("OK: add one now", x + w / 2.0, y + h - 18.0, 11.0, iw, WHITE);
        }
        return;
    };
    let pl = &m.players[i];
    let live = r.live_of(i).filter(|_| s.human());
    let st = state(r, i);
    // Random: the card they're on now (live), else the pick they readied
    let random = pl.random || live.map_or(s.pick.random, |l| l.random || (l.ready && s.pick.random));
    let (who, costume) = m.pick(i);
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
    // the machine: "You · Keyboard", the player's name, or the computer's level
    let level = || super::super::RANK_NAMES.get(super::super::com_rank(&text.ranks, s.pick.char, s.pick.costume, r.doubles)).copied().unwrap_or("COM");
    let whose = match () {
        _ if com => format!("COM - {}", level()),
        _ if mine => match m.seats[p] {
            Some(super::super::Dev::Keys) => "You - Keyboard".into(),
            Some(super::super::Dev::Pad(_)) => "You - Controller".into(),
            _ => "You".into(),
        },
        _ => super::seat_name(m, r, i, &s.peer),
    };
    let tw = d.width(&tag, 14.0) + 6.0;
    let size = if mine { 24.0 } else { 18.0 };
    d.outlined(|d| super::super::left(d, &name, ix + tw, y + 4.0, size, iw - tw, WHITE));
    super::super::left(d, &whose, ix, y + 8.0 + size, 10.0, iw, SOFT[p]);
    // the pills: costume and hand
    let py = y + 22.0 + size;
    let hand = m.hand(text, i);
    let pills = [
        if random { "Costume ?".to_string() } else { format!("Costume {}", costume + 1) },
        if random { "Hand ?".into() } else if hand < 0.0 { "Left-handed".into() } else { "Right-handed".into() },
    ];
    let pw = (iw - 4.0) / 2.0;
    for (k, words) in pills.iter().enumerate() {
        let px = ix + (pw + 4.0) * k as f32;
        d.pill_at([px, py, pw, 16.0], [INK, WELL[p], WELL[p]], 128.0);
        let lx = if k == 0 && pw > 70.0 {
            super::super::shirt(d, [px + 5.0, py + 3.0, 11.0, 10.0], RING[p], WELL[p]);
            px + 18.0
        } else {
            px + 4.0
        };
        d.label(words, (lx + px + pw - 4.0) / 2.0, py + 3.0, 10.0, px + pw - 4.0 - lx, WHITE);
    }
    // the status box, the stand-in note, the ping and the host's action from the foot up
    let note = m.remote_mod_name(i).map(|n| format!("You don't have {n}: playing as its stand-in look"));
    let foot = y + h - 16.0;
    let mut low = foot - 4.0;
    if let Some(n) = &note {
        let lines = super::super::wrap(n, (iw / 5.0).max(8.0) as usize);
        let nh = 6.0 + 11.0 * lines.len().min(3) as f32;
        low -= nh;
        d.box3([ix, low, iw, nh], 6.0, [None, None, Some(STYLE_GOLD)], 128.0);
        for (k, l) in lines.iter().take(3).enumerate() {
            d.label(l, ix + iw / 2.0, low + 3.0 + 11.0 * k as f32, 10.0, iw - 6.0, INK);
        }
        low -= 4.0;
    }
    let (status, colour) = match st {
        State::Ready => ("Ready ✓", STYLE_GOLD),
        State::Picking => ("Picking...", WHITE),
        _ => ("Not ready", SOFT[p]),
    };
    let partner = !mine && r.doubles && (0..r.seats.len()).any(|j| j / 2 == i / 2 && m.mine(j));
    let mut context = vec![match () {
        _ if mine => "You".to_string(),
        _ if com => "Computer player".into(),
        _ if partner => "Your partner".into(),
        _ if (0..r.seats.len()).any(|j| m.mine(j)) => "Opponent".into(),
        _ => "In the lobby".into(),
    }];
    if st == State::Picking && !mine {
        context.push("Choosing on their screen".into());
    } else if let Some(Who::Mod(j)) = (!random).then_some(who) {
        context.push(format!("{} mod", m.mods[j].name));
    } else if let (false, Some(n)) = (random, m.remote_mod_name(i)) {
        context.push(format!("{n} mod"));
    }
    low -= 30.0;
    d.box3([ix, low, iw, 28.0], 8.0, [None, None, Some(WELL[p])], 128.0);
    let sw = d.width(status, 14.0);
    if status.ends_with('✓') {
        // the font has no tick: the words and a drawn one
        let words = status.trim_end_matches(" ✓");
        let ww = d.width(words, 14.0);
        d.text(words, ix + 6.0, low + 1.0, 14.0, colour);
        tick(d, ix + 10.0 + ww, low + 4.0, 10.0, colour);
    } else {
        d.text(status, ix + 6.0, low + 1.0, 14.0, colour);
    }
    let _ = sw;
    super::super::left(d, &context.join(" - "), ix + 6.0, low + 16.0, 9.0, iw - 12.0, SOFT[p]);
    // the ping (a guest's own to the host, the host's to a guest) and the host's action
    let me = if m.net.hosting { "host" } else { m.net.me.as_str() };
    let ping = match () {
        _ if com || mine => None,
        _ if s.peer == "host" => r.ping.get(me),
        _ => r.ping.get(&s.peer),
    };
    let mut used = 0.0;
    if let Some(ms) = ping {
        let words = format!("{ms} ms");
        used = d.width(&words, 10.0) + 8.0;
        d.text(&words, ix, foot, 10.0, if *ms > 150 { STYLE_GOLD } else { SOFT[p] });
    }
    if m.net.hosting && !mine {
        let act = if com { "Host: Triangle removes" } else { "Host: OK removes" };
        let max = iw - used;
        let aw = d.width(act, 10.0).min(max);
        d.label(act, ix + iw - aw / 2.0, foot, 10.0, max, SOFT[p]);
    }
    // the preview and, beside it on a wide card or under it on a narrow one, the style over the stats card
    let top = py + 22.0;
    let stats_w = if mine { 150.0f32.min(iw * 0.5) } else { iw };
    let stats_h = 16.0 + 5.0 * 13.0 + 6.0;
    let (pv, st_at) = if mine {
        ([ix, top, iw - stats_w - 6.0, low - 6.0 - top], [ix + iw - stats_w, top + ((low - 6.0 - top) - stats_h).max(0.0) / 2.0])
    } else {
        ([ix, top, iw, low - 6.0 - stats_h - 6.0 - top], [ix, low - 6.0 - stats_h])
    };
    // the 3D card draws over the menu: none under a prompt or the big QR
    preview(d, i, random, m.net.ask.is_some() || m.net.qr_big, pv, WELL[p]);
    stats(d, text, who, random, [st_at[0], st_at[1], stats_w], p);
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
    let pw = (ph * 0.8).min(w - 4.0);
    let ph = pw / 0.8;
    d.q(Tex::Preview(i), [0.0; 4], [x + (w - pw) / 2.0, y + (h - ph) / 2.0, pw, ph], WHITE, 128.0);
}

/// The play style (gold on the well) over the pink stats card: five rows, each a label and its grade's letter in a
/// light circle; "?" for Random, the style hidden.
fn stats(d: &mut Draw, text: &super::super::Text, who: Who, random: bool, [x, y, w]: [f32; 3], p: usize) {
    let style = match who {
        Who::Disc(c) => text.style[c.min(13)],
        Who::Mod(j) => text.mods.get(j).map_or(0, |t| t.0),
    };
    let grades = super::super::grades_of(text, who);
    d.box3([x, y, w, 15.0], 6.0, [None, None, Some(WELL[p])], 128.0);
    let words = if random { "?" } else { super::super::STYLE_NAMES[style.min(3)] };
    d.label(words, x + w / 2.0, y + 2.0, 11.0, w - 8.0, STYLE_GOLD);
    let (cy, row) = (y + 17.0, 13.0);
    d.box3([x, cy, w, 5.0 * row + 4.0], 8.0, [None, None, Some(PINK)], 128.0);
    for (k, label) in STATS.iter().enumerate() {
        let ry = cy + 2.0 + row * k as f32;
        if k > 0 {
            d.q(Tex::Solid, [0.0; 4], [x + 4.0, ry - 1.0, w - 8.0, 1.0], RULE, 128.0);
        }
        d.outlined(|d| super::super::left(d, label, x + 8.0, ry + 1.0, 10.0, w - 34.0, WHITE));
        let (gx, gs) = (x + w - 20.0, row - 2.0);
        d.pill_at([gx, ry + 1.0, gs, gs], [[90.0; 3], [126.0; 3], [126.0; 3]], 128.0);
        let g = grades[k].min(5) as usize;
        let (letter, rgb) = if random { ("?", GRADE[0]) } else { (&"FEDCBA"[g..g + 1], GRADE[g]) };
        d.label(letter, gx + gs / 2.0, ry + 1.5, 10.0, gs, rgb);
    }
}

/// A tick mark `s` high from (x, y), two strokes.
fn tick(d: &mut Draw, x: f32, y: f32, s: f32, rgb: [f32; 3]) {
    for (dst, a) in [([x, y + s * 0.55, s * 0.45, 2.0], 0.8), ([x + s * 0.25, y + s * 0.35, s * 0.8, 2.0], -1.0)] {
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
    fn this_machine_shares_two_parts() {
        // one local player: 2fr 1fr 1fr 1fr
        let c = columns(&[true, false, false, false], 0.0, 5.0 * 100.0 + 3.0 * GAP);
        assert_eq!(c.iter().map(|c| c.1.round()).collect::<Vec<_>>(), [200.0, 100.0, 100.0, 100.0]);
        // two: 1fr each of the 2
        let c = columns(&[true, true, false, false], 0.0, 4.0 * 100.0 + 3.0 * GAP);
        assert_eq!(c.iter().map(|c| c.1.round()).collect::<Vec<_>>(), [100.0, 100.0, 100.0, 100.0]);
        // nobody's: even
        let c = columns(&[false, false], 10.0, 200.0 + GAP);
        assert_eq!(c, [(10.0, 100.0), (118.0, 100.0)]);
    }
}
