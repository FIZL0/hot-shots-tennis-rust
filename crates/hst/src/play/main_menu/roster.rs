//! B368: the character select (`Screen::Chars`: the local roster and, until B370's lobby, an online
//! room's) as one column per player (`plan/B368.md`, the approved 1280×720 mockup). Each column is its player's own:
//! the header (seat, character, device), the costume and hand pills, the 3D preview beside the play style and stats
//! card, and a grid of face tiles that scrolls inside that column only (B382: one grid, a heading per section,
//! `tabs.rs`). Sizes are the mockup's pixels at 720 lines ([`PY`] units down, [`PX`] across, so a tile is square on screen);
//! a narrower column fits fewer tiles a row (13 / 6 / 4 / 3 for 1–4 columns at 16:9), a taller view more rows.

use bevy::prelude::*;

use super::super::controls::Bindings;
use super::tabs::{self, Line, Section};
use super::{C02, CHAR_NAMES, Dev, Draw, GOLD, Menu, RANDOM_TILE, STYLE_NAMES, Tex, Text, WHITE, Who, dev_name, drop_button, grades_of, home, legend, look, pointer, shirt, tile_face};

/// One mockup pixel in the menu's units: down, and across (a unit across is 14/15 of one down on screen: 640 across
/// a 4:3 screen 448 high).
pub(super) const PY: f32 = 448.0 / 720.0;
pub(super) const PX: f32 = PY * 15.0 / 14.0;

/// The mockup's sizes (px at 720 lines): the top bar over the columns, the side margins and the columns' gap, a
/// column's padding and the gap between its rows, the rows' heights (B422: the seat's header band across the card's
/// top, the section rail over the grid), the stats stack's width, a tile and its gap, the scroll bar, a section's
/// heading (B382).
const TOP_BAR: f32 = 60.0;
pub(super) const MARGIN: f32 = 24.0;
const GAP: f32 = 12.0;
const PAD: f32 = 10.0;
const ROW_GAP: f32 = 8.0;
const BAND: f32 = 50.0;
const RAIL: f32 = 24.0;
const PILL: f32 = 26.0;
const PREVIEW: f32 = 150.0;
const FOOT: f32 = 14.0;
const STATS_W: f32 = 150.0;
const TILE: f32 = 84.0;
const TILE_GAP: f32 = 6.0;
const BAR: f32 = 6.0;
const HEADING: f32 = 22.0;
/// B426: the least of a row that shows under the grid's last whole line.
const PEEK: f32 = 12.0;
/// The pills go on two rows in a column narrower than this inside (px).
const PILLS_W: f32 = 290.0;

/// `#rrggbb` in the menus' 0–128 colour.
pub(super) fn hex(c: u32) -> [f32; 3] {
    [(c >> 16) & 255, (c >> 8) & 255, c & 255].map(|v| v as f32 * 128.0 / 255.0)
}

/// The screen's background.
pub(super) const BG: u32 = 0x0e1a2e;

/// A seat's colours (B422's "Players" board): its own colour (the header band, the cursor's tile) and the words on it,
/// the same character's other tile, the column's body, a light well (counts, the scroll bar's track), a tile, soft
/// words. The four differ in lightness too, not just hue (yellow, green, red, blue from light to dark), so they tell
/// apart side by side for anyone. B471: on the main menu's off-white panel (`look`), the seat's colour its accent.
pub(super) struct Seat {
    pub ring: [f32; 3],
    pub ink: [f32; 3],
    pub sel: [f32; 3],
    pub bg: [f32; 3],
    pub well: [f32; 3],
    pub tile: [f32; 3],
    pub soft: [f32; 3],
}

pub(super) fn seat(p: usize) -> Seat {
    const C: [[u32; 7]; 4] = [
        [0xe0404c, 0xffffff, 0xf4b9be, 0xfbf9fd, 0xe2d9ea, 0xefe8f5, 0x6b6478],
        [0xffd84a, 0x2a2008, 0xf7dd8a, 0xfbf9fd, 0xe2d9ea, 0xefe8f5, 0x6b6478],
        [0x3a6fd8, 0xffffff, 0xb9cbf3, 0xfbf9fd, 0xe2d9ea, 0xefe8f5, 0x6b6478],
        [0x3aa45a, 0xffffff, 0xb6e2c2, 0xfbf9fd, 0xe2d9ea, 0xefe8f5, 0x6b6478],
    ];
    let [ring, ink, sel, bg, well, tile, soft] = C[p % 4].map(hex);
    Seat { ring, ink, sel, bg, well, tile, soft }
}

/// The select's player columns on a view (`Grid::view`): how many tiles a row and rows on view each column's grid
/// has, and where each part of a column goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Columns {
    /// Player columns.
    pub n: usize,
    /// Tiles a row, and the rows on view (under one heading: B382).
    pub cols: usize,
    pub rows: usize,
    /// A tile's width and height (square on screen), and the gaps between them across and down.
    pub tile: [f32; 2],
    gap: [f32; 2],
    /// A full column's box (the first's place), and each column's left and width (M120: online, another machine's
    /// seats and open ones are compact cards, narrower than this machine's players' columns).
    first: [f32; 4],
    xs: [[f32; 2]; 4],
    /// The pills on two rows (a narrow column).
    pub stack: bool,
    /// The preview row's height.
    preview: f32,
}

/// The parts of a player's column.
pub(super) struct Parts {
    pub card: [f32; 4],
    /// B422: the seat's header band, across the card's top.
    pub head: [f32; 4],
    /// The pills' row (both rows when stacked).
    pub pills: [f32; 4],
    pub preview: [f32; 4],
    /// B422: the section rail.
    pub rail: [f32; 4],
    /// The tiles on view, and the scroll bar right of them.
    pub grid: [f32; 4],
    pub bar: [f32; 4],
    pub foot: [f32; 4],
}

/// The select's `n` player columns on `view` (left, top, right, bottom).
#[cfg(test)]
pub(super) fn columns(view: [f32; 4], n: usize) -> Columns {
    columns_mixed(view, n, [false; 4])
}

/// M120: a compact card's width (mockup px), at most an even share of the row.
const COMPACT_W: f32 = 300.0;

/// The select's `n` player columns on `view`, the `compact` ones (online: another machine's players, the open seats)
/// as narrow cards, the rest sharing what's left.
pub(super) fn columns_mixed(view: [f32; 4], n: usize, compact: [bool; 4]) -> Columns {
    let n = n.max(1);
    let [l, t, r, b] = view;
    let top = t + TOP_BAR * PY;
    let room = r - l - 2.0 * MARGIN * PX - GAP * PX * (n - 1) as f32;
    let k = (0..n.min(4)).filter(|&p| compact[p]).count();
    let even = room / n as f32;
    let cw = if k < n { (COMPACT_W * PX).min(even) } else { even };
    let w = if k < n { (room - k as f32 * cw) / (n - k) as f32 } else { even };
    let mut xs = [[0.0; 2]; 4];
    let mut x = l + MARGIN * PX;
    for (p, c) in xs.iter_mut().enumerate().take(n.min(4)) {
        let pw = if compact[p] { cw } else { w };
        *c = [x, pw];
        x += pw + GAP * PX;
    }
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
    let fixed = (BAND + 2.0 * PAD + pills + RAIL + FOOT + 4.0 * ROW_GAP) * PY;
    // the preview gives way to keep a row of tiles on a short view; B382: the grid has room for a heading over its
    // rows
    let room = h - fixed - (HEADING * PY + gy);
    let preview = (PREVIEW * PY).min(room - th).max(60.0 * PY);
    let rows = (((room - preview + gy) / (th + gy)) as usize).max(1);
    // what's left under the last row whole goes to the preview, up to twice its size
    let spare = room - preview - (rows as f32 * (th + gy) - gy);
    let preview = preview + spare.clamp(0.0, PREVIEW * PY);
    Columns { n, cols, rows, tile: [tw, th], gap: [gx, gy], first: [l + MARGIN * PX, top, w, h], xs, stack, preview }
}

impl Columns {
    /// Player `p`'s column.
    pub fn parts(&self, p: usize) -> Parts {
        let [_, top, _, h] = self.first;
        let [x, w] = self.xs[p.min(3)];
        let (ix, iw) = (x + PAD * PX, w - 2.0 * PAD * PX);
        let head = [x, top, w, BAND * PY];
        let mut y = top + (BAND + PAD) * PY;
        let mut next = |hh: f32| {
            let r = [ix, y, iw, hh];
            y += hh + ROW_GAP * PY;
            r
        };
        let pills = next(if self.stack { 2.0 * PILL + ROW_GAP } else { PILL } * PY);
        let preview = next(self.preview);
        let rail = next(RAIL * PY);
        let gh = self.grid_h();
        let grid = next(gh);
        let foot = [ix, top + h - (PAD + FOOT) * PY, iw, FOOT * PY];
        let bar = [ix + iw - BAR * PX, grid[1], BAR * PX, gh];
        Parts { card: [x, top, w, h], head, pills, preview, rail, grid: [ix, grid[1], iw - (BAR + TILE_GAP) * PX, gh], bar, foot }
    }

