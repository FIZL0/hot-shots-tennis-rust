//! M124: the controller assignment (`Screen::Assign`, from the menu and the pause menu's "Controllers") as the match
//! roster: one card per seat in the character select's style (`roster.rs`: the seat's band, its character's face,
//! name, hand and costume, the play style and stats), with what drives it. Characters can't be changed here; the
//! devices move between the seats' characters. Local, 1P focuses a card (the arrows, a tap), picks up its device (✕, a
//! tap), carries it to the next seats (←→, the card's arrows; it swaps with theirs) and puts it down (✕), takes it off (□ / Tab, the ✕ on the band; 1P's own too) or adds a computer player;
//! the other devices join a free seat with ✕ and leave with ○; the keyboard (the mouse its pointer) takes the free seat the
//! focus or the mouse is on, or 1P's from a pad. The focus: a white ring and the menus' pointing hand; no Start button
//! (the key legend has it). The cards are at most the lobby's compact card's size, centred. Online (the pause over an online match,
//! [`Menu::net_pause`]) only this machine's own seats can take or swap its pads: the other machines' and the
//! computer's cards are fixed, and nobody is taken off. Over a match, ○ after a change asks Keep / Undo.
//!
//! The cards share the view across (two rows of two in a view taller than it is wide); a card wider than tall puts the
//! stats beside the face instead of under it.

use bevy::prelude::*;

use super::super::controls::Bindings;
use super::roster::{BG, PX, PY, Seat, band, dashed, ellipsis, hex, ringed, rounded, seat, stats};
use super::{C02, CHAR_NAMES, Dev, Draw, GOLD, Menu, Press, RANDOM_TILE, STYLE_NAMES, Text, WHITE, Who, dev_name, grades_of, legend, look, pointer, tile_face};

/// The top bar's height, the margins round the cards and the gap between them, a card's padding (mockup px).
const TOP_BAR: f32 = 60.0;
const MARGIN: f32 = 64.0;
/// Room between the cards for the pointing hand on the focused one's left.
const GAP: f32 = 64.0;
const PAD: f32 = 10.0;
const BAND: f32 = 50.0;
const FOOT: f32 = 38.0;
/// The cards stop this far over the view's bottom (the room's 432 on 4:3), over the description bar pinned to it.
const OVER_BAR: f32 = 34.0;
/// The face well's tallest (mockup px): a portrait, not the card; and the name lines under it (screen units).
const FACE: f32 = 96.0;
const FIGURE_LINES: f32 = 46.0 * PY;
/// A card's largest (mockup px): the lobby's compact card's width, as tall as the band, face, stats and foot need.
const CARD_W: f32 = 300.0;
const CARD_H: f32 = 460.0;

/// The cards' boxes for `n` seats on `view` (left, top, right, bottom): one row, or two rows of two when four share
/// a view taller than wide.
pub(super) fn cards(view: [f32; 4], n: usize) -> Vec<[f32; 4]> {
    let [l, t, r, b] = view;
    let (top, bottom) = (t + TOP_BAR * PY, b - OVER_BAR);
    let (w, h) = (r - l - 2.0 * MARGIN * PX, bottom - top);
    // on screen a unit across is 14/15 of one down
    let rows = if n == 4 && w * 14.0 / 15.0 < h { 2 } else { 1 };
    let per = n.div_ceil(rows).max(1);
    // the lobby's compact card's size at most (`roster::COMPACT_W`), the group centred in what's left
    let cw = ((w - GAP * PX * (per - 1) as f32) / per as f32).min(CARD_W * PX);
    let ch = ((h - GAP * PY * (rows - 1) as f32) / rows as f32).min(CARD_H * PY);
    let x0 = l + MARGIN * PX + (w - cw * per as f32 - GAP * PX * (per - 1) as f32) / 2.0;
    let y0 = top + (h - ch * rows as f32 - GAP * PY * (rows - 1) as f32) / 2.0;
    (0..n).map(|p| [x0 + (cw + GAP * PX) * (p % per) as f32, y0 + (ch + GAP * PY) * (p / per) as f32, cw, ch]).collect()
}

/// What a seat is, for its card: someone on this machine (a device), nobody yet, the computer, another machine's player.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Kind {
    Device(Dev),
    /// Nobody, a device can take it (`join`: an unseated device here can).
    Free,
    Com,
    Online,
}

