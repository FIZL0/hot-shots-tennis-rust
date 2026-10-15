//! B397: the confirm screen, v3 (ours, the approved board "Proposal: Confirm v3", the online lobby's frame with
//! `screen=confirm`). Along the top the title's tab, the match in a line ("Doubles · 1 set, 4 games") and "Local
//! match". The players are equal cards, TEAM 1 · VS · TEAM 2 (singles: one each side of VS, the menus' flame behind
//! it); a card has its seat tag, a status chip (Ready ✓, a computer's COM · Lv N with a dashed rim), the 3D preview,
//! the name, the device, the costume and hand chips, the play style and the pink stats card. Right of the teams (under
//! them in a portrait window) the side panel: the court's picture and name (OK opens the court select), Mode / Sets /
//! Games / Handicap two by two (Sets and Games: OK, then ←→, as before), the umpire's card (face, name; OK opens the
//! umpire select). Along the bottom ‹ Back, the help line and, under the side panel, the big Start the match!. The
//! court and umpire selects (`court_screen`, `umpire_screen`) are as before.
//!
//! The geometry (`teams`, `side_parts`) and the card's parts (`chip`, `dotted`, `stats`) are the lobby v3's frame
//! (B396), here for the local screen.

use super::{C00, C02, CHAR_NAMES, Draw, GOLD, INK, Menu, Press, RANDOM_TILE, RANK_NAMES, SETS, STYLE_NAMES, Tex, Text, UMPIRES, WHITE, Who};
use super::{COURTS, pointer};
use bevy::prelude::{Entity, default};

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
/// The board's navy (VS, words on light boxes), the dark green over the wall, the grey pills, the ready chip's green,
/// the light boxes' cream and their soft caption.
const NAVY: [f32; 3] = [14.5, 21.5, 34.5];
const LEAF: [f32; 3] = [15.5, 29.0, 11.5];
const LEAF_RULE: [f32; 3] = [55.5, 87.0, 45.0];
const SLATE: [f32; 3] = [54.0, 55.5, 71.0];
const READY: [f32; 3] = [23.5, 84.0, 48.0];
const PAPER: [f32; 3] = [125.5, 123.5, 117.0];
const CAPTION: [f32; 3] = [53.5, 55.0, 71.5];
/// A picked pill's blue (Sets or Games while ←→ change it).
const PICKED: [f32; 3] = [40.0, 90.0, 128.0];
/// The board at 1280×720 is 0.6 of it in 640×448 units: the side panel's width (300), the bottom bar's height (64),
/// ‹ Back's width (170), the VS disc (56), the gap between the cards and the parts.
const SIDE_W: f32 = 180.0;
const BAR_H: f32 = 38.0;
const BACK_W: f32 = 102.0;
const VS: f32 = 34.0;
const GAP: f32 = 7.0;
/// A card's widest (a very wide window: the teams keep to the middle), the team label's row.
const CARD_MAX_W: f32 = 210.0;
const LABEL_H: f32 = 14.0;
/// The side panel's parts: the court card (the board's 150 px picture over its name), the pills' height and the gap
/// between them, the umpire card's least height.
const COURT_H: f32 = 116.0;
const PILL_H: f32 = 28.0;
const PILL_GAP: f32 = 4.0;
const UMPIRE_MIN_H: f32 = 50.0;

/// Where the confirm screen's parts go (640×448 units, `Grid::reach`).
#[derive(Debug)]
pub(super) struct Geo {
    /// A card a player, in seat order; the team labels' rows (doubles) and the VS disc between the teams.
    pub cards: Vec<[f32; 4]>,
    pub labels: Vec<[f32; 4]>,
    pub vs: [f32; 4],
    /// The side panel's parts: the court card, the pills (Mode, Sets, Games, Handicap two by two), the umpire card.
    pub court: [f32; 4],
    pub pills: [[f32; 4]; 4],
    pub umpire: [f32; 4],
    /// The bottom bar: ‹ Back, the help line, Start under the side panel.
    pub back: [f32; 4],
    pub help: [f32; 4],
    pub start: [f32; 4],
}

