//! B368: the character select (`Screen::Chars`: the local roster and, until B370's lobby, an online
//! room's) as one column per player (`plan/B368.md`, the approved 1280×720 mockup). Each column is its player's own:
//! the header (seat, character, device), the costume and hand pills, the 3D preview beside the play style and stats
//! card, a tab row (B369's categories, `tabs.rs`), and a grid of face tiles that scrolls inside that column
//! only. Sizes are the mockup's pixels at 720 lines ([`PY`] units down, [`PX`] across, so a tile is square on screen);
//! a narrower column fits fewer tiles a row (13 / 6 / 4 / 3 for 1–4 columns at 16:9), a taller view more rows.

use bevy::prelude::*;

use super::super::controls::Bindings;
use super::super::widescreen::Anchor;
use super::{C02, CHAR_NAMES, Dev, Draw, Menu, RANDOM_TILE, STYLE_NAMES, Tex, Text, WHITE, Who, dev_name, drop_button, grades_of, home, legend, pointer, shirt, tabs, tile_face};

/// One mockup pixel in the menu's units: down, and across (a unit across is 14/15 of one down on screen: 640 across
/// a 4:3 screen 448 high).
pub(super) const PY: f32 = 448.0 / 720.0;
pub(super) const PX: f32 = PY * 15.0 / 14.0;

/// The mockup's sizes (px at 720 lines): the top bar over the columns, the side margins and the columns' gap, a
/// column's padding and the gap between its rows, the rows' heights, the stats stack's width, a tile and its gap,
/// the scroll bar.
const TOP_BAR: f32 = 60.0;
const MARGIN: f32 = 24.0;
const GAP: f32 = 12.0;
const PAD: f32 = 10.0;
const ROW_GAP: f32 = 8.0;
const HEAD: f32 = 36.0;
const PILL: f32 = 26.0;
const PREVIEW: f32 = 150.0;
const TABS: f32 = 24.0;
const FOOT: f32 = 14.0;
const STATS_W: f32 = 150.0;
const TILE: f32 = 84.0;
const TILE_GAP: f32 = 6.0;
const BAR: f32 = 6.0;
/// The pills go on two rows in a column narrower than this inside (px).
const PILLS_W: f32 = 290.0;

/// `#rrggbb` in the menus' 0–128 colour.
fn hex(c: u32) -> [f32; 3] {
    [(c >> 16) & 255, (c >> 8) & 255, c & 255].map(|v| v as f32 * 128.0 / 255.0)
}

/// The screen's background.
const BG: u32 = 0x0e1a2e;

/// A seat's colours (the mockup's): its ring, a selected tile, the column, the pills' well, a tile, soft words.
struct Seat {
    ring: [f32; 3],
    sel: [f32; 3],
    bg: [f32; 3],
    well: [f32; 3],
    tile: [f32; 3],
    soft: [f32; 3],
}

fn seat(p: usize) -> Seat {
    const C: [[u32; 6]; 4] = [
        [0xff6b6b, 0xb33b3b, 0x4a1c1c, 0x5e1f1f, 0x6a2a2a, 0xffd0d0],
        [0xffd23f, 0x8a6a1a, 0x3d300b, 0x4e3c0c, 0x5e4810, 0xfff0b8],
        [0x5cc8ff, 0x1f6d99, 0x122d44, 0x153a57, 0x1d4a6e, 0xcdeeff],
        [0x7fd99a, 0x2f6b44, 0x15291d, 0x183022, 0x20402c, 0xc9f2d4],
    ];
    let [ring, sel, bg, well, tile, soft] = C[p % 4].map(hex);
    Seat { ring, sel, bg, well, tile, soft }
}

