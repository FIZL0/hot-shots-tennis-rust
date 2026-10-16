//! B450: the court picker over the players (ours, the board "Concept v4: Court pressed", `picker=court`). OK on the
//! Court card of the confirm screen or the lobby opens a panel over the players' area (the screen stays up around
//! it): row 1 the disc's courts, Random first, in the original's strip order; row 2 the court mods, only when any are
//! installed (online: only those every peer in the lobby has, `Room::courts`). Each row scrolls sideways on its own
//! ("< a–b of N >" and a scroll bar; ←→ walk it, L1 / R1 a page); ↑↓ change rows, each row keeping its place. A card
//! is the court's picture, its name and its surface (a mod: its game). ✕ keeps the pick, ○ puts the old one back
//! (`Menu::step`'s picking branch). It replaces B192's separate Select Court screen.

use super::frame::{CAPTION, NAVY, PAPER, SLATE, dotted};
use super::{COURTS, Draw, Effect, GOLD, INK, Menu, Press, STRIP, SURFACE, Tex, Text, WHITE, court_mods, pointer};

/// The panel's ground (the roster's navy) and a row's band.
const GROUND: [f32; 3] = [11.0, 16.5, 30.0];
const BAND: [f32; 3] = [18.0, 25.0, 42.0];
/// A card's widest, its narrowest, the gap between cards; a card's words under its picture. A page shows `PAGE_MIN`
/// cards when they fit at least `CARD_MIN_W` wide, never more than `PAGE_MAX` (a very wide window: the rest around).
const CARD_MAX_W: f32 = 150.0;
const PAGE_MIN: usize = 4;
const PAGE_MAX: usize = 7;
const CARD_MIN_W: f32 = 64.0;
const GAP: f32 = 6.0;
const WORDS_H: f32 = 26.0;
/// The panel's title line, a row's head line (its name, "< a–b of N >") and its scroll bar.
const TITLE_H: f32 = 22.0;
const HEAD_H: f32 = 16.0;
const BAR_H: f32 = 8.0;

/// Each row's place: its first card shown and its chosen card (the row not in use keeps them for ↑↓ back).
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Picker {
    first: [usize; 2],
    keep: [usize; 2],
}

/// The picker's rows: the disc's courts (Random first, the strip's order) and the court mods (maybe none).
pub(super) fn rows(m: &Menu) -> [Vec<usize>; 2] {
    [std::iter::once(COURTS).chain(STRIP).collect(), court_mods::values(m).collect()]
}

/// The chosen court's row and place in it (Random's when it's in neither, as a mod no peer shares).
fn at(m: &Menu, rows: &[Vec<usize>; 2]) -> (usize, usize) {
    (0..2).find_map(|r| rows[r].iter().position(|&c| c == m.court).map(|i| (r, i))).unwrap_or((0, 0))
}

/// The first card shown of a row of `n` at `page` a time from `first`, moved just enough to show card `i`.
fn view(first: usize, i: usize, n: usize, page: usize) -> usize {
    let f = first.min(n.saturating_sub(page));
    if i < f { i } else if i >= f + page { i + 1 - page } else { f }
}

/// Where the picker's parts go (640×448 units).
#[derive(Debug)]
pub(super) struct Geo {
    pub panel: [f32; 4],
    /// Per row shown: its band, its head line, the cards in view's first slot, a card's size, the cards a page, the
    /// scroll bar's track.
    pub band: Vec<[f32; 4]>,
    pub head: Vec<[f32; 4]>,
    pub slot0: Vec<[f32; 2]>,
    pub card: [f32; 2],
    pub page: usize,
    pub bar: Vec<[f32; 4]>,
}

/// The players' area the panel covers: the court stage between the top bar and the strip. The confirm screen's and
/// the lobby's frames agree on it (`frame.rs`).
fn area(m: &Menu) -> [f32; 4] {
    if m.net.venue { m.lobby_geo().stage } else { m.confirm_geo().stage }
}