/// The cards of `n` players in the teams' room `[x, y, w, h]`: two teams either side of the VS disc, a label row over
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

/// The side panel's parts in `[x, y, w, h]`: a column (beside the teams), or a wide strip (under them in a portrait
/// window) with the court card left and the pills over the umpire card right.
fn side_parts([x, y, w, h]: [f32; 4], wide: bool) -> ([f32; 4], [[f32; 4]; 4], [f32; 4]) {
    let (court, px, py, pw) = if wide {
        let cw = (w - GAP) / 2.0;
        ([x, y, cw, h], x + cw + GAP, y, cw)
    } else {
        ([x, y, w, COURT_H], x, y + COURT_H + 6.0, w)
    };
    let half = (pw - PILL_GAP) / 2.0;
    let row = |k: usize| py + (PILL_H + PILL_GAP) * k as f32;
    let pills = [[px, row(0), half, PILL_H], [px + half + PILL_GAP, row(0), half, PILL_H], [px, row(1), half, PILL_H], [px + half + PILL_GAP, row(1), half, PILL_H]];
    let top = row(2) + 2.0;
    (court, pills, [px, top, pw, (y + h - top).max(UMPIRE_MIN_H)])
}

impl Menu {
    pub(super) fn confirm_geo(&self) -> Geo {
        let [l, t, r, b] = self.grid.reach();
        let n = self.players();
        // under the title's tab, over the bottom bar
        let (top, foot) = (t + 46.0, b - BAR_H);
        let body = foot - 8.0 - top;
        let w = r - l;
        let portrait = body > w * 1.05;
        let sw = SIDE_W.min(w * 0.4);
        let (room, side) = if portrait {
            // the side panel under the teams, across; the cards no taller than they look right, the lot centred
            let side_h = 150.0;
            let per = (n / 2).max(1) as f32;
            let cw = ((w - VS - 2.0 * GAP - 2.0 * (per - 1.0) * GAP) / (2.0 * per)).min(CARD_MAX_W);
            let th = (body - side_h - 10.0).min(cw * 2.7 + LABEL_H).max(200.0);
            let y = top + ((body - th - 10.0 - side_h) / 2.0).max(0.0);
            ([l, y, w, th], [l, y + th + 10.0, w, side_h])
        } else {
            ([l, top, w - sw - 10.0, body], [r - sw, top, sw, body])
        };
        let (cards, labels, vs) = teams(n, room);
        let (court, pills, umpire) = side_parts(side, portrait);
        let start = [r - sw, foot, sw, BAR_H];
        let back = [l, foot, BACK_W.min(w * 0.2), BAR_H];
        let hx = back[0] + back[2] + 6.0;
        let help = [hx, foot, start[0] - 8.0 - hx, BAR_H];
        Geo { cards, labels, vs, court, pills, umpire, back, help, start }
    }
}

/// The confirm screen's rows (`Menu::sel`): Start, the court card, Sets, Games, the umpire card.
pub(super) fn rects(m: &Menu) -> [[f32; 4]; super::CONFIRM_ROWS] {
    let g = m.confirm_geo();
    [g.start, g.court, g.pills[1], g.pills[2], g.umpire]
}

/// `s` from `x` at `size`, a "·" drawn as a dot (the font has none), squeezed to fit `max`; `centre` centres it on `x`.
#[allow(clippy::too_many_arguments)]
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

