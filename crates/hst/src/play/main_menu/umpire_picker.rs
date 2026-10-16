//! B451: the umpire picker over the players (ours, the board "Concept v4: Umpire pressed", `picker=umpire`): the same
//! panel as B450's court picker. OK on the Umpire card of the confirm screen or the lobby's Umpire pill opens it over
//! the players' area (the screen stays up around it): row 1 the disc's five, Random first; row 2 the umpire mods, only
//! when any are installed. Each row scrolls sideways on its own ("< a–b of N >" and a scroll bar; ←→ walk it, L1 / R1
//! a page); ↑↓ change rows, each row keeping its place. A card is the umpire's round face, her name and where she's
//! from; △ on a mod that ships its own calls plays one of the chosen umpire's calls
//! (Preview Voice; a disc umpire's or such a mod's, B431). ✕ keeps the pick, ○ puts the old one back.

use bevy::prelude::Entity;

use super::frame::{CAPTION, NAVY, PAPER, SLATE, dotted};
use super::{Anchor, C00, CALLS, Draw, Effect, GOLD, INK, Menu, Press, RANDOM_TILE, Tex, Text, UMPIRES, WHITE, pointer, umpire_mods};

/// The panel's ground (the roster's navy) and a row's band (the court picker's).
const GROUND: [f32; 3] = [11.0, 16.5, 30.0];
const BAND: [f32; 3] = [18.0, 25.0, 42.0];
/// A card's widest, its narrowest, the gap between cards; its face well's height to its width; its words under it.
const CARD_MAX_W: f32 = 120.0;
const CARD_MIN_W: f32 = 64.0;
const GAP: f32 = 6.0;
const FACE: f32 = 0.62;
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

/// The picker's rows: Random and the disc's five, then the umpire mods (maybe none).
pub(super) fn rows(m: &Menu) -> [Vec<usize>; 2] {
    [std::iter::once(UMPIRES).chain(0..UMPIRES).collect(), umpire_mods::values(m).collect()]
}

/// The chosen umpire's row and place in it (Random's when it's in neither).
fn at(m: &Menu, rows: &[Vec<usize>; 2]) -> (usize, usize) {
    (0..2).find_map(|r| rows[r].iter().position(|&u| u == m.umpire as usize).map(|i| (r, i))).unwrap_or((0, 0))
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
    /// Per row shown: its band, its head line, the first card slot's corner, the scroll bar's track; a card's size
    /// and the cards a page.
    pub band: Vec<[f32; 4]>,
    pub head: Vec<[f32; 4]>,
    pub slot0: Vec<[f32; 2]>,
    pub bar: Vec<[f32; 4]>,
    pub card: [f32; 2],
    pub page: usize,
}

/// The players' area the panel covers: the court stage between the top bar and the strip. The confirm screen's and
/// the lobby's frames agree on it (`frame.rs`).
fn area(m: &Menu) -> [f32; 4] {
    if m.net.venue { m.lobby_geo().stage } else { m.confirm_geo().stage }
}

impl Menu {
    pub(super) fn umpire_picker_geo(&self) -> Geo {
        let panel = area(self);
        let [x, y, w, h] = panel;
        let n = if rows(self)[1].is_empty() { 1 } else { 2 };
        // each row: its head, the cards, the scroll bar; a card no taller than its share, no wider than the rows
        let row_h = (h - TITLE_H - 8.0 - 6.0 * (n - 1) as f32) / n as f32;
        let face_h = (row_h - HEAD_H - BAR_H - WORDS_H - 8.0).max(16.0);
        let inner = w - 20.0;
        let cw = (face_h / FACE).clamp(CARD_MIN_W, CARD_MAX_W).min(inner);
        let page = (((inner + GAP) / (cw + GAP)).floor() as usize).max(1);
        let card = [cw, cw * FACE + WORDS_H];
        let span = page as f32 * cw + (page - 1) as f32 * GAP;
        let mut g = Geo { panel, band: vec![], head: vec![], slot0: vec![], bar: vec![], card, page };
        for k in 0..n {
            let ry = y + TITLE_H + 4.0 + (row_h + 6.0) * k as f32;
            g.band.push([x + 6.0, ry, w - 12.0, row_h]);
            g.head.push([x + 10.0, ry + 2.0, w - 20.0, HEAD_H]);
            let cy = ry + HEAD_H + 4.0 + ((row_h - HEAD_H - BAR_H - 8.0 - card[1]) / 2.0).max(0.0);
            g.slot0.push([x + (w - span) / 2.0, cy]);
            g.bar.push([x + (w - span) / 2.0, ry + row_h - BAR_H, span, 3.0]);
        }
        g
    }
}

