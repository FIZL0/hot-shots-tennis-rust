//! B449: the v4 frame both the online lobby (`online/lobby.rs`) and the local confirm screen (`confirm.rs`) draw in,
//! the approved boards "Concept v4: Lobby" / "Ready to play?" (1280×720, the roster's navy). Top bar: the title, the
//! lobby's join code pill (Copy link, Public), the Singles / Doubles button with its □ on the right. Middle: a top-down
//! court, the net down the middle with the VS badge, each player a plate on their half (the seat-coloured header with
//! seat tag, owner and the host's X; the 3D model's well with a mod's stand-in note; the character, style · hand and one
//! status chip). Under it the scoreboard strip (Court, Umpire, Sets, Games, Mods / Handicap), then the bottom row:
//! Leave / Back, the chat box (online), the big Start with its ready count. The hints are the legend strip's alone.
//!
//! Sizes are the boards' pixels at 720 lines (`roster::PY` units down, `PX` across), all scaled by one factor so the
//! lot fits any window: extra room goes to the court. A window taller than wide turns the court: the net across, one
//! team over the other.

use super::roster::{PX, PY};
use super::{C00, C02, Draw, Menu, RANDOM_TILE, Tex, Text, UMPIRES, WHITE, Who};

/// `#rrggbb` in the menus' 0–128 colour.
pub(super) const fn hex(c: u32) -> [f32; 3] {
    [((c >> 16) & 255) as f32 * 128.0 / 255.0, ((c >> 8) & 255) as f32 * 128.0 / 255.0, (c & 255) as f32 * 128.0 / 255.0]
}

/// The boards' colours: the navy ground, the panels and their rules, the soft captions, the ‹ › arrows, the grey
/// buttons, the court's green surround and blue court, the net, the gold, Doubles' green and Singles' blue, the □'s
/// pink, the ready chip's green, the chat's words.
pub(super) const NAVY: [f32; 3] = hex(0x0e1a2e);
pub(super) const PANEL: [f32; 3] = hex(0x16253f);
pub(super) const RULE: [f32; 3] = hex(0x2b3d5e);
pub(super) const CAPTION: [f32; 3] = hex(0xa9b8d0);
/// The court and umpire pickers' lit card and dim caption (B450/B451).
pub(super) const PAPER: [f32; 3] = [125.5, 123.5, 117.0];
pub(super) const SLATE: [f32; 3] = [54.0, 55.5, 71.0];
const ARROW: [f32; 3] = hex(0x6f82a6);
pub(super) const BUTTON: [f32; 3] = hex(0x34466a);
pub(super) const GREEN: [f32; 3] = hex(0x2f7d4f);
const BLUE: [f32; 3] = hex(0x2f5f9d);
const COURT: [f32; 3] = hex(0x2f6f9f);
const NET: [f32; 3] = hex(0xe9eef5);
pub(super) const GOLD: [f32; 3] = hex(0xffd23f);
const PINK: [f32; 3] = hex(0xf58ac0);
const READY: [f32; 3] = hex(0x2fa860);
pub(super) const CHAT: [f32; 3] = hex(0xd3deef);

/// A seat's colours (1P red, 2P yellow, 3P blue, 4P green): the ring and header, the plate, the model's well, the
/// soft words.
pub(super) struct Seat {
    pub ring: [f32; 3],
    pub bg: [f32; 3],
    pub well: [f32; 3],
    pub soft: [f32; 3],
}

pub(super) fn seat(p: usize) -> Seat {
    const C: [[u32; 4]; 4] = [[0xff6b6b, 0x3a1720, 0x5e1f1f, 0xffd0d0], [0xffd23f, 0x33290f, 0x4e3c0c, 0xfff0b8], [0x5cc8ff, 0x10273d, 0x153a57, 0xcdeeff], [0x7fd99a, 0x132619, 0x183022, 0xc9f2d4]];
    let [ring, bg, well, soft] = C[p.min(3)].map(hex);
    Seat { ring, bg, well, soft }
}

/// The boards' sizes (px at 720 lines): the content's padding (top, sides, bottom) and the gap between its rows, the
/// top bar, the strip, the bottom row; the court's inset in the green; the VS column; a plate's width (doubles,
/// singles), its header, its well (doubles, singles), its body under the well; the gap between plates, the team label's
/// row.
const PAD_T: f32 = 14.0;
const PAD_B: f32 = 10.0;
const ROW_GAP: f32 = 12.0;
const BAR_H: f32 = 44.0;
const STRIP_H: f32 = 62.0;
const FOOT_H: f32 = 56.0;
const INSET_X: f32 = 26.0;
const INSET_Y: f32 = 18.0;
const VS_COL: f32 = 72.0;
const PLATE_W: [f32; 2] = [236.0, 300.0];
const HEAD_H: f32 = 30.0;
const WELL_H: [f32; 2] = [170.0, 210.0];
const BODY_H: f32 = 67.0;
const PLATE_GAP: f32 = 16.0;
const LABEL_H: f32 = 30.0;
/// The bar's parts: the title's room, the code pill's (host with Public, guest without), the mode button's.
const TITLE_W: f32 = 104.0;
const CODE_W: [f32; 2] = [254.0, 336.0];
const MODE_W: f32 = 160.0;
/// The bottom row's Back and Start, the chat box's least width.
const BACK_W: f32 = 170.0;
const START_W: f32 = 340.0;
const CHAT_MIN: f32 = 220.0;
/// The strip's cells' shares (Court, Umpire, Sets, Games, Mods / Handicap) and its least width.
const GROW: [f32; 5] = [2.2, 1.6, 1.0, 1.0, 1.3];
const STRIP_MIN: f32 = 680.0;
/// A turned court's plates may grow this much past the board's (a tall window has the room).
const GROW_MAX: f32 = 1.4;