impl Menu {
    /// The cards in a row: all of them, or two of four on a view taller than wide.
    pub(super) fn assign_per(&self) -> usize {
        let n = self.players();
        let c = cards(self.grid.view, n);
        if c[0][1] != c[n - 1][1] { n / 2 } else { n }
    }

    /// M124: seat `p`'s kind on the assignment.
    pub(super) fn assign_kind(&self, p: usize) -> Kind {
        match self.seats[p] {
            Some(d @ (Dev::Keys | Dev::Pad(_))) => Kind::Device(d),
            _ if self.net_pause.is_some_and(|r| r[p]) => Kind::Online,
            _ if self.bots[p] => Kind::Com,
            // B216: on the menu an open seat fills at Start, as a computer player; online only this machine's
            _ if !self.paused && self.online() && p >= self.local_max() => Kind::Online,
            _ => Kind::Free,
        }
    }
}

/// The assignment screen.
pub(super) fn layout(d: &mut Draw, m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity]) {
    let [l, t, r, _] = m.grid.view;
    super::look::backdrop(d);
    let size = 30.0 * PY;
    let title = "Controllers";
    d.outlined(|d| d.text(title, l + 32.0, t + 16.0 * PY, size, WHITE));
    d.hit([l, t, d.width(title, size) + 40.0, TOP_BAR * PY], pointer::press(|k| k.back = true));
    // no Start / Done button: the key legend carries Start
    let (bx, by) = (r - MARGIN * PX, t + 16.0 * PY);
    let what = if m.net_pause.is_some() {
        "Only your own controllers can move".to_string()
    } else {
        format!("{} - characters stay on their seats", if m.doubles { "Doubles" } else { "Singles" })
    };
    let ws = 14.0 * PY;
    let room = bx - 12.0 * PX - (l + 40.0 + d.width(title, size));
    let ww = d.width(&what, ws).min(room);
    if ww > 20.0 {
        d.outlined(|d| d.label(&what, bx - 12.0 * PX - ww / 2.0, by + 8.0 * PY, ws, ww, WHITE));
    }
    let n = m.players();
    let focus = m.sel.min(n - 1);
    let pending = m.lead_pending().flatten();
    for (p, card) in cards(m.grid.view, n).into_iter().enumerate() {
        let start = d.out.len();
        glow(d, card, m.flash[p]);
        seat_card(d, m, text, pads, card, p, p == focus, pending == Some(p));
        bubble(d, m, pads, card, p);
        // P21a: the card shakes while its pad's right stick moves (its hit rects stay put); a held one is lifted
        let lift = if m.held && p == focus { 10.0 * PY } else { 0.0 };
        d.out[start..].iter_mut().for_each(|q| {
            q.dst[0] += m.shake[p];
            q.dst[1] -= lift;
        });
    }
    // B437: 1P's own seat change is pending until they leave, spelled out in the help
    let pending = match m.lead_pending() {
        Some(Some(p)) => format!("Pending: you move to {}P when you leave this screen; until then you keep 1P's controls here. ", p + 1),
        Some(None) => "Pending: you give up your seat when you leave this screen; until then you keep 1P's controls here. ".to_string(),
        None => String::new(),
    };
    // 1P's buttons in its own device's names and bindings; the others' devices vary, so theirs go by what they do
    let dev = match m.seats[0] {
        Some(Dev::Pad(e)) => Some(e),
        _ => None,
    };
    let b = |a: legend::Act| legend::button_names(dev, a, bind).join(" / ");
    let (ok, back, page, card) = (b(legend::Act::Ok), b(legend::Act::Back), b(legend::Act::Page), b(legend::Act::Card));
    let start = b(legend::Act::Name);
    let help = if m.held {
        format!("1P: Left/Right carries this controller to the next seat (it swaps with that seat's), {ok} puts it down.")
    } else if m.net_pause.is_some() {
        format!("1P: the arrows pick one of your seats, {ok} picks up its controller to move it, {start} keeps the change, {back} asks to keep or undo it. Another of your controllers: its OK button takes your seat that has none.")
    } else if m.paused {
        format!("1P: the arrows pick a seat, {ok} picks up its controller to move it, {page} takes a controller off its seat, {start} keeps the change, {back} asks to keep or undo it. Others: OK takes a free seat, Left/Right changes seat, Back or its red X leaves. A controller's right stick shakes its seat, any of its buttons flashes it.")
    } else {
        format!("1P: the arrows pick a seat, {ok} picks up its controller to move it, {page} takes it off, {card} adds or removes a computer player, {start} goes on (empty seats are filled then). Others: OK joins, Left/Right changes seat, Back or its red X leaves. A controller's right stick shakes its seat, any of its buttons flashes it.")
    };
    d.info(&format!("{pending}{help}"));
    keep_layout(d, m);
}