impl Menu {
    pub(super) fn court_picker_geo(&self) -> Geo {
        let panel = area(self);
        let [x, y, w, h] = panel;
        let n = if rows(self)[1].is_empty() { 1 } else { 2 };
        // each row: its head, the cards, the scroll bar; a card no taller than its share, no wider than the rows
        let row_h = (h - TITLE_H - 8.0 - 6.0 * (n - 1) as f32) / n as f32;
        let pic_h = (row_h - HEAD_H - BAR_H - WORDS_H - 8.0).max(16.0);
        let inner = w - 20.0;
        let fit = (inner - (PAGE_MIN - 1) as f32 * GAP) / PAGE_MIN as f32;
        let cw = (pic_h / 0.5).min(CARD_MAX_W).min(fit).max(CARD_MIN_W).min(inner);
        let page = (((inner + GAP) / (cw + GAP)).floor() as usize).clamp(1, PAGE_MAX);
        let card = [cw, cw * 0.5 + WORDS_H];
        let span = page as f32 * cw + (page - 1) as f32 * GAP;
        // a tall window: each band as tall as its cards, the rows centred under the title
        let row_h = row_h.min(HEAD_H + card[1] + BAR_H + 20.0);
        let top = y + TITLE_H + 4.0 + ((h - TITLE_H - 8.0 - (row_h + 6.0) * n as f32 + 6.0) / 2.0).max(0.0);
        let mut g = Geo { panel, band: vec![], head: vec![], slot0: vec![], card, page, bar: vec![] };
        for k in 0..n {
            let ry = top + (row_h + 6.0) * k as f32;
            g.band.push([x + 6.0, ry, w - 12.0, row_h]);
            g.head.push([x + 10.0, ry + 2.0, w - 20.0, HEAD_H]);
            let cy = ry + HEAD_H + 4.0 + ((row_h - HEAD_H - BAR_H - 8.0 - card[1]) / 2.0).max(0.0);
            g.slot0.push([x + (w - span) / 2.0, cy]);
            g.bar.push([x + (w - span) / 2.0, ry + row_h - BAR_H, span, 3.0]);
        }
        g
    }
}

/// Each row's chosen card and its first and last+1 in view.
fn windows(m: &Menu, rows: &[Vec<usize>; 2], page: usize) -> [(usize, usize, usize); 2] {
    let (cr, ci) = at(m, rows);
    std::array::from_fn(|r| {
        let n = rows[r].len();
        let i = if r == cr { ci } else { m.court_pick.keep[r].min(n.saturating_sub(1)) };
        let first = view(m.court_pick.first[r], i, n, page);
        (i, first, (first + page).min(n))
    })
}

/// The cards in view: (row, court value, rect).
pub(super) fn shown(m: &Menu) -> Vec<(usize, usize, [f32; 4])> {
    let g = m.court_picker_geo();
    let rows = rows(m);
    let win = windows(m, &rows, g.page);
    let mut v = Vec::new();
    for r in 0..g.band.len() {
        let (_, first, last) = win[r];
        let [sx, sy] = g.slot0[r];
        v.extend(rows[r][first..last].iter().enumerate().map(|(k, &c)| (r, c, [sx + (g.card[0] + GAP) * k as f32, sy, g.card[0], g.card[1]])));
    }
    v
}

/// ←→ (a card), ↑↓ (the other row), L1 / R1 (a page) on the open picker: the court follows the cursor.
pub(super) fn step(m: &mut Menu, k: Press, out: &mut Vec<Effect>) {
    let rows = rows(m);
    let page = m.court_picker_geo().page;
    let (mut r, mut i) = at(m, &rows);
    let was = m.court;
    let d = k.right as i32 - k.left as i32;
    let n = rows[r].len();
    if d != 0 {
        i = (i as i32 + d).clamp(0, n as i32 - 1) as usize;
    }
    let to = if k.up { Some(0) } else if k.down { Some(1) } else { None };
    if let Some(to) = to.filter(|&to| to != r && !rows[to].is_empty()) {
        m.court_pick.keep[r] = i;
        r = to;
        i = m.court_pick.keep[r].min(rows[r].len() - 1);
    }
    let p = k.next as i32 - k.prev as i32;
    if p != 0 && n > page {
        let n = rows[r].len();
        let first = view(m.court_pick.first[r], i, n, page);
        m.court_pick.first[r] = (first as i32 + p * page as i32).clamp(0, (n - page) as i32) as usize;
        i = (i as i32 + p * page as i32).clamp(0, n as i32 - 1) as usize;
    }
    m.court = rows[r][i];
    m.court_pick.keep[r] = i;
    m.court_pick.first[r] = view(m.court_pick.first[r], i, rows[r].len(), page);
    if m.court != was {
        out.push(Effect::Sound(0));
    }
}