/// What a screen puts in the frame: `n` players, the lobby's code pill (with the host's Public), the chat box.
#[derive(Clone, Copy, Debug)]
pub(super) struct Spec {
    pub n: usize,
    pub code: bool,
    pub public: bool,
    pub chat: bool,
}

/// Where the frame's parts go (640×448 units, `Grid::reach`); `k` scales the boards' px (with `PX` across and `PY`
/// down), a plate's own `s` (its parts and words).
#[derive(Clone, Debug)]
pub(super) struct Geo {
    pub k: f32,
    pub s: f32,
    /// The court turned (the net across, team 1 over team 2).
    pub turned: bool,
    /// The top bar: the title, the code pill (`code` its CODE and join code part) with Copy link and Public in it, the
    /// mode button.
    pub title: [f32; 4],
    pub pill: [f32; 4],
    pub code: [f32; 4],
    pub copy: [f32; 4],
    pub public: [f32; 4],
    pub mode: [f32; 4],
    /// The green, the court in it, the teams' labels' rows, the VS badge, a plate a player in seat order.
    pub stage: [f32; 4],
    pub court: [f32; 4],
    pub labels: [[f32; 4]; 2],
    pub vs: [f32; 4],
    pub plates: Vec<[f32; 4]>,
    /// The strip and its cells: Court, Umpire, Sets, Games, Mods / Handicap.
    pub strip: [f32; 4],
    pub cells: [[f32; 4]; 5],
    /// The bottom row: Leave / Back, the chat box (zero wide without one), Start.
    pub back: [f32; 4],
    pub chat: [f32; 4],
    pub start: [f32; 4],
}

impl Geo {
    /// `px` board pixels across, down (the frame's scale).
    pub fn x(&self, px: f32) -> f32 {
        px * self.k * PX
    }
    pub fn y(&self, px: f32) -> f32 {
        px * self.k * PY
    }
}

/// A plate's height for `per` plates a team.
fn plate_h(per: usize) -> f32 {
    3.0 + HEAD_H + 8.0 + WELL_H[(per == 1) as usize] + BODY_H + 3.0
}

/// A team's plates' width.
fn team_w(per: usize) -> f32 {
    per as f32 * PLATE_W[(per == 1) as usize] + (per - 1) as f32 * PLATE_GAP
}

/// The frame on `reach` (left, top, right, bottom) for `spec`: the court beside the plates' sides or (a tall window,
/// when that gives bigger plates) turned.
pub(super) fn layout(reach: [f32; 4], spec: Spec) -> Geo {
    let side = layout_in(reach, spec, false);
    let turned = layout_in(reach, spec, true);
    if turned.k * turned.s > side.k * side.s * 1.05 { turned } else { side }
}