/// The select's player columns on a view (`Grid::view`): how many tiles a row and rows on view each column's grid
/// has, and where each part of a column goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Columns {
    /// Player columns.
    pub n: usize,
    /// Tiles a row, and the rows on view.
    pub cols: usize,
    pub rows: usize,
    /// A tile's width and height (square on screen), and the gaps between them across and down.
    pub tile: [f32; 2],
    gap: [f32; 2],
    /// The first column's box, and the step to the next.
    first: [f32; 4],
    step: f32,
    /// The pills on two rows (a narrow column).
    pub stack: bool,
    /// The preview row's height.
    preview: f32,
}

/// The parts of a player's column.
pub(super) struct Parts {
    pub card: [f32; 4],
    pub head: [f32; 4],
    /// The pills' row (both rows when stacked).
    pub pills: [f32; 4],
    pub preview: [f32; 4],
    pub tabs: [f32; 4],
    /// The tiles on view, and the scroll bar right of them.
    pub grid: [f32; 4],
    pub bar: [f32; 4],
    pub foot: [f32; 4],
}

/// The select's `n` player columns on `view` (left, top, right, bottom).
pub(super) fn columns(view: [f32; 4], n: usize) -> Columns {
    let n = n.max(1);
    let [l, t, r, b] = view;
    let top = t + TOP_BAR * PY;
    let w = (r - l - 2.0 * MARGIN * PX - GAP * PX * (n - 1) as f32) / n as f32;
    let h = b - top;
    let inner = w - 2.0 * PAD * PX;
    let stack = inner < PILLS_W * PX;
    // the tiles fill the width left of the scroll bar's lane, at least `TILE` wide
    let gx = TILE_GAP * PX;
    let gw = inner - (BAR + TILE_GAP) * PX;
    let cols = (((gw + gx) / (TILE * PX + gx)) as usize).max(1);
    let tw = ((gw - gx * (cols - 1) as f32) / cols as f32).max(1.0);
    let th = tw * PY / PX;
    let gy = TILE_GAP * PY;
    let pills = if stack { 2.0 * PILL + ROW_GAP } else { PILL };
    let fixed = (2.0 * PAD + HEAD + pills + TABS + FOOT + 5.0 * ROW_GAP) * PY;
    // the preview gives way to keep a row of tiles on a short view
    let room = h - fixed;
    let preview = (PREVIEW * PY).min(room - th).max(60.0 * PY);
    let rows = (((room - preview + gy) / (th + gy)) as usize).max(1);
    // what's left under the last row whole goes to the preview, up to twice its size
    let spare = room - preview - (rows as f32 * (th + gy) - gy);
    let preview = preview + spare.clamp(0.0, PREVIEW * PY);
    Columns { n, cols, rows, tile: [tw, th], gap: [gx, gy], first: [l + MARGIN * PX, top, w, h], step: w + GAP * PX, stack, preview }
}

impl Columns {
    /// Tiles on view in a column.
    pub fn per(&self) -> usize {
        self.cols * self.rows
    }

    /// Player `p`'s column.
    pub fn parts(&self, p: usize) -> Parts {
        let [x0, top, w, h] = self.first;
        let x = x0 + self.step * p as f32;
        let (ix, iw) = (x + PAD * PX, w - 2.0 * PAD * PX);
        let mut y = top + PAD * PY;
        let mut next = |hh: f32| {
            let r = [ix, y, iw, hh];
            y += hh + ROW_GAP * PY;
            r
        };
        let head = next(HEAD * PY);
        let pills = next(if self.stack { 2.0 * PILL + ROW_GAP } else { PILL } * PY);
        let preview = next(self.preview);
        let tabs = next(TABS * PY);
        let gh = self.rows as f32 * (self.tile[1] + self.gap[1]) - self.gap[1];
        let grid = next(gh);
        let foot = [ix, top + h - (PAD + FOOT) * PY, iw, FOOT * PY];
        let bar = [ix + iw - BAR * PX, grid[1], BAR * PX, gh];
        Parts { card: [x, top, w, h], head, pills, preview, tabs, grid: [ix, grid[1], iw - (BAR + TILE_GAP) * PX, gh], bar, foot }
    }

