//! B297 (user 2026-10-14, the approved mockups `Main` and `Ultrawide`; ours, menus aren't 1:1): the main menu as a
//! list on the left (a Rematch card once a local match has been played this session, Local, Online, Mods, Settings,
//! a smaller Quit at the foot), a preview panel for the highlighted row (a picture, its words and three facts) and a
//! header line with the pads and the disc. No description bar: the legend strip is the footer. A window wide enough
//! (21:9) keeps the list and panel at their 16:9 size and adds a New in the store column (`mods_screen::news`).
//! Laid out in the centred 640×448 space across the whole view (`Grid::extra` past each side), so it scales
//! uniformly at any aspect ratio.

use super::{
    CHAR_NAMES, COURTS, Draw, GOLD, INK, Menu, SETS, Tex, Text, WHITE, Who, face_frame, mods_screen, options, pointer,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Row {
    Rematch,
    Local,
    Online,
    Mods,
    Settings,
    /// B118: a 1v1 on this phone (touch screens only).
    Duo,
    Quit,
}

impl Menu {
    /// The main screen's rows, top to bottom (`sel` indexes them). A web page can't close itself (an `AppExit` there
    /// only freezes it), so the browser build has no Quit.
    pub(super) fn main_rows(&self) -> Vec<Row> {
        let mut v = Vec::new();
        if self.rematch {
            v.push(Row::Rematch);
        }
        v.extend([Row::Local, Row::Online, Row::Mods, Row::Settings]);
        if super::touch_page() {
            v.push(Row::Duo);
        }
        if cfg!(not(target_arch = "wasm32")) {
            v.push(Row::Quit);
        }
        v
    }

    /// Where `r` is on the main screen (Back from its pages lands there).
    pub(super) fn main_at(&self, r: Row) -> usize {
        self.main_rows().iter().position(|x| *x == r).unwrap_or(0)
    }

    /// The Rematch card's words: "Singles - Carol vs Kent", "Random Court - 1 set".
    fn rematch_lines(&self, text: &Text) -> (String, String) {
        let name = |p: usize| match self.pick(p).0 {
            Who::Disc(c) => CHAR_NAMES[c].to_string(),
            Who::Mod(i) => self.mods.get(i).map_or("?".into(), |m| m.name.clone()),
        };
        let teams = if self.doubles {
            format!("Doubles - {} & {} vs {} & {}", name(0), name(1), name(2), name(3))
        } else {
            format!("Singles - {} vs {}", name(0), name(1))
        };
        let court = if self.court == COURTS { "Random Court".to_string() } else { text.msg(276 + 2 * self.court).to_string() };
        let sets = SETS[self.sets];
        (teams, format!("{court} - {sets} set{}", if sets == 1 { "" } else { "s" }))
    }
}

/// The panels' sizes (640 units): the list, the gaps, the preview panel's widest, the store column; the content's
/// top and foot (under the title tab, over the legend strip).
const LIST_W: f32 = 200.0;
const GAP: f32 = 16.0;
const PANEL_MAX: f32 = 540.0;
const STORE_W: f32 = 170.0;
const TOP: f32 = 50.0;
const FOOT: f32 = 426.0;
/// Room left of the list for the hand, and right of the last panel.
const LEFT: f32 = 64.0;
const RIGHT: f32 = 16.0;
/// The list's pills.
const PILL_H: f32 = 38.0;
const PITCH: f32 = 46.0;
const CARD_H: f32 = 62.0;
/// The preview's picture height and the facts' tiles.
const PIC_H: f32 = 170.0;
const FACT_H: f32 = 44.0;

/// Where the panels go on a view `extra` wider than 640 each side: the list's x, the panel's x and width, and the
/// store column's x if it fits.
pub(super) fn columns(extra: f32) -> (f32, f32, f32, Option<f32>) {
    let room = 640.0 + 2.0 * extra - LEFT - RIGHT;
    let rest = room - LIST_W - GAP;
    let store = rest - PANEL_MAX >= GAP + STORE_W;
    let panel = rest.min(PANEL_MAX);
    let used = LIST_W + GAP + panel + if store { GAP + STORE_W } else { 0.0 };
    let x = -extra + LEFT + (room - used) / 2.0;
    let px = x + LIST_W + GAP;
    (x, px, panel, store.then_some(px + panel + GAP))
}

/// The tints: the chosen pill, the others, a picture's backdrop by row.
const ON: [f32; 3] = [128.0, 108.0, 38.0];
const OFF: [f32; 3] = [56.0, 56.0, 67.0];
const PAPER: [f32; 3] = [128.0, 128.0, 128.0];
const MUTED: [f32; 3] = [45.0, 40.0, 52.0];

fn backdrop(r: Row) -> [f32; 3] {
    match r {
        Row::Rematch => [118.0, 104.0, 70.0],
        Row::Local => [100.0, 116.0, 96.0],
        Row::Online => [92.0, 108.0, 122.0],
        Row::Mods => [114.0, 104.0, 122.0],
        Row::Settings => [118.0, 104.0, 98.0],
        Row::Duo => [100.0, 116.0, 96.0],
        Row::Quit => [100.0; 3],
    }
}

/// The main screen.
pub(super) fn layout(d: &mut Draw, m: &Menu, text: &Text, pads: usize, hand_x: f32) {
    d.frame(0, 5);
    // the header's right: pads and disc (only SCUS-97610 gets this far, `exe`)
    d.anchor = super::Anchor::Sides;
    let head = format!("{pads} pad{}  -  Disc: SCUS-97610", if pads == 1 { "" } else { "s" });
    let w = d.width(&head, 16.0);
    d.outlined(|d| d.text(&head, 624.0 - w, 20.0, 16.0, WHITE));
    d.anchor = super::Anchor::Centre;

    let rows = m.main_rows();
    let sel = m.sel.min(rows.len() - 1);
    let (x, px, pw, store) = columns(m.grid.extra);
    let mut y = TOP;
    let mut hand = [0.0; 2];
    for (i, &r) in rows.iter().enumerate() {
        let on = i == sel;
        let rect = match r {
            Row::Rematch => {
                let rect = [x, y, LIST_W, CARD_H];
                d.box3(rect, 12.0, [Some(INK), Some(if on { ON } else { WHITE }), Some(PAPER)], 128.0);
                let (a, b) = m.rematch_lines(text);
                d.text("REMATCH", x + 14.0, y + 6.0, 13.0, [90.0, 60.0, 0.0]);
                fit_text(d, &a, x + 14.0, y + 21.0, 18.0, LIST_W - 26.0, INK);
                fit_text(d, &b, x + 14.0, y + 41.0, 14.0, LIST_W - 26.0, MUTED);
                y += CARD_H + 12.0;
                rect
            }
            Row::Quit => {
                let rect = [x, FOOT - 32.0, 100.0, 30.0];
                d.pill_at(rect, [INK, if on { ON } else { [80.0, 70.0, 90.0] }, if on { ON } else { [96.0, 84.0, 108.0] }], if on { 128.0 } else { 100.0 });
                d.text("Quit", x + 18.0, rect[1] + 6.0, 18.0, if on { WHITE } else { INK });
                rect
            }
            _ => {
                let rect = [x, y, LIST_W, PILL_H];
                d.pill_at(rect, [INK, WHITE, if on { ON } else { OFF }], 128.0);
                d.text(name(r), x + 20.0, y + 7.0, 24.0, if on { INK } else { [100.0; 3] });
                if on {
                    d.text(">", x + LIST_W - 28.0, y + 7.0, 24.0, INK);
                }
                // B205: mods installed or removed since the game started
                if r == Row::Mods && mods_screen::reload_due(m) {
                    d.outlined(|d| d.text("Reload", x + LIST_W - 84.0, y + 10.0, 18.0, GOLD));
                }
                y += PITCH;
                rect
            }
        };
        d.hit(rect, pointer::Act::Row(i, pointer::OK));
        if on {
            hand = [rect[0] - 60.0, rect[1] + rect[3] / 2.0 - 16.0];
        }
    }
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [hand[0] + hand_x, hand[1], 64.0, 32.0], WHITE, 128.0);
    preview(d, m, text, rows[sel], pads, [px, TOP, pw, FOOT - TOP]);
    if let Some(sx) = store {
        mods_screen::news(d, m, [sx, TOP, STORE_W, FOOT - TOP]);
    }
    super::version_corner(d);
    mods_screen::ask_layout(d, m, hand_x);
}