fn layout_in([l, t, r, b]: [f32; 4], spec: Spec, turned: bool) -> Geo {
    let per = (spec.n / 2).max(1);
    let (pw, ph) = (team_w(per), plate_h(per));
    let rows = PAD_T + BAR_H + ROW_GAP + ROW_GAP + STRIP_H + ROW_GAP + FOOT_H + PAD_B;
    let bar_w = TITLE_W + if spec.code { CODE_W[spec.public as usize] + 14.0 } else { 0.0 } + 14.0 + MODE_W;
    let foot_w = BACK_W + START_W + 12.0 + if spec.chat { CHAT_MIN + 12.0 } else { 0.0 };
    let (stage_w, stage_h) = if turned {
        (2.0 * INSET_X + pw + 40.0, 2.0 * INSET_Y + 2.0 * (LABEL_H + ph) + VS_COL)
    } else {
        (2.0 * INSET_X + VS_COL + 2.0 * (pw + 40.0), 2.0 * INSET_Y + LABEL_H + ph)
    };
    let need_w = stage_w.max(bar_w).max(foot_w).max(STRIP_MIN);
    let (w_px, h_px) = ((r - l) / PX, (b - t) / PY);
    let k = (w_px / need_w).min(h_px / (rows + stage_h)).min(1.0);
    let x = |px: f32| px * k * PX;
    let y = |px: f32| px * k * PY;
    // the rows from the top and from the foot; the green takes what's left
    let top = t + y(PAD_T);
    let foot = b - y(PAD_B) - y(FOOT_H);
    let strip = [l, foot - y(ROW_GAP) - y(STRIP_H), r - l, y(STRIP_H)];
    let stage_y = top + y(BAR_H) + y(ROW_GAP);
    let stage = [l, stage_y, r - l, strip[1] - y(ROW_GAP) - stage_y];
    let court = [l + x(INSET_X), stage[1] + y(INSET_Y), stage[2] - 2.0 * x(INSET_X), stage[3] - 2.0 * y(INSET_Y)];
    // the bar
    let title = [l, top, x(TITLE_W), y(BAR_H)];
    let pill = if spec.code { [l + x(TITLE_W), top + y(2.0), x(CODE_W[spec.public as usize]), y(40.0)] } else { [l + x(TITLE_W), top, 0.0, 0.0] };
    let copy = [pill[0] + x(14.0 + 46.0 + 8.0 + 90.0 + 8.0), top + y(8.0), x(84.0), y(28.0)];
    let code = [pill[0], pill[1], copy[0] - x(4.0) - pill[0], pill[3]];
    let public = if spec.public { [copy[0] + copy[2] + x(8.0), copy[1], x(74.0), copy[3]] } else { [copy[0] + copy[2], copy[1], 0.0, 0.0] };
    let mode = [r - x(MODE_W), top + y(2.0), x(MODE_W), y(40.0)];
    // the plates: a team a half, centred in it, the label over them; the VS badge on the net
    let (cw, ch) = (court[2] / (k * PX), court[3] / (k * PY));
    let s = if turned {
        ((cw - 40.0) / pw).min(((ch - VS_COL) / 2.0 - LABEL_H) / ph).clamp(0.5, GROW_MAX)
    } else {
        (((cw - VS_COL) / 2.0 - 40.0) / pw).min((ch - LABEL_H) / ph).clamp(0.5, 1.0)
    };
    let (tw, th) = (pw * s * k * PX, (LABEL_H + ph * s) * k * PY);
    let halves: [[f32; 2]; 2] = if turned {
        let hh = (court[3] - y(VS_COL)) / 2.0;
        let cx = court[0] + (court[2] - tw) / 2.0;
        [[cx, court[1] + (hh - th) / 2.0], [cx, court[1] + hh + y(VS_COL) + (hh - th) / 2.0]]
    } else {
        let hw = (court[2] - x(VS_COL)) / 2.0;
        let cy = court[1] + (court[3] - th) / 2.0;
        [[court[0] + (hw - tw) / 2.0, cy], [court[0] + hw + x(VS_COL) + (hw - tw) / 2.0, cy]]
    };
    let labels = halves.map(|[hx, hy]| [hx, hy, tw, y(LABEL_H - 8.0)]);
    let plates = (0..spec.n)
        .map(|i| {
            let [hx, hy] = halves[(i / per).min(1)];
            let j = (i % per) as f32;
            [hx + j * (PLATE_W[(per == 1) as usize] + PLATE_GAP) * s * k * PX, hy + y(LABEL_H), PLATE_W[(per == 1) as usize] * s * k * PX, ph * s * k * PY]
        })
        .collect();
    let vs = [court[0] + court[2] / 2.0 - x(32.0), court[1] + court[3] / 2.0 - y(32.0), x(64.0), y(64.0)];
    // the strip's cells by their shares
    let sum: f32 = GROW.iter().sum();
    let mut at = strip[0];
    let cells = GROW.map(|g| {
        let w = strip[2] * g / sum;
        at += w;
        [at - w, strip[1], w, strip[3]]
    });
    // the bottom row
    let back = [l, foot, x(BACK_W), y(FOOT_H)];
    let start = [r - x(START_W), foot, x(START_W), y(FOOT_H)];
    let chat = if spec.chat { [back[0] + back[2] + x(12.0), foot, start[0] - x(12.0) - (back[0] + back[2] + x(12.0)), y(FOOT_H)] } else { [back[0] + back[2], foot, 0.0, 0.0] };
    Geo { k, s, turned, title, pill, code, copy, public, mode, stage, court, labels, vs, plates, strip, cells, back, chat, start }
}

/// The strip with only its first `n` cells, spread by their shares across all of it (the rest left zero wide at its
/// right end): the confirm screen leaves Handicap out until handicaps work (B482, back with B40g1).
pub(super) fn spread(g: &mut Geo, n: usize) {
    let sum: f32 = GROW[..n].iter().sum();
    let mut at = g.strip[0];
    for (cell, grow) in g.cells.iter_mut().zip(GROW.iter().enumerate().map(|(i, &w)| if i < n { w } else { 0.0 })) {
        let w = g.strip[2] * grow / sum;
        *cell = [at, g.strip[1], w, g.strip[3]];
        at += w;
    }
}

// ---------------------------------------------------------------- drawing

/// A box filled `rgb`, its corners rounded `r` units.
pub(super) fn rounded(d: &mut Draw, rect: [f32; 4], r: f32, rgb: [f32; 3], alpha: f32) {
    d.q(Tex::Solid, [0.0; 4], rect, rgb, alpha);
    d.out.last_mut().unwrap().round = [r.min(rect[2] / 2.0).min(rect[3] / 2.0).max(0.0); 4];
}

/// A box with a rim `t` units thick in `rim` round `fill` (none: the rim only), corners `r`.
pub(super) fn ringed(d: &mut Draw, [x, y, w, h]: [f32; 4], r: f32, t: f32, rim: [f32; 3], fill: Option<[f32; 3]>) {
    match fill {
        Some(f) => {
            rounded(d, [x, y, w, h], r, rim, 128.0);
            rounded(d, [x + t, y + t * PY / PX, w - 2.0 * t, h - 2.0 * t * PY / PX], r - t, f, 128.0);
        }
        None => {
            rounded(d, [x, y, w, h], r, rim, 128.0);
            d.out.last_mut().unwrap().rim = t;
        }
    }
}

