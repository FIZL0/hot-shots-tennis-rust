//! B448: the character select's look (B422, `roster.rs`) for the Mods and Settings pages: the main menu's tiled wall,
//! an outlined white "< Title", tab pills with a ring round them (the chosen one filled), columns
//! with a 3 px ring and 14 px corners, pills in the well colour (chosen ones filled with the ring colour), tiles,
//! section headings and the 6 px scroll bar, all in one of the four seats' colour sets. Sizes are the mockup's px at
//! 720 lines (`roster::PY` down, `roster::PX` across).

use super::roster::{self, PX, PY, Seat, ellipsis, rounded};
use super::Anchor;
use super::{Draw, Tex, WHITE, legend, pointer};

/// The top bar over the columns, the side margins and the columns' gap, a column's padding, the gap between its rows,
/// a pill's height, the scroll bar's width, a heading's height (`roster.rs`'s).
pub(super) const TOP_BAR: f32 = 60.0;
pub(super) const MARGIN: f32 = 24.0;
pub(super) const GAP: f32 = 12.0;
pub(super) const PAD: f32 = 10.0;
pub(super) const ROW_GAP: f32 = 8.0;
pub(super) const PILL: f32 = 26.0;
pub(super) const BAR: f32 = 6.0;
pub(super) const HEADING: f32 = 22.0;

/// The background over the whole window: the main menu's tiled wall.
pub(super) fn backdrop(d: &mut Draw) {
    d.wall(5);
}

/// The plain white "< `title`" at the view's top left (`view` left, top, right, bottom); a tap on it is Back. Returns
/// where the top bar's room starts right of it.
pub(super) fn title(d: &mut Draw, view: [f32; 4], title: &str) -> f32 {
    let [l, t, ..] = view;
    let size = 30.0 * PY;
    d.outlined(|d| d.text(title, l + 32.0, t + 16.0 * PY, size, WHITE));
    let w = d.width(title, size) + 40.0;
    d.hit([l, t, w, TOP_BAR * PY], pointer::press(|k| k.back = true));
    d.outlined(|d| d.text("<", l + 10.0, t + 5.0, 30.0, WHITE));
    l + w + 8.0 * PX
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
    d.width(s, 15.0 * PY) + 24.0 * PX
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

/// Tab pills in a row from `x` (`y` its top, `room` wide), between the button art `caps` (L1 / R1, L2 / R2; none while
/// there's one tab): each tab its words and colour set, a ring round it, the chosen one (`on`) filled; a tap on one is
/// `act(i)`. The row shrinks as one where it doesn't fit. Returns its right end.
pub(super) fn tabs(d: &mut Draw, [x, y, room]: [f32; 3], words: &[String], seats: &[Seat], on: Option<usize>, caps: Option<(u8, u8)>, dim: [bool; 2], act: impl Fn(usize) -> pointer::Act) -> f32 {
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
        let st = &seats[i.min(seats.len() - 1)];
        let b = [cx, y, w * k, h];
        pill(d, b, st, on == Some(i), true);
        let size = 15.0 * PY * k.max(0.7);
        let words = ellipsis(d, s, size, b[2] - 10.0 * PX);
        d.centred(&words, cx + b[2] / 2.0, y + (h - size) / 2.0, size, if on == Some(i) { st.ink } else { WHITE });
        d.hit(b, act(i));
        cx += (w + gap) * k;
    }
    if let Some((_, b)) = caps {
        cx += glyph(d, b, [cx, y + (h - gh * k) / 2.0], gh * k);
        fade(d, 1);
    } else {
        cx -= gap * k;
    }
    cx
}

/// A pill `[x, y, w, h]`: the well colour, or filled with the ring colour when `on`; `rim` a 2 px ring round it
/// (the tabs').
pub(super) fn pill(d: &mut Draw, [x, y, w, h]: [f32; 4], s: &Seat, on: bool, rim: bool) {
    let r = h / 2.0 / PY;
    if on {
        rounded(d, [x, y, w, h], r, s.ring, 128.0);
    } else if rim {
        roster::ringed(d, [x, y, w, h], r, 2.0, s.ring, s.well);
    } else {
        rounded(d, [x, y, w, h], r, s.well, 128.0);
    }
}