    /// The grid's height: `rows` rows of tiles and a heading.
    fn grid_h(&self) -> f32 {
        self.rows as f32 * (self.tile[1] + self.gap[1]) + HEADING * PY
    }

    /// B382: a line's height (without the gap under it).
    fn line_h(&self, l: Line) -> f32 {
        match l {
            Line::Head(_) => HEADING * PY,
            Line::Tiles(..) => self.tile[1],
        }
    }

    /// B382: how many of `lines` from `top` the grid shows whole (at least one).
    pub fn fit(&self, lines: &[Line], top: usize) -> usize {
        let room = self.grid_h() + self.gap[1] + 0.01;
        let mut used = 0.0;
        let n = lines.iter().skip(top).take_while(|&&l| {
            used += self.line_h(l) + self.gap[1];
            used <= room
        });
        n.count().max(1)
    }

    /// B382: the furthest the grid scrolls: the first line that shows the last.
    pub fn last_top(&self, lines: &[Line]) -> usize {
        (0..lines.len()).find(|&t| t + self.fit(lines, t) >= lines.len()).unwrap_or(0)
    }

    /// B382: player `p`'s column's lines on view in `v`: each heading's section and box, each tile's place
    /// (`tabs::flat`), cell and box.
    pub fn laid(&self, p: usize, v: &View) -> (Vec<(usize, [f32; 4])>, Vec<(usize, Option<Who>, [f32; 4])>) {
        let [x, mut y, w, h] = self.parts(p).grid;
        let [tw, th] = self.tile;
        let (mut heads, mut tiles) = (Vec::new(), Vec::new());
        for &l in &v.lines[v.top..v.end] {
            match l {
                Line::Head(s) => heads.push((s, [x, y, w, HEADING * PY])),
                Line::Tiles(s, i) => {
                    for (j, &c) in v.secs[s].cells[i..].iter().take(self.cols).enumerate() {
                        tiles.push((tabs::flat(&v.secs, s, i + j), c, [x + (tw + self.gap[0]) * j as f32, y, tw, th]));
                    }
                }
            }
            y += self.line_h(l) + self.gap[1];
        }
        // B426: the next row's top in what's left under the last whole line (so a heading at the foot isn't all
        // there is of its section), its boxes cut at the grid's foot
        let left = h + self.parts(p).grid[1] - y;
        if let Some(&Line::Tiles(s, i)) = v.lines.get(v.end)
            && left >= PEEK * PY
        {
            for (j, &c) in v.secs[s].cells[i..].iter().take(self.cols).enumerate() {
                tiles.push((tabs::flat(&v.secs, s, i + j), c, [x + (tw + self.gap[0]) * j as f32, y, tw, th.min(left)]));
            }
        }
        (heads, tiles)
    }
}

/// B382: a column's grid: the sections, their lines, and the lines on view (`top..end`).
#[derive(Debug)]
pub(super) struct View {
    pub secs: Vec<Section>,
    pub lines: Vec<Line>,
    pub top: usize,
    pub end: usize,
}

impl Menu {
    /// B382: player `p`'s column's grid as scrolled (`scroll`'s line, kept on the grid).
    pub(super) fn view(&self, p: usize) -> View {
        let c = self.columns();
        let secs = self.sections();
        let lines = tabs::lines(&secs, c.cols);
        let top = self.scroll[p].min(c.last_top(&lines));
        let end = (top + c.fit(&lines, top)).min(lines.len());
        View { secs, lines, top, end }
    }

    /// B368: the tiles on view in every column and the players' picks (the face icons to read).
    pub(super) fn on_view(&self) -> Vec<Who> {
        let c = self.columns();
        let mut v: Vec<Who> = (0..self.players()).flat_map(|p| c.laid(p, &self.view(p)).1).filter_map(|t| t.1).collect();
        v.extend((0..self.players()).map(|p| self.who(p)));
        v
    }

    /// The box of the tile at place `k` in player `p`'s column, if it's on view.
    #[cfg(test)]
    pub(super) fn tile_rect(&self, p: usize, k: usize) -> Option<[f32; 4]> {
        self.columns().laid(p, &self.view(p)).1.into_iter().find(|t| t.0 == k).map(|t| t.2)
    }
}

/// B426: the quads from `from` on cut off at `foot` (down the menu's units), their pictures with them; the corners
/// on the cut square.
pub(super) fn cut(d: &mut Draw, from: usize, foot: f32) {
    let mut rest = d.out.split_off(from);
    rest.retain_mut(|q| {
        let [_, y, _, h] = q.dst;
        if y + h <= foot {
            return true;
        }
        let keep = foot - y;
        if keep <= 0.0 {
            return false;
        }
        q.src[3] *= keep / h;
        q.dst[3] = keep;
        q.round[2] = 0.0;
        q.round[3] = 0.0;
        true
    });
    d.out.append(&mut rest);
}

/// A box `[x, y, w, h]` filled `rgb`, its corners rounded `r` mockup px.
pub(super) fn rounded(d: &mut Draw, rect: [f32; 4], r: f32, rgb: [f32; 3], alpha: f32) {
    d.q(Tex::Solid, [0.0; 4], rect, rgb, alpha);
    d.out.last_mut().unwrap().round = [r * PX; 4];
}

/// A box with a `t`-px rim in `rim` round a `fill`, corners `r` px.
pub(super) fn ringed(d: &mut Draw, [x, y, w, h]: [f32; 4], r: f32, t: f32, rim: [f32; 3], fill: [f32; 3]) {
    rounded(d, [x, y, w, h], r, rim, 128.0);
    rounded(d, [x + t * PX, y + t * PY, w - 2.0 * t * PX, h - 2.0 * t * PY], (r - t).max(0.0), fill, 128.0);
}

/// A dashed `t`-px rim round a box with `r`-px corners on a `fill` box: a computer player's column. The corners are
/// whole arcs; each side's dashes are spread to end flush with them.
fn dashed(d: &mut Draw, [x, y, w, h]: [f32; 4], r: f32, t: f32, rgb: [f32; 3], fill: [f32; 3]) {
    let (dash, gap) = (10.0, 6.0);
    let (tx, ty) = (t * PX, t * PY);
    // the dashes along a side from `r` to `len - r` (`k` units to the axis'), first and last touching the corners
    let run = |d: &mut Draw, len: f32, k: f32, at: &dyn Fn(f32, f32) -> [[f32; 4]; 2]| {
        let span = len - 2.0 * r * k;
        let n = ((span + gap * k) / ((dash + gap) * k)).floor().max(1.0);
        let l = (dash * k).min(span);
        let step = if n > 1.0 { (span - l) / (n - 1.0) } else { 0.0 };
        let first = if n > 1.0 { r * k } else { r * k + (span - l) / 2.0 };
        for i in 0..n as usize {
            for rect in at(first + step * i as f32, l) {
                d.q(Tex::Solid, [0.0; 4], rect, rgb, 128.0);
            }
        }
    };
    run(d, w, PX, &|s, l| [[x + s, y, l, ty], [x + s, y + h - ty, l, ty]]);
    run(d, h, PY, &|s, l| [[x, y + s, tx, l], [x + w - tx, y + s, tx, l]]);
    // the corners: an arc of rim with the box's fill inside it
    let (cw, ch) = (r * PX, r * PY);
    for (c, [cx, cy]) in [[x, y], [x + w - cw, y], [x + w - cw, y + h - ch], [x, y + h - ch]].into_iter().enumerate() {
        let mut round = [0.0; 4];
        round[c] = cw;
        d.q(Tex::Solid, [0.0; 4], [cx, cy, cw, ch], rgb, 128.0);
        d.out.last_mut().unwrap().round = round;
        // inside: inset `t` from the two outer sides
        let ix = if c == 0 || c == 3 { cx + tx } else { cx };
        let iy = if c < 2 { cy + ty } else { cy };
        round[c] = (cw - tx).max(0.0);
        d.q(Tex::Solid, [0.0; 4], [ix, iy, cw - tx, ch - ty], fill, 128.0);
        d.out.last_mut().unwrap().round = round;
    }
}