fn name(r: Row) -> &'static str {
    match r {
        Row::Rematch => "Rematch",
        Row::Local => "Local",
        Row::Online => "Online",
        Row::Mods => "Mods",
        Row::Settings => "Settings",
        Row::Duo => "1v1 on this phone",
        Row::Quit => "Quit",
    }
}

/// The umpire's name, or Random.
fn umpire(m: &Menu, text: &Text) -> String {
    match m.umpire as usize {
        super::UMPIRES => "Random".into(),
        u => text.msg(669 + 2 * u).trim_start_matches("Umpire ").to_string(),
    }
}

/// `s` from (x, y) at `size`, squeezed across to fit `max`.
pub(super) fn fit_text(d: &mut Draw, s: &str, x: f32, y: f32, size: f32, max: f32, rgb: [f32; 3]) {
    let w = d.width(s, size);
    d.label(s, x + w.min(max) / 2.0, y, size, max, rgb);
}

/// `s` word-wrapped to `w` at `size` from (x, y), at most `lines` lines; the y under it.
fn para(d: &mut Draw, s: &str, x: f32, y: f32, w: f32, size: f32, lines: usize, rgb: [f32; 3]) -> f32 {
    let mut out: Vec<String> = Vec::new();
    for word in s.split_whitespace() {
        match out.last_mut() {
            Some(l) if d.width(&format!("{l} {word}"), size) <= w => {
                l.push(' ');
                l.push_str(word);
            }
            _ => out.push(word.into()),
        }
    }
    let mut y = y;
    for l in out.iter().take(lines) {
        fit_text(d, l, x, y, size, w, rgb);
        y += size * 1.35;
    }
    y
}