/// A dashed rim `t` units thick round `[x, y, w, h]` (corners `r` left open): a computer's or open seat's plate, the
/// Picking… chip.
pub(super) fn dashed(d: &mut Draw, [x, y, w, h]: [f32; 4], r: f32, t: f32, dash: f32, rgb: [f32; 3]) {
    let gap = dash * 0.6;
    let ty = t * PY / PX;
    let mut i = r;
    while i < w - r - 0.01 {
        let l = dash.min(w - r - i);
        d.q(Tex::Solid, [0.0; 4], [x + i, y, l, ty], rgb, 128.0);
        d.q(Tex::Solid, [0.0; 4], [x + i, y + h - ty, l, ty], rgb, 128.0);
        i += dash + gap;
    }
    let ry = r * PY / PX;
    let mut i = ry;
    while i < h - ry - 0.01 {
        let l = (dash * PY / PX).min(h - ry - i);
        d.q(Tex::Solid, [0.0; 4], [x, y + i, t, l], rgb, 128.0);
        d.q(Tex::Solid, [0.0; 4], [x + w - t, y + i, t, l], rgb, 128.0);
        i += (dash + gap) * PY / PX;
    }
}

/// The focus ring: white, `t` thick, `off` out from `[x, y, w, h]` (corners `r` + `off`).
pub(super) fn focus(d: &mut Draw, [x, y, w, h]: [f32; 4], r: f32, off: f32, t: f32) {
    let oy = off * PY / PX;
    ringed(d, [x - off, y - oy, w + 2.0 * off, h + 2.0 * oy], r + off, t, WHITE, None);
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

/// The ground over the whole window: B471: the main menu's wall (`look::backdrop`), as every menu page.
pub(super) fn ground(d: &mut Draw) {
    super::look::backdrop(d);
}

/// The top bar's title, and the Singles / Doubles button with its □ (`page`: the host's □ / a tap switches).
pub(super) fn bar(d: &mut Draw, g: &Geo, title: &str, doubles: bool, page: bool, sel: bool) {
    let [tx, ty, tw, th] = g.title;
    let size = g.y(30.0);
    let _ = tw;
    d.outlined(|d| d.text(title, tx, ty + (th - size) / 2.0, size, WHITE));
    let [mx, my, mw, mh] = g.mode;
    let (words, fill) = if doubles { ("Doubles", GREEN) } else { ("Singles", BLUE) };
    ringed(d, g.mode, mh / 2.0, g.x(2.0), WHITE, Some(fill));
    if sel {
        focus(d, g.mode, mh / 2.0, g.x(3.0), g.x(3.0));
    }
    // the □ on its navy disc
    let disc = [mx + g.x(8.0), my + (mh - g.y(28.0)) / 2.0, g.x(28.0), g.y(28.0)];
    rounded(d, disc, g.x(14.0), NAVY, 128.0);
    let sq = [disc[0] + g.x(9.0), disc[1] + g.y(9.0), g.x(10.0), g.y(10.0)];
    ringed(d, sq, 0.0, g.x(2.0), PINK, Some(NAVY));
    let ws = g.y(20.0);
    let lx = disc[0] + disc[2] + g.x(10.0);
    super::left(d, words, lx, my + (mh - ws) / 2.0, ws, mx + mw - g.x(16.0) - lx, WHITE);
    if page {
        d.hit(g.mode, super::pointer::press(|k| k.page = true));
    }
}

/// The court: the green, the blue court with its lines, the net down the middle (across, turned), the teams' labels
/// and the VS badge on it.
pub(super) fn stage(d: &mut Draw, g: &Geo, labels: [&str; 2]) {
    rounded(d, g.stage, g.x(18.0), GREEN, 128.0);
    let [x, y, w, h] = g.court;
    let (t, ty) = (g.x(3.0), g.y(3.0));
    d.q(Tex::Solid, [0.0; 4], [x, y, w, h], WHITE, 128.0);
    d.q(Tex::Solid, [0.0; 4], [x + t, y + ty, w - 2.0 * t, h - 2.0 * ty], COURT, 128.0);
    // the lines: the board's along its length, across when turned
    let lines: Vec<[f32; 4]> = if g.turned {
        let (sx, sy) = (w * 0.12, h * 0.22);
        vec![[x + sx, y, t, h], [x + w - sx - t, y, t, h], [x + sx, y + sy, w - 2.0 * sx, ty], [x + sx, y + h - sy - ty, w - 2.0 * sx, ty], [x + w / 2.0 - t / 2.0, y + sy, t, h - 2.0 * sy]]
    } else {
        let (sx, sy) = (w * 0.22, h * 0.12);
        vec![[x, y + sy, w, ty], [x, y + h - sy - ty, w, ty], [x + sx, y + sy, t, h - 2.0 * sy], [x + w - sx - t, y + sy, t, h - 2.0 * sy], [x + sx, y + h / 2.0 - ty / 2.0, w - 2.0 * sx, ty]]
    };
    for l in lines {
        d.q(Tex::Solid, [0.0; 4], l, WHITE, 128.0);
    }
    let net = if g.turned { [x - g.x(12.0), y + h / 2.0 - g.y(3.0), w + g.x(24.0), g.y(6.0)] } else { [x + w / 2.0 - g.x(3.0), y - g.y(12.0), g.x(6.0), h + g.y(24.0)] };
    rounded(d, net, g.x(3.0), NET, 128.0);
    // the labels
    for (k, &[lx, ly, lw, lh]) in g.labels.iter().enumerate() {
        let size = g.y(14.0) * g.s.min(1.2);
        let words = labels[k];
        let ww = d.width(words, size) + words.len() as f32 * g.x(2.0) + g.x(24.0);
        let pill = [lx + (lw - ww) / 2.0, ly, ww, lh];
        rounded(d, pill, lh / 2.0, NAVY, 128.0);
        spaced(d, words, lx + lw / 2.0, ly + (lh - size) / 2.0, size, g.x(2.0), WHITE);
    }
    // VS
    let [vx, vy, vw, vh] = g.vs;
    ringed(d, g.vs, vw / 2.0, g.x(3.0), WHITE, Some(NAVY));
    let size = g.y(26.0);
    let at = d.out.len();
    d.centred("VS", vx + vw / 2.0, vy + (vh - size) / 2.0, size, GOLD);
    // italic: each glyph leaned
    d.out[at..].iter_mut().for_each(|q| q.angle = -0.12);
}

/// `s` centred on `cx` with `gap` between its letters (the boards' letter-spacing).
pub(super) fn spaced(d: &mut Draw, s: &str, cx: f32, y: f32, size: f32, gap: f32, rgb: [f32; 3]) {
    let w = d.width(s, size) + gap * (s.chars().count().max(1) - 1) as f32;
    let mut x = cx - w / 2.0;
    for c in s.chars() {
        let cs = c.to_string();
        d.text(&cs, x, y, size, rgb);
        x += d.width(&cs, size) + gap;
    }
}

/// A plate's status chip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Chip {
    Ready,
    Picking,
    /// Picked, not readied yet (a guest's Ready).
    Waiting,
    Com,
    Open,
}