/// The confirm screen (the pickers open over it as their own screens).
pub(super) fn draw(d: &mut Draw, m: &Menu, text: &Text, pads: &[Entity], hand_x: f32) {
    d.frame(4, 8);
    let g = m.confirm_geo();
    let [l, t, r, _] = m.grid.reach();
    // the top line: the match beside the title's tab, where it's played at the right
    let sets = SETS[m.sets];
    let summary = format!("{} · {sets} set{}, {} games", if m.doubles { "Doubles" } else { "Singles" }, if sets > 1 { "s" } else { "" }, m.games);
    let tab = d.width("Confirm Settings", 30.0) + 90.0;
    let sx = l - 12.0 + tab;
    d.outlined(|d| dotted(d, &summary, sx, t + 12.0, 15.0, (r - sx) * 0.6, STYLE_GOLD, false));
    let where_ = if m.online() { "Online match" } else { "Local match" };
    let ww = (r - sx) * 0.35;
    let wd = d.width(where_, 11.0).min(ww);
    dotted(d, where_, r - wd, t + 15.0, 11.0, ww, LEAF, false);
    // the team labels, the flame and the VS disc
    for (k, &[x, y, w, _]) in g.labels.iter().enumerate() {
        let words = format!("TEAM {}", k + 1);
        let tw = d.width(&words, 10.0);
        d.text(&words, x + 1.0, y + 1.0, 10.0, LEAF);
        d.q(Tex::Solid, [0.0; 4], [x + tw + 6.0, y + 6.0, (w - tw - 6.0).max(0.0), 1.5], LEAF_RULE, 128.0);
    }
    let [vx, vy, vw, _] = g.vs;
    super::flame::draw(d, m, [vx + vw / 2.0, vy + vw / 2.0]);
    d.pill_at([vx, vy, vw, vw], [INK, WHITE, NAVY], 128.0);
    d.centred("VS", vx + vw / 2.0, vy + vw / 2.0 - 8.0, 16.0, [127.5, 77.5, 30.5]);
    for (p, &rect) in g.cards.iter().enumerate() {
        card(d, m, text, pads, p, rect);
    }
    let rows = rects(m);
    side_panel(d, m, text, &g, &rows);
    // the bottom bar: ‹ Back, the help line, Start
    let [bx, by, bw, bh] = g.back;
    d.pill_at(g.back, [INK, WHITE, SLATE], 128.0);
    d.label("< Back", bx + bw / 2.0, by + bh / 2.0 - 8.5, 17.0, bw - 16.0, WHITE);
    d.hit(g.back, pointer::press(|k| k.back = true));
    let (head, help) = help_words(m, text);
    help_line(d, g.help, head, &help);
    start_button(d, g.start, m.sel == 0);
    if m.picking.is_some() {
        picked_pill(d, m, rows[m.sel]);
    }
    let [hx, hy, _, hh] = rows[m.sel];
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [hx - 22.0 + hand_x, hy + hh.min(40.0) / 2.0 - 9.0, 36.0, 18.0], WHITE, 128.0);
}

/// The help line's bold head (the game's own words for the chosen row) and what its buttons do.
fn help_words<'a>(m: &Menu, text: &'a Text) -> (&'a str, String) {
    let back = if m.online() { "Back: the lobby" } else { "Back: the character select" };
    match m.sel {
        0 => (text.msg(323), format!("OK: start the match · {back}")),
        1 => (text.msg(326), format!("OK: choose the court · {back}")),
        2 if m.picking.is_some() => (text.msg(324), "Left / Right: sets · OK: keep it · Back: put it back".into()),
        2 => (text.msg(324), format!("OK, then Left / Right: change it · {back}")),
        3 if m.picking.is_some() => (text.msg(325), "Left / Right: games · OK: keep it · Back: put it back".into()),
        3 => (text.msg(325), format!("OK, then Left / Right: change it · {back}")),
        _ => (text.msg(328), format!("OK: choose the umpire · {back}")),
    }
}