/// The open picker's presses: ←→ (a card), ↑↓ (the other row), L1 / R1 (a page), the umpire following the cursor; △
/// her call; ✕ keeps her (the next run's starting choice when she's from the disc), ○ puts the old one back. Opened
/// from the lobby: back there once it closes.
pub(super) fn step(m: &mut Menu, k: Press, out: &mut Vec<Effect>) {
    let rows = rows(m);
    let page = m.umpire_picker_geo().page;
    let (mut r, mut i) = at(m, &rows);
    let was = m.umpire;
    let d = k.right as i32 - k.left as i32;
    let n = rows[r].len();
    if d != 0 {
        i = (i as i32 + d).clamp(0, n as i32 - 1) as usize;
    }
    let to = if k.up { Some(0) } else if k.down { Some(1) } else { None };
    if let Some(to) = to.filter(|&to| to != r && !rows[to].is_empty()) {
        m.umpire_pick.keep[r] = i;
        r = to;
        i = m.umpire_pick.keep[r].min(rows[r].len() - 1);
    }
    let p = k.next as i32 - k.prev as i32;
    let n = rows[r].len();
    if p != 0 && n > page {
        let first = view(m.umpire_pick.first[r], i, n, page);
        m.umpire_pick.first[r] = (first as i32 + p * page as i32).clamp(0, (n - page) as i32) as usize;
        i = (i as i32 + p * page as i32).clamp(0, n as i32 - 1) as usize;
    }
    m.umpire = rows[r][i] as u8;
    m.umpire_pick.keep[r] = i;
    m.umpire_pick.first[r] = view(m.umpire_pick.first[r], i, n, page);
    if m.umpire != was {
        out.push(Effect::Sound(0));
    }
    if k.card && m.umpire_voiced(m.umpire as usize) {
        // the lob key (△): Preview Voice
        let (program, key) = CALLS[m.calls % CALLS.len()];
        m.calls += 1;
        out.push(Effect::Hear(m.umpire, program, key));
    }
    if k.ok {
        m.picking = None;
        out.push(Effect::Sound(2));
        // B191: a disc pick is the next confirm screen's (and the next run's) starting choice
        if m.settings.6 != m.umpire && m.umpire as usize <= UMPIRES {
            m.settings.6 = m.umpire;
            out.push(Effect::SaveSettings);
        }
    } else if k.back {
        m.umpire = m.picking.take().unwrap_or_default() as u8;
        out.push(Effect::Sound(1));
    }
    if m.picking.is_none() {
        // opened from the lobby's Umpire pill: back there
        m.venue_picked(out);
    }
}

/// Whether ↑↓ (a mods row), L1 / R1 (the chosen row longer than a page) and △ (the chosen umpire has a voice) do
/// anything: the legend's.
pub(super) fn axes(m: &Menu) -> (bool, bool, bool) {
    let rows = rows(m);
    let (r, _) = at(m, &rows);
    (!rows[1].is_empty(), rows[r].len() > m.umpire_picker_geo().page, m.umpire_voiced(m.umpire as usize))
}

/// The picker over the screen it was opened from (the confirm screen's players, or the lobby's).
pub(super) fn draw_over(d: &mut Draw, m: &Menu, text: &Text, pads: &[Entity], hand_x: f32) {
    if m.net.venue {
        super::online::layout(d, &m.lobby_under_umpire(), hand_x);
    } else {
        super::confirm::draw(d, m, text, pads, hand_x);
    }
    // what the panel hides isn't drawn (the players' cards would take most of the menu's quad pool)
    let [px, py, pw, ph] = m.umpire_picker_geo().panel;
    d.out.retain(|q| q.anchor != Anchor::Centre || !(q.dst[0] >= px && q.dst[1] >= py && q.dst[0] + q.dst[2] <= px + pw && q.dst[1] + q.dst[3] <= py + ph));
    draw(d, m, text);
}