/// What a plate shows.
pub(super) struct Plate<'a> {
    /// The seat (its colours and tag).
    pub p: usize,
    pub owner: &'a str,
    /// Its owner's ping is high (a navy dot after the owner).
    pub slow: bool,
    pub name: &'a str,
    pub sub: &'a str,
    pub chip: Chip,
    pub chip_words: &'a str,
    /// A computer's or open seat's rim is dashed.
    pub dashed: bool,
    pub remove: bool,
    pub focus: bool,
    /// The X has the focus.
    pub remove_focus: bool,
    /// The 3D preview: Random's "?" tile, hidden (a popup over it), an open seat's empty well.
    pub random: bool,
    pub hide: bool,
    pub empty: bool,
    pub mod_note: Option<String>,
}

/// A plate's X (the host's remove) in plate `rect` at scale `s` (`Geo::k * Geo::s`).
pub(super) fn remove_rect([x, y, w, _]: [f32; 4], s: f32) -> [f32; 4] {
    let (sx, sy) = (s * PX, s * PY);
    [x + w - 3.0 * sx - 8.0 * sx - 22.0 * sx, y + 3.0 * sy + 4.0 * sy, 22.0 * sx, 22.0 * sy]
}

/// Player plate `pl` in `rect` at scale `s` (`Geo::k * Geo::s`).
pub(super) fn plate(d: &mut Draw, pl: &Plate, [x, y, w, h]: [f32; 4], s: f32) {
    let c = seat(pl.p);
    let (sx, sy) = (s * PX, s * PY);
    let (t, r) = (3.0 * sx, 14.0 * sx);
    if pl.focus {
        focus(d, [x, y, w, h], r, 6.0 * sx, 3.0 * sx);
    }
    if pl.dashed {
        rounded(d, [x, y, w, h], r, c.bg, 128.0);
        dashed(d, [x, y, w, h], r, t, 10.0 * sx, c.ring);
    } else {
        ringed(d, [x, y, w, h], r, t, c.ring, Some(c.bg));
    }
    // the header: the ring's colour, the seat tag on navy, the owner, the X
    let (hx, hy, hw, hh) = (x + t, y + 3.0 * sy, w - 2.0 * t, HEAD_H * sy);
    d.q(Tex::Solid, [0.0; 4], [hx, hy, hw, hh], c.ring, 128.0);
    d.out.last_mut().unwrap().round = [r - t, r - t, 0.0, 0.0];
    let tag = format!("{}P", pl.p + 1);
    let ts = 15.0 * sy;
    let tw = d.width(&tag, ts) + 20.0 * sx;
    d.q(Tex::Solid, [0.0; 4], [hx, hy, tw, hh], NAVY, 128.0);
    d.out.last_mut().unwrap().round = [r - t, 0.0, 0.0, 0.0];
    d.text(&tag, hx + 10.0 * sx, hy + (hh - ts) / 2.0, ts, c.ring);
    let ox = hx + tw + 8.0 * sx;
    let right = if pl.remove { remove_rect([x, y, w, h], s)[0] - 8.0 * sx } else { hx + hw - 8.0 * sx };
    let os = 14.0 * sy;
    let owner = ellipsis(d, pl.owner, os, right - ox - if pl.slow { 10.0 * sx } else { 0.0 });
    d.text(&owner, ox, hy + (hh - os) / 2.0, os, NAVY);
    if pl.slow {
        // a high ping: a dot after it in the plate's colour on navy
        let dx = ox + d.width(&owner, os) + 4.0 * sx;
        rounded(d, [dx, hy + hh / 2.0 - 3.0 * sy, 6.0 * sx, 6.0 * sy], 3.0 * sx, NAVY, 128.0);
    }
    if pl.remove {
        let xr = remove_rect([x, y, w, h], s);
        ringed(d, xr, xr[2] / 2.0, 2.0 * sx, NAVY, Some(c.ring));
        let xs = 12.0 * sy;
        d.centred("X", xr[0] + xr[2] / 2.0, xr[1] + (xr[3] - xs) / 2.0, xs, NAVY);
        if pl.remove_focus {
            focus(d, xr, xr[2] / 2.0, 3.0 * sx, 3.0 * sx);
        }
    }
    // the well: the 3D model, a mod's stand-in note over its foot
    let wh = h - 6.0 * sy - hh - 8.0 * sy - BODY_H * sy;
    let well = [x + t + 8.0 * sx, hy + hh + 8.0 * sy, w - 2.0 * t - 16.0 * sx, wh];
    rounded(d, well, 10.0 * sx, c.well, 128.0);
    if pl.empty {
        let fs = 13.0 * sy;
        d.label("Open", well[0] + well[2] / 2.0, well[1] + (well[3] - fs) / 2.0, fs, well[2] - 8.0 * sx, c.soft);
    } else {
        preview(d, pl.p, pl.random, pl.hide, well);
    }
    if let Some(note) = &pl.mod_note {
        let ns = 12.0 * sy;
        let lines = super::wrap(note, ((well[2] - 28.0 * sx) / (ns * 0.5)).max(8.0) as usize);
        let n = lines.len().min(3);
        let nh = (6.0 + 14.4 * n as f32) * sy;
        let nr = [well[0] + 6.0 * sx, well[1] + well[3] - 6.0 * sy - nh, well[2] - 12.0 * sx, nh];
        rounded(d, nr, 8.0 * sx, GOLD, 128.0);
        for (k, l) in lines.iter().take(3).enumerate() {
            super::left(d, l, nr[0] + 8.0 * sx, nr[1] + 3.0 * sy + 14.4 * sy * k as f32, ns, nr[2] - 16.0 * sx, NAVY);
        }
    }
    // the body: the name, then style · hand and the chip
    let bx = x + t + 10.0 * sx;
    let bw = w - 2.0 * t - 20.0 * sx;
    let ny = well[1] + well[3] + 6.0 * sy;
    let ns = 24.0 * sy;
    let name = ellipsis(d, pl.name, ns, bw);
    d.text(&name, bx, ny, ns, WHITE);
    let ry = ny + 25.0 * sy + 4.0 * sy;
    let (cs, chh) = (13.0 * sy, 21.0 * sy);
    let cw = (d.width(pl.chip_words, cs) + 24.0 * sx).min(bw * 0.6);
    let chip_rect = [bx + bw - cw, ry, cw, chh];
    chip(d, pl.chip, pl.chip_words, chip_rect, s);
    let ss = 13.0 * sy;
    dotted(d, pl.sub, bx, ry + (chh - ss) / 2.0, ss, bw - cw - 6.0 * sx, c.soft, false);
}