/// The help line on its light box: the head over what the buttons do (a head too long for one line: the lot wrapped
/// on two).
fn help_line(d: &mut Draw, [x, y, w, h]: [f32; 4], head: &str, s: &str) {
    d.box3([x, y, w, h], 10.0, [None, None, Some(WHITE)], 108.0);
    if w < 30.0 {
        return;
    }
    if d.width(head, 13.0) > (w - 18.0) * 1.15 {
        let all = format!("{head} {s}");
        let mut size = 11.0;
        let lines = loop {
            let lines = super::wrap(&all, ((w - 18.0) / (size * 0.5)) as usize);
            if lines.len() <= 2 || size < 7.5 {
                break lines;
            }
            size -= 0.5;
        };
        let top = y + (h - (size + 2.0) * lines.len().min(2) as f32) / 2.0;
        for (k, l) in lines.iter().take(2).enumerate() {
            dotted(d, l, x + 9.0, top + (size + 2.0) * k as f32, size, w - 18.0, NAVY, false);
        }
        return;
    }
    dotted(d, head, x + 9.0, y + 4.0, 13.0, w - 18.0, NAVY, false);
    dotted(d, s, x + 9.0, y + 21.0, 10.0, w - 18.0, NAVY, false);
}

/// The big Start the match! ("Everyone ready" under it): the board's yellow, gold-rimmed while it's the chosen row.
fn start_button(d: &mut Draw, [x, y, w, h]: [f32; 4], sel: bool) {
    let fill = if sel { [128.0, 118.0, 60.0] } else { GOLD };
    d.pill_at([x, y, w, h], [INK, if sel { WHITE } else { NAVY }, fill], 128.0);
    d.label("Start the match!", x + w / 2.0, y + 3.0, 19.0, w - 24.0, NAVY);
    dotted(d, "Everyone ready", x + w / 2.0, y + 23.0, 9.0, w - 26.0, NAVY, true);
    d.hit([x, y, w, h], pointer::Act::Row(0, pointer::OK));
}

/// Sets or Games picked (B212): its pill blue with ‹ › (each gone at its end, as the original's); the pill's thirds
/// step it, the middle keeps it, a tap off puts it back (as ○).
fn picked_pill(d: &mut Draw, m: &Menu, [x, y, w, h]: [f32; 4]) {
    if !matches!(m.sel, 2 | 3) {
        return;
    }
    let (at, n) = if m.sel == 2 { (m.sets, SETS.len()) } else { (super::GAMES.iter().position(|&g| g >= m.games).unwrap_or(1), super::GAMES.len()) };
    let (cap, value) = if m.sel == 2 { ("SETS", SETS[m.sets].to_string()) } else { ("GAMES", m.games.to_string()) };
    d.pill_at([x, y, w, h], [INK, WHITE, PICKED], 128.0);
    d.text(cap, x + 12.0, y + 3.5, 6.5, [110.0, 116.0, 128.0]);
    d.label(&value, x + w / 2.0, y + 11.0, 12.0, w - 34.0, WHITE);
    if at > 0 {
        d.text("<", x + 9.0, y + 11.0, 12.0, WHITE);
    }
    if at + 1 < n {
        d.text(">", x + w - 9.0 - d.width(">", 12.0), y + 11.0, 12.0, WHITE);
    }
    d.popup(Some(pointer::press(|k| k.back = true)));
    d.hit([x, y, w, h], pointer::Act::Row(m.sel, pointer::OK));
    d.hit([x, y, w / 3.0, h], pointer::Act::Row(m.sel, Press { left: true, ..default() }));
    d.hit([x + w * 2.0 / 3.0, y, w / 3.0, h], pointer::Act::Row(m.sel, Press { right: true, ..default() }));
}

