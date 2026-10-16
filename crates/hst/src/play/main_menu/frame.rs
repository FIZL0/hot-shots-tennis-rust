//! B407: the v3 frame both the online lobby (B396, `online/lobby.rs`) and the local confirm screen (B397,
//! `confirm.rs`) draw in: the board's colours, the geometry (the teams either side of the VS disc, the side panel's
//! parts) and a player card's parts (the status chip, the 3D preview, the costume / hand / style rows over the pink
//! stats card, the stand-in note) plus the dotted words, the settings pills and the court card. Each screen keeps
//! what is its own (the lobby's seats, YOU, ✕ and invite box; the confirm screen's umpire card and picked pill).

use super::{C02, Draw, INK, RANDOM_TILE, STYLE_NAMES, Tex, Text, WHITE, Who};

/// Seat colours (1P red, 2P yellow, 3P blue, 4P green), the board's: ring, card, well, soft words.
pub(super) const RING: [[f32; 3]; 4] = [[127.5, 53.5, 53.5], [127.5, 105.0, 31.5], [46.0, 100.0, 127.5], [63.5, 108.5, 77.0]];
pub(super) const CARD: [[f32; 3]; 4] = [[37.0, 14.0, 14.0], [30.5, 24.0, 5.5], [9.0, 22.5, 34.0], [10.5, 20.5, 14.5]];
pub(super) const WELL: [[f32; 3]; 4] = [[47.0, 15.5, 15.5], [39.0, 30.0, 6.0], [10.5, 29.0, 43.5], [12.0, 24.0, 17.0]];
pub(super) const SOFT: [[f32; 3]; 4] = [[127.5, 104.0, 104.0], [127.5, 120.0, 92.0], [102.5, 119.0, 127.5], [100.5, 121.0, 106.0]];
/// The play style's gold, the stats card's pink and its rules, each grade's colour (F, E, D, C, B, A: grade 0..5).
pub(super) const STYLE_GOLD: [f32; 3] = [127.5, 105.0, 31.5];
const PINK: [f32; 3] = [122.5, 69.0, 96.0];
const RULE: [f32; 3] = [127.5, 97.0, 111.5];
const GRADE: [[f32; 3]; 6] = [[67.0, 71.0, 75.0], [78.0, 27.0, 90.5], [14.0, 63.0, 107.0], [23.5, 79.0, 34.0], [107.0, 25.5, 54.0], [120.0, 70.0, 0.0]];
const STATS: [&str; 5] = ["Serve", "Stroke", "Volley", "Impact", "Footwork"];
/// The board's navy (VS, words on light boxes), the soft blue-grey over the background (B448: was a dark green over
/// the grid paper), the grey pills, the ready chip's green,
/// the light boxes' cream and their soft caption; a chosen pill's gold fill.
pub(super) const NAVY: [f32; 3] = [14.5, 21.5, 34.5];
pub(super) const LEAF: [f32; 3] = [84.3, 90.4, 100.4];
const LEAF_RULE: [f32; 3] = [21.0, 29.0, 44.0];
pub(super) const SLATE: [f32; 3] = [54.0, 55.5, 71.0];
const READY: [f32; 3] = [23.5, 84.0, 48.0];
pub(super) const PAPER: [f32; 3] = [125.5, 123.5, 117.0];
pub(super) const CAPTION: [f32; 3] = [53.5, 55.0, 71.5];
pub(super) const SEL_FILL: [f32; 3] = [128.0, 108.0, 37.0];
/// The board at 1280×720 is 0.6 of it in 640×448 units: the side panel's width (300), the bottom bar's height (64),
/// the VS disc (56), the gap between the cards and the parts.
pub(super) const SIDE_W: f32 = 180.0;
pub(super) const BAR_H: f32 = 38.0;
pub(super) const VS: f32 = 34.0;
pub(super) const GAP: f32 = 7.0;
/// A card's widest (a very wide window: the teams keep to the middle), the team label's row.
pub(super) const CARD_MAX_W: f32 = 210.0;
pub(super) const LABEL_H: f32 = 14.0;
/// The side panel's pills: their height and the gap between them.
const PILL_H: f32 = 28.0;
const PILL_GAP: f32 = 4.0;