/// The status chip in `rect`: Ready (green), Picking… (dashed gold), Not ready (dashed soft), COM (white), Open
/// (dashed white).
fn chip(d: &mut Draw, kind: Chip, words: &str, [x, y, w, h]: [f32; 4], s: f32) {
    let t = 2.0 * s * PX;
    let ink = match kind {
        Chip::Ready => {
            ringed(d, [x, y, w, h], h / 2.0 * PX / PY, t, WHITE, Some(READY));
            WHITE
        }
        Chip::Com => {
            rounded(d, [x, y, w, h], h, WHITE, 128.0);
            NAVY
        }
        Chip::Picking | Chip::Waiting | Chip::Open => {
            let ink = match kind {
                Chip::Picking => GOLD,
                Chip::Waiting => CAPTION,
                _ => WHITE,
            };
            dashed(d, [x, y, w, h], h / 2.0 * PX / PY, t, 5.0 * s * PX, ink);
            ink
        }
    };
    let size = 13.0 * s * PY;
    dotted(d, words, x + w / 2.0, y + (h - size) / 2.0, size, w - 12.0 * s * PX, ink, true);
}

/// The 3D preview (`previews.rs`: `Tex::Preview(i)`), 4:5 and as big as `[x, y, w, h]` allows; the Random "?" tile;
/// nothing with `hide`.
pub(super) fn preview(d: &mut Draw, i: usize, random: bool, hide: bool, [x, y, w, h]: [f32; 4]) {
    if h < 12.0 || hide {
        return;
    }
    if random {
        let fw = (w - 8.0).min((h - 8.0) * 92.0 / 68.0).min(110.0);
        d.q(C02, RANDOM_TILE, [x + (w - fw) / 2.0, y + (h - fw * 68.0 / 92.0) / 2.0, fw, fw * 68.0 / 92.0], WHITE, 128.0);
        return;
    }
    let ph = h - 4.0;
    let pw = (ph * super::previews::CARD_W).min(w - 4.0);
    let ph = pw / super::previews::CARD_W;
    d.q(Tex::Preview(i), [0.0; 4], [x + (w - pw) / 2.0, y + (h - ph) / 2.0, pw, ph], WHITE, 128.0);
}