/// Over a match, ○ after a change: Keep / Undo, in the newer-version popup's style (`home::stale_layout`).
fn keep_layout(d: &mut Draw, m: &Menu) {
    use super::super::menu::{CHOSEN, PANEL, UNCHOSEN};
    use super::{Anchor, INK, Tex};
    let Some(a) = m.keep else { return };
    // a tap off the popup is ○ (stay on the screen)
    d.popup(Some(pointer::Act::Row(a, Press { back: true, ..default() })));
    d.anchor = Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 80.0);
    d.anchor = Anchor::Centre;
    let top = 226.0;
    d.box3([160.0, top - 56.0, 320.0, 56.0 + 72.0 + 16.0], 16.0, [Some(INK), Some(WHITE), Some(PANEL)], 128.0);
    d.outlined(|d| d.label("Keep the controller changes?", 320.0, top - 40.0, 22.0, 288.0, WHITE));
    for (k, word) in ["Keep", "Undo"].into_iter().enumerate() {
        let y = top + 40.0 * k as f32;
        let on = k == a;
        d.pill_at([176.0, y, 288.0, 32.0], [INK, WHITE, if on { CHOSEN } else { UNCHOSEN }], 128.0);
        d.hit([176.0, y, 288.0, 32.0], pointer::Act::Row(k, pointer::OK));
        d.label(word, 320.0, y + 6.0, 20.0, 160.0, if on { WHITE } else { UNCHOSEN.map(|c| c * 1.4) });
    }
    d.said = "Keep the controller changes?".into();
}

/// Seat `p`'s card in `card`: `on` 1P's focus, `yours` where 1P's driver goes when it leaves (B437).
#[allow(clippy::too_many_arguments)]
fn seat_card(d: &mut Draw, m: &Menu, text: &Text, pads: &[Entity], card: [f32; 4], p: usize, on: bool, yours: bool) {
    let s = seat(p);
    let kind = m.assign_kind(p);
    let [x, y, w, h] = card;
    let human = matches!(kind, Kind::Device(_) | Kind::Online);
    if human {
        ringed(d, card, 14.0, look::RIM, hex(look::INK), s.bg);
    } else {
        rounded(d, card, 14.0, s.bg, 128.0);
        dashed(d, card, 14.0, look::RIM, hex(look::INK), s.bg);
    }
    // a tap focuses it and picks up (or puts down) its controller
    let pick = if m.held { on } else { m.pickable(p) || m.open(p) && m.lead().is_some_and(|l| !m.seats.contains(&Some(l))) };
    d.hit(card, pointer::Act::Row(p, if pick { pointer::OK } else { Press::default() }));
    let pl = m.players[p];
    let picked = !pl.random || pl.ready;
    let who = m.pick(p).0;
    let name = match who {
        _ if !picked => "Not picked yet".to_string(),
        Who::Mod(i) => m.mods.get(i).map_or("Custom", |md| md.name.as_str()).to_string(),
        Who::Disc(c) => m.remote_mod_name(p).unwrap_or(CHAR_NAMES[c]).to_string(),
    };
    let device = match kind {
        Kind::Device(dv) => dev_name(Some(dv), pads),
        Kind::Free if m.paused => "No controller".into(),
        Kind::Free => "Open".into(),
        Kind::Com => "COM".into(),
        Kind::Online => "Online".into(),
    };
    let side = if p < (m.players() / 2).max(1) { "Near" } else { "Far" };
    band(d, &s, [x, y, w, BAND * PY], human, p, m.player_name(p).as_deref(), &name, &device, side, None, m.droppable(p));
    if m.droppable(p) {
        super::drop_button(d, p, x + w - 30.0, y - 6.0);
    }

    let (ix, iw) = (x + PAD * PX, w - 2.0 * PAD * PX);
    let top = y + (BAND + PAD) * PY;
    let foot_y = y + h - (PAD + FOOT) * PY;
    let body = [ix, top, iw, foot_y - 8.0 * PY - top];
    // wider than tall (on screen): the stats beside the face, else under it
    if body[2] * 14.0 / 15.0 > body[3] * 1.1 {
        let half = (body[2] - 8.0 * PX) / 2.0;
        figure(d, m, text, &s, [ix, top, half, body[3]], p, picked, who);
        style_stats(d, text, [ix + half + 8.0 * PX, top, half, body[3]], picked, who);
    } else {
        let fh = (body[3] * 0.5).max(body[3] - 190.0 * PY).min(FACE * PY + FIGURE_LINES);
        figure(d, m, text, &s, [ix, top, iw, fh], p, picked, who);
        style_stats(d, text, [ix, top + fh + 8.0 * PY, iw, body[3] - fh - 8.0 * PY], picked, who);
    }
    foot(d, m, pads, &s, [ix, foot_y, iw, FOOT * PY], p, kind, on, yours);
    // 1P's focus: the lobby's ring round the card (gold while its controller is held) and the menus' pointing hand
    if on {
        let rim = if m.held { GOLD } else { WHITE };
        super::frame::ringed(d, [x - 4.0 * PX, y - 4.0 * PY, w + 8.0 * PX, h + 8.0 * PY], 18.0 * PX, 3.0 * PX, rim, None);
        d.q(super::Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x - 52.0, y + (BAND * PY - 32.0) / 2.0, 64.0, 32.0], WHITE, 128.0);
    }
}