    /// The `k`th tile on view in player `p`'s column.
    pub fn cell(&self, p: usize, k: usize) -> [f32; 4] {
        let [x, y, ..] = self.parts(p).grid;
        let [tw, th] = self.tile;
        [x + (tw + self.gap[0]) * (k % self.cols) as f32, y + (th + self.gap[1]) * (k / self.cols) as f32, tw, th]
    }
}

impl Menu {
    /// B368: the tiles on view in every column and the players' picks (the face icons to read).
    pub(super) fn on_view(&self) -> Vec<Who> {
        let per = self.columns().per();
        let mut v: Vec<Who> = (0..self.players()).flat_map(|p| {
            let (start, roster) = self.page(p);
            roster.into_iter().skip(start).take(per).flatten()
        }).collect();
        v.extend((0..self.players()).map(|p| self.who(p)));
        v
    }
}

/// A box `[x, y, w, h]` filled `rgb`, its corners rounded `r` mockup px.
fn rounded(d: &mut Draw, rect: [f32; 4], r: f32, rgb: [f32; 3], alpha: f32) {
    d.q(Tex::Solid, [0.0; 4], rect, rgb, alpha);
    d.out.last_mut().unwrap().round = [r * PX; 4];
}

/// A box with a `t`-px rim in `rim` round a `fill`, corners `r` px.
fn ringed(d: &mut Draw, [x, y, w, h]: [f32; 4], r: f32, t: f32, rim: [f32; 3], fill: [f32; 3]) {
    rounded(d, [x, y, w, h], r, rim, 128.0);
    rounded(d, [x + t * PX, y + t * PY, w - 2.0 * t * PX, h - 2.0 * t * PY], (r - t).max(0.0), fill, 128.0);
}

/// A dashed `t`-px rim round a box with `r`-px corners (left open): a computer player's column.
fn dashed(d: &mut Draw, [x, y, w, h]: [f32; 4], r: f32, t: f32, rgb: [f32; 3]) {
    let (dash, gap) = (10.0, 6.0);
    let (tx, ty) = (t * PX, t * PY);
    let mut i = r * PX;
    while i < w - r * PX {
        let l = (dash * PX).min(w - r * PX - i);
        d.q(Tex::Solid, [0.0; 4], [x + i, y, l, ty], rgb, 128.0);
        d.q(Tex::Solid, [0.0; 4], [x + i, y + h - ty, l, ty], rgb, 128.0);
        i += (dash + gap) * PX;
    }
    let mut i = r * PY;
    while i < h - r * PY {
        let l = (dash * PY).min(h - r * PY - i);
        d.q(Tex::Solid, [0.0; 4], [x, y + i, tx, l], rgb, 128.0);
        d.q(Tex::Solid, [0.0; 4], [x + w - tx, y + i, tx, l], rgb, 128.0);
        i += (dash + gap) * PY;
    }
}

/// `s` cut to fit `max` wide at `size`, "..." where it was cut.
fn ellipsis(d: &Draw, s: &str, size: f32, max: f32) -> String {
    if d.width(s, size) <= max {
        return s.to_string();
    }
    let mut cut: String = s.to_string();
    while !cut.is_empty() && d.width(&format!("{}...", cut.trim_end()), size) > max {
        cut.pop();
    }
    format!("{}...", cut.trim_end())
}

/// Words with a thin dark ring (four ways round: the stats card's labels).
fn ringed_text(d: &mut Draw, s: &str, x: f32, y: f32, size: f32, rgb: [f32; 3]) {
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        d.text(s, x + dx * PX, y + dy * PY, size, [40.0, 14.0, 30.0]);
    }
    d.text(s, x, y, size, rgb);
}