/// `s` cut to fit `max` wide at `size`, "..." where it was cut.
pub(super) fn ellipsis(d: &Draw, s: &str, size: f32, max: f32) -> String {
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

/// The legend's □ / Tab art (`Tex::Glyph`'s), the mode pill's button.
const MODE_GLYPH: u8 = 4;

/// A button's name as a small chip in `rgb` with dark words (the legend's L1 / R1, Q / E, Select…), `h` high from
/// `[x, y]`; its width.
pub(super) fn chip(d: &mut Draw, s: &str, [x, y]: [f32; 2], h: f32, rgb: [f32; 3]) -> f32 {
    let size = h * 0.72;
    let w = d.width(s, size) + 8.0 * PX;
    rounded(d, [x, y, w, h], 5.0, rgb, 128.0);
    d.centred(s, x + w / 2.0, y + (h - size) / 2.0, size, hex(BG));
    w
}

/// The width of a [`chip`] for `s`.
pub(super) fn chip_w(d: &Draw, s: &str, h: f32) -> f32 {
    d.width(s, h * 0.72) + 8.0 * PX
}

/// The character select, one column per player.
pub(super) fn chars(d: &mut Draw, m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity]) {
    let [l, t, r, _] = m.grid.view;
    // the background over the whole window
    super::look::backdrop(d);
    // the top bar: the title (Back for the pointer, after the "<" `drawn` puts over it), the players and the mode
    let size = 30.0 * PY;
    // M121: online, the select is the lobby
    let title = if m.online() { "Online lobby" } else { "Choose your player" };
    d.outlined(|d| d.text(title, l + 32.0, t + 16.0 * PY, size, WHITE));
    d.hit([l, t, d.width(title, size) + 40.0, TOP_BAR * PY], pointer::press(|k| k.back = true));
    if m.online() {
        return online_chars(d, m, text, bind, pads);
    }
    let (mode, fill) = if m.doubles { ("Doubles", 0x2f7d4f) } else { ("Singles", 0x2f5f9d) };
    let ms = 16.0 * PY;
    let swaps = m.local_roster();
    // B427: the button that swaps it (□ / X / Tab) in 1P's device's art, as the costume and hand pills show theirs
    let glyph = match m.seats[0] {
        Some(Dev::Pad(e)) if swaps => Some(legend::dev_style(Some(e))),
        Some(Dev::Keys) if swaps => Some(legend::dev_style(None)),
        _ => None,
    };
    let gh = 20.0 * PY;
    let gw = glyph.map_or(0.0, |s| gh * legend::glyph_aspect(s, MODE_GLYPH) + 6.0 * PX);
    let mw = d.width(mode, ms) + 28.0 * PX + gw;
    let mx = r - MARGIN * PX - mw;
    let my = t + 16.0 * PY;
    rounded(d, [mx, my, mw, 30.0 * PY], 15.0, WHITE, 128.0);
    rounded(d, [mx + 2.0 * PX, my + 2.0 * PY, mw - 4.0 * PX, 26.0 * PY], 13.0, hex(fill), 128.0);
    if let Some(style) = glyph {
        d.q(Tex::Glyph(style, MODE_GLYPH), [0.0; 4], [mx + 8.0 * PX, my + 15.0 * PY - gh / 2.0, gw - 6.0 * PX, gh], WHITE, 128.0);
    }
    d.centred(mode, mx + gw + (mw - gw) / 2.0, my + 7.0 * PY, ms, WHITE);
    if swaps {
        // B375: a tap swaps Singles ↔ Doubles, as 1P's □ / Tab (step checks the device is 1P's; online: `lobby::over`)
        d.hit([mx, my, mw, 30.0 * PY], pointer::press(|k| k.page = true));
    }
    let humans = m.humans() + (0..m.players()).filter(|&p| m.seats[p].is_none() && m.remote(p).is_some_and(|s| s.human())).count();
    let who = if humans > 1 { format!("{humans} players, each browses on their own") } else { "1 player".into() };
    let right = mx - 12.0 * PX;
    let ws = 14.0 * PY;
    let ww = d.width(&who, ws).min(right - (l + 40.0 + d.width(title, size)));
    if ww > 20.0 {
        d.outlined(|d| d.label(&who, right - ww / 2.0, my + 8.0 * PY, ws, ww, WHITE));
    }
    let c = player_columns(d, m, text, bind, pads);
    names(d, m, &c);
}

/// M121: the online select, which is the lobby: the relay's notice band, the columns, the lobby around them (its top
/// bar's buttons, the strip, Leave, the chat box and Start: `lobby::over`), the name pickers over it all.
fn online_chars(d: &mut Draw, m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity]) {
    let [l, t, r, _] = m.grid.view;
    super::online::band(d, m, l + MARGIN * PX, t + TOP_BAR * PY, r - l - 2.0 * MARGIN * PX);
    let c = player_columns(d, m, text, bind, pads);
    super::online::lobby_over(d, m);
    names(d, m, &c);
}

/// The player columns.
fn player_columns(d: &mut Draw, m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity]) -> Columns {
    let c = m.columns();
    let pointer_p = m.driven(pointer::device(m, pads));
    for p in 0..c.n {
        column(d, m, text, bind, pads, &c, p, pointer_p == Some(p));
    }
    c
}

/// B371: the name pickers over their players' columns (at least 180 wide).
fn names(d: &mut Draw, m: &Menu, c: &Columns) {
    let [l, _, r, _] = m.grid.view;
    for p in 0..c.n {
        let [x, y, w, h] = c.parts(p).card;
        let nw = w.max(180.0).min(r - l);
        super::names::draw(d, m, p, [(x + (w - nw) / 2.0).clamp(l, r - nw), y, nw, h]);
    }
}

/// Whether player `p` of `n` is picking or has picked: who's picking (the old select's rules) is a seated human, else
/// a computer player once the humans before it are done; online, a seat with a player on it or the one being edited.
fn is_active(m: &Menu, n: usize, p: usize) -> bool {
    let picking = |q: usize| m.seats[q].is_some() || m.bot(q) && (0..n).filter(|&o| m.bot(o)).all(|o| o >= q || m.players[o].ready) && (0..n).all(|o| m.seats[o].is_none() || m.players[o].ready);
    let picking = |q: usize| if m.select_online() { m.seats[q].is_some() || m.remote(q).is_some() || m.net.edit == Some(q) } else { picking(q) };
    picking(p) || m.players[p].ready
}

/// B434: an open seat's card `[x, y, w, h]`: empty, dashed and dimmer than a computer player's, seat `p`'s number in
/// the middle over "Press ✕ to join" (`join`: a device here can take it; the keyboard's Enter works but isn't shown,
/// the keyboard being 1P's if anyone's), else "Waiting for a player".
fn open_seat(d: &mut Draw, s: &Seat, card: [f32; 4], p: usize, join: bool) {
    open_seat_saying(d, s, card, p, join, "Waiting for a player");
}

/// [`open_seat`] with its own words for "Waiting for a player" (wrapped to the card).
fn open_seat_saying(d: &mut Draw, s: &Seat, card: [f32; 4], p: usize, join: bool, wait: &str) {
    let [x, y, w, h] = card;
    rounded(d, card, 14.0, s.tile, 100.0);
    dashed(d, card, 14.0, 2.0, s.soft, s.tile);
    let big = (64.0 * PY).min(w * 0.4);
    let cy = y + h / 2.0;
    let start = d.out.len();
    d.centred(&format!("{}P", p + 1), x + w / 2.0, cy - big - 8.0 * PY, big, s.ring);
    d.out[start..].iter_mut().for_each(|q| q.alpha = 45.0);
    let start = d.out.len();
    if join {
        home::press_hint(d, " to join", x + w / 2.0, cy + 8.0 * PY, 17.0 * PY, w - 24.0 * PX, hex(look::INK));
    } else {
        let size = 17.0 * PY;
        let cols = ((w - 24.0 * PX) / (size * 0.55)).max(6.0) as usize;
        for (i, l) in super::wrap(wait, cols).iter().enumerate() {
            d.label(l, x + w / 2.0, cy + 8.0 * PY + size * 1.3 * i as f32, size, w - 24.0 * PX, hex(look::INK));
        }
    }
    d.out[start..].iter_mut().for_each(|q| q.alpha = 110.0);
}