/// A pill with words in it (smaller to fit, never squeezed); `on` filled with the ring colour, its words the ring's ink.
pub(super) fn word_pill(d: &mut Draw, b: [f32; 4], s: &Seat, on: bool, words: &str, size: f32) {
    pill(d, b, s, on, false);
    let fit = size.min(size * (b[2] - 14.0 * PX) / d.width(words, size).max(1.0)).max(size * 0.6);
    let words = ellipsis(d, words, fit, b[2] - 10.0 * PX);
    d.centred(&words, b[0] + b[2] / 2.0, b[1] + (b[3] - fit) / 2.0, fit, if on { s.ink } else { WHITE });
}

/// A "< value >" pill: the value between arrows in the well colour (`on`: the ring colour), `dim` words soft (Off).
pub(super) fn steps(d: &mut Draw, b @ [x, y, w, h]: [f32; 4], s: &Seat, on: bool, value: &str, size: f32, dim: bool) {
    pill(d, b, s, on, false);
    let ink = if on { s.ink } else { WHITE };
    let aw = d.width("<", size);
    d.text("<", x + 9.0 * PX, y + (h - size) / 2.0, size, ink);
    d.text(">", x + w - 9.0 * PX - aw, y + (h - size) / 2.0, size, ink);
    let room = w - 2.0 * aw - 26.0 * PX;
    let value = ellipsis(d, value, size, room);
    d.centred(&value, x + w / 2.0, y + (h - size) / 2.0, size, if dim && !on { s.soft.map(|c| c * 0.75) } else { ink });
}

/// A column: a 3 px ring in the colour set's own colour round its fill, 14 px corners.
pub(super) fn column(d: &mut Draw, rect: [f32; 4], s: &Seat) {
    roster::ringed(d, rect, 14.0, 3.0, s.ring, s.bg);
}

/// A section's heading in `[x, y, w, h]`: its name (the first section's in the ring colour), its count, and a rule
/// on to the edge.
pub(super) fn heading(d: &mut Draw, name: &str, count: usize, [x, y, w, h]: [f32; 4], s: &Seat, first: bool) {
    let size = 15.0 * PY;
    let ty = y + (h - size) / 2.0;
    let count = count.to_string();
    let cs = 12.0 * PY;
    let cw = d.width(&count, cs) + 10.0 * PX;
    let name = ellipsis(d, name, size, (w - cw - 20.0 * PX).max(10.0));
    let mut tx = x + 2.0 * PX;
    d.text(&name, tx, ty, size, if first { s.ring } else { WHITE });
    tx += d.width(&name, size) + 6.0 * PX;
    rounded(d, [tx, y + (h - cs - 4.0 * PY) / 2.0, cw, cs + 4.0 * PY], (cs + 4.0) / 2.0, s.well, 128.0);
    d.centred(&count, tx + cw / 2.0, y + (h - cs) / 2.0, cs, s.soft);
    tx += cw + 8.0 * PX;
    if tx < x + w {
        d.q(Tex::Solid, [0.0; 4], [tx, y + h / 2.0 - 1.0 * PY, x + w - tx, 2.0 * PY], s.well, 128.0);
    }
}

/// The scroll bar in `rect`: its thumb `shown` of `total` from `at` of the last place it scrolls to (`last`).
pub(super) fn scroll_bar(d: &mut Draw, rect @ [bx, by, bw, bh]: [f32; 4], s: &Seat, shown: usize, total: usize, at: usize, last: usize) {
    rounded(d, rect, 3.0, s.well, 128.0);
    let thumb = (bh * shown as f32 / total.max(1) as f32).min(bh).max(12.0 * PY);
    let ty = by + (bh - thumb) * at.min(last) as f32 / last.max(1) as f32;
    rounded(d, [bx, ty, bw, thumb], 3.0, s.ring, 128.0);
}

/// A tile `[x, y, w, h]`: the tile colour, `on` the selected colour with a 3 px ring inset in the ring colour.
pub(super) fn tile(d: &mut Draw, rect @ [x, y, w, h]: [f32; 4], s: &Seat, on: bool) {
    if on {
        rounded(d, rect, 10.0, s.ring, 128.0);
        rounded(d, [x + 3.0 * PX, y + 3.0 * PY, w - 6.0 * PX, h - 6.0 * PY], 7.0, s.sel, 128.0);
    } else {
        rounded(d, rect, 10.0, s.tile, 128.0);
    }
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