/// A button's name as a small chip in `rgb` with dark words (the legend's L1 / R1, Q / E, Select…), `h` high from
/// `[x, y]`; its width.
fn chip(d: &mut Draw, s: &str, [x, y]: [f32; 2], h: f32, rgb: [f32; 3]) -> f32 {
    let size = h * 0.72;
    let w = d.width(s, size) + 8.0 * PX;
    rounded(d, [x, y, w, h], 5.0, rgb, 128.0);
    d.centred(s, x + w / 2.0, y + (h - size) / 2.0, size, hex(BG));
    w
}

/// The width of a [`chip`] for `s`.
fn chip_w(d: &Draw, s: &str, h: f32) -> f32 {
    d.width(s, h * 0.72) + 8.0 * PX
}

/// The character select, one column per player.
pub(super) fn chars(d: &mut Draw, m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity]) {
    let [l, t, r, _] = m.grid.view;
    // the background over the whole window
    d.anchor = Anchor::Span;
    d.q(Tex::Solid, [0.0; 4], [0.0, 0.0, 640.0, 448.0], hex(BG), 128.0);
    d.anchor = Anchor::Centre;
    // the top bar: the title (Back for the pointer, after the "<" `drawn` puts over it), the players and the mode
    let size = 30.0 * PY;
    let title = "Choose your player";
    d.text(title, l + 32.0, t + 16.0 * PY, size, WHITE);
    d.hit([l, t, d.width(title, size) + 40.0, TOP_BAR * PY], pointer::press(|k| k.back = true));
    let (mode, fill) = if m.doubles { ("Doubles", 0x2f7d4f) } else { ("Singles", 0x2f5f9d) };
    let ms = 16.0 * PY;
    let mw = d.width(mode, ms) + 28.0 * PX;
    let mx = r - MARGIN * PX - mw;
    let my = t + 16.0 * PY;
    rounded(d, [mx, my, mw, 30.0 * PY], 15.0, WHITE, 128.0);
    rounded(d, [mx + 2.0 * PX, my + 2.0 * PY, mw - 4.0 * PX, 26.0 * PY], 13.0, hex(fill), 128.0);
    d.centred(mode, mx + mw / 2.0, my + 7.0 * PY, ms, WHITE);
    if m.local_roster() {
        // B375: a tap swaps Singles ↔ Doubles, as 1P's □ / Tab (step checks the device is 1P's)
        d.hit([mx, my, mw, 30.0 * PY], pointer::press(|k| k.page = true));
    }
    let humans = m.humans() + (0..m.players()).filter(|&p| m.seats[p].is_none() && m.remote(p).is_some_and(|s| s.human())).count();
    let who = if humans > 1 { format!("{humans} players, each browses on their own") } else { "1 player".into() };
    let ws = 14.0 * PY;
    let ww = d.width(&who, ws).min(mx - 12.0 * PX - (l + 40.0 + d.width(title, size)));
    if ww > 20.0 {
        d.label(&who, mx - 12.0 * PX - ww / 2.0, my + 8.0 * PY, ws, ww, hex(0xa8b4c8));
    }
    let c = m.columns();
    let pointer_p = m.driven(pointer::device(m, pads));
    for p in 0..c.n {
        column(d, m, text, bind, pads, &c, p, pointer_p == Some(p));
    }
    // B371: the name pickers over their players' columns (at least 180 wide)
    for p in 0..c.n {
        let [x, y, w, h] = c.parts(p).card;
        let nw = w.max(180.0).min(r - l);
        super::names::draw(d, m, p, [(x + (w - nw) / 2.0).clamp(l, r - nw), y, nw, h]);
    }
}

