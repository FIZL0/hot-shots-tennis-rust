//! B448: the shared parts of the Mods and Settings pages. B471 ("Menus on the wall", `plan/B471.md`): in the main
//! menu's look (`home.rs`, `Draw::frame`): the tiled wall, the coloured band along the top with the hanging title tab
//! (Back for the pointer), off-white panels in the main menu's thick dark outline with a coloured header strip and a
//! dark rule under it, the main menu's grey buttons (the chosen one yellow), white key caps and value pills, dark
//! words on the panels with grey-violet secondary ones and light rules. A kind's or seat's colour is an accent only:
//! header strips, a tile's ring, a placeholder letter, the chosen choice. Sizes are the mockup's px at 720 lines
//! (`roster::PY` down, `roster::PX` across).

use super::roster::{PX, PY, ellipsis, hex, rounded};
use super::{Anchor, Draw, Tex, WHITE, legend, pointer};

/// The top bar over the panels (the band and the title tab hang in it), the side margins and the panels' gap, a
/// panel's padding, the gap between its rows, a pill's height, the scroll bar's width, a heading's height.
pub(super) const TOP_BAR: f32 = 72.0;
pub(super) const MARGIN: f32 = 24.0;
pub(super) const GAP: f32 = 12.0;
pub(super) const PAD: f32 = 12.0;
pub(super) const ROW_GAP: f32 = 8.0;
pub(super) const PILL: f32 = 28.0;
pub(super) const BAR: f32 = 6.0;
pub(super) const HEADING: f32 = 24.0;
/// A panel's header strip (one row of pills in it).
pub(super) const STRIP: f32 = 46.0;
/// The gap over the legend bar at the view's foot (the view already ends above the bar: `Grid::fit_at`) and the
/// title band's depth, in view units (the top bar's pills sit under the band).
pub(super) const LEGEND: f32 = 6.0;
pub(super) const BAND: f32 = 18.0;
/// The panels' outline, mockup px.
pub(super) const RIM: f32 = 3.0;

/// The panels' body, their outline, the grey buttons, the chosen one's yellow, the secondary words, the light rules,
/// a tile's fill and border, the chosen choice's blue, the light green of Settings' side strip.
pub(super) const PAPER: u32 = 0xfbf9fd;
pub(super) const INK: u32 = 0x1d1a24;
pub(super) const GREY: u32 = 0x7b7690;
pub(super) const YELLOW: u32 = 0xffd84a;
pub(super) const MUTED: u32 = 0x6b6478;
pub(super) const RULE: u32 = 0xd8d0e2;
pub(super) const TILE: u32 = 0xefe8f5;
pub(super) const TILE_RIM: u32 = 0xddd3e8;
pub(super) const BLUE: u32 = 0x3a6fd8;
pub(super) const MINT: u32 = 0x8fd6a4;

/// The kinds' accents (`mods_kind::seat_of`'s order: Characters red, Textures gold, Umpires blue, Courts green).
pub(super) fn accent(i: usize) -> [f32; 3] {
    hex([0xe0404c, 0xf2c230, BLUE, 0x3aa45a][i % 4])
}

/// `c` in the menus' 0–128 colour (shorthand for the consts above).
pub(super) fn c(c: u32) -> [f32; 3] {
    hex(c)
}

/// The background over the whole window: the main menu's tiled wall.
pub(super) fn backdrop(d: &mut Draw) {
    d.wall(super::WALL);
}