/// The face well (the seat's number faint behind the character's face), the character's name, hand and costume under it.
#[allow(clippy::too_many_arguments)]
fn figure(d: &mut Draw, m: &Menu, text: &Text, s: &Seat, [x, y, w, h]: [f32; 4], p: usize, picked: bool, who: Who) {
    let wh = (h - FIGURE_LINES).clamp(30.0 * PY, FACE * PY);
    rounded(d, [x, y, w, wh], 10.0, s.tile, 128.0);
    let num = format!("{}P", p + 1);
    let big = (wh * 0.78).min(w * 0.9 * 32.0 / d.width(&num, 32.0));
    let start = d.out.len();
    d.centred(&num, x + w / 2.0, y + (wh - big) / 2.0, big, s.ring);
    d.out[start..].iter_mut().for_each(|q| q.alpha = 26.0);
    // a square face (on screen) as big as the well allows
    let fh = (wh - 10.0 * PY).min((w - 10.0 * PX) * PY / PX);
    let fw = fh * PX / PY;
    let face = [x + (w - fw) / 2.0, y + (wh - fh) / 2.0, fw, fh];
    if picked {
        let (tex, src) = tile_face(Some(who));
        d.q(tex, src, face, WHITE, 128.0);
        d.out.last_mut().unwrap().round = [8.0 * PX; 4];
    } else {
        let tw = fw.min(fh * 92.0 / 68.0);
        d.q(C02, RANDOM_TILE, [x + (w - tw) / 2.0, y + (wh - tw * 68.0 / 92.0) / 2.0, tw, tw * 68.0 / 92.0], WHITE, 128.0);
    }
    let pl = m.players[p];
    let mut cy = y + wh + 6.0 * PY;
    let ns = 22.0 * PY;
    let name = match who {
        _ if !picked => "?".to_string(),
        Who::Mod(i) => m.mods.get(i).map_or("Custom", |md| md.name.as_str()).to_string(),
        Who::Disc(c) => m.remote_mod_name(p).unwrap_or(CHAR_NAMES[c]).to_string(),
    };
    d.text(&ellipsis(d, &name, ns, w), x, cy, ns, hex(look::INK));
    cy += ns + 2.0 * PY;
    let hand = if !picked { "Hand ?" } else if m.hand(text, p) < 0.0 { "Left" } else { "Right" };
    let costume = if picked { format!("Costume {}", pl.costume + 1) } else { "Costume ?".into() };
    let hs = 14.0 * PY;
    d.text(&ellipsis(d, &format!("{hand} - {costume}"), hs, w), x, cy, hs, s.soft);
}