/// Player `p`'s column; `mine`: the pointer's device picks for it (its pills take taps).
#[allow(clippy::too_many_arguments)]
fn column(d: &mut Draw, m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity], c: &Columns, p: usize, mine: bool) {
    let n = c.n;
    let online = m.select_online();
    let s = seat(p);
    let pl = m.players[p];
    let at = c.parts(p);
    // who's picking (the old select's rules): a seated human, else a computer player once the humans before it are done
    let picking = |q: usize| m.seats[q].is_some() || m.bot(q) && (0..n).filter(|&o| m.bot(o)).all(|o| o >= q || m.players[o].ready) && (0..n).all(|o| m.seats[o].is_none() || m.players[o].ready);
    let picking = |q: usize| if online { m.seats[q].is_some() || m.remote(q).is_some() || m.net.edit == Some(q) } else { picking(q) };
    let active = picking(p) || pl.ready;
    let human = matches!(m.seats[p], Some(Dev::Keys | Dev::Pad(_))) || m.remote(p).is_some_and(|s| s.human());
    // the card: a rim in the seat's colour, dashed for a computer player
    let [x, y, w, _] = at.card;
    if human || m.seats[p] == Some(Dev::Open) {
        ringed(d, at.card, 14.0, 3.0, s.ring, s.bg);
    } else {
        rounded(d, at.card, 14.0, s.bg, 128.0);
        dashed(d, at.card, 14.0, 3.0, s.ring);
    }
    if mine {
        d.hit(at.card, pointer::Act::Grid(p));
    }

    // the header: seat tag, the character's name, the device on the right, on one baseline
    let [hx, hy, hw, hh] = at.head;
    let (big, small) = (34.0 * PY, 16.0 * PY);
    // B371: the player's name in place of "1P" (a tap on it opens the name picker)
    let tag = m.seat_tag(p);
    let tag = ellipsis(d, &tag, small, hw * 0.3);
    d.text(&tag, hx, hy + hh - small - 2.0 * PY, small, s.ring);
    let tw = d.width(&tag, small) + 8.0 * PX;
    if mine && m.seats[p].is_some() {
        d.hit([hx - 4.0, hy, tw + 4.0, hh], pointer::press(|k| k.name = true));
    }
    let device = match m.remote(p).filter(|s| s.human() && online) {
        Some(st) => m.net.room.as_ref().and_then(|r| r.names.get(&st.peer)).map_or("Online".to_string(), |n| n.clone()),
        None if m.seats[p] == Some(Dev::Open) => "Open".into(),
        None => dev_name(m.seats[p], pads),
    };
    let ds = 14.0 * PY;
    let dev_w = d.width(&device, ds).min(hw * 0.3);
    let drop = m.droppable(p);
    let dx = hx + hw - dev_w - if drop { 22.0 } else { 0.0 };
    d.label(&device, dx + dev_w / 2.0, hy + hh - ds - 3.0 * PY, ds, dev_w, s.soft);
    let name = match m.who(p) {
        _ if pl.random => "Random".to_string(),
        Who::Mod(i) => m.mods[i].name.clone(),
        // B234: another machine's mod this one doesn't have: its own name over the donor's looks
        Who::Disc(ch) => m.remote_mod_name(p).unwrap_or(CHAR_NAMES[ch]).to_string(),
    };
    let room = (dx - 8.0 * PX - (hx + tw)).max(10.0);
    let name = ellipsis(d, &name, big, room);
    d.text(&name, hx + tw, hy + hh - big + 2.0 * PY, big, WHITE);
    // B238: a joined device's ✕ on its column's corner
    if drop {
        drop_button(d, p, x + w - 30.0, y - 6.0);
    }

    // the pills: the costume (L1 / R1) on the left, the hand (Select) on the right
    pills(d, m, text, bind, &s, p, at.pills, c.stack, mine);

    // the preview, and the play style over the stats card on its right
    let [px, py, pw, ph] = at.preview;
    let sw = (STATS_W * PX).min(pw * 0.56);
    let (sx, area) = (px + pw - sw, pw - sw - 8.0 * PX);
    rounded(d, [px, py, area, ph], 10.0, s.tile, 128.0);
    if active {
        if pl.random {
            let fw = (area - 8.0 * PX).min((ph - 8.0 * PY) * 92.0 / 68.0);
            d.q(C02, RANDOM_TILE, [px + (area - fw) / 2.0, py + (ph - fw * 68.0 / 92.0) / 2.0, fw, fw * 68.0 / 92.0], WHITE, 128.0);
        } else {
            // the bake's 128×160, its shape on screen
            let k = 0.8 * PX / PY;
            let rw = area.min(ph * k);
            let rh = rw / k;
            d.q(Tex::Preview(p), [0.0; 4], [px + (area - rw) / 2.0, py + (ph - rh) / 2.0, rw, rh], WHITE, 128.0);
        }
    }
    let style_of = |w: Who| match w {
        Who::Mod(i) => text.mods[i].0,
        Who::Disc(ch) => text.style[ch],
    };
    let hidden = pl.random || !active;
    let sh = 24.0 * PY;
    rounded(d, [sx, py, sw, sh], 8.0, s.well, 128.0);
    let style = if hidden { "?" } else { STYLE_NAMES[style_of(m.who(p)).min(3)] };
    d.label(style, sx + sw / 2.0, py + 5.0 * PY, 14.0 * PY, sw - 10.0 * PX, hex(0xffd23f));
    stats(d, if hidden { None } else { Some(grades_of(text, m.who(p))) }, [sx, py + sh + 4.0 * PY, sw, ph - sh - 4.0 * PY]);
    if pl.ready {
        // "Ready!" across the preview's foot
        let rh = 24.0 * PY;
        rounded(d, [px + 6.0 * PX, py + ph - rh - 6.0 * PY, area - 12.0 * PX, rh], 12.0, hex(0xffd23f), 128.0);
        d.centred("Ready!", px + area / 2.0, py + ph - rh - 6.0 * PY + 4.0 * PY, 16.0 * PY, hex(BG));
    }

    // the tabs (B369's categories)
    let (start, roster) = m.page(p);
    tabs::row(d, m, p, at.tabs);

    // the grid: the tiles on view, this player's darker with a rim in the seat's colour
    let mine_cell = m.cell(p);
    for (k, &cell) in roster.iter().enumerate().skip(start).take(c.per()) {
        let rect = c.cell(p, k - start);
        let on = active && cell == mine_cell;
        if on {
            ringed(d, rect, 10.0, 3.0, s.ring, s.sel);
        } else {
            rounded(d, rect, 10.0, s.tile, 128.0);
        }
        let full = active && m.full(p, cell);
        let inset = if on { 5.0 } else { 3.0 };
        let [cx, cy, cw, ch] = rect;
        let (tex, src) = tile_face(cell);
        d.q(tex, src, [cx + inset * PX, cy + inset * PY, cw - 2.0 * inset * PX, ch - 2.0 * inset * PY], if full { [45.0; 3] } else { WHITE }, if active { 128.0 } else { 70.0 });
        d.out.last_mut().unwrap().round = [(10.0 - inset) * PX; 4];
        if let Some(w) = cell && m.favourite(w) {
            tabs::star(d, rect);
        }
        d.hit(rect, pointer::Act::Cell(p, k));
    }
    // the scroll bar: its thumb the rows on view over all of them
    let total = roster.len().div_ceil(c.cols);
    let [bx, by, bw, bh] = at.bar;
    rounded(d, at.bar, 3.0, s.well, 128.0);
    let thumb = (bh * c.rows as f32 / total.max(1) as f32).min(bh).max(12.0 * PY);
    let top = start / c.cols;
    let ty = by + (bh - thumb) * top as f32 / (total.saturating_sub(c.rows)).max(1) as f32;
    rounded(d, [bx, ty, bw, thumb], 3.0, s.ring, 128.0);

    // an open seat: who fills it, over its grid
    if !active {
        let [gx, gy, gw, gh] = at.grid;
        let bh = 52.0 * PY;
        let b = [gx + 6.0 * PX, gy + (gh - bh).min(60.0 * PY).max(0.0), gw - 12.0 * PX, bh];
        rounded(d, b, 12.0, s.well, 120.0);
        let line = if m.seats[p] == Some(Dev::Open) || !online && m.local_roster() {
            home::JOIN
        } else if online {
            "Waiting for a player"
        } else {
            "Picks after the players"
        };
        let size = 15.0 * PY;
        let cy = b[1] + (bh - size) / 2.0;
        if line == home::JOIN {
            home::press_hint(d, " to join", b[0] + b[2] / 2.0, cy, size, b[2] - 12.0 * PX, WHITE);
        } else {
            d.label(line, b[0] + b[2] / 2.0, cy, size, b[2] - 12.0 * PX, WHITE);
        }
    }

    // the footer: the tab and where the tile is in it
    let [fx, fy, fw, fh] = at.foot;
    let line = tabs::foot(m, p);
    let fs = 12.0 * PY;
    let lw = d.width(&line, fs).min(fw);
    d.label(&line, fx + lw / 2.0, fy + (fh - fs) / 2.0, fs, lw, s.soft);
}