/// The main menu's title band (`Draw::frame`): `band` along the top over a white and a dark rule, a tab hanging from
/// it with `title` outlined white (a tap on it is Back). `view` is the page's (unzoomed) view: returns where the top
/// bar's room starts right of the tab on it.
pub(super) fn top(d: &mut Draw, view: [f32; 4], title: &str, band: [f32; 3]) -> f32 {
    d.anchor = Anchor::Span;
    d.q(Tex::Solid, [0.0; 4], [0.0, 0.0, 640.0, 14.0], band, 128.0);
    d.q(Tex::Solid, [0.0; 4], [0.0, 14.0, 640.0, 2.5], WHITE, 128.0);
    d.q(Tex::Solid, [0.0; 4], [0.0, 16.5, 640.0, 1.5], [15.0; 3], 128.0);
    let size = 30.0;
    let w = d.width(title, size);
    d.box3([-30.0, -30.0, w + 110.0, 70.0], 16.0, [Some([15.0; 3]), Some(WHITE), Some(band)], 128.0);
    let start = d.out.len();
    d.text(title, 42.0, 7.0, size, [0.0; 3]);
    d.out[start..].iter_mut().for_each(|q| q.alpha = 40.0);
    d.outlined(|d| d.text(title, 40.0, 5.0, size, WHITE));
    d.hit([-30.0, -30.0, w + 110.0, 70.0], pointer::press(|k| k.back = true));
    d.anchor = Anchor::Centre;
    // the view's left is the window's (`Grid::fit_at`), where the tab's box starts 30 units out
    view[0] + w + 80.0 + GAP * PX
}

/// The top of the top bar's row of pills right of the title tab, on a view whose top is `t`.
pub(super) fn bar_y(t: f32) -> f32 {
    t + BAND + (TOP_BAR * PY - BAND - PILL * PY) / 2.0
}

/// B448: a portrait view drawn larger: the page laid out on a view `1 / z` as wide (and tall) about its top centre
/// `c`, then every quad and hit scaled back out (`apply`), so the columns read as on a phone mockup's 640 px width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Zoom {
    pub c: [f32; 2],
    pub z: f32,
}

impl Zoom {
    /// The zoom for `view` (left, top, right, bottom): 1 unless `portrait`, then about 420 units across.
    pub(super) fn of(view: [f32; 4], portrait: bool) -> Self {
        let [l, t, r, _] = view;
        Zoom { c: [(l + r) / 2.0, t], z: if portrait { ((r - l) / 420.0).max(1.0) } else { 1.0 } }
    }

    /// The view the page lays itself out on.
    pub(super) fn inner(&self, [l, t, r, b]: [f32; 4]) -> [f32; 4] {
        let [cx, cy] = self.c;
        [cx + (l - cx) / self.z, cy + (t - cy) / self.z, cx + (r - cx) / self.z, cy + (b - cy) / self.z]
    }

    /// Rect `r` on the inner view, on the screen.
    pub(super) fn at(&self, [x, y, w, h]: [f32; 4]) -> [f32; 4] {
        let [cx, cy] = self.c;
        [cx + (x - cx) * self.z, cy + (y - cy) * self.z, w * self.z, h * self.z]
    }

    /// Scales the quads drawn from `n0` and every hit out to the screen (the whole-window ones stay).
    pub(super) fn apply(&self, d: &mut Draw, n0: usize) {
        if self.z == 1.0 {
            return;
        }
        for q in d.out[n0..].iter_mut().filter(|q| q.anchor == Anchor::Centre) {
            q.dst = self.at(q.dst);
            q.round = q.round.map(|r| r * self.z);
        }
        for h in d.hits.iter_mut().filter(|h| h.anchor == Anchor::Centre) {
            h.rect = self.at(h.rect);
        }
    }
}

/// A tab pill's natural width for `s`.
pub(super) fn tab_w(d: &Draw, s: &str) -> f32 {
    d.width(s, 15.0 * PY) + 26.0 * PX
}

/// The button art `i` (`legend::page_glyphs`) `h` high at `[x, y]` in the device last used's style; its width.
pub(super) fn glyph(d: &mut Draw, i: u8, [x, y]: [f32; 2], h: f32) -> f32 {
    let style = legend::last_style();
    let w = h * legend::glyph_aspect(style, i);
    d.q(Tex::Glyph(style, i), [0.0; 4], [x, y, w, h], WHITE, 128.0);
    w
}

/// The width [`glyph`] `i` takes `h` high.
pub(super) fn glyph_w(i: u8, h: f32) -> f32 {
    h * legend::glyph_aspect(legend::last_style(), i)
}