/// The play style bar over the stats card (the roster card's own).
fn style_stats(d: &mut Draw, text: &Text, [x, y, w, h]: [f32; 4], picked: bool, who: Who) {
    let sh = 24.0 * PY;
    rounded(d, [x, y, w, sh], 8.0, hex(look::INK), 128.0);
    let style = match who {
        _ if !picked => "?",
        Who::Mod(i) => STYLE_NAMES[text.mods.get(i).map_or(0, |t| t.0).min(3)],
        Who::Disc(c) => STYLE_NAMES[text.style[c].min(3)],
    };
    d.label(style, x + w / 2.0, y + 5.0 * PY, 14.0 * PY, w - 10.0 * PX, hex(0xffd23f));
    let grades = (picked && !matches!(who, Who::Mod(i) if i >= text.mods.len())).then(|| grades_of(text, who));
    // rows as the select's, not stretched down a tall card
    stats(d, grades, [x, y + sh + 4.0 * PY, w, (h - sh - 4.0 * PY).clamp(0.0, 170.0 * PY)]);
}

/// B564: a pressed controller's card glows: gold rings round it, fading with `f` (1 just pressed, 0 gone).
fn glow(d: &mut Draw, [x, y, w, h]: [f32; 4], f: f32) {
    if f <= 0.0 {
        return;
    }
    for (g, a) in [(14.0, 30.0), (10.0, 55.0), (7.0, 128.0)] {
        rounded(d, [x - g * PX, y - g * PY, w + 2.0 * g * PX, h + 2.0 * g * PY], 14.0 + g, GOLD, a * f);
    }
}

/// B564: over a pressed controller's card, "Controller N pressed - this is you" in a bubble pointing down at it.
fn bubble(d: &mut Draw, m: &Menu, pads: &[Entity], [x, y, w, _]: [f32; 4], p: usize) {
    let f = m.flash[p];
    let Kind::Device(dv) = m.assign_kind(p) else { return };
    if f <= 0.0 {
        return;
    }
    let says = format!("{} pressed - this is you", dev_name(Some(dv), pads));
    let s = 14.0 * PY;
    let bw = (d.width(&says, s) + 24.0 * PX).min(w + 20.0 * PX);
    let (bh, cx) = (26.0 * PY, x + w / 2.0);
    let by = y - 18.0 * PY;
    let a = 128.0 * f.min(0.25) * 4.0;
    // the tail: a small square turned on its corner under the bubble
    let t = 7.0;
    d.q(super::Tex::Solid, [0.0; 4], [cx - t * PX, by + bh - t * PY, 2.0 * t * PX, 2.0 * t * PY], GOLD, a);
    d.out.last_mut().unwrap().round = [t * PX; 4];
    rounded(d, [cx - bw / 2.0 - 2.0 * PX, by - 2.0 * PY, bw + 4.0 * PX, bh + 4.0 * PY], 15.0, GOLD, a);
    rounded(d, [cx - bw / 2.0, by, bw, bh], 13.0, hex(look::INK), a);
    let start = d.out.len();
    d.label(&says, cx, by + (bh - s) / 2.0, s, bw - 12.0 * PX, WHITE);
    d.out[start..].iter_mut().for_each(|q| q.alpha = a);
}

/// B564: what a seat's badge shows: the controller's number (`pad` true: over a pad's outline), KB, COM, or - when
/// nobody has it.
fn badge(kind: Kind, pads: &[Entity]) -> (String, bool) {
    match kind {
        Kind::Device(Dev::Pad(e)) => (pads.iter().position(|x| *x == e).map_or(0, |i| i + 1).to_string(), true),
        Kind::Device(_) => ("KB".into(), false),
        Kind::Com => ("COM".into(), false),
        Kind::Free | Kind::Online => ("-".into(), false),
    }
}