/// A strip cell: what it shows.
pub(super) struct Cell {
    pub label: &'static str,
    pub value: String,
    /// ‹ › (a value that steps).
    pub arrows: bool,
    pub sel: bool,
    /// Its thumb: the court's picture (court value) or the umpire's face (umpire value).
    pub thumb: Thumb,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Thumb {
    None,
    Court(usize),
    Umpire(usize),
}

/// The strip: its cells with the rules between them.
pub(super) fn strip(d: &mut Draw, m: &Menu, g: &Geo, cells: &[Cell]) {
    let [x, y, w, h] = g.strip;
    let last = cells.len() - 1;
    ringed(d, [x, y, w, h], g.x(12.0), g.x(2.0), RULE, Some(PANEL));
    for (k, (c, &[cx, cy, cw, ch])) in cells.iter().zip(&g.cells).enumerate() {
        let inner = [cx + if k == 0 { g.x(2.0) } else { 0.0 }, cy + g.y(2.0), cw - if k == 0 || k == last { g.x(2.0) } else { 0.0 }, ch - g.y(4.0)];
        if c.sel {
            d.q(Tex::Solid, [0.0; 4], inner, RULE, 128.0);
            let r = if k == 0 || k == last { g.x(10.0) } else { 0.0 };
            d.out.last_mut().unwrap().round = if k == 0 { [r, 0.0, 0.0, r] } else { [0.0, r, r, 0.0] };
            ringed(d, inner, g.x(3.0), g.x(3.0), WHITE, None);
        }
        if k > 0 {
            d.q(Tex::Solid, [0.0; 4], [cx - g.x(1.0), cy + g.y(2.0), g.x(2.0), ch - g.y(4.0)], RULE, 128.0);
        }
        let mut tx = cx + g.x(14.0);
        match c.thumb {
            Thumb::Court(v) => {
                let pic = [tx, cy + (ch - g.y(40.0)) / 2.0, g.x(58.0), g.y(40.0)];
                let first = d.out.len();
                super::court_picture(d, m, v, pic);
                let rr = g.x(6.0);
                d.out[first..].iter_mut().for_each(|q| q.round = [rr; 4]);
                tx += g.x(58.0 + 10.0);
            }
            Thumb::Umpire(u) => {
                let size = g.y(40.0);
                umpire_face(d, m, u, [tx + g.x(20.0), cy + ch / 2.0], size);
                tx += g.x(40.0 + 10.0);
            }
            Thumb::None => {}
        }
        let right = cx + cw - g.x(14.0);
        let (ls, vs) = (g.y(11.0), g.y(20.0));
        let top = cy + (ch - ls - g.y(2.0) - vs) / 2.0;
        super::left(d, c.label, tx, top, ls, right - tx, CAPTION);
        let vy = top + ls + g.y(2.0);
        if c.arrows {
            let aw = d.width("<", vs);
            d.text("<", tx, vy, vs, ARROW);
            d.text(">", right - aw, vy, vs, ARROW);
            let vx = tx + aw + g.x(8.0);
            let value = ellipsis(d, &c.value, vs, right - aw - g.x(8.0) - vx);
            let vw = d.width(&value, vs);
            d.text(&value, vx + ((right - aw - g.x(8.0) - vx) - vw).max(0.0) / 2.0, vy, vs, WHITE);
        } else {
            let value = ellipsis(d, &c.value, vs, right - tx);
            d.text(&value, tx, vy, vs, WHITE);
        }
    }
}

/// Umpire value `u`'s round face centred on `[x, y]`, `size` high: a mod's own head, Random's "?", the disc's face
/// (a mod without its own: its donor's).
pub(super) fn umpire_face(d: &mut Draw, m: &Menu, u: usize, [fx, fy]: [f32; 2], size: f32) {
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
}

/// The bottom row's Leave / ‹ Back pill.
pub(super) fn back(d: &mut Draw, g: &Geo, words: &str, sel: bool) {
    let [x, y, w, h] = g.back;
    ringed(d, g.back, h / 2.0 * PX / PY, g.x(3.0), WHITE, Some(BUTTON));
    if sel {
        focus(d, g.back, h / 2.0 * PX / PY, g.x(4.0), g.x(3.0));
    }
    let size = g.y(20.0);
    d.label(words, x + w / 2.0, y + (h - size) / 2.0, size, w - g.x(24.0), WHITE);
}

/// The big Start: `big` over `small` (a ▶ after `big` with `play`).
pub(super) fn start(d: &mut Draw, g: &Geo, big: &str, small: &str, play: bool, sel: bool) {
    let [x, y, w, h] = g.start;
    ringed(d, g.start, h / 2.0 * PX / PY, g.x(4.0), WHITE, Some(GOLD));
    if sel {
        focus(d, g.start, h / 2.0 * PX / PY, g.x(4.0), g.x(3.0));
    }
    let (bs, ss) = (g.y(28.0), g.y(13.0));
    let top = y + (h - bs - ss) / 2.0;
    let tri = if play { g.x(22.0) } else { 0.0 };
    let bw = d.width(big, bs).min(w - g.x(40.0) - tri);
    let bx = x + (w - bw - tri) / 2.0;
    super::left(d, big, bx, top, bs, bw, NAVY);
    if play {
        // ▶: the font has none
        let (tx, ty, s) = (bx + bw + g.x(8.0), top + bs * 0.22, bs * 0.55);
        for c in 0..8 {
            let f = c as f32 / 8.0;
            d.q(Tex::Solid, [0.0; 4], [tx + s * 0.85 * f * PX / PY, ty + s * f / 2.0, s * 0.85 / 8.0 * PX / PY + 0.3, s * (1.0 - f)], NAVY, 128.0);
        }
    }
    dotted(d, small, x + w / 2.0, top + bs, ss, w - g.x(40.0), NAVY, true);
}

/// The chat box: the last lines heard ("who" bold in their colour, then the words).
pub(super) fn chat(d: &mut Draw, g: &Geo, lines: &[(String, String)], sel: bool) {
    let [x, y, w, h] = g.chat;
    if w <= 0.0 {
        return;
    }
    rounded(d, g.chat, g.x(12.0), PANEL, 128.0);
    if sel {
        focus(d, g.chat, g.x(12.0), g.x(4.0), g.x(3.0));
    }
    let size = g.y(14.0);
    let pitch = size * 1.25 + g.y(2.0);
    let n = lines.len().min(2);
    let top = y + (h - pitch * n as f32) / 2.0;
    for (k, (who, words)) in lines.iter().rev().take(2).rev().enumerate() {
        let ly = top + pitch * k as f32;
        let mut lx = x + g.x(16.0);
        let room = w - g.x(32.0);
        if !who.is_empty() {
            let ww = d.width(who, size).min(room * 0.5);
            super::left(d, who, lx, ly, size, ww, GOLD);
            lx += ww + g.x(6.0);
        }
        let rest = ellipsis(d, words, size, x + w - g.x(16.0) - lx);
        d.text(&rest, lx, ly, size, CHAT);
    }
}

/// A play style's short name (the boards': "Net Player · Right"), Random's "?".
pub(super) fn style_short(text: &Text, who: Who) -> &'static str {
    ["All-round", "Baseline", "Net Player", "Big Server"][style_of(text, who).min(3)]
}