/// How a row of tab pills draws: the top bar's (grey, the chosen one yellow) or a header strip's (translucent dark,
/// the chosen one white).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Tabs {
    Bar,
    Strip,
}

/// Tab pills in a row from `x` (`y` its top, `room` wide), between the button art `caps` (L1 / R1, L2 / R2; none while
/// there's one tab), the chosen one (`on`) lit (`style`), the cursor's (`cursor`) in a white halo; a tap on one is
/// `act(i)`. The row shrinks as one where it doesn't fit. Returns its right end.
#[allow(clippy::too_many_arguments)]
pub(super) fn tabs(d: &mut Draw, [x, y, room]: [f32; 3], words: &[String], style: Tabs, on: Option<usize>, cursor: Option<usize>, caps: Option<(u8, u8)>, dim: [bool; 2], act: impl Fn(usize) -> pointer::Act) -> f32 {
    let (h, gh) = (PILL * PY, 20.0 * PY);
    let caps = caps.filter(|_| words.len() > 1);
    let gap = 6.0 * PX;
    let ws: Vec<f32> = words.iter().map(|s| tab_w(d, s)).collect();
    let cw = caps.map_or(0.0, |(a, b)| glyph_w(a, gh) + glyph_w(b, gh) + 2.0 * gap);
    let natural = cw + ws.iter().sum::<f32>() + gap * ws.len().saturating_sub(1) as f32;
    let k = (room / natural).min(1.0);
    let mut cx = x;
    let fade = |d: &mut Draw, i: usize| {
        if dim[i] {
            d.out.last_mut().unwrap().alpha = legend::DIM_ALPHA;
        }
    };
    if let Some((a, _)) = caps {
        cx += glyph(d, a, [cx, y + (h - gh * k) / 2.0], gh * k) + gap * k;
        fade(d, 0);
    }
    for (i, (s, w)) in words.iter().zip(&ws).enumerate() {
        let b = [cx, y, w * k, h];
        let lit = on == Some(i);
        if cursor == Some(i) {
            halo(d, b);
        }
        let size = 15.0 * PY * k.max(0.7);
        let ink = match style {
            Tabs::Bar => {
                button(d, b, lit);
                if lit { c(INK) } else { WHITE }
            }
            Tabs::Strip if lit => {
                outlined_pill(d, b, c(PAPER), 2.0);
                c(INK)
            }
            Tabs::Strip => {
                rounded(d, b, h / 2.0 / PY, [10.0; 3], 46.0);
                WHITE
            }
        };
        let words = ellipsis(d, s, size, b[2] - 10.0 * PX);
        d.centred(&words, cx + b[2] / 2.0, y + (h - size) / 2.0, size, ink);
        d.hit(b, act(i));
        cx += (w + gap) * k;
    }
    if let Some((_, b)) = caps {
        cx += glyph(d, b, [cx, y + (h - gh * k) / 2.0], gh * k) + gap * k;
        fade(d, 1);
    }
    cx
}

/// The cursor's white halo round a pill `[x, y, w, h]` (drawn under it).
pub(super) fn halo(d: &mut Draw, [x, y, w, h]: [f32; 4]) {
    let t = 3.0;
    rounded(d, [x - t * PX, y - t * PY, w + 2.0 * t * PX, h + 2.0 * t * PY], h / 2.0 / PY + t, WHITE, 128.0);
}

/// A pill `b` filled `fill` in a `t`-px dark outline.
pub(super) fn outlined_pill(d: &mut Draw, b @ [x, y, w, h]: [f32; 4], fill: [f32; 3], t: f32) {
    let r = h / 2.0 / PY;
    rounded(d, b, r, c(INK), 128.0);
    rounded(d, [x + t * PX, y + t * PY, w - 2.0 * t * PX, h - 2.0 * t * PY], (r - t).max(0.0), fill, 128.0);
}

/// The main menu's button as a pill: grey in a dark outline, the chosen one (`on`) yellow.
pub(super) fn button(d: &mut Draw, b: [f32; 4], on: bool) {
    outlined_pill(d, b, c(if on { YELLOW } else { GREY }), 2.0);
}