/// The highlighted row's panel in `[x, y, w, h]`: its picture with its name over the foot, its words, three facts.
fn preview(d: &mut Draw, m: &Menu, text: &Text, r: Row, pads: usize, [x, y, w, h]: [f32; 4]) {
    d.box3([x, y, w, h], 16.0, [Some(INK), Some(WHITE), Some(PAPER)], 118.0);
    let pic = [x + 3.0, y + 3.0, w - 6.0, PIC_H];
    if r != Row::Quit {
        d.box3(pic, 13.0, [None, None, Some(backdrop(r))], 128.0);
        d.q(Tex::Solid, [0.0; 4], [x, y + 3.0 + PIC_H, w, 2.5], INK, 128.0);
        picture(d, m, r, pic);
        d.outlined(|d| d.text(name(r), x + 18.0, y + PIC_H - 34.0, 34.0, WHITE));
    } else {
        d.outlined(|d| d.text(name(r), x + 18.0, y + 14.0, 34.0, WHITE));
    }
    let words = match r {
        Row::Rematch => "The last match again: the same players, characters and costumes, court, sets, games and umpire. A random court or umpire is drawn again.".to_string(),
        Row::Local => text.msg(188).to_string(),
        Row::Online => "Play over the internet or on this network: host a lobby, find one, or join by code.".into(),
        Row::Mods => "Browse the mod store, add characters from other games and manage the ones you have.".into(),
        Row::Settings => text.msg(190).to_string(),
        Row::Duo => "Two players, one phone: lay it flat between you, each plays from their own end.".into(),
        Row::Quit => "Close the game.".into(),
    };
    let top = if r == Row::Quit { y + 60.0 } else { y + PIC_H + 16.0 };
    para(d, &words, x + 18.0, top, w - 36.0, 17.0, 4, INK);
    let on = |b: bool| if b { "On" } else { "Off" }.to_string();
    let facts: Vec<(&str, String)> = match r {
        Row::Rematch => {
            let humans = m.seats[..m.players()].iter().filter(|s| s.is_some()).count();
            vec![("Players", format!("{humans} + {} COM", m.players() - humans)), ("Games", m.games.to_string()), ("Umpire", umpire(m, text))]
        }
        Row::Local => vec![("Singles", "1-2 players".into()), ("Doubles", "1-4 players".into()), ("Pads", format!("{pads} connected"))],
        Row::Online => vec![("Host", "Singles, doubles".into()), ("Join", "Find or by code".into()), ("Players", "Up to 4".into())],
        Row::Mods => {
            let store = mods_screen::store_len(m).map_or("Browse".into(), |n| format!("{n} mods"));
            vec![("Installed", format!("{} characters", m.mods.len())), ("Store", store), ("Share", "Upload yours".into())]
        }
        Row::Duo => vec![("Players", "2".into()), ("Pads", "On screen".into()), ("Phone", "Flat".into())],
        Row::Settings => vec![("Pages", options::tabs_shown().len().to_string()), ("Upscaled", on(m.settings.0[0])), ("Menu music", on(m.settings.0[1]))],
        Row::Quit => vec![],
    };
    let n = facts.len().max(1) as f32;
    let tw = (w - 36.0 - 10.0 * (n - 1.0)) / n;
    for (i, (k, v)) in facts.iter().enumerate() {
        let tx = x + 18.0 + (tw + 10.0) * i as f32;
        let ty = y + h - 16.0 - FACT_H;
        d.box3([tx, ty, tw, FACT_H], 10.0, [Some(INK), Some(PAPER), Some(PAPER)], 128.0);
        fit_text(d, &k.to_uppercase(), tx + 10.0, ty + 6.0, 12.0, tw - 20.0, MUTED);
        fit_text(d, v, tx + 10.0, ty + 21.0, 17.0, tw - 20.0, INK);
    }
}