/// The costume and hand pills in `[x, y, w, h]` (two rows when `stack`); taps on them when `mine`.
#[allow(clippy::too_many_arguments)]
fn pills(d: &mut Draw, m: &Menu, text: &Text, bind: &Bindings, s: &Seat, p: usize, [x, y, w, _]: [f32; 4], stack: bool, mine: bool) {
    let pl = m.players[p];
    let dev = match m.seats[p] {
        Some(Dev::Pad(e)) => Some(e),
        _ => None,
    };
    let ph = PILL * PY;
    let (ts, gh) = (13.0 * PY, 18.0 * PY);
    let names = |a: legend::Act| legend::button_names(dev, a, bind);
    let keys = !pl.ready && (m.seats[p].is_some() || m.driven(Dev::Keys) == Some(p));
    let costume = if pl.random { "Costume ?".to_string() } else if m.blocked(p) { "Taken".to_string() } else { format!("Costume {}", pl.costume + 1) };
    let hand = if pl.random { "Hand ?" } else if m.hand(text, p) < 0.0 { "Left-handed" } else { "Right-handed" };
    let cn = names(legend::Act::Costume);
    let (l1, r1) = (cn.first().cloned().unwrap_or_default(), cn.get(1).cloned().unwrap_or_default());
    let sel = names(legend::Act::Hand).first().cloned().unwrap_or_default();
    let icon = 18.0 * PX;
    let pad = 8.0 * PX;
    let gap = 6.0 * PX;
    let chips = |d: &Draw, a: &str| if keys && !a.is_empty() { chip_w(d, a, gh) + gap } else { 0.0 };
    let cw_nat = pad + icon + gap + chips(d, &l1) + d.width(&costume, ts) + gap + chips(d, &r1) + pad - gap;
    let hw_nat = pad + icon + gap + d.width(hand, ts) + gap + chips(d, &sel) + pad - gap;
    // side by side, or the costume over the hand; squeezed to fit
    let (cw, hw) = if stack { (cw_nat.min(w), hw_nat.min(w)) } else {
        let k = ((w - 8.0 * PX) / (cw_nat + hw_nat)).min(1.0);
        (cw_nat * k, hw_nat * k)
    };
    let (cy, hy) = if stack { (y, y + ph + ROW_GAP * PY) } else { (y, y) };
    let fits = |nat: f32, real: f32, words: f32| (words - (nat - real)).max(10.0);
    // the costume pill
    let cb = [x, cy, cw, ph];
    rounded(d, cb, PILL / 2.0, s.well, 128.0);
    let mut cx = x + pad;
    shirt(d, [cx, cy + 5.0 * PY, icon, 16.0 * PY], s.ring, s.well);
    cx += icon + gap;
    if keys && !l1.is_empty() {
        cx += chip(d, &l1, [cx, cy + (ph - gh) / 2.0], gh, s.ring) + gap;
    }
    let words = fits(cw_nat, cw, d.width(&costume, ts));
    d.label(&costume, cx + words / 2.0, cy + (ph - ts) / 2.0, ts, words, if m.blocked(p) && !pl.random { [70.0; 3] } else { WHITE });
    cx += words + gap;
    if keys && !r1.is_empty() {
        chip(d, &r1, [cx, cy + (ph - gh) / 2.0], gh, s.ring);
    }
    // the hand pill, at the right end
    let hx = x + w - hw;
    let hb = [hx, hy, hw, ph];
    rounded(d, hb, PILL / 2.0, s.well, 128.0);
    let mut cx = hx + pad;
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [cx - 4.0 * PX, hy + 6.0 * PY, icon + 6.0 * PX, 14.0 * PY], WHITE, 128.0);
    cx += icon + gap;
    let words = fits(hw_nat, hw, d.width(hand, ts));
    d.label(hand, cx + words / 2.0, hy + (ph - ts) / 2.0, ts, words, WHITE);
    cx += words + gap;
    if keys && !sel.is_empty() {
        chip(d, &sel, [cx, hy + (ph - gh) / 2.0], gh, s.ring);
    }
    if mine && !pl.ready && !pl.random {
        d.hit([cb[0], cb[1], cb[2] / 2.0, ph], pointer::press(|k| k.prev = true));
        d.hit([cb[0] + cb[2] / 2.0, cb[1], cb[2] / 2.0, ph], pointer::press(|k| k.next = true));
        d.hit(hb, pointer::press(|k| k.hand = true));
    }
}