/// The side panel: the court card, the settings pills, the umpire card.
fn side_panel(d: &mut Draw, m: &Menu, text: &Text, g: &Geo, rows: &[[f32; 4]; super::CONFIRM_ROWS]) {
    let picking = m.picking.is_some();
    // the court card: its picture, COURT over its name, "OK: change" while it's the chosen row
    let [x, y, w, h] = g.court;
    let sel = m.sel == 1 && !picking;
    d.box3([x, y, w, h], 12.0, [Some(INK), Some(if sel { GOLD } else { WHITE }), Some(PAPER)], 128.0);
    let ph = h - 34.0;
    let pic = [x + 3.0, y + 3.0, w - 6.0, ph];
    let first = d.out.len();
    super::court_picture(d, m.court_layout(m.court), pic);
    let rr = 12.0 * 11.0 / 16.0 - (3.0 - 12.0 * 4.0 / 16.0);
    d.out[first..].iter_mut().filter(|q| q.dst == pic).for_each(|q| q.round = [rr, rr, 0.0, 0.0]);
    let name = m.court_title(text, m.court);
    let ny = y + 3.0 + ph + 3.0;
    d.text("COURT", x + 9.0, ny, 7.0, CAPTION);
    let change = sel.then_some("OK: change");
    let cw = change.map_or(0.0, |c| d.width(c, 8.0) + 6.0);
    super::left(d, &name, x + 9.0, ny + 9.0, 15.0, w - 18.0 - cw, NAVY);
    if let Some(c) = change {
        d.text(c, x + w - 9.0 - d.width(c, 8.0), ny + 13.0, 8.0, CAPTION);
    }
    d.hit(rows[1], pointer::Act::Row(1, pointer::OK));
    // the pills: Mode and Handicap say how it's set up (Handicap's Off until the original's handicaps are in, B40g1);
    // Sets and Games are rows, their ‹ › for OK then ←→
    let values = [(None, "MODE", if m.doubles { "Doubles" } else { "Singles" }.to_string()), (Some(2), "SETS", SETS[m.sets].to_string()), (Some(3), "GAMES", m.games.to_string()), (None, "HANDICAP", "Off".to_string())];
    for (rect, (row, cap, value)) in g.pills.iter().zip(values) {
        let [x, y, w, _] = *rect;
        let sel = row == Some(m.sel);
        if sel && picking {
            // drawn blue on top (`picked_pill`)
            continue;
        }
        d.pill_at(*rect, [INK, if sel { GOLD } else { WHITE }, if sel { [128.0, 108.0, 37.0] } else { SLATE }], 128.0);
        let (cap_rgb, rgb) = if sel { (INK, INK) } else { ([108.0, 109.0, 117.0], WHITE) };
        d.text(cap, x + 12.0, y + 3.5, 6.5, cap_rgb);
        if row.is_some() {
            let a = if sel { INK } else { [100.5, 101.5, 112.0] };
            d.text("<", x + 9.0, y + 11.0, 12.0, a);
            d.text(">", x + w - 9.0 - d.width(">", 12.0), y + 11.0, 12.0, a);
        }
        d.label(&value, x + w / 2.0, y + 11.0, 12.0, w - 34.0, rgb);
        if let Some(k) = row {
            d.hit(*rect, pointer::Act::Row(k, pointer::OK));
        }
    }
    // the umpire card: her round face (the "?" for a random one), UMPIRE over her name, a line under it
    let [x, y, w, h] = g.umpire;
    let sel = m.sel == 4 && !picking;
    d.box3([x, y, w, h], 12.0, [Some(INK), Some(if sel { GOLD } else { WHITE }), Some(PAPER)], 128.0);
    let u = m.umpire as usize;
    let size = (h - 16.0).clamp(20.0, 46.0).min(w * 0.3);
    let (fx, fy) = (x + 10.0 + size / 2.0, y + h / 2.0);
    let donor = m.umpire_donor(u);
    if super::umpire_mods::face(d, m, u, [fx, fy], size) {
        // B400: a mod's own head
    } else if donor == UMPIRES {
        // B356: the "?" in the same circle as a disc face
        let [fu, fv, tw, th] = RANDOM_TILE;
        super::random_face(d, [fu + (tw - th) / 2.0 + 4.0, fv + 4.0, th - 8.0, th - 8.0], [fx, fy], size);
    } else {
        // B350: the disc's round face (`Confirmation_00`) has its own rim; M8f3: a mod shows its donor's
        let fw = size * 46.0 / 44.0;
        d.q(C00, [63.0 + 48.0 * donor as f32, 272.0, 46.0, 44.0], [fx - fw / 2.0, fy - size / 2.0, fw, size], WHITE, 128.0);
    }
    let (name, sub) = match (u, m.umpire_mod_of(u)) {
        (UMPIRES, _) => ("Random Umpire".to_string(), "Picked at the start".to_string()),
        (_, Some(x)) => (x.name.clone(), format!("Calls as {}", m.umpire_name(text, x.donor as usize))),
        _ => (m.umpire_name(text, u), format!("Umpire {} of {UMPIRES}", u + 1)),
    };
    let tx = fx + size / 2.0 + 8.0;
    let tw = x + w - 9.0 - tx;
    let change = if sel { " · OK: change" } else { "" };
    let mid = y + h / 2.0;
    d.text("UMPIRE", tx, mid - 17.0, 7.0, CAPTION);
    super::left(d, &name, tx, mid - 9.0, 15.0, tw, NAVY);
    dotted(d, &format!("{sub}{change}"), tx, mid + 7.0, 8.0, tw, CAPTION, false);
    d.hit(rows[4], pointer::Act::Row(4, pointer::OK));
}