/// The panel: the title, each row's head ("< a–b of N >"), its cards in view and its scroll bar. A tap on a card
/// chooses it, on the chosen one keeps it; on a row's < > pages it; off the panel puts the old umpire back.
fn draw(d: &mut Draw, m: &Menu, text: &Text) {
    let g = m.umpire_picker_geo();
    let rows = rows(m);
    let (cr, ci) = at(m, &rows);
    d.popup(Some(pointer::press(|k| k.back = true)));
    let [px, py, pw, _] = g.panel;
    d.box3(g.panel, 14.0, [Some(INK), Some(WHITE), Some(GROUND)], 128.0);
    d.outlined(|d| super::left(d, "Choose the umpire", px + 12.0, py + 5.0, 14.0, pw * 0.6, WHITE));
    for (r, row) in rows.iter().enumerate().take(g.band.len()) {
        let n = row.len();
        let i = if r == cr { ci } else { m.umpire_pick.keep[r].min(n.saturating_sub(1)) };
        let first = view(m.umpire_pick.first[r], i, n, g.page);
        let last = (first + g.page).min(n);
        d.box3(g.band[r], 10.0, [None, if r == cr { Some([60.0, 70.0, 96.0]) } else { None }, Some(BAND)], 128.0);
        // the head: the row's name, "< a–b of N >" at its right
        let [hx, hy, hw, _] = g.head[r];
        let name = if r == 0 { "UMPIRES" } else { "MOD UMPIRES" };
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
        let [sx, sy] = g.slot0[r];
        for (k, &u) in row[first..last].iter().enumerate() {
            let rect = [sx + (g.card[0] + GAP) * k as f32, sy, g.card[0], g.card[1]];
            card(d, m, text, u, rect, first + k == i, r == cr);
            d.hit(rect, pointer::Act::Pick(u));
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

/// Umpire `u`'s name and where she's from on her card: the disc's, a mod's game (B408), Random's.
fn words(m: &Menu, text: &Text, u: usize) -> (String, String) {
    match m.umpire_mod_of(u) {
        Some(x) => (x.name.clone(), x.game().to_string()),
        None if u < UMPIRES => (m.umpire_name(text, u), "Hot Shots Tennis".to_string()),
        None => ("Random".to_string(), "Picked at the start".to_string()),
    }
}

/// One umpire's card: her round face in its well, her name and where she's from under it;
/// gold-rimmed while she's the chosen card of the row in use (the other row's chosen card dimly).
fn card(d: &mut Draw, m: &Menu, text: &Text, u: usize, [x, y, w, h]: [f32; 4], on: bool, live: bool) {
    let rim = match (on, live) {
        (true, true) => GOLD,
        (true, false) => [90.0, 84.0, 60.0],
        _ => [40.0, 46.0, 62.0],
    };
    let lit = on && live;
    d.box3([x, y, w, h], 8.0, [Some(INK), Some(rim), Some(if lit { PAPER } else { [96.0, 98.0, 104.0] })], 128.0);
    let well = [x + 3.0, y + 3.0, w - 6.0, w * FACE - 3.0];
    d.q(Tex::Solid, [0.0; 4], well, if lit { [70.0, 66.0, 86.0] } else { [44.0, 40.0, 54.0] }, 128.0);
    d.out.last_mut().unwrap().round = [5.0; 4];
    let size = (well[3] - 6.0).min(well[2] - 6.0);
    face(d, m, u, [well[0] + well[2] / 2.0, well[1] + well[3] / 2.0], size);
    let ty = well[1] + well[3] + 2.0;
    let (name, sub) = words(m, text, u);
    super::left(d, &name, x + 5.0, ty, 10.0, w - 10.0, NAVY);
    dotted(d, &sub, x + 5.0, ty + 12.0, 7.5, w - 10.0, if lit { CAPTION } else { SLATE }, false);
}

/// Umpire `u`'s round face `size` across centred on `[x, y]`: a mod's own head (B400), the disc's face
/// (`Confirmation_00`, a mod without a picture showing its donor's), Random's "?" (B356). As the confirm card's.
fn face(d: &mut Draw, m: &Menu, u: usize, [x, y]: [f32; 2], size: f32) {
    if umpire_mods::face(d, m, u, [x, y], size) {
        return;
    }
    let donor = m.umpire_donor(u);
    if donor == UMPIRES {
        let [fu, fv, tw, th] = RANDOM_TILE;
        super::random_face(d, [fu + (tw - th) / 2.0 + 4.0, fv + 4.0, th - 8.0, th - 8.0], [x, y], size);
    } else {
        let fw = size * 46.0 / 44.0;
        d.q(C00, [63.0 + 48.0 * donor as f32, 272.0, 46.0, 44.0], [x - fw / 2.0, y - size / 2.0, fw, size], WHITE, 128.0);
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Dev, Grid, Screen, default};
    use super::*;

    fn press(f: fn(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    fn ump(i: usize) -> crate::mods_umpire::Umpire {
        crate::mods_umpire::Umpire { dir: format!("u{i}").into(), id: format!("u{i}"), name: format!("Umpire {i}"), source: "Hot Shots Tennis: Get a Grip".into(), donor: (i % 5) as u8, voice: "voice/".into() }
    }

    /// The confirm screen with `mods` umpire mods (every other one voiced), the picker opened on its Umpire card.
    fn picker(mods: usize, aspect: f32) -> Menu {
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], grid: Grid::fit(aspect), ..default() };
        m.umpire_mods = (0..mods).map(ump).collect();
        m.umpire_voices = (0..mods).map(|i| i % 2 == 0).collect();
        m.umpire = UMPIRES as u8;
        m.sel = 4;
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!((m.sel, m.picking), (4, Some(UMPIRES)), "OK on Umpire opens the picker");
        m
    }

    /// Random first, the disc's five; ↓ only with mods, each row keeping its place; L1 / R1 a page; ✕ keeps it (a disc
    /// pick saved, a mod not), ○ puts the umpire it opened on back.
    #[test]
    fn rows_walk_and_page() {
        let mut m = picker(0, 4.0 / 3.0);
        assert_eq!(rows(&m)[0], [UMPIRES, 0, 1, 2, 3, 4]);
        m.step(Dev::Keys, press(|p| &mut p.down));
        assert_eq!(m.umpire as usize, UMPIRES, "no mods row: ↓ does nothing");
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_eq!(m.umpire, 0);
        m.step(Dev::Keys, press(|p| &mut p.back));
        assert_eq!((m.umpire as usize, m.picking), (UMPIRES, None), "○ puts it back");

        let mut m = picker(30, 4.0 / 3.0);
        let page = m.umpire_picker_geo().page;
        assert!((3..30).contains(&page), "{page}");
        m.step(Dev::Keys, press(|p| &mut p.right));
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_eq!(m.umpire, 1);
        m.step(Dev::Keys, press(|p| &mut p.down));
        assert_eq!(m.umpire as usize, UMPIRES + 1, "the mods row from its first");
        m.step(Dev::Keys, press(|p| &mut p.next));
        assert_eq!(m.umpire as usize, UMPIRES + 1 + page, "R1: a page on");
        assert_eq!(m.umpire_pick.first[1], page, "the row scrolled a page");
        m.step(Dev::Keys, press(|p| &mut p.up));
        assert_eq!(m.umpire, 1, "↑: back where the disc row was");
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!((m.picking, m.settings.6), (None, 1), "✕ keeps a disc umpire, saved");
        m.step(Dev::Keys, press(|p| &mut p.ok));
        m.step(Dev::Keys, press(|p| &mut p.down));
        assert_eq!(m.umpire as usize, UMPIRES + 1 + page, "↓: back where the mods row was");
        for _ in 0..20 {
            m.step(Dev::Keys, press(|p| &mut p.next));
        }
        assert_eq!(m.umpire as usize, UMPIRES + 30, "R1 stops at the end");
        m.step(Dev::Keys, press(|p| &mut p.ok));
        assert_eq!((m.umpire as usize, m.picking, m.settings.6), (UMPIRES + 30, None, 1), "✕ keeps a mod, not saved");
    }

    /// △ plays a call for a disc umpire and a mod with its own calls, never Random or a mod without; the legend says
    /// so only then.
    #[test]
    fn voice_only_where_there_is_one() {
        let hear = |m: &mut Menu| m.clone().step(Dev::Keys, press(|p| &mut p.card)).iter().any(|e| matches!(e, Effect::Hear(..)));
        let mut m = picker(2, 4.0 / 3.0);
        for (u, voiced) in [(UMPIRES, false), (0, true), (4, true), (UMPIRES + 1, true), (UMPIRES + 2, false)] {
            m.umpire = u as u8;
            assert_eq!(hear(&mut m), voiced, "umpire {u}");
            assert_eq!(axes(&m).2, voiced);
            let hint = legend::acts(&m, Dev::Keys).iter().any(|(a, l)| *a == legend::Act::Card && l == "Hear voice");
            assert_eq!(hint, voiced, "umpire {u}: legend");
        }
    }

    /// The panel inside the players' area, never over the side panel, its rows and cards inside it, at any aspect.
    #[test]
    fn lays_out_at_any_aspect() {
        let inside = |[x, y, w, h]: [f32; 4], [l, t, ww, hh]: [f32; 4]| x >= l - 0.01 && y >= t - 0.01 && x + w <= l + ww + 0.01 && y + h <= t + hh + 0.01;
        for aspect in [0.5, 0.75, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 32.0 / 9.0] {
            for mods in [0, 40] {
                for doubles in [false, true] {
                    let mut m = picker(mods, aspect);
                    m.doubles = doubles;
                    let g = m.umpire_picker_geo();
                    let [l, t, r, b] = m.grid.reach();
                    assert!(inside(g.panel, [l, t, r - l, b - t]), "{aspect}: {:?}", g.panel);
                    assert_eq!(g.band.len(), if mods > 0 { 2 } else { 1 });
                    let side = m.confirm_geo().cells[1];
                    assert!(!(g.panel[0] < side[0] + side[2] && side[0] < g.panel[0] + g.panel[2] && g.panel[1] < side[1] + side[3] && side[1] < g.panel[1] + g.panel[3]), "{aspect}: the panel covers the umpire card");
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
}

#[cfg(test)]
mod pool {
    use super::super::*;

    /// The picker over the busiest confirm screen and lobby (doubles, both rows full) draws within the menu's quad pool.
    #[test]
    fn picker_fits_the_pool() {
        let t = pointer::tests::text_with_mods(100);
        for a in [21.0 / 9.0, 16.0 / 9.0, 4.0 / 3.0, 9.0 / 16.0] {
            let mut m = Menu { screen: Screen::Confirm, doubles: true, seats: [Some(Dev::Keys), None, None, None], grid: Grid::fit(a), ..default() };
            m.umpire_mods = (0..40).map(|i| crate::mods_umpire::Umpire { dir: format!("u{i}").into(), id: format!("u{i}"), name: format!("Longer Umpire {i}"), source: "Series: Game".into(), donor: 0, voice: "voice/".into() }).collect();
            m.umpire_voices = vec![true; 40];
            (m.sel, m.picking) = (4, Some(UMPIRES));
            let n = drawn(&m, &t, &Bindings::default(), &[], 0, 0.0).out.len();
            assert!(n < POOL * 9 / 10, "{a}: {n} quads");
            // opened from the lobby's Umpire pill: over the lobby
            online::sample(&mut m);
            m.net.venue = true;
            let n = drawn(&m, &t, &Bindings::default(), &[], 0, 0.0).out.len();
            assert!(n < POOL * 9 / 10, "{a} lobby: {n} quads");
        }
    }
}