/// Player `p`'s column; `mine`: the pointer's device picks for it (its pills take taps).
#[allow(clippy::too_many_arguments)]
fn column(d: &mut Draw, m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity], c: &Columns, p: usize, mine: bool) {
    let n = c.n;
    let online = m.select_online();
    let s = seat(p);
    let pl = m.players[p];
    let at = c.parts(p);
    // M120: another machine's seat (or an open one) online: a compact card, no roster
    if m.compact(p) {
        return compact_card(d, m, text, &s, at.card, p);
    }
    let active = is_active(m, n, p);
    let human = matches!(m.seats[p], Some(Dev::Keys | Dev::Pad(_))) || m.remote(p).is_some_and(|s| s.human());
    // an unjoined seat: an empty card, dashed and dim, its number and the join prompt in the middle; B434: a seat held
    // open for a device and an online seat waiting for a player too: no figure in either, it reads as an open seat
    if m.seats[p] == Some(Dev::Open) || !active && (online || m.local_roster()) {
        open_seat(d, &s, at.card, p, !online || m.seats[p] == Some(Dev::Open));
        return;
    }
    // the card: the main menu's panel (B471: off-white in a dark outline), the outline dashed for a computer player
    let [x, y, w, _] = at.card;
    if human || m.seats[p] == Some(Dev::Open) {
        ringed(d, at.card, 14.0, look::RIM, hex(look::INK), s.bg);
    } else {
        rounded(d, at.card, 14.0, s.bg, 128.0);
        dashed(d, at.card, 14.0, look::RIM, hex(look::INK), s.bg);
    }
    if mine {
        d.hit(at.card, pointer::Act::Grid(p));
    }

    // B422: the header band: the seat's colour across the card's top (a computer player's dark, its words in the
    // seat's colour), a big 1P–4P, the character's name over the player's own name (B371) and the device, the side pill
    let solid = human || m.seats[p] == Some(Dev::Open);
    let device = match m.remote(p).filter(|s| s.human() && online) {
        Some(st) => m.net.room.as_ref().and_then(|r| r.names.get(&st.peer)).map_or("Online".to_string(), |n| n.clone()),
        None if m.seats[p] == Some(Dev::Open) => "Open".into(),
        None => dev_name(m.seats[p], pads),
    };
    let device = match m.player_name(p) {
        Some(own) => format!("{own} - {device}"),
        None => device,
    };
    let name = match m.who(p) {
        _ if pl.random => "Random".to_string(),
        _ if !active => "-".to_string(),
        Who::Mod(i) => m.mods[i].name.clone(),
        // B234: another machine's mod this one doesn't have: its own name over the donor's looks
        Who::Disc(ch) => m.remote_mod_name(p).unwrap_or(CHAR_NAMES[ch]).to_string(),
    };
    // the side pill: "Swap teams" with the seat's own □ / X / Tab while its device can switch (doubles), else its side
    let swap = match m.seats[p] {
        // B522: online, a guest's too
        Some(dev @ Dev::Pad(e)) if m.team_switch(dev).is_some() || m.online_team(dev).is_some() => Some(legend::dev_style(Some(e))),
        Some(Dev::Keys) if m.team_switch(Dev::Keys).is_some() || m.online_team(Dev::Keys).is_some() => Some(legend::dev_style(None)),
        _ => None,
    };
    let side = if swap.is_some() { "Swap teams" } else if p < (n / 2).max(1) { "Near" } else { "Far" };
    band(d, &s, at.head, solid, p, &name, &device, side, swap, m.droppable(p));
    if mine && m.seats[p].is_some() {
        // B371: a tap on the seat's number or names opens the name picker
        let [hx, hy, hw, hh] = at.head;
        d.hit([hx, hy, hw * 0.6, hh], pointer::press(|k| k.name = true));
    }
    // B238: a joined device's ✕ on its column's corner
    if m.droppable(p) {
        drop_button(d, p, x + w - 30.0, y - 6.0);
    }

    // the pills: the costume (L1 / R1) on the left, the hand (Select) on the right
    pills(d, m, text, bind, &s, p, at.pills, c.stack, mine);

    // the preview, and the play style over the stats card on its right
    let [px, py, pw, ph] = at.preview;
    let sw = (STATS_W * PX).min(pw * 0.56);
    let (sx, area) = (px + pw - sw, pw - sw - 8.0 * PX);
    rounded(d, [px, py, area, ph], 10.0, s.tile, 128.0);
    // B422: the seat's number, big and faint, behind the figure
    let num = format!("{}P", p + 1);
    let big = (ph * 0.78).min(area * 0.9 * 32.0 / d.width(&num, 32.0));
    let start = d.out.len();
    d.centred(&num, px + area / 2.0, py + (ph - big) / 2.0, big, s.ring);
    d.out[start..].iter_mut().for_each(|q| q.alpha = 26.0);
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
    if online && active && m.seats[p].is_some() {
        // M95 (M120): online, this machine's platform and input in the well's corner, as the other players' cards
        device_pill(d, *crate::device::CURRENT.lock().unwrap_or_else(|e| e.into_inner()), px + area, py, &s);
    }
    let style_of = |w: Who| match w {
        Who::Mod(i) => text.mods[i].0,
        Who::Disc(ch) => text.style[ch],
    };
    let hidden = pl.random || !active;
    let sh = 24.0 * PY;
    rounded(d, [sx, py, sw, sh], 8.0, hex(look::INK), 128.0);
    let style = if hidden { "?" } else { STYLE_NAMES[style_of(m.who(p)).min(3)] };
    d.label(style, sx + sw / 2.0, py + 5.0 * PY, 14.0 * PY, sw - 10.0 * PX, hex(0xffd23f));
    stats(d, if hidden { None } else { Some(grades_of(text, m.who(p))) }, [sx, py + sh + 4.0 * PY, sw, ph - sh - 4.0 * PY]);
    if pl.ready {
        // "Ready!" across the preview's foot
        let rh = 24.0 * PY;
        rounded(d, [px + 6.0 * PX, py + ph - rh - 6.0 * PY, area - 12.0 * PX, rh], 12.0, hex(0xffd23f), 128.0);
        d.centred("Ready!", px + area / 2.0, py + ph - rh - 6.0 * PY + 4.0 * PY, 16.0 * PY, hex(BG));
    }

    // B481: a computer player a device here can take over: the join bar across its preview and stats
    if let Some(style) = join_glyph(m, p) {
        join_bar(d, style, [px, py, pw, ph]);
    }

    // B382: the grid: the lines on view, a heading per section, the tiles this player's darker with a rim in the
    // seat's colour (the same character in its other section filled, without the rim)
    let v = m.view(p);
    let (mine_cell, mine_at) = (m.cell(p), m.place(p, &v.secs));
    let active_at: Vec<Option<usize>> = (0..n).map(|q| if is_active(m, n, q) { m.place(q, &v.secs) } else { None }).collect();
    let (heads, tiles) = c.laid(p, &v);
    // B425: with no mods at all, 1P's rail ends in a ghost chip that opens the mod store
    let mut rail_at = at.rail;
    if m.store_chip(p) {
        let [rx, ry, rw, rh] = at.rail;
        let chips = rail_widths(d, &v.secs);
        let total = chips.iter().sum::<f32>() + RAIL_GAP * PX * chips.len().saturating_sub(1) as f32;
        let labels: Vec<f32> = GET_MORE.iter().map(|l| more_w(d, l)).collect();
        let k = more_label(total, rw, RAIL_GAP * PX, &labels);
        let gw = labels[k];
        rail_at[2] = (rw - gw - RAIL_GAP * PX).max(0.0);
        let gx = rx + total.min(rail_at[2]) + RAIL_GAP * PX;
        get_more(d, GET_MORE[k], [gx, ry, gw, rh], &s);
        d.hit([gx, ry, gw, rh], pointer::Act::Store(p));
    }
    rail(d, &v.secs, m.section_at(p, &v.secs), active, &s, rail_at, p);
    for (k, rect) in heads {
        heading(d, &v.secs[k], rect, &s);
        d.hit(rect, pointer::Act::Section(p, k));
    }
    let foot = at.grid[1] + at.grid[3];
    for (k, cell, hit) in tiles {
        // B426: a row peeking under the last whole line comes cut at the grid's foot: drawn whole, then cut
        let rect = [hit[0], hit[1], c.tile[0], c.tile[1]];
        let n0 = d.out.len();
        let on = active && Some(k) == mine_at;
        if on {
            // B422: the cursor's tile filled in the seat's colour
            rounded(d, rect, 10.0, s.ring, 128.0);
        } else if active && cell == mine_cell {
            rounded(d, rect, 10.0, s.sel, 128.0);
        } else {
            rounded(d, rect, 10.0, s.tile, 128.0);
        }
        let full = active && m.full(p, cell);
        let inset = if on { 5.0 } else { 3.0 };
        let [cx, cy, cw, ch] = rect;
        let (tex, src) = tile_face(cell);
        d.q(tex, src, [cx + inset * PX, cy + inset * PY, cw - 2.0 * inset * PX, ch - 2.0 * inset * PY], if full { [45.0; 3] } else { WHITE }, if active { 128.0 } else { 70.0 });
        d.out.last_mut().unwrap().round = [(10.0 - inset) * PX; 4];
        if let Some(w) = cell && m.favorite(w) {
            tabs::star(d, rect);
        }
        // B422: a 1P–4P tag for each player whose cursor is on this tile (this column's own and the others')
        let on_it: Vec<usize> = (0..n).filter(|&q| active_at[q] == Some(k)).collect();
        tags(d, rect, &on_it);
        cut(d, n0, foot);
        d.hit(hit, pointer::Act::Cell(p, k));
    }
    // B423: lines off view: the grid's cut edge fades into the card, a pill names what's there (a tap pages)
    peek(d, &v, c.cols, &s, at.grid, p);
    // the scroll bar: its thumb the lines on view over all of them
    let total = v.lines.len();
    let [bx, by, bw, bh] = at.bar;
    rounded(d, at.bar, 3.0, s.well, 128.0);
    let thumb = (bh * (v.end - v.top) as f32 / total.max(1) as f32).min(bh).max(12.0 * PY);
    let ty = by + (bh - thumb) * v.top as f32 / c.last_top(&v.lines).max(1) as f32;
    rounded(d, [bx, ty, bw, thumb], 3.0, s.ring, 128.0);

    // the footer: where the tile is in its tab and the whole cast (B424), without the tab's name if that won't fit
    let [fx, fy, fw, fh] = at.foot;
    let fs = 12.0 * PY;
    let [long, short] = tabs::foot(m, p);
    let line = if d.width(&long, fs) <= fw { long } else { short };
    let lw = d.width(&line, fs).min(fw);
    d.label(&line, fx + lw / 2.0, fy + (fh - fs) / 2.0, fs, lw, s.soft);
}