/// The status chip ending at `right` on the header row at `y`, at most `max` wide: Ready ✓ (green) for a person,
/// COM · Lv N (white) for the computer; its left edge.
fn chip(d: &mut Draw, words: &str, com: bool, right: f32, y: f32, max: f32) -> f32 {
    let size = 8.5;
    let tick = !com;
    let tw = d.width(words, size) + if tick { 10.0 } else { 0.0 };
    let w = (tw + 12.0).min(max);
    let (x, h) = (right - w, 14.0);
    let (fill, ink) = if com { (WHITE, NAVY) } else { (READY, WHITE) };
    d.pill_at([x, y, w, h], [INK, WHITE, fill], 128.0);
    let inner = w - 12.0 - if tick { 10.0 } else { 0.0 };
    let words_w = d.width(words, size).min(inner);
    let wx = x + (w - words_w - if tick { 10.0 } else { 0.0 }) / 2.0;
    dotted(d, words, wx, y + 2.5, size, inner, ink, false);
    if tick {
        self::tick(d, wx + words_w + 3.0, y + 3.0, 8.0, ink);
    }
    x
}

/// Player `p`'s card: the seat's tag and status chip, the preview, the name, the device, the costume and hand chips,
/// the play style over the stats card; a computer's rim dashed.
fn card(d: &mut Draw, m: &Menu, text: &Text, pads: &[Entity], p: usize, [x, y, w, h]: [f32; 4]) {
    let c = p.min(3);
    let pl = m.players[p];
    let (who, costume) = m.pick(p);
    let remote = m.remote(p);
    let human = m.seats[p].is_some() || remote.is_some_and(|s| s.human());
    d.box3([x, y, w, h], 13.0, [Some(INK), None, Some(CARD[c])], 128.0);
    if human {
        d.box3([x, y, w, h], 13.0, [None, Some(RING[c]), None], 128.0);
    } else {
        super::dashed(d, [x + 1.0, y + 1.0, w - 2.0, h - 2.0], RING[c]);
    }
    let (ix, iw) = (x + 6.0, w - 12.0);
    // the header: seat tag, the status chip
    let tag = format!("{}P", p + 1);
    let tag_w = d.width(&tag, 10.0) + 8.0;
    d.box3([ix, y + 6.0, tag_w, 14.0], 4.0, [None, None, Some(RING[c])], 128.0);
    d.text(&tag, ix + 4.0, y + 8.0, 10.0, NAVY);
    let donor = match who {
        Who::Mod(i) => m.mods[i].donor,
        Who::Disc(c) => c,
    };
    let chip_words = if human {
        "Ready".to_string()
    } else {
        let rank = super::com_rank(&text.ranks, donor, costume, m.doubles);
        format!("COM · {}", RANK_NAMES.get(rank).copied().unwrap_or("COM").trim_start_matches("Tennis "))
    };
    let hx = ix + tag_w + 4.0;
    chip(d, &chip_words, !human, x + w - 6.0, y + 6.0, (x + w - 6.0 - hx).max(24.0));
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
    let random = pl.random;
    preview(d, p, random, well, WELL[c]);
    if let Some(n) = m.remote_mod_name(p) {
        // the stand-in note over the preview's foot (online: a mod this machine lacks)
        let note = format!("{n}: you don't have it, so you'll see its stand-in");
        let lines = super::wrap(&note, ((iw - 10.0) / 4.4).max(8.0) as usize);
        let nh = 5.0 + 9.5 * lines.len().min(3) as f32;
        let ny = well[1] + well[3] - 4.0 - nh;
        d.box3([ix + 4.0, ny, iw - 8.0, nh], 5.0, [None, None, Some(STYLE_GOLD)], 128.0);
        for (k, l) in lines.iter().take(3).enumerate() {
            super::left(d, l, ix + 8.0, ny + 3.0 + 9.5 * k as f32, 8.0, iw - 16.0, NAVY);
        }
    }
    let name = match who {
        _ if random => "Random".to_string(),
        Who::Mod(i) => m.mods[i].name.clone(),
        Who::Disc(c) => m.remote_mod_name(p).unwrap_or(CHAR_NAMES[c]).to_string(),
    };
    let sub = match remote.filter(|s| s.human()) {
        Some(s) => m.net.room.as_ref().and_then(|r| r.names.get(&s.peer)).map_or("Online".to_string(), |n| n.clone()),
        None if human => super::dev_name(m.seats[p], pads),
        None => "Computer".into(),
    };
    d.outlined(|d| super::left(d, &name, ix, name_y, 16.0, iw, WHITE));
    dotted(d, &sub, ix, sub_y, 8.5, iw, SOFT[c], false);
    // the chips: costume (the shirt) and hand
    let hand = m.hand(text, p);
    let words = [
        if random { "Costume ?".to_string() } else { format!("Costume {}", costume + 1) },
        if random { "?".into() } else if hand < 0.0 { "Left".into() } else { "Right".into() },
    ];
    let cw = (iw - 4.0) / 2.0;
    for (k, words) in words.iter().enumerate() {
        let px = ix + (cw + 4.0) * k as f32;
        d.pill_at([px, chips_y, cw, 12.0], [WELL[c]; 3], 128.0);
        let lx = if k == 0 && cw > 46.0 {
            super::shirt(d, [px + 5.0, chips_y + 2.5, 8.0, 7.0], RING[c], WELL[c]);
            px + 15.0
        } else {
            px + 4.0
        };
        d.label(words, (lx + px + cw - 4.0) / 2.0, chips_y + 2.0, 8.5, px + cw - 4.0 - lx, WHITE);
    }
    // the play style over the pink stats card
    let style = match who {
        Who::Mod(i) => text.mods.get(i).map_or(0, |t| t.0),
        Who::Disc(c) => text.style[c.min(13)],
    };
    d.box3([ix, style_y, iw, 12.0], 3.0, [None, None, Some(WELL[c])], 128.0);
    d.label(if random { "?" } else { STYLE_NAMES[style.min(3)] }, ix + iw / 2.0, style_y + 2.0, 9.0, iw - 8.0, STYLE_GOLD);
    stats(d, text, who, random, [ix, stats_y, iw, stats_h], row);
}

