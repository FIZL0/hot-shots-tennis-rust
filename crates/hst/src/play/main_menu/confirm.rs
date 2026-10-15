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
//! The geometry (`teams`, `side_parts`) and the card's parts (`chip`, `dotted`, the preview, the stats card) are the
//! lobby v3's frame (B396), shared with it in `frame.rs` (B407).

use super::frame::{self, BAR_H, CAPTION, CARD, Chip, LEAF, NAVY, PAPER, RING, SIDE_W, SLATE, SOFT, STYLE_GOLD, WELL, dotted};
use super::{C00, CHAR_NAMES, Draw, GOLD, INK, Menu, Press, RANDOM_TILE, RANK_NAMES, SETS, Tex, Text, UMPIRES, WHITE, Who};
use super::pointer;
use bevy::prelude::{Entity, default};

/// A picked pill's blue (Sets or Games while ←→ change it).
const PICKED: [f32; 3] = [40.0, 90.0, 128.0];
/// ‹ Back's width (the board's 170 at 0.6).
const BACK_W: f32 = 102.0;
/// The side panel's parts: the court card (the board's 150 px picture over its name), the umpire card's least height.
const COURT_H: f32 = 116.0;
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
            let th = frame::portrait_teams_h(n, w, body, side_h);
            let y = top + ((body - th - 10.0 - side_h) / 2.0).max(0.0);
            ([l, y, w, th], [l, y + th + 10.0, w, side_h])
        } else {
            ([l, top, w - sw - 10.0, body], [r - sw, top, sw, body])
        };
        let (cards, labels, vs) = frame::teams(n, room);
        let (court, pills, umpire) = frame::side_parts(side, portrait, COURT_H, UMPIRE_MIN_H);
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
    frame::team_labels(d, &g.labels);
    let [vx, vy, vw, _] = g.vs;
    super::flame::draw(d, m, [vx + vw / 2.0, vy + vw / 2.0]);
    frame::vs_disc(d, g.vs);
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
    let sel = m.sel == 1 && !picking;
    frame::court_card(d, m, m.court, &m.court_title(text, m.court), g.court, sel, sel);
    d.hit(rows[1], pointer::Act::Row(1, pointer::OK));
    // the pills: Mode and Handicap say how it's set up (Handicap's Off until the original's handicaps are in, B40g1);
    // Sets and Games are rows, their ‹ › for OK then ←→
    let values = [(None, "MODE", if m.doubles { "Doubles" } else { "Singles" }.to_string()), (Some(2), "SETS", SETS[m.sets].to_string()), (Some(3), "GAMES", m.games.to_string()), (None, "HANDICAP", "Off".to_string())];
    for (rect, (row, cap, value)) in g.pills.iter().zip(values) {
        let sel = row == Some(m.sel);
        if sel && picking {
            // drawn blue on top (`picked_pill`)
            continue;
        }
        frame::setting_pill(d, *rect, cap, &value, sel, row.is_some());
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
        // B408: the mod's game, not its donor
        (_, Some(x)) => (x.name.clone(), x.game().to_string()),
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
    let r = frame::rows([x, y, w, h]);
    let (ix, iw) = (r.ix, r.iw);
    // the header: seat tag, the status chip
    let tag_w = frame::seat_tag(d, p, ix, y + 6.0);
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
    frame::chip(d, &chip_words, if human { Chip::Ready } else { Chip::Com }, x + w - 6.0, y + 6.0, (x + w - 6.0 - hx).max(24.0));
    // from the foot up: stats, style, chips, the name's two lines; the preview takes the rest
    let random = pl.random;
    frame::preview(d, p, random, false, r.well, WELL[c]);
    if let Some(n) = m.remote_mod_name(p) {
        // the stand-in note over the preview's foot (online: a mod this machine lacks)
        frame::stand_in(d, n, &r);
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
    d.outlined(|d| super::left(d, &name, ix, r.name_y, 16.0, iw, WHITE));
    dotted(d, &sub, ix, r.sub_y, 8.5, iw, SOFT[c], false);
    // the costume and hand chips, the play style over the pink stats card
    frame::card_foot(d, text, c, &r, who, random, costume, m.hand(text, p));
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