/// M120: seat `p`'s compact card online (another machine's player or the computer's; an open seat is the open card):
/// the header band (the player's name over "Online" and their ping), the figure full width with their platform and
/// input (M95) in its corner, the character's name, hand and costume, then the roster card's own style bar and stats
/// list where a local card has its grid, and along the foot what it is and where they are.
/// M121: the host's ✕ on another machine's seat in the lobby: the figure's top-left corner, clear of the band's
/// Near / Far and the device pill.
pub(super) fn remove_rect([x, y, _, _]: [f32; 4]) -> [f32; 4] {
    [x + (PAD + 6.0) * PX, y + (BAND + PAD + 6.0) * PY, 26.0 * PX, 26.0 * PY]
}

fn compact_card(d: &mut Draw, m: &Menu, text: &Text, s: &Seat, card: [f32; 4], p: usize) {
    let Some(f) = m.card_facts(text, p) else {
        // B519: nobody can take it till the relay's up
        // B526: a browser's LAN lobby says how it's joined
        let line = match super::online::reach(&m.net) {
            _ if m.lan_host() => super::online::HOST_LAN,
            super::online::Reach::Up => "Waiting for a player",
            _ => "Not open yet: friends can join once the relay is up",
        };
        return open_seat_saying(d, s, card, p, false, line);
    };
    let [x, y, w, h] = card;
    let com = f.rank.is_some();
    if com {
        rounded(d, card, 14.0, s.bg, 128.0);
        dashed(d, card, 14.0, look::RIM, hex(look::INK), s.bg);
    } else {
        ringed(d, card, 14.0, look::RIM, hex(look::INK), s.bg);
    }
    let pl = m.players[p];
    let who = m.who(p);
    let name = match who {
        _ if pl.random => "Random".to_string(),
        Who::Mod(i) => m.mods[i].name.clone(),
        Who::Disc(ch) => m.remote_mod_name(p).unwrap_or(CHAR_NAMES[ch]).to_string(),
    };
    let sub = match f.ping {
        Some(ms) => format!("Online - {ms} ms"),
        None if com => "Computer".into(),
        None => "Online".into(),
    };
    let side = if p < (m.players() / 2).max(1) { "Near" } else { "Far" };
    band(d, s, [x, y, w, BAND * PY], !com, p, &f.owner, &sub, side, None, false);
    let (ix, iw) = (x + PAD * PX, w - 2.0 * PAD * PX);
    let mut cy = y + (BAND + PAD) * PY;
    let foot_h = 30.0 * PY;
    let foot_y = y + h - PAD * PY - foot_h;
    let body = foot_y - cy;
    // the figure, full width, the seat's number faint behind it
    let ph = (body * 0.4).max(60.0 * PY);
    rounded(d, [ix, cy, iw, ph], 10.0, s.tile, 128.0);
    let num = format!("{}P", p + 1);
    let big = (ph * 0.78).min(iw * 0.9 * 32.0 / d.width(&num, 32.0));
    let start = d.out.len();
    d.centred(&num, ix + iw / 2.0, cy + (ph - big) / 2.0, big, s.ring);
    d.out[start..].iter_mut().for_each(|q| q.alpha = 26.0);
    if pl.random {
        let fw = (iw - 8.0 * PX).min((ph - 8.0 * PY) * 92.0 / 68.0);
        d.q(C02, RANDOM_TILE, [ix + (iw - fw) / 2.0, cy + (ph - fw * 68.0 / 92.0) / 2.0, fw, fw * 68.0 / 92.0], WHITE, 128.0);
    } else {
        let k = 0.8 * PX / PY;
        let rw = iw.min(ph * k);
        let rh = rw / k;
        d.q(Tex::Preview(p), [0.0; 4], [ix + (iw - rw) / 2.0, cy + (ph - rh) / 2.0, rw, rh], WHITE, 128.0);
    }
    if let Some(dev) = f.device {
        device_pill(d, dev, ix + iw, cy, s);
    }
    cy += ph + ROW_GAP * PY;
    // the character's name, its hand and costume under it
    let ns = 24.0 * PY;
    d.text(&ellipsis(d, &name, ns, iw), ix, cy, ns, hex(look::INK));
    cy += ns + 2.0 * PY;
    let hand = if pl.random { "Hand ?" } else if m.hand(text, p) < 0.0 { "Left" } else { "Right" };
    let costume = if pl.random { "Costume ?".to_string() } else { format!("Costume {}", pl.costume + 1) };
    let hs = 14.0 * PY;
    d.text(&ellipsis(d, &format!("{hand} - {costume}"), hs, iw), ix, cy, hs, s.soft);
    cy += hs + ROW_GAP * PY;
    // a mod this machine lacks: what happens about it
    if let Some(note) = m.remote_mod_note(p) {
        let ns = 12.0 * PY;
        d.label(&ellipsis(d, &note, ns, iw), ix + iw / 2.0, cy, ns, iw, hex(look::INK));
        cy += ns + ROW_GAP * PY;
    }
    // the roster card's style bar and stats list
    let style_of = |w: Who| match w {
        Who::Mod(i) => text.mods[i].0,
        Who::Disc(ch) => text.style[ch],
    };
    let sh = 24.0 * PY;
    rounded(d, [ix, cy, iw, sh], 8.0, hex(look::INK), 128.0);
    let style = if pl.random { "?" } else { STYLE_NAMES[style_of(who).min(3)] };
    d.label(style, ix + iw / 2.0, cy + 5.0 * PY, 14.0 * PY, iw - 10.0 * PX, hex(0xffd23f));
    cy += sh + 4.0 * PY;
    stats(d, if pl.random { None } else { Some(grades_of(text, who)) }, [ix, cy, iw, (foot_y - ROW_GAP * PY - cy).max(0.0)]);
    // the foot: what it is on the left, where they are in a pill on the right
    let what = match (&f.rank, who) {
        (Some(rank), _) => format!("COM - {rank}"),
        (None, Who::Mod(_)) => "Mod".to_string(),
        (None, Who::Disc(_)) if m.remote_mod_name(p).is_some() => "Mod".to_string(),
        (None, Who::Disc(_)) => "Disc character".to_string(),
    };
    let (fill, ink) = match f.state {
        "Ready" => (hex(0x3f9d2e), WHITE),
        "Picking..." => (hex(look::INK), WHITE),
        _ => (s.well, hex(look::INK)),
    };
    let ps = 14.0 * PY;
    let pw = (d.width(f.state, ps) + 20.0 * PX).min(iw * 0.55);
    rounded(d, [ix + iw - pw, foot_y, pw, foot_h], 15.0, fill, 128.0);
    d.label(f.state, ix + iw - pw / 2.0, foot_y + (foot_h - ps) / 2.0, ps, pw - 12.0 * PX, ink);
    let ws = 13.0 * PY;
    let what = ellipsis(d, &what, ws, iw - pw - 8.0 * PX);
    d.text(&what, ix, foot_y + (foot_h - ws) / 2.0, ws, s.soft);
}

/// M95: a player's platform and input in a navy pill in the top right corner of the well ending at `right` from `top`.
fn device_pill(d: &mut Draw, dev: crate::device::Device, right: f32, top: f32, s: &Seat) {
    let navy = super::frame::NAVY;
    let pill = [right - 6.0 * PX - 61.0 * PX, top + 6.0 * PY, 61.0 * PX, 22.0 * PY];
    rounded(d, pill, 11.0, navy, 128.0);
    super::frame::device_icons(d, dev, pill[0] + pill[2] - 10.0 * PX, pill[1] + pill[3] / 2.0, (PX, PY), s.ring, navy);
}

/// B481: the button art on computer player `p`'s join bar, if an unseated pad can take the seat: on a local roster
/// each pad's ✕ takes the next computer seat [`Menu::join_seat`] picks (held-open seats first, then in seat order), so
/// the first so many computer cards as there are unseated pads show one, in pad order.
fn join_glyph(m: &Menu, p: usize) -> Option<legend::Style> {
    if !m.local_roster() || m.seats[p].is_some() || !m.can_join() {
        return None;
    }
    let n = m.players();
    let order = (0..n).filter(|&q| m.seats[q] == Some(Dev::Open)).chain((0..n).filter(|&q| m.seats[q].is_none()));
    let k = order.into_iter().position(|q| q == p)?;
    let seated: Vec<Entity> = m.seats.iter().filter_map(|s| if let Some(Dev::Pad(e)) = s { Some(*e) } else { None }).collect();
    legend::unseated_styles(&seated).get(k).copied()
}