/// A [`button`] with words in it (smaller to fit, never squeezed): white on grey, dark on yellow.
pub(super) fn word_pill(d: &mut Draw, b: [f32; 4], on: bool, words: &str, size: f32) {
    button(d, b, on);
    pill_words(d, b, words, size, if on { c(INK) } else { WHITE });
}

/// `words` centred in pill `b`, smaller to fit (down to 0.6 of `size`), cut with "..." past that.
pub(super) fn pill_words(d: &mut Draw, b: [f32; 4], words: &str, size: f32, ink: [f32; 3]) {
    let fit = size.min(size * (b[2] - 14.0 * PX) / d.width(words, size).max(1.0)).max(size * 0.6);
    let words = ellipsis(d, words, fit, b[2] - 10.0 * PX);
    d.centred(&words, b[0] + b[2] / 2.0, b[1] + (b[3] - fit) / 2.0, fit, ink);
}

/// A white "‹ value ›" pill in a dark outline (`on`: yellow), `dim` words soft (Off); `arrows` which arrows show.
#[allow(clippy::too_many_arguments)]
pub(super) fn steps(d: &mut Draw, b @ [x, y, w, h]: [f32; 4], on: bool, value: &str, size: f32, dim: bool, arrows: [bool; 2]) {
    outlined_pill(d, b, c(if on { YELLOW } else { PAPER }), 2.0);
    let ink = c(INK);
    let aw = d.width("<", size);
    if arrows[0] {
        d.text("<", x + 10.0 * PX, y + (h - size) / 2.0, size, c(MUTED));
    }
    if arrows[1] {
        d.text(">", x + w - 10.0 * PX - aw, y + (h - size) / 2.0, size, c(MUTED));
    }
    let room = w - 2.0 * aw - 28.0 * PX;
    let value = ellipsis(d, value, size, room);
    d.centred(&value, x + w / 2.0, y + (h - size) / 2.0, size, if dim && !on { c(MUTED) } else { ink });
}

/// A panel: the off-white body in the main menu's 3 px dark outline, 18 px corners, and a header strip `strip` px
/// (mockup) high in `band` with a dark rule under it (none at 0). Returns the strip's box inside the outline.
pub(super) fn panel(d: &mut Draw, [x, y, w, h]: [f32; 4], strip: f32, band: [f32; 3]) -> [f32; 4] {
    let (t, r) = (RIM, 18.0);
    rounded(d, [x, y, w, h], r, c(INK), 128.0);
    let inner @ [ix, iy, iw, _] = [x + t * PX, y + t * PY, w - 2.0 * t * PX, h - 2.0 * t * PY];
    rounded(d, inner, r - t, c(PAPER), 128.0);
    let sh = strip * PY;
    if sh > 0.0 {
        d.q(Tex::Solid, [0.0; 4], [ix, iy, iw, sh], band, 128.0);
        d.out.last_mut().unwrap().round = [(r - t) * PX, (r - t) * PX, 0.0, 0.0];
        d.q(Tex::Solid, [0.0; 4], [ix, iy + sh, iw, 2.5 * PY], c(INK), 128.0);
    }
    [ix, iy, iw, sh]
}

/// Words outlined dark (white on a header strip).
pub(super) fn bold(d: &mut Draw, s: &str, [x, y]: [f32; 2], size: f32, rgb: [f32; 3]) {
    d.outlined(|d| d.text(s, x, y, size, rgb));
}

