//! B297 (user 2026-10-14, the approved mockups `Main` and `Ultrawide`; ours, menus aren't 1:1): the main menu as a
//! list on the left (a Rematch card once a local match has been played this session, Local, Online, Mods, Settings,
//! a smaller Quit at the foot), a preview panel for the highlighted row (a picture, its words and three facts) and a
//! header line with the pads and the disc. No description bar: the legend strip is the footer. A window wide enough
//! (21:9) keeps the list and panel at their 16:9 size and adds a New in the store column (`mods_screen::news`).
//! Laid out in the centred 640×448 space across the whole view (`Grid::extra` past each side), so it scales
//! uniformly at any aspect ratio.

use super::{
    CARD_FILL, CHAR_NAMES, COURTS, Draw, GOLD, INK, Menu, SETS, Tex, Text, WHITE, Who, face_frame, legend, mods_screen, options, pointer,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Row {
    Rematch,
    Local,
    Online,
    Mods,
    Settings,
    /// B317: a newer build is out (`update.rs`): Refresh (browser) or Download (desktop).
    Update,
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
        if crate::update::latest().is_some() {
            v.push(Row::Update);
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
/// B317's new-version card.
const UPDATE_H: f32 = 48.0;
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
        Row::Update => [118.0, 104.0, 70.0],
        Row::Quit => [100.0; 3],
    }
}

/// The main screen.
pub(super) fn layout(d: &mut Draw, m: &Menu, text: &Text, pads: usize, hand_x: f32) {
    d.frame(0, 5);
    // the header's right: pads and disc (only SCUS-97610 gets this far, `exe`), B302's build version and commit
    // under it (`+` if built with local changes; the font has no `·`); `Sides` pins each side of 320 to its own
    // edge, so neither line may reach the middle
    d.anchor = super::Anchor::Sides;
    let head = format!("{pads} pad{}  -  Disc: SCUS-97610", if pads == 1 { "" } else { "s" });
    let w = d.width(&head, 16.0);
    d.outlined(|d| d.text(&head, 624.0 - w, 18.0, 16.0, WHITE));
    // B374: a release build's own tag
    let version = format!("{} - {}", crate::update::VERSION.unwrap_or(concat!("v", env!("CARGO_PKG_VERSION"))), env!("HST_COMMIT"));
    let w = d.width(&version, 10.0);
    d.outlined(|d| d.text(&version, 624.0 - w, 37.0, 10.0, WHITE));
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
            Row::Update => {
                // the notice: a gold card under the rows, its button's word big
                let rect = [x, y + 4.0, LIST_W, UPDATE_H];
                d.box3(rect, 12.0, [Some(INK), Some(if on { ON } else { WHITE }), Some(if on { ON } else { [118.0, 100.0, 50.0] })], 128.0);
                d.text("NEW VERSION OUT", x + 14.0, rect[1] + 6.0, 13.0, if on { WHITE } else { [90.0, 60.0, 0.0] });
                d.text(name(r), x + 14.0, rect[1] + 21.0, 22.0, if on { WHITE } else { INK });
                if on {
                    d.text(">", x + LIST_W - 28.0, rect[1] + 21.0, 22.0, WHITE);
                }
                y += UPDATE_H + 4.0;
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
                // B318: squeezed to stay clear of the chosen row's `>` (the words keep their height)
                fit_text(d, name(r), x + 20.0, y + 7.0, 24.0, LIST_W - 56.0, if on { INK } else { [100.0; 3] });
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
    mods_screen::ask_layout(d, m, hand_x);
}

fn name(r: Row) -> &'static str {
    match r {
        Row::Rematch => "Rematch",
        Row::Local => "Local",
        Row::Online => "Online",
        Row::Mods => "Mods",
        Row::Settings => "Settings",
        Row::Update => crate::update::button(),
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
        picture(d, m, r, pads, pic);
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
        Row::Update if cfg!(target_arch = "wasm32") => "A new version is out. Refresh loads this page again with it; your settings and mods stay.".into(),
        Row::Update if crate::update::installs() => "A new version is out. Update downloads it, checks its signature and restarts the game into it; your settings and mods stay.".into(),
        Row::Update => "A new version is out. Download opens its release page: swap this game for the new build, keeping your settings and mods beside it.".into(),
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
        Row::Local => {
            let mut v = vec![("Singles", "1-2 players".to_string()), ("Doubles", "3-4 players".into())];
            if m.rematch {
                let court = if m.court == COURTS { "Random Court".to_string() } else { text.msg(276 + 2 * m.court).to_string() };
                v.push(("Last match", format!("{}, {court}", if m.doubles { "Doubles" } else { "Singles" })));
            }
            v
        }
        Row::Online => vec![("Host", "Singles, doubles".into()), ("Join", "Find or by code".into()), ("Players", "Up to 4".into())],
        Row::Mods => {
            let mut v = vec![("Installed", format!("{} character{}", m.mods.len(), if m.mods.len() == 1 { "" } else { "s" }))];
            if let Some(u) = mods_screen::updates(m) {
                v.push(("Updates", format!("{} ready", u.len())));
            }
            if let Some(n) = mods_screen::fresh_len(m) {
                v.push(("Store", format!("{n} not installed")));
            }
            v
        }
        Row::Update if cfg!(target_arch = "wasm32") => vec![("Refresh", "Reloads this page".into()), ("Keeps", "Settings, mods, ISO".into())],
        Row::Update if crate::update::installs() => vec![("Update", "Download, check, restart".into()), ("Keeps", "Settings, mods, ISO".into())],
        Row::Update => vec![("Download", "GitHub release page".into()), ("Builds", "Linux, Windows".into())],
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

/// The picture: the Rematch's players, a tile per input device (a seat each), the courts, the installed mods'
/// portraits, or the Settings pages.
fn picture(d: &mut Draw, m: &Menu, r: Row, pads: usize, [x, y, w, h]: [f32; 4]) {
    // a player's card `ph` tall from (cx - its half, top)
    let card = |d: &mut Draw, who: (Who, usize), cx: f32, top: f32, ph: f32| {
        let pw = ph * 128.0 / 168.0;
        // ponytail: face icons for both (B295 dropped the costume portrait sheets)
        face_frame(d, (Tex::Face(who.0), [0.0, 0.0, 64.0, 64.0]), cx, top + ph / 2.0, pw * 0.9, WHITE);
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
        Row::Local => {
            // the devices plugged in now, each a seat (the keyboard first, then the pads), and the next seat free
            let mut seats: Vec<(&str, String)> = vec![if super::touch_page() { ("phone", "Touch".into()) } else { ("keys", "Keyboard".into()) }];
            seats.extend((1..=pads).map(|k| ("pad", format!("Pad {k}"))));
            seats.truncate(4);
            device_tiles(d, &seats, [x, y, w, h]);
            caption(d, "The devices plugged in now: each takes a seat", x + w - 13.0, y + h - 21.0, [23.0, 45.0, 22.0]);
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
            mod_tiles(d, m, [x, y, w, h]);
            if !m.mods.is_empty() {
                caption(d, "Newest installed first", x + w - 13.0, y + h - 21.0, [45.0, 30.0, 51.0]);
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
        Row::Update => {
            // this build's and the new one's, side by side
            let (old, new) = (crate::update::ours(), crate::update::latest().unwrap_or_default());
            let tw = ((w - 80.0) / 2.0).min(150.0);
            let x0 = x + (w - 2.0 * tw - 40.0) / 2.0;
            for (k, (label, v, fill)) in [("NOW", old, OFF), ("NEW", new, ON)].into_iter().enumerate() {
                let r = [x0 + (tw + 40.0) * k as f32, y + 34.0, tw, 64.0];
                d.box3(r, 10.0, [Some(INK), Some(WHITE), Some(fill)], 128.0);
                d.centred(label, r[0] + tw / 2.0, r[1] + 8.0, 13.0, WHITE);
                d.label(&v, r[0] + tw / 2.0, r[1] + 26.0, 24.0, tw - 16.0, WHITE);
            }
            d.outlined(|d| d.centred(">", x0 + tw + 20.0, y + 52.0, 28.0, WHITE));
            // B374: the install's progress bar and words
            if let Some((says, k)) = crate::update::progress() {
                let bw = 2.0 * tw + 40.0;
                if let Some(k) = k {
                    let r = [x0, y + 108.0, bw, 14.0];
                    d.box3(r, 7.0, [Some(INK), Some(WHITE), Some(OFF)], 128.0);
                    d.box3([r[0], r[1], (bw * k).max(14.0), 14.0], 7.0, [None, None, Some(ON)], 128.0);
                }
                d.outlined(|d| d.label(&says, x0 + bw / 2.0, y + 126.0, 12.0, bw, WHITE));
            }
        }
        Row::Quit => {}
    }
}

/// `s` at 8.5 units ending at `right`.
fn caption(d: &mut Draw, s: &str, right: f32, y: f32, rgb: [f32; 3]) {
    let w = d.width(s, 8.5);
    d.text(s, right - w, y, 8.5, rgb);
}

/// The seats' tiles across the top of picture `[x, y, w, _]`: one per device (`seats`: its glyph and name) in the
/// players' colours, then a dashed one for the next seat (Press to join) while there's room.
fn device_tiles(d: &mut Draw, seats: &[(&str, String)], [x, y, w, _]: [f32; 4]) {
    let (pad, gap, th) = (14.0, 7.0, 98.0);
    let tw = (w - 2.0 * pad - 3.0 * gap) / 4.0;
    let n = (seats.len() + 1).min(4);
    let x0 = x + (w - n as f32 * tw - (n - 1) as f32 * gap) / 2.0;
    for k in 0..n {
        let r = [x0 + (tw + gap) * k as f32, y + 14.0, tw, th];
        let cx = r[0] + tw / 2.0;
        let p = format!("{}P", k + 1);
        match seats.get(k) {
            Some((glyph, name)) => {
                d.box3(r, 8.0, [Some(INK), Some([96.0; 3]), Some(CARD_FILL[k])], 128.0);
                d.centred(&p, cx, r[1] + 14.0, 16.0, WHITE);
                device_glyph(d, glyph, cx, r[1] + 51.0, CARD_FILL[k]);
                d.label(name, cx, r[1] + 70.0, 10.0, tw - 12.0, WHITE);
            }
            None => {
                d.box3(r, 8.0, [None, None, Some([40.0, 36.0, 44.0])], 40.0);
                dashed(d, r, 8.0, [72.0, 66.0, 78.0]);
                d.centred(&p, cx, r[1] + 22.0, 16.0, WHITE);
                // B362: "Press ✕ / Enter" in the pads' and the keyboard's button art
                press_hint(d, "", cx, r[1] + 49.0, 8.5, tw - 8.0, WHITE);
                d.centred("to join", cx, r[1] + 62.0, 8.5, WHITE);
            }
        }
    }
}

/// B362: what an open seat's card says (`press_hint` draws it in buttons).
pub(super) const JOIN: &str = "Press ✕ / Enter to join";

/// B362: "Press" and each connected pad's ✕ then the keyboard's Enter, a slash between, in the legend's button art
/// (`legend::join_styles`, as the Button Icons setting has them), then `tail`: centred on `cx`, the words `size`
/// high from `y`, the buttons a little taller around their middle, all shrunk as one to fit `max` wide.
pub(super) fn press_hint(d: &mut Draw, tail: &str, cx: f32, y: f32, size: f32, max: f32, rgb: [f32; 3]) {
    let styles = legend::join_styles();
    let ih = size * 1.6;
    let (head, slash) = (d.width("Press ", size), d.width(" / ", size));
    let icons: f32 = styles.iter().map(|&s| legend::ok_aspect(s) * ih).sum::<f32>() + slash * (styles.len() - 1) as f32;
    let full = head + icons + d.width(tail, size);
    let k = (max / full.max(1.0)).min(1.0);
    let y = y + size * (1.0 - k) / 2.0;
    let (size, ih) = (size * k, ih * k);
    let mut x = cx - full * k / 2.0;
    d.text("Press ", x, y, size, rgb);
    x += head * k;
    for (n, &s) in styles.iter().enumerate() {
        if n > 0 {
            d.text(" / ", x, y, size, rgb);
            x += slash * k;
        }
        let iw = legend::ok_aspect(s) * ih;
        d.q(Tex::Button(s), [0.0; 4], [x, y + size / 2.0 - ih / 2.0, iw, ih], WHITE, 128.0);
        x += iw;
    }
    d.text(tail, x, y, size, rgb);
}

/// A device's line drawing centred on (cx, cy) in white over the tile's fill `bg`: a keyboard, a pad or a phone.
fn device_glyph(d: &mut Draw, glyph: &str, cx: f32, cy: f32, bg: [f32; 3]) {
    let solid = |d: &mut Draw, r: [f32; 4]| d.q(Tex::Solid, [0.0; 4], r, WHITE, 128.0);
    match glyph {
        "keys" => {
            d.box3([cx - 15.0, cy - 7.0, 30.0, 14.0], 3.0, [Some(WHITE), None, None], 128.0);
            d.box3([cx - 13.5, cy - 5.5, 27.0, 11.0], 2.0, [Some(bg), None, None], 128.0);
            for k in 0..6 {
                solid(d, [cx - 11.0 + 4.0 * k as f32, cy - 3.5, 2.0, 2.0]);
            }
            solid(d, [cx - 8.0, cy + 1.0, 16.0, 1.5]);
        }
        "pad" => {
            d.pill_at([cx - 16.0, cy - 8.0, 32.0, 16.0], [WHITE, WHITE, bg], 128.0);
            solid(d, [cx - 9.5, cy - 4.0, 1.6, 8.0]);
            solid(d, [cx - 12.5, cy - 0.8, 7.6, 1.6]);
            for (dx, dy) in [(6.5, -2.0), (9.5, 1.0)] {
                d.pill_at([cx + dx - 1.4, cy + dy - 1.4, 2.8, 2.8], [WHITE, WHITE, WHITE], 128.0);
            }
        }
        _ => {
            d.box3([cx - 7.0, cy - 11.0, 14.0, 22.0], 3.0, [Some(WHITE), None, None], 128.0);
            d.box3([cx - 5.5, cy - 9.5, 11.0, 19.0], 2.0, [Some(bg), None, None], 128.0);
            solid(d, [cx - 2.0, cy + 6.5, 4.0, 1.2]);
        }
    }
}

/// A dashed outline round `[x, y, w, h]` (its straight runs, clear of the `c`-unit corners).
fn dashed(d: &mut Draw, [x, y, w, h]: [f32; 4], c: f32, rgb: [f32; 3]) {
    let (dash, t) = (5.0, 1.5);
    let run = |d: &mut Draw, len: f32, at: &dyn Fn(f32) -> [f32; 4]| {
        let n = ((len - 2.0 * c) / (2.0 * dash)).floor().max(1.0);
        let step = (len - 2.0 * c) / n;
        for k in 0..n as usize {
            d.q(Tex::Solid, [0.0; 4], at(c + step * k as f32 + (step - dash) / 2.0), rgb, 128.0);
        }
    };
    run(d, w, &|s| [x + s, y, dash, t]);
    run(d, w, &|s| [x + s, y + h - t, dash, t]);
    run(d, h, &|s| [x, y + s, t, dash]);
    run(d, h, &|s| [x + w - t, y + s, t, dash]);
}

/// The Mods picture's grid: 7 across, 2 down.
const MOD_COLS: usize = 7;
const MOD_TILES: usize = 2 * MOD_COLS;

/// The installed mods the Mods picture shows, newest first: every one if they fit, else one tile short (the last
/// tile is "+N", the rest).
fn shown_mods(m: &Menu) -> Vec<usize> {
    let order: Vec<usize> = if m.newest.len() == m.mods.len() { m.newest.clone() } else { (0..m.mods.len()).collect() };
    let n = if order.len() > MOD_TILES { MOD_TILES - 1 } else { order.len() };
    order.into_iter().take(n).collect()
}

/// The gold behind a mod's portrait (and its initial before the face is baked), and the initial's.
const TILE_GOLD: [f32; 3] = [104.0, 88.0, 43.0];
const TILE_LETTER: [f32; 3] = [69.0, 53.0, 25.0];

/// The Mods picture's portrait tiles in `[x, y, w, _]`: each mod's face cropped square on gold (its initial until
/// the face is baked), its name along the foot, UPDATE if the store has a newer one; "+N" for the rest.
fn mod_tiles(d: &mut Draw, m: &Menu, [x, y, w, _]: [f32; 4]) {
    if m.mods.is_empty() {
        d.text("No mods installed yet: browse the store or add one.", x + 18.0, y + 20.0, 14.0, INK);
        return;
    }
    let (pad, gap) = (12.0, 4.0);
    let ts = ((w - 2.0 * pad - (MOD_COLS - 1) as f32 * gap) / MOD_COLS as f32).min(50.0);
    let x0 = x + (w - MOD_COLS as f32 * ts - (MOD_COLS - 1) as f32 * gap) / 2.0;
    let ups = mods_screen::updates(m).unwrap_or_default();
    let shown = shown_mods(m);
    let rest = m.mods.len() - shown.len();
    for k in 0..shown.len() + usize::from(rest > 0) {
        let r = [x0 + (ts + gap) * (k % MOD_COLS) as f32, y + 11.0 + (ts + gap) * (k / MOD_COLS) as f32, ts, ts];
        let Some(&i) = shown.get(k) else {
            d.box3(r, 5.0, [Some(INK), None, Some([62.0, 60.0, 75.0])], 128.0);
            d.label(&format!("+{rest}"), r[0] + ts / 2.0, r[1] + ts / 2.0 - 9.0, 18.0, ts - 8.0, WHITE);
            continue;
        };
        let md = &m.mods[i];
        d.box3(r, 5.0, [Some(INK), None, Some(TILE_GOLD)], 128.0);
        let initial = md.name.chars().take(1).collect::<String>().to_uppercase();
        d.centred(&initial, r[0] + ts / 2.0, r[1] + ts * 0.12, ts * 0.56, TILE_LETTER);
        // once baked: the gold again over the initial, then the face's middle (its round icon's corners are clear)
        let inner = [r[0] + 1.5, r[1] + 1.5, ts - 3.0, ts - 3.0];
        let who = Who::Mod(i);
        d.q(Tex::Cover(who), [0.0; 4], [inner[0] + 3.0, inner[1] + 3.0, inner[2] - 6.0, inner[3] - 6.0], TILE_GOLD, 128.0);
        d.q(Tex::Face(who), [8.0, 6.0, 48.0, 48.0], inner, WHITE, 128.0);
        let sh = (ts * 0.2).max(8.0);
        d.box3([inner[0], inner[1] + inner[3] - sh, inner[2], sh], 2.5, [None, None, Some([21.0, 18.0, 24.0])], 92.0);
        d.label(&md.name, r[0] + ts / 2.0, inner[1] + inner[3] - sh + sh * 0.12, sh * 0.85, ts - 6.0, WHITE);
        if ups.contains(&md.dir.as_path()) {
            let bw = 26.0;
            d.box3([r[0] + ts - bw - 2.5, r[1] + 2.5, bw, 8.0], 3.0, [None, None, Some([23.0, 72.0, 45.0])], 128.0);
            d.label("UPDATE", r[0] + ts - bw / 2.0 - 2.5, r[1] + 3.0, 7.0, bw - 4.0, WHITE);
        }
    }
}

/// Every face the main screen's pictures can show (the Rematch's players, the Mods row's tiles), in the order they
/// draw: what `previews.rs` bakes and reads first while this screen is up, so they fill in on their own.
pub(super) fn faces(m: &Menu) -> Vec<Who> {
    let mut v: Vec<Who> = Vec::new();
    if m.rematch {
        v.extend((0..m.players()).map(|p| m.pick(p).0));
    }
    v.extend(shown_mods(m).into_iter().map(Who::Mod));
    let mut seen = std::collections::HashSet::new();
    v.retain(|w| seen.insert(*w));
    v
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