/// The cards of `n` players in the teams' room `[x, y, w, h]`: two teams either side of the VS disc, a label row over
/// each in doubles; the cards no wider than `CARD_MAX_W`, the lot centred.
pub(super) fn teams(n: usize, [x, y, w, h]: [f32; 4]) -> (Vec<[f32; 4]>, Vec<[f32; 4]>, [f32; 4]) {
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

/// The portrait layout's teams' height for `n` players across `w` in a `body` high with a `side_h` panel under them:
/// the cards no taller than they look right.
pub(super) fn portrait_teams_h(n: usize, w: f32, body: f32, side_h: f32) -> f32 {
    let per = (n / 2).max(1) as f32;
    let cw = ((w - VS - 2.0 * GAP - 2.0 * (per - 1.0) * GAP) / (2.0 * per)).min(CARD_MAX_W);
    (body - side_h - 10.0).min(cw * 2.7 + LABEL_H).max(200.0)
}

/// The side panel's parts in `[x, y, w, h]`: a column (beside the teams) with a `court_h` court card on top, or a wide
/// strip (under them in a portrait window) with the court card left and the pills right; `N` pills two by two (an odd
/// last one across), then the last part (at least `min_h` high) under them.
pub(super) fn side_parts<const N: usize>([x, y, w, h]: [f32; 4], wide: bool, court_h: f32, min_h: f32) -> ([f32; 4], [[f32; 4]; N], [f32; 4]) {
    let (court, px, py, pw) = if wide {
        let cw = (w - GAP) / 2.0;
        ([x, y, cw, h], x + cw + GAP, y, cw)
    } else {
        ([x, y, w, court_h], x, y + court_h + 6.0, w)
    };
    let half = (pw - PILL_GAP) / 2.0;
    let row = |k: usize| py + (PILL_H + PILL_GAP) * k as f32;
    let pills = std::array::from_fn(|k| match k {
        _ if k + 1 == N && N % 2 == 1 => [px, row(k / 2), pw, PILL_H],
        _ => [px + (k % 2) as f32 * (half + PILL_GAP), row(k / 2), half, PILL_H],
    });
    let top = row(N.div_ceil(2)) + 2.0;
    (court, pills, [px, top, pw, (y + h - top).max(min_h)])
}

/// `s` from `x` at `size`, a "·" drawn as a dot (the font has none), squeezed to fit `max`; `centre` centres it on `x`.
#[allow(clippy::too_many_arguments)]
pub(super) fn dotted(d: &mut Draw, s: &str, x: f32, y: f32, size: f32, max: f32, rgb: [f32; 3], centre: bool) {
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

/// B448: `d.frame(title, wall)` with the character select's navy for the wall (no grid paper).
pub(super) fn navy_frame(d: &mut Draw, title: usize, wall: usize) {
    let n0 = d.out.len();
    d.frame(title, wall);
    if let Some(q) = d.out.get_mut(n0) {
        (q.tex, q.tile, q.rgb) = (Tex::Solid, false, super::roster::hex(super::roster::BG));
    }
}

/// The team labels ("TEAM 1" and a rule along the row).
pub(super) fn team_labels(d: &mut Draw, labels: &[[f32; 4]]) {
    for (k, &[x, y, w, _]) in labels.iter().enumerate() {
        let words = format!("TEAM {}", k + 1);
        let tw = d.width(&words, 10.0);
        d.text(&words, x + 1.0, y + 1.0, 10.0, LEAF);
        d.q(Tex::Solid, [0.0; 4], [x + tw + 6.0, y + 6.0, (w - tw - 6.0).max(0.0), 1.5], LEAF_RULE, 128.0);
    }
}

/// The VS disc.
pub(super) fn vs_disc(d: &mut Draw, [vx, vy, vw, _]: [f32; 4]) {
    d.pill_at([vx, vy, vw, vw], [INK, WHITE, NAVY], 128.0);
    d.centred("VS", vx + vw / 2.0, vy + vw / 2.0 - 8.0, 16.0, [127.5, 77.5, 30.5]);
}

/// The court card: court value `c`'s picture, COURT over `name`, "OK: change" with `change`; gold-rimmed with `sel`.
pub(super) fn court_card(d: &mut Draw, m: &super::Menu, c: usize, name: &str, [x, y, w, h]: [f32; 4], sel: bool, change: bool) {
    d.box3([x, y, w, h], 12.0, [Some(INK), Some(if sel { super::GOLD } else { WHITE }), Some(PAPER)], 128.0);
    let ph = h - 34.0;
    let pic = [x + 3.0, y + 3.0, w - 6.0, ph];
    let first = d.out.len();
    super::court_picture(d, m, c, pic);
    let rr = 12.0 * 11.0 / 16.0 - (3.0 - 12.0 * 4.0 / 16.0);
    d.out[first..].iter_mut().filter(|q| q.dst == pic).for_each(|q| q.round = [rr, rr, 0.0, 0.0]);
    let ny = y + 3.0 + ph + 3.0;
    d.text("COURT", x + 9.0, ny, 7.0, CAPTION);
    let change = change.then_some("OK: change");
    let cw = change.map_or(0.0, |c| d.width(c, 8.0) + 6.0);
    super::left(d, name, x + 9.0, ny + 9.0, 15.0, w - 18.0 - cw, NAVY);
    if let Some(c) = change {
        d.text(c, x + w - 9.0 - d.width(c, 8.0), ny + 13.0, 8.0, CAPTION);
    }
}

/// A settings pill: `cap` small over `value`, ‹ › with `arrows`; gold while `sel`.
pub(super) fn setting_pill(d: &mut Draw, [x, y, w, h]: [f32; 4], cap: &str, value: &str, sel: bool, arrows: bool) {
    d.pill_at([x, y, w, h], [INK, if sel { super::GOLD } else { WHITE }, if sel { SEL_FILL } else { SLATE }], 128.0);
    let (cap_rgb, rgb) = if sel { (INK, INK) } else { ([108.0, 109.0, 117.0], WHITE) };
    d.text(cap, x + 12.0, y + 3.5, 6.5, cap_rgb);
    if arrows {
        let a = if sel { INK } else { [100.5, 101.5, 112.0] };
        d.text("<", x + 9.0, y + 11.0, 12.0, a);
        d.text(">", x + w - 9.0 - d.width(">", 12.0), y + 11.0, 12.0, a);
    }
    d.label(value, x + w / 2.0, y + 11.0, 12.0, w - 34.0, rgb);
}

/// A status chip's look: Ready ✓ (green), the computer's COM · Lv N (white), Picking… (dashed gold), Not ready (dashed
/// in the seat's soft colour), Open (dashed white).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Chip {
    Ready,
    Com,
    Picking,
    Waiting([f32; 3]),
    Open,
}

/// The status chip `words` ending at `right` on the header row at `y`, at most `max` wide; its left edge.
pub(super) fn chip(d: &mut Draw, words: &str, kind: Chip, right: f32, y: f32, max: f32) -> f32 {
    let size = 8.5;
    let tick = kind == Chip::Ready;
    let tw = d.width(words, size) + if tick { 10.0 } else { 0.0 };
    let w = (tw + 12.0).min(max);
    let (x, h) = (right - w, 14.0);
    let (fill, ink, ring, dashed) = match kind {
        Chip::Com => (Some(WHITE), NAVY, WHITE, false),
        Chip::Ready => (Some(READY), WHITE, WHITE, false),
        Chip::Picking => (None, STYLE_GOLD, STYLE_GOLD, true),
        Chip::Waiting(soft) => (None, soft, soft, true),
        Chip::Open => (None, WHITE, WHITE, true),
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

/// The seat tag ("1P" on the seat's colour) at the card's inner left `ix`, header row `y`; its width.
pub(super) fn seat_tag(d: &mut Draw, i: usize, ix: f32, y: f32) -> f32 {
    let tag = format!("{}P", i + 1);
    let tag_w = d.width(&tag, 10.0) + 8.0;
    d.box3([ix, y, tag_w, 14.0], 4.0, [None, None, Some(RING[i.min(3)])], 128.0);
    d.text(&tag, ix + 4.0, y + 2.0, 10.0, NAVY);
    tag_w
}

/// A card's rows inside `[x, y, w, h]`, from the foot up: the stats card, the style, the chips, the name's two lines;
/// the preview's well takes the rest.
pub(super) struct Rows {
    pub ix: f32,
    pub iw: f32,
    pub well: [f32; 4],
    pub name_y: f32,
    pub sub_y: f32,
    pub chips_y: f32,
    pub style_y: f32,
    pub stats: [f32; 4],
}

/// A stats row's height.
const STATS_ROW: f32 = 11.5;

pub(super) fn rows([x, y, w, h]: [f32; 4]) -> Rows {
    let (ix, iw) = (x + 6.0, w - 12.0);
    let foot = y + h - 6.0;
    let stats_h = 5.0 * STATS_ROW + 4.0;
    let stats_y = foot - stats_h;
    let style_y = stats_y - 15.0;
    let chips_y = style_y - 15.0;
    let sub_y = chips_y - 12.0;
    let name_y = sub_y - 17.0;
    let well = [ix, y + 25.0, iw, name_y - 3.0 - (y + 25.0)];
    Rows { ix, iw, well, name_y, sub_y, chips_y, style_y, stats: [ix, stats_y, iw, stats_h] }
}

/// The stand-in note over the preview's foot (online: `name` is a mod this machine lacks).
pub(super) fn stand_in(d: &mut Draw, name: &str, r: &Rows) {
    let (ix, iw, well) = (r.ix, r.iw, r.well);
    let note = format!("{name}: you don't have it, so you'll see its stand-in");
    let lines = super::wrap(&note, ((iw - 10.0) / 4.4).max(8.0) as usize);
    let nh = 5.0 + 9.5 * lines.len().min(3) as f32;
    let ny = well[1] + well[3] - 4.0 - nh;
    d.box3([ix + 4.0, ny, iw - 8.0, nh], 5.0, [None, None, Some(STYLE_GOLD)], 128.0);
    for (k, l) in lines.iter().take(3).enumerate() {
        super::left(d, l, ix + 8.0, ny + 3.0 + 9.5 * k as f32, 8.0, iw - 16.0, NAVY);
    }
}

/// The card's foot for seat colour `p`: the costume (the shirt) and hand chips, the play style over the pink stats
/// card; "?"s for Random.
#[allow(clippy::too_many_arguments)]
pub(super) fn card_foot(d: &mut Draw, text: &Text, p: usize, r: &Rows, who: Who, random: bool, costume: usize, hand: f32) {
    let (ix, iw) = (r.ix, r.iw);
    let words = [
        if random { "Costume ?".to_string() } else { format!("Costume {}", costume + 1) },
        if random { "?".into() } else if hand < 0.0 { "Left".into() } else { "Right".into() },
    ];
    let cw = (iw - 4.0) / 2.0;
    for (k, words) in words.iter().enumerate() {
        let px = ix + (cw + 4.0) * k as f32;
        d.pill_at([px, r.chips_y, cw, 12.0], [WELL[p]; 3], 128.0);
        let lx = if k == 0 && cw > 46.0 {
            super::shirt(d, [px + 5.0, r.chips_y + 2.5, 8.0, 7.0], RING[p], WELL[p]);
            px + 15.0
        } else {
            px + 4.0
        };
        d.label(words, (lx + px + cw - 4.0) / 2.0, r.chips_y + 2.0, 8.5, px + cw - 4.0 - lx, WHITE);
    }
    d.box3([ix, r.style_y, iw, 12.0], 3.0, [None, None, Some(WELL[p])], 128.0);
    let style = if random { "?" } else { STYLE_NAMES[style_of(text, who).min(3)] };
    d.label(style, ix + iw / 2.0, r.style_y + 2.0, 9.0, iw - 8.0, STYLE_GOLD);
    stats(d, text, who, random, r.stats, STATS_ROW);
}

/// `who`'s play style (an index into `STYLE_NAMES`).
pub(super) fn style_of(text: &Text, who: Who) -> usize {
    match who {
        Who::Disc(c) => text.style[c.min(13)],
        Who::Mod(j) => text.mods.get(j).map_or(0, |t| t.0),
    }
}

/// The 3D preview (`previews.rs`: `Tex::Preview(i)`), 4:5 and as big as `[x, y, w, h]` allows, on the seat's well;
/// the Random card's "?" tile; just the well with `hide`.
pub(super) fn preview(d: &mut Draw, i: usize, random: bool, hide: bool, [x, y, w, h]: [f32; 4], well: [f32; 3]) {
    if h < 12.0 {
        return;
    }
    d.box3([x, y, w, h], 8.0, [None, None, Some(well)], 128.0);
    if random {
        let fw = (w - 8.0).min((h - 8.0) * 92.0 / 68.0);
        d.q(C02, RANDOM_TILE, [x + (w - fw) / 2.0, y + (h - fw * 68.0 / 92.0) / 2.0, fw, fw * 68.0 / 92.0], WHITE, 128.0);
        return;
    }
    if hide {
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
    use super::*;

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
    fn side_pills_two_by_two_an_odd_one_across() {
        let (court, four, last) = side_parts::<4>([0.0, 0.0, 180.0, 400.0], false, 116.0, 50.0);
        assert_eq!(court, [0.0, 0.0, 180.0, 116.0]);
        assert_eq!(four[1][0], four[0][0] + four[0][2] + PILL_GAP);
        assert_eq!(four[2][1], four[0][1] + PILL_H + PILL_GAP);
        assert_eq!(last[1], four[3][1] + PILL_H + PILL_GAP + 2.0);
        let (_, five, last) = side_parts::<5>([0.0, 0.0, 180.0, 400.0], false, 88.0, 56.0);
        assert_eq!(five[4][2], 180.0);
        assert_eq!(last[1], five[4][1] + PILL_H + PILL_GAP + 2.0);
    }
}