/// The pink stats card in `[x, y, w, h]`: Serve, Stroke, Volley, Impact, Footwork, each grade's letter (A at 5, F at 0)
/// as the original draws it, "?" for a hidden pick. Browse mods' detail panel shows the same card.
pub(super) fn stats(d: &mut Draw, grades: Option<[u8; 5]>, [x, y, w, h]: [f32; 4]) {
    rounded(d, [x, y, w, h], 10.0, hex(0xf58ac0), 128.0);
    let rh = (h - 8.0 * PY) / 5.0;
    let size = (rh * 0.62).min(14.0 * PY);
    let disc = (rh - 4.0 * PY).min(20.0 * PY);
    for (k, label) in ["Serve", "Stroke", "Volley", "Impact", "Footwork"].into_iter().enumerate() {
        let ry = y + 4.0 * PY + rh * k as f32;
        if k > 0 {
            d.q(Tex::Solid, [0.0; 4], [x + 6.0 * PX, ry - 1.0 * PY, w - 12.0 * PX, 2.0 * PY], hex(0xffc2df), 128.0);
        }
        let room = w - 20.0 * PX - disc * PX / PY;
        let k_fit = (room / d.width(label, size).max(1.0)).min(1.0);
        ringed_text(d, label, x + 8.0 * PX, ry + (rh - size * k_fit) / 2.0, size * k_fit, WHITE);
        // B384: the original's own letter (the attribute sheet's A-F: a gradient fill in the grade's colour with a dark
        // rim, straight on the card, no badge); its 24-px cell the row's height up to 30 px (the letter fills 17 of it)
        let dw = disc * PX / PY;
        let (cx, cy) = (x + w - 8.0 * PX - dw, ry + (rh - disc) / 2.0);
        match grades {
            Some(g) => {
                let (i, c) = ((5 - g[k].min(5)) as f32, rh.min(30.0 * PY));
                let cell = [cx + (dw - c * PX / PY) / 2.0, cy + (disc - c) / 2.0, c * PX / PY, c];
                d.q(super::ATTRS, [232.0, 24.0 * i, 24.0, 24.0], cell, WHITE, 128.0);
            }
            None => {
                let ls = disc * 0.72;
                let lw = d.width("?", ls);
                ringed_text(d, "?", cx + (dw - lw) / 2.0, cy + (disc - ls) / 2.0, ls, hex(0xc8ccd0));
            }
        }
    }
}