/// B481: a translucent dark bar across `[x, y, w, h]` (a computer card's preview and stats) with thin gold edges,
/// "<✕> to join" in `style`'s art in the middle, its opacity pulsing slowly.
fn join_bar(d: &mut Draw, style: legend::Style, [x, y, w, h]: [f32; 4]) {
    static START: std::sync::OnceLock<bevy::platform::time::Instant> = std::sync::OnceLock::new();
    let t = START.get_or_init(bevy::platform::time::Instant::now).elapsed().as_secs_f32();
    // 2.4 s a breath, between 70% and full
    let k = 0.85 + 0.15 * (t * std::f32::consts::TAU / 2.4).cos();
    let start = d.out.len();
    let bh = (h * 0.24).max(40.0 * PY).min(h);
    let by = y + (h - bh) / 2.0;
    d.q(Tex::Solid, [0.0; 4], [x, by, w, bh], [0.0; 3], 100.0);
    let edge = 1.5 * PY;
    d.q(Tex::Solid, [0.0; 4], [x, by, w, edge], GOLD, 128.0);
    d.q(Tex::Solid, [0.0; 4], [x, by + bh - edge, w, edge], GOLD, 128.0);
    // the button, a little taller than the words, then " to join", the pair centred and shrunk to fit
    let mut size = (bh * 0.42).min(26.0 * PY);
    let tail = " to join";
    let mut ih = size * 1.4;
    let full = legend::ok_aspect(style) * ih + d.width(tail, size);
    let fit = ((w - 16.0 * PX) / full.max(1.0)).min(1.0);
    (size, ih) = (size * fit, ih * fit);
    let iw = legend::ok_aspect(style) * ih;
    let mut cx = x + (w - full * fit) / 2.0;
    let cy = by + bh / 2.0;
    d.q(Tex::Button(style), [0.0; 4], [cx, cy - ih / 2.0, iw, ih], WHITE, 128.0);
    cx += iw;
    d.outlined(|d| d.text(tail, cx, cy - size / 2.0, size, WHITE));
    d.out[start..].iter_mut().for_each(|q| q.alpha *= k);
}

/// B423: what's off `v`'s view with `cols` tiles a row: above, the sections with lines over the top; below, the
/// tiles not wholly on view and the sections whose headings are past it (the next ones, not the one going on), in order.
pub(super) fn off_view(v: &View, cols: usize) -> (Vec<usize>, usize, Vec<usize>) {
    let mut above: Vec<usize> = Vec::new();
    for &l in &v.lines[..v.top] {
        let (Line::Head(s) | Line::Tiles(s, _)) = l;
        if above.last() != Some(&s) {
            above.push(s);
        }
    }
    let below = v.lines[v.end..].iter().filter_map(|&l| if let Line::Head(s) = l { Some(s) } else { None }).collect();
    let more = v.lines[v.end..].iter().map(|&l| if let Line::Tiles(s, i) = l { v.secs[s].cells[i..].len().min(cols) } else { 0 }).sum();
    (above, more, below)
}

/// `lead` and `names` joined by commas in `max` wide at `size`: as many as fit whole, then "+n" for the rest; with
/// none fitting, `lead` alone ("84 more", a bare "+4" says nothing).
fn fit_names(d: &Draw, lead: &str, names: &[&str], size: f32, max: f32) -> String {
    let join = |k: usize| {
        let mut s = format!("{lead}{}", names[..k].join(", "));
        if k < names.len() {
            s += &format!("{}+{}", if k > 0 { ", " } else { "" }, names.len() - k);
        }
        s
    };
    let k = (1..=names.len()).rev().find(|&k| d.width(&join(k), size) <= max).unwrap_or(0);
    let s = if k == 0 && !lead.is_empty() { lead.trim_end_matches(": ").to_string() } else { join(k) };
    if d.width(&s, size) <= max { s } else { ellipsis(d, &join(1), size, max) }
}

/// The grid's fade at a cut edge, and the more pill's height (px).
const PEEK_FADE: f32 = 30.0;
const PEEK_PILL: f32 = 20.0;

/// B423: player `p`'s grid `[x, y, w, h]` (`v` as scrolled, `cols` tiles a row) with lines off view: that edge fades
/// into the card's tint and a pill over it says what's there: `▼ 12 more: Golf, Mods` centred at the foot (the sections
/// still to come, as many as fit, then `+n`), `▲ Favorites` at the top's right (clear of a heading's name). A tap on it
/// scrolls a page that way.
fn peek(d: &mut Draw, v: &View, cols: usize, s: &Seat, [x, y, w, h]: [f32; 4], p: usize) {
    let (above, more, below) = off_view(v, cols);
    let fh = PEEK_FADE * PY;
    let strips = 10;
    let fade = |d: &mut Draw, top: bool| {
        for i in 0..strips {
            let sh = fh / strips as f32;
            // the edge itself all but hidden, clear by `fh` in
            let a = 118.0 * (1.0 - i as f32 / strips as f32);
            let sy = if top { y + sh * i as f32 } else { y + h - sh * (i + 1) as f32 };
            d.q(Tex::Solid, [0.0; 4], [x, sy, w, sh], s.bg, a);
        }
    };
    let names = |list: &[usize]| list.iter().map(|&k| v.secs[k].name.as_str()).collect::<Vec<_>>();
    let ph = PEEK_PILL * PY;
    let size = 12.0 * PY;
    let pill = |d: &mut Draw, head: &str, list: &[&str], down: bool| {
        let tri = 8.0 * PX;
        let lead = tri + 6.0 * PX;
        // at most ¾ of the grid's width: the tiles stay readable round it
        let max = (w * 0.75).min(w - 8.0 * PX) - lead - 20.0 * PX;
        let words = fit_names(d, head, list, size, max.max(10.0));
        let pw = d.width(&words, size) + lead + 20.0 * PX;
        let px = if down { x + (w - pw) / 2.0 } else { x + w - pw - 4.0 * PX };
        let py = if down { y + h - ph - 3.0 * PY } else { y + 3.0 * PY };
        ringed(d, [px, py, pw, ph], PEEK_PILL / 2.0, 1.5, s.ring, s.well);
        triangle(d, [px + 10.0 * PX, py + (ph - tri * PY / PX) / 2.0, tri, tri * PY / PX], down, s.ring);
        d.text(&words, px + 10.0 * PX + lead, py + (ph - size) / 2.0, size, hex(look::INK));
        d.hit([px, py, pw, ph], pointer::Act::Page(p, if down { 1 } else { -1 }));
    };
    if !above.is_empty() {
        // a heading on top is a clean edge: left as it is
        if !matches!(v.lines.get(v.top), Some(Line::Head(_))) {
            fade(d, true);
        }
        pill(d, "", &names(&above), false);
    }
    if more > 0 || !below.is_empty() {
        fade(d, false);
        let head = match (more, below.is_empty()) {
            (0, _) => String::new(),
            (n, true) => format!("{n} more"),
            (n, false) => format!("{n} more: "),
        };
        pill(d, &head, &names(&below), true);
    }
}

/// A solid triangle in `[x, y, w, h]` pointing down (else up), as thin rows.
pub(super) fn triangle(d: &mut Draw, [x, y, w, h]: [f32; 4], down: bool, rgb: [f32; 3]) {
    let n = 8;
    let rh = h / n as f32;
    for i in 0..n {
        // row `i` from the top: widest at the base
        let k = if down { n - i } else { i + 1 } as f32 / n as f32;
        let rw = w * k;
        d.q(Tex::Solid, [0.0; 4], [x + (w - rw) / 2.0, y + rh * i as f32, rw, rh], rgb, 128.0);
    }
}

/// B382: a section's heading in `[x, y, w, h]`: its name (a gold star before Favorites), its count, and a divider
/// line on to the column's edge.
fn heading(d: &mut Draw, sec: &Section, [x, y, w, h]: [f32; 4], s: &Seat) {
    let size = 15.0 * PY;
    let ty = y + (h - size) / 2.0;
    let mut tx = x + 2.0 * PX;
    if sec.id == tabs::FAVORITES {
        let ss = size * 1.3;
        d.outlined(|d| d.text("*", tx, ty - 1.0 * PY, ss, GOLD));
        tx += d.width("*", ss) + 4.0 * PX;
    }
    let count = sec.cells.len().to_string();
    let cs = 12.0 * PY;
    let cw = d.width(&count, cs) + 10.0 * PX;
    let name = ellipsis(d, &sec.name, size, (x + w - tx - cw - 16.0 * PX).max(10.0));
    d.text(&name, tx, ty, size, hex(look::INK));
    tx += d.width(&name, size) + 6.0 * PX;
    rounded(d, [tx, y + (h - cs - 4.0 * PY) / 2.0, cw, cs + 4.0 * PY], (cs + 4.0) / 2.0, s.well, 128.0);
    d.centred(&count, tx + cw / 2.0, y + (h - cs) / 2.0, cs, s.soft);
    tx += cw + 8.0 * PX;
    if tx < x + w {
        d.q(Tex::Solid, [0.0; 4], [tx, y + h / 2.0 - 1.0 * PY, x + w - tx, 2.0 * PY], hex(look::RULE), 128.0);
    }
}