/// The 3D preview (`previews.rs`: `Tex::Preview(i)`), 4:5 and as big as `[x, y, w, h]` allows, on the seat's well;
/// the Random card's "?" tile.
fn preview(d: &mut Draw, i: usize, random: bool, [x, y, w, h]: [f32; 4], well: [f32; 3]) {
    if h < 12.0 {
        return;
    }
    d.box3([x, y, w, h], 8.0, [None, None, Some(well)], 128.0);
    if random {
        let fw = (w - 8.0).min((h - 8.0) * 92.0 / 68.0);
        d.q(C02, RANDOM_TILE, [x + (w - fw) / 2.0, y + (h - fw * 68.0 / 92.0) / 2.0, fw, fw * 68.0 / 92.0], WHITE, 128.0);
        return;
    }
    let ph = h - 4.0;
    let pw = (ph * super::previews::CARD_W).min(w - 4.0);
    let ph = pw / super::previews::CARD_W;
    d.q(Tex::Preview(i), [0.0; 4], [x + (w - pw) / 2.0, y + (h - ph) / 2.0, pw, ph], WHITE, 128.0);
}

/// The pink stats card: five rows `row` high, each a label and its grade's letter in a light circle; "?" for Random.
fn stats(d: &mut Draw, text: &Text, who: Who, random: bool, [x, y, w, h]: [f32; 4], row: f32) {
    let grades = super::grades_of(text, who);
    d.box3([x, y, w, h], 3.0, [None, None, Some(PINK)], 128.0);
    for (k, label) in STATS.iter().enumerate() {
        let ry = y + 2.0 + row * k as f32;
        if k > 0 {
            d.q(Tex::Solid, [0.0; 4], [x + 3.0, ry - 0.5, w - 6.0, 1.2], RULE, 128.0);
        }
        d.outlined(|d| super::left(d, label, x + 5.0, ry + 1.5, 8.5, w - 22.0, WHITE));
        let gs = row - 2.0;
        let gx = x + w - 3.0 - gs;
        d.pill_at([gx, ry + 1.0, gs, gs], [[126.0; 3]; 3], 128.0);
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
    use super::super::{Grid, Screen};
    use super::*;

    fn inside([x, y, w, h]: [f32; 4], [l, t, r, b]: [f32; 4]) -> bool {
        x >= l - 0.01 && y >= t - 0.01 && x + w <= r + 0.01 && y + h <= b + 0.01
    }

    fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
        a[0] < b[0] + b[2] - 0.01 && b[0] < a[0] + a[2] - 0.01 && a[1] < b[1] + b[3] - 0.01 && b[1] < a[1] + a[3] - 0.01
    }

    #[test]
    fn equal_cards_either_side_of_vs() {
        let (cards, labels, vs) = teams(4, [0.0, 0.0, 500.0, 300.0]);
        assert_eq!(labels.len(), 2);
        assert!(cards.windows(2).all(|c| (c[0][2] - c[1][2]).abs() < 0.01), "{cards:?}");
        // team 1 left of VS, team 2 right of it
        assert!(cards[1][0] + cards[1][2] <= vs[0] && vs[0] + vs[2] <= cards[2][0]);
        let (cards, labels, _) = teams(2, [0.0, 0.0, 500.0, 300.0]);
        assert!(labels.is_empty() && cards.len() == 2 && cards[0][3] == 300.0);
    }

    #[test]
    fn lays_out_at_any_aspect() {
        for aspect in [0.5, 9.0 / 16.0 * 1.2, 0.75, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 32.0 / 9.0] {
            for doubles in [false, true] {
                let m = Menu { screen: Screen::Confirm, doubles, grid: Grid::fit(aspect), ..default() };
                let g = m.confirm_geo();
                let reach = m.grid.reach();
                let mut parts: Vec<[f32; 4]> = g.cards.clone();
                parts.extend([g.court, g.umpire, g.back, g.help, g.start]);
                parts.extend(g.pills);
                for (i, a) in parts.iter().enumerate() {
                    assert!(inside(*a, reach), "{aspect} {doubles}: part {i} {a:?} outside {reach:?}");
                    assert!(a[2] > 20.0 && a[3] > 10.0, "{aspect} {doubles}: part {i} {a:?} too small");
                    for b in &parts[i + 1..] {
                        assert!(!overlap(*a, *b), "{aspect} {doubles}: {a:?} overlaps {b:?}");
                    }
                }
                // a card keeps a portrait shape and room for its preview
                for c in &g.cards {
                    assert!(c[3] > c[2] && c[3] >= 180.0, "{aspect} {doubles}: card {c:?}");
                }
                assert!(g.umpire[3] >= UMPIRE_MIN_H - 0.01);
            }
        }
    }
}