/// Whether ↑↓ (a mods row) and L1 / R1 (the chosen row longer than a page) do anything: the legend's.
pub(super) fn axes(m: &Menu) -> (bool, bool) {
    let rows = rows(m);
    let (r, _) = at(m, &rows);
    (!rows[1].is_empty(), rows[r].len() > m.court_picker_geo().page)
}

/// The picker over the players' area (the screen under it is drawn first): the title, each row's head ("< a–b of N >"),
/// its cards in view and its scroll bar. A tap on a card chooses it, on the chosen one keeps it; on a row's < > pages
/// it; off the panel puts the old court back.
pub(super) fn draw(d: &mut Draw, m: &Menu, text: &Text) {
    let g = m.court_picker_geo();
    let rows = rows(m);
    let (cr, _) = at(m, &rows);
    let win = windows(m, &rows, g.page);
    d.popup(Some(pointer::press(|k| k.back = true)));
    // what the panel hides of the screen under it isn't drawn (the menu's quad pool)
    let [px, py, pw, ph] = g.panel;
    let anchor = d.anchor;
    d.out.retain(|q| q.anchor != anchor || q.dst[0] < px || q.dst[1] < py || q.dst[0] + q.dst[2] > px + pw || q.dst[1] + q.dst[3] > py + ph);
    d.box3(g.panel, 14.0, [Some(INK), Some(WHITE), Some(GROUND)], 128.0);
    d.outlined(|d| super::left(d, "Choose the court", px + 12.0, py + 5.0, 14.0, pw * 0.6, WHITE));
    let note = if m.online() && !m.court_mods.is_empty() { "Mods every player has" } else { "" };
    if !note.is_empty() {
        let nw = d.width(note, 8.0).min(pw * 0.35);
        dotted(d, note, px + pw - 12.0 - nw, py + 9.0, 8.0, pw * 0.35, CAPTION, false);
    }
    for (r, row) in rows.iter().enumerate().take(g.band.len()) {
        let n = row.len();
        let (i, first, last) = win[r];
        d.box3(g.band[r], 10.0, [None, if r == cr { Some([60.0, 70.0, 96.0]) } else { None }, Some(BAND)], 128.0);
        // the head: the row's name, "< a–b of N >" at its right
        let [hx, hy, hw, _] = g.head[r];
        let name = if r == 0 { "COURTS" } else { "MOD COURTS" };
        d.text(name, hx + 2.0, hy + 3.0, 9.0, if r == cr { GOLD } else { [96.0, 100.0, 112.0] });
        let range = format!("{}-{} of {n}", first + 1, last);
        let rw = d.width(&range, 9.0);
        let rx = hx + hw - 16.0 - rw;
        d.text(&range, rx, hy + 3.0, 9.0, [104.0, 108.0, 120.0]);
        for (left, ax, more) in [(true, rx - 14.0, first > 0), (false, hx + hw - 10.0, last < n)] {
            d.text(if left { "<" } else { ">" }, ax, hy + 1.0, 12.0, if more { WHITE } else { [50.0, 54.0, 66.0] });
            if more {
                // a page over in this row: the card as far along (it then chooses this row)
                let j = if left { i.saturating_sub(g.page) } else { (i + g.page).min(n - 1) };
                d.hit([ax - 4.0, hy - 2.0, 16.0, HEAD_H + 2.0], pointer::Act::Pick(row[j]));
            }
        }
        // the cards in view
        for (_, c, rect) in shown(m).into_iter().filter(|s| s.0 == r) {
            card(d, m, text, c, rect, c == row[i], r == cr);
            d.hit(rect, pointer::Act::Pick(c));
        }
        // the scroll bar: the cards in view's share of the row
        let [bx, by, bw, bh] = g.bar[r];
        d.q(Tex::Solid, [0.0; 4], [bx, by, bw, bh], [30.0, 36.0, 52.0], 128.0);
        if n > 0 {
            let (tx, tw) = (bx + bw * first as f32 / n as f32, (bw * (last - first) as f32 / n as f32).max(bh));
            d.q(Tex::Solid, [0.0; 4], [tx, by, tw, bh], if r == cr { GOLD } else { [80.0, 86.0, 104.0] }, 128.0);
        }
    }
}