/// B422: seat `p`'s header band in `[x, y, w, h]`: `solid` the seat's colour (a computer player's dark, its words in
/// the seat's colour), its top corners the card's; a big `1P`, the character's `name` over `device`, and the `side`
/// pill on the right (`glyph`'s button art before it; left of the ✕ when `drop`; left out when the column is too
/// narrow for it and a name).
#[allow(clippy::too_many_arguments)]
fn band(d: &mut Draw, s: &Seat, [x, y, w, h]: [f32; 4], solid: bool, p: usize, name: &str, device: &str, side: &str, glyph: Option<legend::Style>, drop: bool) {
    // B471: the panel's header strip inside the card's outline, a dark rule under it; a computer player's in the
    // seat's light tint with dark words
    let (fill, ink) = if solid { (s.ring, s.ink) } else { (s.sel, hex(look::INK)) };
    let t = look::RIM;
    d.q(Tex::Solid, [0.0; 4], [x + t * PX, y + t * PY, w - 2.0 * t * PX, h - t * PY], fill, 128.0);
    d.out.last_mut().unwrap().round = [(14.0 - t) * PX, (14.0 - t) * PX, 0.0, 0.0];
    d.q(Tex::Solid, [0.0; 4], [x + t * PX, y + h, w - 2.0 * t * PX, 2.5 * PY], hex(look::INK), 128.0);
    let big = 36.0 * PY;
    let tag = format!("{}P", p + 1);
    let tx = x + 12.0 * PX;
    d.text(&tag, tx, y + (h - big) / 2.0 + 2.0 * PY, big, ink);
    let nx = tx + d.width(&tag, big) + 10.0 * PX;
    let (ss, sh) = (12.0 * PY, 20.0 * PY);
    let gh = 14.0 * PY;
    let gw = glyph.map_or(0.0, |g| gh * legend::glyph_aspect(g, MODE_GLYPH) + 4.0 * PX);
    let sw = d.width(side, ss) + 16.0 * PX + gw;
    let sx = x + w - 12.0 * PX - sw - if drop { 30.0 } else { 0.0 };
    let end = if sx - 8.0 * PX - nx >= 48.0 * PX {
        rounded(d, [sx, y + (h - sh) / 2.0, sw, sh], 10.0, hex(BG), 128.0);
        if let Some(g) = glyph {
            d.q(Tex::Glyph(g, MODE_GLYPH), [0.0; 4], [sx + 6.0 * PX, y + (h - gh) / 2.0, gw - 4.0 * PX, gh], WHITE, 128.0);
        }
        d.centred(side, sx + gw + (sw - gw) / 2.0, y + (h - ss) / 2.0, ss, WHITE);
        sx - 8.0 * PX
    } else {
        x + w - 8.0 * PX - if drop { 30.0 } else { 0.0 }
    };
    let room = (end - nx).max(10.0);
    let (ns, ds) = (20.0 * PY, 12.0 * PY);
    let top = y + (h - ns - 3.0 * PY - ds) / 2.0;
    let name = ellipsis(d, name, ns, room);
    d.text(&name, nx, top, ns, ink);
    let device = ellipsis(d, device, ds, room);
    d.text(&device, nx, top + ns + 3.0 * PY, ds, ink);
}

/// B422: how far the section rail scrolls (px) to show chip `cur` of chips `widths` wide, `gap` apart, in `room`:
/// none while they all fit or `cur` is on view from the start; else as little as keeps `cur` clear of the `fade` at
/// a cut edge (no fade past the last chip), never past the last chip's end.
pub(super) fn rail_offset(widths: &[f32], gap: f32, room: f32, cur: usize, fade: f32) -> f32 {
    let n = widths.len();
    let total = widths.iter().sum::<f32>() + gap * n.saturating_sub(1) as f32;
    if total <= room || cur >= n {
        return 0.0;
    }
    let left = widths[..cur].iter().sum::<f32>() + gap * cur as f32;
    let right = left + widths[cur];
    let edge = if cur + 1 == n { 0.0 } else { fade };
    let mut off = (right - (room - edge)).max(0.0);
    if off > 0.0 && left - off < fade {
        off = (left - fade).max(0.0);
    }
    off.clamp(0.0, total - room)
}

/// The rail's fade at a cut edge (px).
const FADE: f32 = 22.0;
/// The rail's gap between chips (px).
const RAIL_GAP: f32 = 5.0;

/// B422: each section's chip width on the rail (its name, or a star for Favorites, and its count).
fn rail_widths(d: &Draw, secs: &[Section]) -> Vec<f32> {
    let (size, cs) = (13.0 * PY, 12.0 * PY);
    let (pad, inner) = (9.0 * PX, 6.0 * PX);
    let star = size * 1.3;
    secs.iter().map(|sec| 2.0 * pad + if sec.id == tabs::FAVORITES { d.width("*", star) } else { d.width(&sec.name, size) } + inner + d.width(&sec.cells.len().to_string(), cs)).collect()
}

/// B425: the Get more chip's words, longest first: the first that fits after the section chips is shown.
const GET_MORE: [&str; 3] = ["+ Get more characters", "+ Get more", "+ More"];

/// B425: a [`GET_MORE`] chip's width for `s`.
fn more_w(d: &Draw, s: &str) -> f32 {
    2.0 * 9.0 * PX + d.width(s, 13.0 * PY)
}

/// B425: which of the Get more chip's `labels` (widths, longest first) ends the rail: the longest that fits after
/// the section chips (`total` wide) and a `gap` in `room`; else the longest that leaves the sections half the rail
/// (they scroll for the rest); else the shortest. Never squeezed.
pub(super) fn more_label(total: f32, room: f32, gap: f32, labels: &[f32]) -> usize {
    let last = labels.len().saturating_sub(1);
    labels.iter().position(|&w| total + gap + w <= room).or_else(|| labels.iter().position(|&w| w + gap <= room / 2.0)).unwrap_or(last)
}

/// B425: the Get more ghost chip in `[x, y, w, h]`: a dashed outline in the seat's soft colour, its words the same.
fn get_more(d: &mut Draw, label: &str, [x, y, w, h]: [f32; 4], s: &Seat) {
    let size = 13.0 * PY;
    dashed(d, [x, y, w, h], h / PY / 2.0, 1.5, s.soft, s.bg);
    d.text(label, x + 9.0 * PX, y + (h - size) / 2.0, size, s.soft);
}

/// B422: the section rail in `[x, y, w, h]`, one row that never scrolls with the grid: a chip per section (a gold star
/// and the count for Favorites, else the name and count, so "16" reads as "16 here"), section `cur`'s filled in the
/// seat's colour while the seat picks, the rest outlined. Chips keep their size (never squeezed): past the rail's
/// width it scrolls sideways to keep `cur` on view, a fade and « / » at a cut edge. A tap on a chip jumps there.
#[allow(clippy::too_many_arguments)]
fn rail(d: &mut Draw, secs: &[Section], cur: usize, active: bool, s: &Seat, [x, y, w, h]: [f32; 4], p: usize) {
    let (size, cs) = (13.0 * PY, 12.0 * PY);
    let (pad, gap, inner) = (9.0 * PX, RAIL_GAP * PX, 6.0 * PX);
    let star = size * 1.3;
    let lead = |d: &Draw, sec: &Section| if sec.id == tabs::FAVORITES { d.width("*", star) } else { d.width(&sec.name, size) };
    let widths = rail_widths(d, secs);
    let fade = FADE * PX;
    let off = rail_offset(&widths, gap, w, cur, fade);
    let total = widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
    let mut cx = x - off;
    for (k, sec) in secs.iter().enumerate() {
        let cw = widths[k];
        let (l, r) = (cx.max(x), (cx + cw).min(x + w));
        if r > l + 1.0 {
            let start = d.out.len();
            let filled = k == cur && active;
            if filled {
                rounded(d, [cx, y, cw, h], h / PY / 2.0, s.ring, 128.0);
            } else {
                ringed(d, [cx, y, cw, h], h / PY / 2.0, 1.5, if k == cur { s.ring } else { s.sel }, s.bg);
            }
            let (words, soft) = if filled { (s.ink, s.ink) } else { (hex(look::INK), s.soft) };
            let mut tx = cx + pad;
            if sec.id == tabs::FAVORITES {
                d.outlined(|d| d.text("*", tx, y + (h - star) / 2.0 + 1.0 * PY, star, GOLD));
            } else {
                d.text(&sec.name, tx, y + (h - size) / 2.0, size, words);
            }
            tx += lead(d, sec) + inner;
            d.text(&sec.cells.len().to_string(), tx, y + (h - cs) / 2.0, cs, soft);
            clip_x(d, start, x, x + w);
            d.hit([l, y, r - l, h], pointer::Act::Section(p, k));
        }
        cx += cw + gap;
    }
    // the cut edges: the card's tint fading in over the chips, an arrow on it
    let edge = |d: &mut Draw, left: bool| {
        let n = 8;
        for i in 0..n {
            let sw = fade / n as f32;
            let sx = if left { x + sw * i as f32 } else { x + w - sw * (i + 1) as f32 };
            d.q(Tex::Solid, [0.0; 4], [sx, y, sw, h], s.bg, 128.0 * (1.0 - i as f32 / n as f32));
        }
        let a = if left { "«" } else { "»" };
        let aw = d.width(a, size);
        d.text(a, if left { x + 2.0 * PX } else { x + w - aw - 2.0 * PX }, y + (h - size) / 2.0, size, s.soft);
    };
    if off > 0.5 {
        edge(d, true);
    }
    if off + w < total - 0.5 {
        edge(d, false);
    }
}