/// `who`'s play style (an index into `STYLE_NAMES`).
pub(super) fn style_of(text: &Text, who: Who) -> usize {
    match who {
        Who::Disc(c) => text.style[c.min(13)],
        Who::Mod(j) => text.mods.get(j).map_or(0, |t| t.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside([x, y, w, h]: [f32; 4], [l, t, r, b]: [f32; 4]) -> bool {
        x >= l - 0.01 && y >= t - 0.01 && x + w <= r + 0.01 && y + h <= b + 0.01
    }

    fn apart(a: [f32; 4], b: [f32; 4]) -> bool {
        a[0] + a[2] <= b[0] + 0.01 || b[0] + b[2] <= a[0] + 0.01 || a[1] + a[3] <= b[1] + 0.01 || b[1] + b[3] <= a[1] + 0.01
    }

    /// Any window: the parts inside, apart, the plates on the court either side of the net, all at one scale (a
    /// part's shape doesn't change with the window's).
    #[test]
    fn fits_any_window_at_one_scale() {
        for aspect in [0.4, 9.0 / 16.0, 0.75, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 32.0 / 9.0] {
            for (n, code, public, chat) in [(4, true, true, true), (2, true, false, true), (4, false, false, false), (2, false, false, false)] {
                let reach = super::super::Grid::fit(aspect).reach();
                let g = layout(reach, Spec { n, code, public, chat });
                let mut parts = vec![g.title, g.mode, g.stage, g.strip, g.back, g.start];
                if code {
                    parts.push(g.pill);
                }
                if chat {
                    parts.push(g.chat);
                }
                for (i, a) in parts.iter().enumerate() {
                    assert!(inside(*a, reach), "{aspect} {n}: part {i} {a:?} outside {reach:?}");
                    assert!(a[2] > 10.0 && a[3] > 10.0, "{aspect} {n}: part {i} {a:?}");
                    for b in &parts[i + 1..] {
                        if !(code && (*a == g.title || *b == g.title)) {
                            assert!(apart(*a, *b), "{aspect} {n}: {a:?} over {b:?}");
                        }
                    }
                }
                assert!(apart(g.title, g.mode) && apart(g.pill, g.mode));
                for (i, p) in g.plates.iter().enumerate() {
                    assert!(inside(*p, [g.court[0], g.court[1], g.court[0] + g.court[2], g.court[1] + g.court[3]]), "{aspect} {n}: plate {p:?}");
                    assert!(apart(*p, g.vs), "{aspect} {n}: plate {p:?} over VS");
                    for q in &g.plates[i + 1..] {
                        assert!(apart(*p, *q));
                    }
                    // the board's shape at any scale
                    let per = (n / 2).max(1);
                    let want = PLATE_W[(per == 1) as usize] / plate_h(per) * PX / PY;
                    assert!((p[2] / p[3] - want).abs() < 1e-3, "{aspect} {n}: {p:?}");
                }
                // team 1 left of (over) the net, team 2 right of (under) it
                let first = g.plates[0];
                let last = g.plates[n - 1];
                if g.turned {
                    assert!(first[1] + first[3] <= g.vs[1] && g.vs[1] + g.vs[3] <= last[1]);
                } else {
                    assert!(first[0] + first[2] <= g.vs[0] && g.vs[0] + g.vs[2] <= last[0]);
                }
                assert!(g.k * g.s > 0.6, "{aspect} {n}: plates at {}", g.k * g.s);
            }
        }
        // 16:9 is the board's own: scale 1, the plates its size
        let g = layout(super::super::Grid::fit(16.0 / 9.0).reach(), Spec { n: 4, code: true, public: true, chat: true });
        assert!(!g.turned && g.k > 0.99 && g.s > 0.99, "{} {}", g.k, g.s);
        // a tall window turns the court
        let g = layout(super::super::Grid::fit(9.0 / 16.0).reach(), Spec { n: 4, code: true, public: true, chat: true });
        assert!(g.turned);
    }
}