/// A section's heading in `[x, y, w, h]`: its name bold and dark, its count grey, and a light rule on to the edge.
pub(super) fn heading(d: &mut Draw, name: &str, count: usize, [x, y, w, h]: [f32; 4]) {
    let size = 15.0 * PY;
    let ty = y + (h - size) / 2.0;
    let count = count.to_string();
    let cs = 13.0 * PY;
    let cw = d.width(&count, cs) + 8.0 * PX;
    let name = ellipsis(d, name, size, (w - cw - 20.0 * PX).max(10.0));
    let mut tx = x + 2.0 * PX;
    // bold: the words twice, a hair apart
    d.text(&name, tx, ty, size, c(INK));
    d.text(&name, tx + 0.6 * PX, ty, size, c(INK));
    tx += d.width(&name, size) + 8.0 * PX;
    d.text(&count, tx, y + (h - cs) / 2.0 + 1.0 * PY, cs, c(MUTED));
    tx += cw + 4.0 * PX;
    if tx < x + w {
        d.q(Tex::Solid, [0.0; 4], [tx, y + h / 2.0 - 1.0 * PY, x + w - tx, 2.0 * PY], c(RULE), 128.0);
    }
}

/// The scroll bar in `rect`: its thumb in `a` `shown` of `total` from `at` of the last place it scrolls to (`last`).
pub(super) fn scroll_bar(d: &mut Draw, rect @ [bx, by, bw, bh]: [f32; 4], a: [f32; 3], shown: usize, total: usize, at: usize, last: usize) {
    rounded(d, rect, 3.0, c(RULE), 128.0);
    let thumb = (bh * shown as f32 / total.max(1) as f32).min(bh).max(12.0 * PY);
    let ty = by + (bh - thumb) * at.min(last) as f32 / last.max(1) as f32;
    rounded(d, [bx, ty, bw, thumb], 3.0, a, 128.0);
}

/// A tile `[x, y, w, h]`: light with a light border; `on` white in a dark outline with a 3 px ring in `a` round it.
pub(super) fn tile(d: &mut Draw, rect @ [x, y, w, h]: [f32; 4], a: [f32; 3], on: bool) {
    let inset = |k: f32| [x + k * PX, y + k * PY, w - 2.0 * k * PX, h - 2.0 * k * PY];
    if on {
        rounded(d, rect, 12.0, a, 128.0);
        rounded(d, inset(3.0), 9.0, c(INK), 128.0);
        rounded(d, inset(5.0), 7.0, WHITE, 128.0);
    } else {
        rounded(d, rect, 12.0, c(TILE_RIM), 128.0);
        rounded(d, inset(1.5), 10.5, c(TILE), 128.0);
    }
}

/// A small outlined badge `s` filled `rgb` with dark words, its right top at `[x, y]`; its width.
pub(super) fn badge(d: &mut Draw, s: &str, [x, y]: [f32; 2], rgb: [f32; 3]) -> f32 {
    let size = 10.5 * PY;
    let w = d.width(s, size) + 12.0 * PX;
    let h = size + 7.0 * PY;
    outlined_pill(d, [x - w, y, w, h], rgb, 1.5);
    d.centred(s, x - w / 2.0, y + 3.5 * PY, size, c(INK));
    w
}

/// `s` broken at spaces into lines `w` wide at `size` from `[x, y]`, `gap` apart, at most `max` lines (the last cut
/// with "..."); the y under the last.
#[allow(clippy::too_many_arguments)]
pub(super) fn para(d: &mut Draw, s: &str, [x, y]: [f32; 2], w: f32, size: f32, gap: f32, max: usize, rgb: [f32; 3]) -> f32 {
    let lines = wrap(d, s, w, size, max);
    for (i, l) in lines.iter().enumerate() {
        let l = ellipsis(d, l, size, w);
        d.text(&l, x, y + gap * i as f32, size, rgb);
    }
    y + gap * lines.len() as f32
}

/// `s` broken into lines `w` wide, at most `max` (the last cut short with "...").
pub(super) fn wrap(d: &Draw, s: &str, w: f32, size: f32, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in s.split_whitespace() {
        match lines.last_mut() {
            Some(l) if d.width(&format!("{l} {word}"), size) <= w => *l = format!("{l} {word}"),
            _ => lines.push(word.into()),
        }
    }
    if lines.len() > max {
        let rest = lines[max - 1..].join(" ");
        lines.truncate(max - 1);
        lines.push(ellipsis(d, &rest, size, w));
    }
    lines
}