/// The quads drawn since `start` cut to `l..r` across: a picture's source with its box, rounded corners squared on a
/// cut side, those wholly outside dropped.
pub(super) fn clip_x(d: &mut Draw, start: usize, l: f32, r: f32) {
    let mut kept = Vec::new();
    for mut q in d.out.drain(start..) {
        let [qx, _, qw, _] = q.dst;
        let (a, b) = (qx.max(l), (qx + qw).min(r));
        if b <= a {
            continue;
        }
        if qw > 0.0 && (a > qx || b < qx + qw) {
            let k = q.src[2] / qw;
            q.src[0] += (a - qx) * k;
            q.src[2] = (b - a) * k;
            if a > qx {
                q.round[0] = 0.0;
                q.round[3] = 0.0;
            }
            if b < qx + qw {
                q.round[1] = 0.0;
                q.round[2] = 0.0;
            }
            q.dst[0] = a;
            q.dst[2] = b - a;
        }
        kept.push(q);
    }
    d.out.extend(kept);
}

/// B422: a `1P`–`4P` tag in each of `seats`' colours along tile `[x, y, w, h]`'s foot, left to right (as many as fit).
fn tags(d: &mut Draw, [x, y, w, h]: [f32; 4], seats: &[usize]) {
    let th = (16.0 * PY).min(h * 0.3);
    let size = th * 0.72;
    let mut tx = x + 2.0 * PX;
    for &q in seats {
        let t = format!("{}P", q + 1);
        let tw = d.width(&t, size) + 6.0 * PX;
        if tx + tw > x + w + 0.01 {
            break;
        }
        let s = seat(q);
        ringed(d, [tx, y + h - th - 2.0 * PY, tw, th], 5.0, 1.5, hex(BG), s.ring);
        d.centred(&t, tx + tw / 2.0, y + h - th - 2.0 * PY + (th - size) / 2.0, size, s.ink);
        tx += tw + 2.0 * PX;
    }
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
    // B471: the main menu's grey buttons, white key caps
    let grey = hex(look::GREY);
    look::outlined_pill(d, cb, grey, 2.0);
    let mut cx = x + pad;
    shirt(d, [cx, cy + 5.0 * PY, icon, 16.0 * PY], s.ring, grey);
    cx += icon + gap;
    if keys && !l1.is_empty() {
        cx += chip(d, &l1, [cx, cy + (ph - gh) / 2.0], gh, WHITE) + gap;
    }
    let words = fits(cw_nat, cw, d.width(&costume, ts));
    d.label(&costume, cx + words / 2.0, cy + (ph - ts) / 2.0, ts, words, if m.blocked(p) && !pl.random { [70.0; 3] } else { WHITE });
    cx += words + gap;
    if keys && !r1.is_empty() {
        chip(d, &r1, [cx, cy + (ph - gh) / 2.0], gh, WHITE);
    }
    // the hand pill, at the right end
    let hx = x + w - hw;
    let hb = [hx, hy, hw, ph];
    look::outlined_pill(d, hb, grey, 2.0);
    let mut cx = hx + pad;
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [cx - 4.0 * PX, hy + 6.0 * PY, icon + 6.0 * PX, 14.0 * PY], WHITE, 128.0);
    cx += icon + gap;
    let words = fits(hw_nat, hw, d.width(hand, ts));
    d.label(hand, cx + words / 2.0, hy + (ph - ts) / 2.0, ts, words, WHITE);
    cx += words + gap;
    if keys && !sel.is_empty() {
        chip(d, &sel, [cx, hy + (ph - gh) / 2.0], gh, WHITE);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// B422: the rail stays put while its chips fit or the cursor's is on view, else scrolls as little as keeps that
    /// chip clear of the fade, never past the last chip.
    #[test]
    fn the_rail_scrolls_to_the_cursors_chip() {
        let w = [30.0, 100.0, 80.0, 120.0, 50.0];
        // 380 + 4 gaps of 5 = 400 wide
        assert_eq!(rail_offset(&w, 5.0, 400.0, 3, 20.0), 0.0, "all fit");
        assert_eq!(rail_offset(&w, 5.0, 250.0, 0, 20.0), 0.0);
        assert_eq!(rail_offset(&w, 5.0, 250.0, 1, 20.0), 0.0, "on view from the start");
        // chip 2 ends at 220: 220 - (250 - 20) < 0, on view; chip 3 ends at 345: shifted to end at 230
        assert_eq!(rail_offset(&w, 5.0, 250.0, 2, 20.0), 0.0);
        assert_eq!(rail_offset(&w, 5.0, 250.0, 3, 20.0), 115.0);
        // the last chip: right to the end, no fade after it
        assert_eq!(rail_offset(&w, 5.0, 250.0, 4, 20.0), 150.0);
        // a chip wider than the room keeps its start clear of the left fade
        assert_eq!(rail_offset(&[50.0, 300.0, 50.0], 0.0, 200.0, 1, 20.0), 30.0);
    }

    /// B425: the Get more chip's words: the longest that fits after the sections, else one leaving them half the rail.
    #[test]
    fn the_get_more_chip_takes_the_longest_words_that_fit() {
        let l = [150.0, 80.0, 50.0];
        assert_eq!(more_label(200.0, 400.0, 5.0, &l), 0);
        assert_eq!(more_label(200.0, 300.0, 5.0, &l), 1);
        assert_eq!(more_label(200.0, 260.0, 5.0, &l), 2);
        // the sections scroll: the chip keeps no more than half the rail
        assert_eq!(more_label(400.0, 200.0, 5.0, &l), 1);
        assert_eq!(more_label(400.0, 100.0, 5.0, &l), 2);
        assert_eq!(more_label(400.0, 40.0, 5.0, &l), 2);
    }

    /// B422: the header band across the card's top, the rail between the preview and the grid, at every aspect and
    /// column count.
    #[test]
    fn band_and_rail_in_the_card() {
        let inside = |r: [f32; 4], c: [f32; 4]| r[0] >= c[0] - 1e-3 && r[1] >= c[1] - 1e-3 && r[0] + r[2] <= c[0] + c[2] + 1e-3 && r[1] + r[3] <= c[1] + c[3] + 1e-3;
        for a in [0.4, 9.0 / 16.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 4.0] {
            let g = super::super::Grid::fit(a);
            for n in 1..=4 {
                let c = columns(g.view, n);
                for p in 0..n {
                    let at = c.parts(p);
                    assert_eq!((at.head[0], at.head[1], at.head[2]), (at.card[0], at.card[1], at.card[2]), "{a} {n}");
                    assert!(at.head[1] + at.head[3] < at.pills[1], "{a} {n}");
                    assert!(inside(at.rail, at.card) && at.preview[1] + at.preview[3] <= at.rail[1] + 1e-3 && at.rail[1] + at.rail[3] <= at.grid[1] + 1e-3, "{a} {n} {p}");
                }
            }
        }
    }

    /// M120: online, the other machines' compact cards no wider than `COMPACT_W` and never wider than a full column;
    /// every column side by side inside the view, at every aspect and mix.
    #[test]
    fn compact_cards_leave_the_rest_to_the_rosters() {
        for a in [0.4, 9.0 / 16.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 4.0] {
            let g = super::super::Grid::fit(a);
            let [l, _, r, _] = g.view;
            for n in 2..=4 {
                for mask in 1..(1u8 << n) - 1 {
                    let compact = std::array::from_fn(|p| mask >> p & 1 == 1);
                    let c = columns_mixed(g.view, n, compact);
                    let mut x = l;
                    for (p, &small) in compact.iter().enumerate().take(n) {
                        let [cx, _, cw, _] = c.parts(p).card;
                        assert!(cx >= x - 1e-3 && cw > 0.0, "{a} {n} {mask} {p}");
                        if small {
                            assert!(cw <= COMPACT_W * PX + 1e-3 && cw <= c.first[2] + 1e-3, "{a} {n} {mask} {p}");
                        } else {
                            assert!((cw - c.first[2]).abs() < 1e-3, "{a} {n} {mask} {p}");
                        }
                        x = cx + cw;
                    }
                    assert!(x <= r + 1e-3, "{a} {n} {mask}");
                }
            }
        }
    }
}