/// The picture: the Rematch's players, a row of the disc's characters (the last pick first), the courts, the
/// installed mods' faces, or the Settings pages.
fn picture(d: &mut Draw, m: &Menu, r: Row, [x, y, w, h]: [f32; 4]) {
    // a player's card `ph` tall from (cx - its half, top)
    let card = |d: &mut Draw, who: (Who, usize), cx: f32, top: f32, ph: f32| {
        let pw = ph * 128.0 / 168.0;
        // ponytail: face icons for both (B295 dropped the costume portrait sheets)
        face_frame(d, (Tex::Icon(who.0), [0.0, 0.0, 64.0, 64.0]), cx, top + ph / 2.0, pw * 0.9, WHITE);
    };
    let ph = h - 20.0;
    let pw = ph * 128.0 / 168.0;
    match r {
        Row::Rematch => {
            let n = m.players();
            let step = (pw + 8.0).min((w - 40.0) / n as f32);
            let x0 = x + w / 2.0 - step * (n as f32 - 1.0) / 2.0;
            // as the confirm screen orders them: team 0, then team 1
            let order: &[usize] = if m.doubles { &[0, 1, 3, 2] } else { &[0, 1] };
            for (k, &p) in order.iter().enumerate() {
                card(d, m.pick(p), x0 + step * k as f32, y + 10.0, ph);
            }
        }
        Row::Local | Row::Duo => {
            let first = if m.players[0].modded { 6 } else { m.players[0].char() };
            let n = (((w - 16.0) / (pw * 0.8)) as usize).clamp(1, super::CHARS);
            let step = pw * 0.8;
            let x0 = x + w / 2.0 - step * (n as f32 - 1.0) / 2.0;
            for k in 0..n {
                let c = (first + k) % super::CHARS;
                card(d, (Who::Disc(c), 0), x0 + step * k as f32, y + 10.0, ph);
            }
        }
        Row::Online => {
            // the courts' tiles (`Confirmation_02`: courts 0–6 down its left column, 7–10 its right)
            let (tw, th) = (92.0 * 0.9, 68.0 * 0.9);
            let cols = ((w - 20.0) / (tw + 8.0)) as usize;
            for k in 0..(cols * 2).min(COURTS) {
                let tile = if k < 7 { 2 * k } else { 2 * (k - 7) + 1 };
                let (col, row) = (k % cols, k / cols);
                let x0 = x + (w - cols as f32 * (tw + 8.0) + 8.0) / 2.0;
                let dst = [x0 + (tw + 8.0) * col as f32, y + 14.0 + (th + 8.0) * row as f32, tw, th];
                d.q(super::C02, [96.0 * (tile % 2) as f32, 72.0 * (tile / 2) as f32, 92.0, 68.0], dst, WHITE, 128.0);
            }
        }
        Row::Mods => {
            let size = 60.0;
            let cols = ((w - 20.0) / (size + 8.0)) as usize;
            let x0 = x + (w - cols as f32 * (size + 8.0) + 8.0) / 2.0 + size / 2.0;
            for i in 0..m.mods.len().min(cols * 2) {
                let (cx, cy) = (x0 + (size + 8.0) * (i % cols) as f32, y + 12.0 + size / 2.0 + (size + 6.0) * (i / cols) as f32);
                face_frame(d, (Tex::Icon(Who::Mod(i)), [0.0, 0.0, 64.0, 64.0]), cx, cy, size, WHITE);
            }
        }
        Row::Settings => {
            let pw = 150.0;
            let cols = (((w - 20.0) / (pw + 10.0)) as usize).clamp(1, 3);
            let x0 = x + (w - cols as f32 * (pw + 10.0) + 10.0) / 2.0;
            for (k, (page, _)) in options::tabs_shown().iter().enumerate() {
                let dst = [x0 + (pw + 10.0) * (k % cols) as f32, y + 14.0 + 40.0 * (k / cols) as f32, pw, 32.0];
                d.pill_at(dst, [INK, WHITE, OFF], 128.0);
                d.label(page, dst[0] + pw / 2.0, dst[1] + 6.0, 20.0, pw - 24.0, [110.0; 3]);
            }
        }
        Row::Quit => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The list and panel keep their size from 16:9 to 21:9 and the store column only comes in when it fits; a 4:3
    /// view still has them all on screen, the hand included.
    #[test]
    fn columns_fit_each_aspect() {
        let extra = |a: f32| (640.0 / super::super::super::widescreen::share(a) - 640.0) / 2.0;
        let (x, px, pw, store) = columns(extra(4.0 / 3.0));
        assert!(x - 60.0 >= 0.0 && px + pw <= 640.0 && store.is_none(), "{x} {px} {pw}");
        let (_, _, pw169, store) = columns(extra(16.0 / 9.0));
        assert!(store.is_none());
        let (x, _, pw219, store) = columns(extra(21.0 / 9.0));
        assert_eq!(pw169, pw219);
        let s = store.expect("21:9 has the store column");
        assert!(x - 60.0 >= -extra(21.0 / 9.0) && s + STORE_W <= 640.0 + extra(21.0 / 9.0));
    }
}