/// One court's card: its picture, its name, its surface (a mod: its game; Random: how many it picks from); gold-rimmed
/// while it's the chosen card of the row in use (the other row's chosen card dimly).
fn card(d: &mut Draw, m: &Menu, text: &Text, c: usize, [x, y, w, h]: [f32; 4], on: bool, live: bool) {
    let rim = match (on, live) {
        (true, true) => GOLD,
        (true, false) => [90.0, 84.0, 60.0],
        _ => [40.0, 46.0, 62.0],
    };
    d.box3([x, y, w, h], 8.0, [Some(INK), Some(rim), Some(if on && live { PAPER } else { [96.0, 98.0, 104.0] })], 128.0);
    let pic = [x + 3.0, y + 3.0, w - 6.0, (w - 6.0) * 0.5];
    super::court_picture(d, m, c, pic);
    let ty = pic[1] + pic[3] + 2.0;
    let (name, sub) = match m.court_mod_of(c) {
        Some(_) => (m.court_title(text, c), m.court_lines(text, c)[1].clone()),
        None if c < COURTS => (m.court_title(text, c), ["Grass Court", "Clay Court", "Hard Court"][SURFACE[c]].to_string()),
        None => ("Random".to_string(), format!("Any of the {COURTS} courts")),
    };
    super::left(d, &name, x + 5.0, ty, 10.0, w - 10.0, NAVY);
    dotted(d, &sub, x + 5.0, ty + 12.0, 7.5, w - 10.0, if on && live { CAPTION } else { SLATE }, false);
}

#[cfg(test)]
mod tests {
    use super::super::{Dev, Grid, Screen, default};
    use super::*;
    use std::path::PathBuf;

    fn press(f: fn(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    fn court(i: usize) -> crate::mods_court::Court {
        let (id, name, dir) = (format!("c{i}"), format!("Court {i}"), PathBuf::from(format!("c{i}")));
        crate::mods_court::Court { dir, id, name, source: "Get a Grip".into(), layout: 5, light: None, sky: false, gimmick: false, bgm: None, ambience: Vec::new(), lens_flare: None, people: None }
    }

    fn picker(mods: usize, aspect: f32) -> Menu {
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], grid: Grid::fit(aspect), ..default() };
        m.court_mods = (0..mods).map(court).collect();
        m.sel = 1;
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!((m.sel, m.picking), (1, Some(COURTS)), "OK on Court opens the picker");
        m
    }