/// The card's foot: on the left 1P's buttons for it (swap arrows on the focused card, + / - COM); the rest a chip in
/// the seat's colour (B564): a dark badge with the controller's number (KB, COM, - when free) beside what drives the
/// seat and its role (1P controls / Player, how a free seat joins, "Yours when you leave").
#[allow(clippy::too_many_arguments)]
fn foot(d: &mut Draw, m: &Menu, pads: &[Entity], s: &Seat, [x, y, w, h]: [f32; 4], p: usize, kind: Kind, on: bool, yours: bool) {
    let ps = 14.0 * PY;
    let mut lx = x;
    if on && m.held {
        // the held controller's arrows: they carry it the way 1P's ←→ do
        for (right, arrow) in [(false, "<"), (true, ">")] {
            if m.host_swap(right).is_some() && lx + h * PX / PY <= x + w / 2.0 {
                let r = [lx, y, h * PX / PY, h];
                rounded(d, r, 15.0, hex(look::INK), 128.0);
                d.centred(arrow, lx + r[2] / 2.0, y + (h - 18.0 * PY) / 2.0, 18.0 * PY, WHITE);
                d.hit(r, pointer::Act::Row(p, if right { Press { right: true, ..default() } } else { Press { left: true, ..default() } }));
                lx += r[2] + 4.0 * PX;
            }
        }
    }
    if m.can_bot(p) {
        let (word, rgb) = if m.bots[p] { ("- COM", [120.0, 30.0, 30.0]) } else { ("+ COM", [40.0, 100.0, 40.0]) };
        let bw = (d.width(word, ps) + 20.0 * PX).min(x + w / 2.0 - lx);
        if bw > 20.0 * PX {
            rounded(d, [lx, y, bw, h], 15.0, rgb, 128.0);
            d.label(word, lx + bw / 2.0, y + (h - ps) / 2.0, ps, bw - 8.0 * PX, WHITE);
            d.hit([lx, y, bw, h], pointer::Act::Row(p, Press { card: true, ..default() }));
            lx += bw + 4.0 * PX;
        }
    }
    let lead = Some(p) == m.seats[..m.players()].iter().position(|d| *d == m.lead());
    let (fill, ink) = match kind {
        Kind::Device(_) => (s.ring, s.ink),
        Kind::Free | Kind::Com => (s.well, hex(look::INK)),
        Kind::Online => (hex(BG), WHITE),
    };
    let device = match kind {
        Kind::Device(dv) => dev_name(Some(dv), pads),
        Kind::Free if m.paused => "No controller".into(),
        Kind::Free => "Open".into(),
        Kind::Com => "Computer".into(),
        Kind::Online => "Online".into(),
    };
    let role = match kind {
        _ if yours => "Yours when you leave".to_string(),
        Kind::Device(_) if lead => "1P controls".into(),
        Kind::Device(_) => "Player".into(),
        Kind::Free if can_join(m, p) => dev_name(Some(Dev::Open), pads),
        Kind::Free => "Free".into(),
        Kind::Com => "Plays itself".into(),
        Kind::Online => "Another machine".into(),
    };
    let cw = x + w - lx;
    if cw < 30.0 * PX {
        return;
    }
    rounded(d, [lx, y, cw, h], 10.0, fill, 128.0);
    // the badge: a dark square (on screen) inset in the chip
    let i = 3.0;
    let bh = h - 2.0 * i * PY;
    let bw = bh * PX / PY;
    let (bx, by) = (lx + i * PX, y + i * PY);
    rounded(d, [bx, by, bw, bh], 6.0, hex(look::INK), 128.0);
    let (num, outline) = badge(kind, pads);
    if outline {
        // a pad's outline, faint behind the number: the body and its two grips
        let rim = [s.ring[0] * 0.5 + 20.0, s.ring[1] * 0.5 + 20.0, s.ring[2] * 0.5 + 20.0];
        let (gw, gh) = (bw * 0.3, bh * 0.45);
        for gx in [bx + bw * 0.1, bx + bw * 0.6] {
            rounded(d, [gx, by + bh * 0.42, gw, gh], 4.0, rim, 70.0);
        }
        rounded(d, [bx + bw * 0.08, by + bh * 0.22, bw * 0.84, bh * 0.42], 5.0, rim, 70.0);
    }
    let ns = if num.len() > 1 { bh * 0.42 } else { bh * 0.78 };
    d.label(&num, bx + bw / 2.0, by + (bh - ns) / 2.0, ns, bw - 2.0 * PX, if outline { s.sel } else { WHITE });
    let tx = bx + bw + 6.0 * PX;
    let tw = lx + cw - 6.0 * PX - tx;
    if tw < 20.0 * PX {
        return;
    }
    let (ds, rs) = (15.0 * PY, 12.0 * PY);
    let ty = y + (h - ds - rs - 1.0 * PY) / 2.0;
    d.text(&ellipsis(d, &device, ds, tw), tx, ty, ds, ink);
    let start = d.out.len();
    d.text(&ellipsis(d, &role, rs, tw), tx, ty + ds + 1.0 * PY, rs, ink);
    d.out[start..].iter_mut().for_each(|q| q.alpha = 100.0);
}

/// Whether a device not seated could take seat `p` with ✕ (it takes the first free one).
fn can_join(m: &Menu, p: usize) -> bool {
    let n = if m.online() && !m.paused { m.local_max() } else { m.players() };
    p < n && m.seats[p].is_none() && !m.bots[p]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::main_menu::Screen;

    /// The pause overlay draws on a smaller quad pool than the menu: four full cards fit it on every aspect, all four
    /// flashing (B564).
    #[test]
    fn four_cards_fit_the_pause_pool() {
        let t = super::super::pointer::tests::text_with_mods(0);
        let mut m = Menu { screen: Screen::Assign, doubles: true, paused: true, seats: [Some(Dev::Keys), None, None, None], bots: [false, true, true, false], flash: [1.0; 4], ..Default::default() };
        for p in &mut m.players {
            p.ready = true;
        }
        for a in [4.0, 16.0 / 9.0, 4.0 / 3.0, 9.0 / 16.0, 0.4] {
            m.grid = super::super::Grid::fit(a);
            let q = super::super::drawn(&m, &t, &super::super::controls::Bindings::default(), &[], 0, 0.0).out.len();
            assert!(q < super::super::pause::POOL_HERE, "{a}: {q} quads");
        }
    }

    /// One row of cards on a wide view, two rows of two for four seats on a tall one; every card inside the view and
    /// over the description bar, none overlapping.
    #[test]
    fn cards_fit_any_aspect() {
        for (view, n, rows) in [([0.0, 0.0, 640.0, 432.0], 4, 1), ([-200.0, 0.0, 840.0, 432.0], 2, 1), ([0.0, -300.0, 640.0, 732.0], 4, 2), ([0.0, -300.0, 640.0, 732.0], 2, 1)] {
            let c = cards(view, n);
            assert_eq!(c.len(), n);
            let tops: std::collections::BTreeSet<i32> = c.iter().map(|r| r[1] as i32).collect();
            assert_eq!(tops.len(), rows, "{view:?} {n}");
            for (i, a) in c.iter().enumerate() {
                assert!(a[0] >= view[0] && a[0] + a[2] <= view[2] && a[1] >= view[1] && a[1] + a[3] <= view[3] - OVER_BAR + 0.01, "{a:?}");
                for b in &c[i + 1..] {
                    let apart = a[0] + a[2] <= b[0] || b[0] + b[2] <= a[0] || a[1] + a[3] <= b[1] || b[1] + b[3] <= a[1];
                    assert!(apart, "{a:?} {b:?}");
                }
            }
        }
    }

    /// The kinds: a device, a free seat, the computer's, another machine's player online.
    #[test]
    fn kinds() {
        let mut m = Menu { screen: Screen::Assign, doubles: true, seats: [Some(Dev::Keys), None, None, None], ..default() };
        m.bots[2] = true;
        assert_eq!([0, 1, 2].map(|p| m.assign_kind(p)), [Kind::Device(Dev::Keys), Kind::Free, Kind::Com]);
        m.paused = true;
        m.bots = [false, true, true, false];
        m.net_pause = Some([false, true, false, false]);
        assert_eq!([0, 1, 2, 3].map(|p| m.assign_kind(p)), [Kind::Device(Dev::Keys), Kind::Online, Kind::Com, Kind::Free]);
    }

    /// B564: the foot's badge: a pad's number in the app's controller order over its outline, KB, COM, - when free.
    #[test]
    fn badges() {
        let mut w = World::new();
        let pads: Vec<Entity> = (0..3).map(|_| w.spawn_empty().id()).collect();
        assert_eq!(badge(Kind::Device(Dev::Pad(pads[1])), &pads), ("2".to_string(), true));
        assert_eq!(badge(Kind::Device(Dev::Keys), &pads), ("KB".to_string(), false));
        assert_eq!(badge(Kind::Com, &pads).0, "COM");
        assert_eq!(badge(Kind::Free, &pads).0, "-");
    }
}