    /// Random first, the disc's courts in the strip's order; ↓ only with mods, each row keeping its place; L1 / R1 a
    /// page; ✕ keeps it, ○ puts the court it opened on back.
    #[test]
    fn rows_walk_and_page() {
        let mut m = picker(0, 4.0 / 3.0);
        assert_eq!(rows(&m)[0][..3], [COURTS, STRIP[0], STRIP[1]]);
        m.step(Dev::Keys, press(|p| &mut p.down));
        assert_eq!(m.court, COURTS, "no mods row: ↓ does nothing");
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_eq!(m.court, STRIP[0]);
        m.step(Dev::Keys, press(|p| &mut p.back));
        assert_eq!((m.court, m.picking), (COURTS, None), "○ puts it back");

        let mut m = picker(30, 4.0 / 3.0);
        let page = m.court_picker_geo().page;
        assert!((PAGE_MIN..=PAGE_MAX).contains(&page), "{page}");
        m.step(Dev::Keys, press(|p| &mut p.right));
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_eq!(m.court, STRIP[1]);
        m.step(Dev::Keys, press(|p| &mut p.down));
        assert_eq!(m.court, COURTS + 1, "the mods row from its first");
        m.step(Dev::Keys, press(|p| &mut p.next));
        assert_eq!(m.court, COURTS + 1 + page, "R1: a page on");
        assert_eq!(m.court_pick.first[1], page, "the row scrolled a page");
        m.step(Dev::Keys, press(|p| &mut p.up));
        assert_eq!(m.court, STRIP[1], "↑: back where the disc row was");
        m.step(Dev::Keys, press(|p| &mut p.down));
        assert_eq!(m.court, COURTS + 1 + page, "↓: back where the mods row was");
        for _ in 0..20 {
            m.step(Dev::Keys, press(|p| &mut p.next));
        }
        assert_eq!(m.court, COURTS + 30, "R1 stops at the end");
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!((m.court, m.picking), (COURTS + 30, None), "✕ keeps it");
    }

    /// The panel inside the players' area, its rows and cards inside the panel and apart, at any aspect.
    #[test]
    fn lays_out_at_any_aspect() {
        let inside = |[x, y, w, h]: [f32; 4], [l, t, ww, hh]: [f32; 4]| x >= l - 0.01 && y >= t - 0.01 && x + w <= l + ww + 0.01 && y + h <= t + hh + 0.01;
        for aspect in [0.5, 0.75, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 32.0 / 9.0] {
            for mods in [0, 40] {
                for doubles in [false, true] {
                    let mut m = picker(mods, aspect);
                    m.doubles = doubles;
                    let g = m.court_picker_geo();
                    let [l, t, r, b] = m.grid.reach();
                    assert!(inside(g.panel, [l, t, r - l, b - t]), "{aspect}: {:?}", g.panel);
                    assert_eq!(g.band.len(), if mods > 0 { 2 } else { 1 });
                    let side = m.confirm_geo().cells[0];
                    assert!(!(g.panel[0] < side[0] + side[2] && side[0] < g.panel[0] + g.panel[2] && g.panel[1] < side[1] + side[3] && side[1] < g.panel[1] + g.panel[3]), "{aspect}: the panel covers the court card");
                    for (k, band) in g.band.iter().enumerate() {
                        assert!(inside(*band, g.panel), "{aspect}: band {band:?}");
                        let [sx, sy] = g.slot0[k];
                        let last = [sx + (g.card[0] + GAP) * (g.page - 1) as f32, sy, g.card[0], g.card[1]];
                        assert!(inside([sx, sy, g.card[0], g.card[1]], *band) && inside(last, *band), "{aspect} {mods}: cards {:?} in {band:?}", g.card);
                        assert!(inside(g.bar[k], *band));
                    }
                    assert!(g.card[0] >= CARD_MIN_W - 0.01 && g.page >= 1);
                }
            }
        }
    }

    /// Online, the mods row holds only the court mods every peer in the lobby has.
    #[test]
    fn online_only_shared_mods() {
        let mut m = picker(3, 4.0 / 3.0);
        m.net.room = Some(crate::net::Room::default());
        (m.net.go, m.net.hosting) = (Some(super::super::online::Go::Host), true);
        let r = m.net.room.as_mut().unwrap();
        r.names.insert("host".into(), "h".into());
        r.names.insert("g".into(), "g".into());
        r.courts.insert("host".into(), vec!["c0".into(), "c1".into(), "c2".into()]);
        r.courts.insert("g".into(), vec!["c2".into(), "c0".into()]);
        assert!(m.online());
        assert_eq!(rows(&m)[1], [COURTS + 1, COURTS + 3]);
        m.net.room.as_mut().unwrap().courts.insert("g".into(), vec![]);
        assert!(rows(&m)[1].is_empty(), "a peer with none: no mods row");
    }
}
