//! B371: player names, Smash Bros style. △ on the character select opens a name picker over that player's card: the
//! names saved before (most recently used first), New name (typed on the keyboard, or picked from an on-screen letter
//! grid on a pad or a phone) and No name (back to "1P"–"4P"). A name belongs to the device that picked it, so it
//! follows its player from seat to seat and through the matches of this session. The names are kept in `names.txt`
//! beside settings.txt (the browser's localStorage), one a line, and offered again next time.
//!
//! The chosen name replaces the seat tag above the player, set as the original's own tags are (B541,
//! `play::name_tags::Style`): on the select's card, over the player's head before each serve (`play::name_tags`, from
//! the match's `--names`) and on the result screen. Online, each of this machine's
//! players' names goes out with its live pick (`net::Live::player`).

use bevy::prelude::*;

use super::legend::Act as Key;
use super::pointer::Act;
use super::{COLOURS, Dev, Draw, GOLD, INK, Menu, Press, Screen, WHITE};

/// A name's longest (in characters): B541, as wide as fits where the original's "1P" tag goes.
pub(super) const MAX: usize = 6;
/// The most names kept.
const KEEP: usize = 40;
/// The on-screen grid's letters, eight to a row.
const KEYS: &[u8; 40] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -.!";
const COLS: usize = 8;
/// The grid's Back and OK buttons, after the letters.
const BACK: usize = KEYS.len();
const OK: usize = KEYS.len() + 1;
const FILE: &str = "names.txt";

/// The names: those saved, each device's pick and each seat's open picker.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Names {
    /// Saved on disk, most recently picked first.
    pub(super) saved: Vec<String>,
    /// Each device's name (a seat shows the name of the device on it).
    chosen: Vec<(Dev, String)>,
    /// Each seat's picker, when open.
    pub(super) open: [Option<Picker>; 4],
    /// `saved` changed since it was written (`persist`).
    dirty: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Picker {
    /// The list (the saved names, New name, No name) and the row on.
    List(usize),
    /// A new name so far and the grid's key on (the letters, then Back and OK).
    Typing(String, usize),
}

/// A character a name may have: a space or what the menu font draws, but no comma (the match's `--names` separates by
/// them).
fn allowed(c: char) -> bool {
    c == ' ' || c != ',' && super::FONT.iter().any(|r| r.contains(c))
}

/// A name as kept: trimmed, at most [`MAX`] characters, only [`allowed`] ones.
pub(super) fn clean(s: &str) -> String {
    s.chars().filter(|&c| allowed(c)).take(MAX).collect::<String>().trim().to_string()
}

/// `names.txt`'s names (blank lines and `#` comments skipped, the same name once).
fn parse(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in text.lines().filter(|l| !l.trim_start().starts_with('#')) {
        let n = clean(l);
        if !n.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(&n)) && out.len() < KEEP {
            out.push(n);
        }
    }
    out
}

fn text(saved: &[String]) -> String {
    let mut s = "# Hot Shots Tennis player names, the last picked first\n".to_string();
    saved.iter().for_each(|n| s += &format!("{n}\n"));
    s
}

fn path(iso: &str) -> std::path::PathBuf {
    super::Settings::path(iso).with_file_name(FILE)
}

/// The names saved beside settings.txt.
pub(super) fn load(iso: &str) -> Vec<String> {
    crate::store::read_to_string(path(iso)).map(|t| parse(&t)).unwrap_or_default()
}

impl Names {
    fn of(&self, d: Dev) -> Option<&str> {
        self.chosen.iter().find(|(x, _)| *x == d).map(|(_, n)| n.as_str())
    }

    /// `d` takes name `n` (`None`: no name); a name is saved at the top of the list.
    fn set(&mut self, d: Dev, n: Option<String>) {
        self.chosen.retain(|(x, _)| *x != d);
        if let Some(n) = n {
            self.saved.retain(|s| !s.eq_ignore_ascii_case(&n));
            self.saved.insert(0, n.clone());
            self.saved.truncate(KEEP);
            self.chosen.push((d, n));
            self.dirty = true;
        }
    }

    /// `HST_MENU=names` / `names-type` (for `--shot`s): `d` on seat `p` named ANNA, a few names saved, `p`'s picker
    /// open on its list or typing; nothing is written to disk.
    pub(super) fn demo(&mut self, p: usize, d: Dev, typing: bool) {
        self.saved = ["ANNA", "ROBIN", "SAM", "KIM-LI", "JO"].map(String::from).to_vec();
        self.chosen = vec![(d, "ANNA".into())];
        self.open[p] = Some(if typing { Picker::Typing("ROB".into(), 17) } else { Picker::List(1) });
    }
}

impl Menu {
    /// Seat `p`'s player's name: its device's here, or (online) the one another machine sent with its pick.
    pub(super) fn player_name(&self, p: usize) -> Option<String> {
        match self.seats.get(p).copied().flatten() {
            Some(Dev::Open) | None => {
                let r = self.net.room.as_ref()?;
                self.remote(p).filter(|s| s.human())?;
                r.live_of(p).map(|l| clean(&l.player)).filter(|n| !n.is_empty())
            }
            Some(d) => self.names.of(d).map(str::to_string),
        }
    }

    /// The tag above seat `p`'s player: its name, else "1P".."4P".
    #[cfg(test)]
    pub(super) fn seat_tag(&self, p: usize) -> String {
        self.player_name(p).unwrap_or_else(|| format!("{}P", p + 1))
    }

    /// The match's `--names` (match order, as `--chars`), or nothing when nobody has one.
    pub(super) fn name_flags(&self, order: &[usize]) -> Vec<String> {
        let mut names = vec![String::new(); order.len()];
        for (p, &m) in order.iter().enumerate() {
            names[m] = self.player_name(p).unwrap_or_default();
        }
        if names.iter().all(String::is_empty) { vec![] } else { vec!["--names".into(), names.join(",")] }
    }

    /// The online match's `--names`: this machine's players' own (its k-th seat in the room is its k-th device's,
    /// as `netplay::flags` seats them), the others' as their machines sent them with their picks.
    pub(super) fn online_name_flags(&self, room: &crate::net::Room, me: &str) -> Vec<String> {
        let mut mine = (0..self.players()).filter_map(|p| self.seats[p]).map(|d| self.names.of(d).map(str::to_string));
        let seats: Vec<String> = (0..room.seats.len())
            .map(|s| match &room.seats[s] {
                Some(x) if x.peer == me => mine.next().flatten().unwrap_or_default(),
                Some(x) if x.human() => room.live_of(s).map(|l| clean(&l.player)).unwrap_or_default(),
                _ => String::new(),
            })
            .collect();
        let order: &[usize] = if room.doubles { &[0, 2, 1, 3] } else { &[0, 1] };
        let names: Vec<&str> = order.iter().map(|&s| seats.get(s).map_or("", String::as_str)).collect();
        if names.iter().all(|n| n.is_empty()) { vec![] } else { vec!["--names".into(), names.join(",")] }
    }

    /// Whether seat `p`'s picker is open.
    pub(super) fn naming(&self, p: usize) -> bool {
        self.names.open.get(p).is_some_and(Option::is_some)
    }

    /// The device on seat `p`, which a name is for (a computer's seat has none).
    fn name_dev(&self, p: usize) -> Option<Dev> {
        self.seats[p].filter(|d| *d != Dev::Open)
    }

    /// The list's rows: the saved names, then New name, then No name.
    fn rows(&self) -> usize {
        self.names.saved.len() + 2
    }

    /// Saved name `i` is another device's.
    fn taken(&self, p: usize, i: usize) -> bool {
        let Some(n) = self.names.saved.get(i) else { return false };
        self.names.chosen.iter().any(|(d, m)| Some(*d) != self.name_dev(p) && m.eq_ignore_ascii_case(n))
    }

    /// A press on the select from `dev`, which drives seat `p`: Start opens the picker (on a human's own seat); while it's
    /// open the press is the picker's. Whether it took the press.
    pub(super) fn name_step(&mut self, p: usize, dev: Dev, k: Press, out: &mut Vec<super::Effect>) -> bool {
        use super::Effect::Sound;
        let Some(picker) = self.names.open[p].clone() else {
            if k.name && self.name_dev(p) == Some(dev) {
                let at = self.player_name(p).and_then(|n| self.names.saved.iter().position(|s| *s == n));
                let first = (0..self.names.saved.len()).find(|&i| !self.taken(p, i)).unwrap_or(self.names.saved.len());
                self.names.open[p] = Some(Picker::List(at.unwrap_or(first)));
                out.push(Sound(0));
                return true;
            }
            return false;
        };
        let Some(d) = self.name_dev(p) else {
            self.names.open[p] = None;
            return false;
        };
        match picker {
            Picker::List(sel) => {
                let (rows, saved) = (self.rows(), self.names.saved.len());
                if k.up || k.down {
                    self.names.open[p] = Some(Picker::List((sel + if k.down { 1 } else { rows - 1 }) % rows));
                    out.push(Sound(0));
                } else if k.ok && sel == saved {
                    self.names.open[p] = Some(Picker::Typing(String::new(), 0));
                    out.push(Sound(0));
                } else if k.ok && sel > saved {
                    self.names.set(d, None);
                    self.names.open[p] = None;
                    out.push(Sound(1));
                } else if k.ok && !self.taken(p, sel) {
                    let n = self.names.saved[sel].clone();
                    self.names.set(d, Some(n));
                    self.names.open[p] = None;
                    out.push(Sound(2));
                } else if k.ok {
                    out.push(Sound(1));
                } else if k.page && sel < saved {
                    // □ forgets a saved name (whoever has it keeps it for now)
                    self.names.saved.remove(sel);
                    self.names.dirty = true;
                    self.names.open[p] = Some(Picker::List(sel.min(self.rows() - 1)));
                    out.push(Sound(1));
                } else if k.back {
                    self.names.open[p] = None;
                    out.push(Sound(1));
                }
            }
            Picker::Typing(mut entry, sel) => {
                let rows = KEYS.len() / COLS;
                let mut to = sel;
                if k.left || k.right {
                    to = if sel < KEYS.len() { sel / COLS * COLS + (sel % COLS + if k.right { 1 } else { COLS - 1 }) % COLS } else { BACK + OK - sel };
                } else if k.up || k.down {
                    let (row, col) = if sel < KEYS.len() { (sel / COLS, sel % COLS) } else { (rows, (sel - BACK) * COLS / 2) };
                    let row = (row + if k.down { 1 } else { rows }) % (rows + 1);
                    to = if row == rows { BACK + col * 2 / COLS } else { row * COLS + col };
                }
                if to != sel {
                    self.names.open[p] = Some(Picker::Typing(entry, to));
                    out.push(Sound(0));
                    return true;
                }
                if k.ok && sel == OK {
                    let n = clean(&entry);
                    if !n.is_empty() && self.names.chosen.iter().any(|(x, m)| *x != d && m.eq_ignore_ascii_case(&n)) {
                        // someone else has it
                        out.push(Sound(1));
                        return true;
                    }
                    self.names.set(d, Some(n).filter(|n| !n.is_empty()));
                    self.names.open[p] = None;
                    out.push(Sound(2));
                } else if k.ok && sel == BACK || k.back {
                    let at = self.names.saved.len();
                    self.names.open[p] = Some(Picker::List(at));
                    out.push(Sound(1));
                } else if k.ok {
                    if entry.chars().count() < MAX {
                        entry.push(KEYS[sel] as char);
                        self.names.open[p] = Some(Picker::Typing(entry, sel));
                    } else {
                        // full: on to OK
                        self.names.open[p] = Some(Picker::Typing(entry, OK));
                    }
                    out.push(Sound(0));
                } else if k.page && entry.pop().is_some() {
                    self.names.open[p] = Some(Picker::Typing(entry, sel));
                    out.push(Sound(1));
                }
            }
        }
        true
    }

    /// A character typed on the keyboard into seat `p`'s new name; Backspace (`None`) takes the last one off.
    fn type_name(&mut self, p: usize, c: Option<char>) {
        if let Some(Picker::Typing(entry, _)) = &mut self.names.open[p] {
            match c {
                None => _ = entry.pop(),
                Some(c) if entry.chars().count() < MAX && allowed(c) => entry.push(c),
                Some(_) => {}
            }
        }
    }

    /// The legend while seat `p`'s picker is open (`None`: it isn't).
    pub(super) fn name_acts(&self, p: usize, dev: Dev) -> Option<Vec<(Key, &'static str)>> {
        let picker = self.names.open.get(p)?.as_ref()?;
        let mut v = Vec::new();
        match picker {
            Picker::List(sel) => {
                v.push((Key::UpDown, "Move"));
                if !self.taken(p, *sel) {
                    v.push((Key::Ok, "Select"));
                }
                if *sel < self.names.saved.len() {
                    v.push((Key::Page, "Forget"));
                }
                v.push((Key::Back, "Cancel"));
            }
            Picker::Typing(entry, sel) => {
                if dev == Dev::Keys {
                    v.push((Key::Type, "Type"));
                }
                // ✕ types the letter under the cursor, or presses the grid's OK / Back
                v.extend([(Key::Move, "Letter"), (Key::Ok, if *sel == OK { "OK" } else if *sel == BACK { "Back" } else { "Type" })]);
                if !entry.is_empty() {
                    v.push((Key::Page, "Delete"));
                }
                v.push((Key::Back, "Back"));
            }
        }
        Some(v)
    }

    /// A pointer act on seat `p`'s picker: row or key `i` hovered, or tapped (its ✕).
    pub(super) fn name_point(&mut self, p: usize, i: usize, tap: bool) -> Vec<super::Effect> {
        let mut out = Vec::new();
        // the picker's own background: taken, nothing done
        if i == usize::MAX {
            return out;
        }
        let moved = match &mut self.names.open[p] {
            Some(Picker::List(sel) | Picker::Typing(_, sel)) => *sel != std::mem::replace(sel, i),
            None => return out,
        };
        if moved {
            out.push(super::Effect::Sound(0));
        }
        if tap && let Some(d) = self.name_dev(p) {
            self.name_step(p, d, Press { ok: true, ..default() }, &mut out);
        }
        out
    }
}

/// Seat `p`'s picker over `[x, y, w, h]` (its card and the roster above it), when open.
pub(super) fn draw(d: &mut Draw, m: &Menu, p: usize, [x, y, w, h]: [f32; 4]) {
    let Some(picker) = &m.names.open[p] else { return };
    const ON: [f32; 3] = [128.0, 110.0, 40.0];
    const OFF: [f32; 3] = [70.0, 70.0, 80.0];
    d.box3([x, y, w, h], 12.0, [Some(INK), Some(COLOURS[p]), Some([21.0, 18.0, 24.0])], 124.0);
    // swallow the taps on the roster beneath
    d.hit([x, y, w, h], Act::Name(p, usize::MAX));
    let title = format!("{}P's name", p + 1);
    d.outlined(|d| d.label(&title, x + w / 2.0, y + 8.0, 20.0, w - 16.0, WHITE));
    let (ix, iw) = (x + 8.0, w - 16.0);
    match picker {
        Picker::List(sel) => {
            let mut rows: Vec<(String, bool)> = m.names.saved.iter().enumerate().map(|(i, n)| (n.clone(), m.taken(p, i))).collect();
            rows.extend([("New name...".into(), false), ("No name".into(), false)]);
            let (top, pitch) = (y + 36.0, 30.0);
            let fit = (((y + h - 8.0 - top) / pitch).floor() as usize).max(1);
            let first = (*sel + 1).saturating_sub(fit).min(rows.len().saturating_sub(fit));
            let now = m.player_name(p);
            for (i, (n, taken)) in rows.iter().enumerate().skip(first).take(fit) {
                let ry = top + pitch * (i - first) as f32;
                let on = i == *sel;
                d.pill_at([ix, ry, iw, 26.0], [INK, if on { WHITE } else { [60.0; 3] }, if on { ON } else { OFF }], if *taken { 64.0 } else { 128.0 });
                let rgb = if *taken { [60.0; 3] } else if now.as_deref() == Some(n.as_str()) { GOLD } else if on { WHITE } else { [100.0; 3] };
                d.label(n, x + w / 2.0, ry + 4.0, 18.0, iw - 16.0, rgb);
                d.hit([ix, ry, iw, 26.0], Act::Name(p, i));
            }
            if rows.len() > fit {
                // more above / below
                if first > 0 {
                    d.centred("^", x + w - 14.0, top - 18.0, 16.0, WHITE);
                }
                if first + fit < rows.len() {
                    d.centred("v", x + w - 14.0, y + h - 18.0, 16.0, WHITE);
                }
            }
        }
        Picker::Typing(entry, sel) => {
            // the name so far, the cursor after it
            d.pill_at([ix, y + 34.0, iw, 28.0], [INK, WHITE, [40.0, 40.0, 50.0]], 128.0);
            let shown = if entry.chars().count() < MAX { format!("{entry}_") } else { entry.clone() };
            d.label(&shown, x + w / 2.0, y + 38.0, 20.0, iw - 12.0, WHITE);
            let grid = !super::online::touch_typing();
            let top = y + 70.0;
            let rows = KEYS.len() / COLS;
            let kw = iw / COLS as f32;
            let kh = ((y + h - 8.0 - top) / (rows + 1) as f32).min(kw).min(30.0);
            for (i, c) in KEYS.iter().enumerate().filter(|_| grid) {
                let (kx, ky) = (ix + kw * (i % COLS) as f32, top + kh * (i / COLS) as f32);
                let on = *sel == i;
                d.box3([kx + 1.0, ky + 1.0, kw - 2.0, kh - 2.0], 4.0, [Some(INK), None, Some(if on { ON } else { OFF })], 128.0);
                let label = if *c == b' ' { "_".to_string() } else { (*c as char).to_string() };
                d.label(&label, kx + kw / 2.0, ky + kh * 0.15, kh * 0.7, kw - 4.0, if on { WHITE } else { [100.0; 3] });
                d.hit([kx, ky, kw, kh], Act::Name(p, i));
            }
            let by = top + kh * rows as f32 + 4.0;
            for (i, l) in [(BACK, "Back"), (OK, "OK")] {
                let bx = ix + iw / 2.0 * (i - BACK) as f32;
                let on = *sel == i;
                d.pill_at([bx + 1.0, by, iw / 2.0 - 2.0, 26.0], [INK, WHITE, if on { ON } else { OFF }], 128.0);
                d.label(l, bx + iw / 4.0, by + 4.0, 18.0, iw / 2.0 - 12.0, if on { WHITE } else { [100.0; 3] });
                d.hit([bx, by, iw / 2.0, 26.0], Act::Name(p, i));
            }
        }
    }
}

/// A keyboard player's new name: typed letters, Backspace and Enter (OK) go into it, not to the menu's key bindings
/// (Esc still goes back to the list).
pub(super) fn typing(
    mut menu: ResMut<Menu>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut typed: MessageReader<bevy::input::keyboard::KeyboardInput>,
    gamepads: Query<(Entity, &Gamepad)>,
) {
    use bevy::input::keyboard::Key;
    let typing = |m: &Menu, d: Dev| (0..4).find(|&p| m.seats[p] == Some(d) && matches!(m.names.open[p], Some(Picker::Typing(..))));
    if menu.screen != Screen::Chars {
        typed.clear();
        return;
    }
    // Start on a pad finishes its player's entry, as Enter: the cursor onto OK, which its ✕ (Start is one) takes
    for (e, g) in &gamepads {
        if g.just_pressed(GamepadButton::Start) && let Some(p) = typing(&menu, Dev::Pad(e)) && let Some(Picker::Typing(_, sel)) = &mut menu.names.open[p] {
            *sel = OK;
        }
    }
    let Some(p) = typing(&menu, Dev::Keys) else {
        typed.clear();
        return;
    };
    for e in typed.read().filter(|e| e.state.is_pressed()) {
        match &e.logical_key {
            Key::Character(c) => {
                if !e.repeat {
                    c.chars().for_each(|c| menu.type_name(p, Some(c)));
                }
                keys.clear_just_pressed(e.key_code);
            }
            Key::Space => {
                menu.type_name(p, Some(' '));
                keys.clear_just_pressed(e.key_code);
            }
            Key::Backspace => {
                menu.type_name(p, None);
                keys.clear_just_pressed(e.key_code);
            }
            // Enter: OK, which the menu's ✕ then takes
            Key::Enter => {
                if let Some(Picker::Typing(_, sel)) = &mut menu.names.open[p] {
                    *sel = OK;
                }
            }
            _ => {}
        }
    }
}

/// B541: each seat's name set as the original's slot-label tag (`play::name_tags::Style::Plate`) for its select card
/// (`Tex::Name`), with the name it was set for.
#[derive(Resource, Default)]
pub(super) struct NameArt(pub(super) [Option<(String, Handle<Image>)>; 4]);

/// Sets a seat's tag again when its name changes, and drops it when the name goes.
pub(super) fn name_art(menu: Res<Menu>, mut art: ResMut<NameArt>, mut images: ResMut<Assets<Image>>) {
    for p in 0..4 {
        let name = menu.player_name(p);
        if art.0[p].as_ref().map(|a| &a.0) != name.as_ref() {
            art.0[p] = name.map(|n| (n.clone(), images.add(crate::play::name_tags::tag_image(&n, crate::play::name_tags::Style::Plate, 1.0))));
        }
    }
}

/// Where `name`'s tag goes on the select card with its words' box `h` high (menu units) from `[x, y]`: the whole
/// image, accents' rows over `y` included, at its own aspect.
pub(super) fn tag_rect(name: &str, [x, y]: [f32; 2], h: f32) -> [f32; 4] {
    let ([w, th], above) = crate::play::name_tags::tag_size(name, crate::play::name_tags::Style::Plate);
    let k = h / crate::play::name_tags::PLATE_H;
    [x, y - above * k, w * k / super::roster::PY * super::roster::PX, th * k]
}

/// Writes the saved names when they change; closes the pickers off the select.
pub(super) fn persist(mut menu: ResMut<Menu>, args: Res<crate::Args>) {
    if menu.screen != Screen::Chars && menu.names.open.iter().any(Option::is_some) {
        menu.names.open = default();
    }
    if menu.names.dirty {
        menu.names.dirty = false;
        let p = path(&args.iso);
        if let Err(e) = crate::store::write(&p, text(&menu.names.saved)) {
            warn!("{}: {e}", p.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu() -> Menu {
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let mut m = Menu { screen: Screen::Chars, seats: [Some(Dev::Keys), Some(pad), None, None], ..default() };
        m.names.saved = vec!["ANNA".into(), "Bo".into()];
        m
    }

    fn press(m: &mut Menu, p: usize, f: impl FnOnce(&mut Press)) -> Vec<super::super::Effect> {
        let mut k = Press::default();
        f(&mut k);
        let dev = m.seats[p].unwrap();
        m.step(dev, k)
    }

    #[test]
    fn the_file_round_trips() {
        let saved = parse("# x\nANNA\n\n Bo \nanna\nlong name over ten,\n");
        assert_eq!(saved, ["ANNA", "Bo", "long n"]);
        assert_eq!(parse(&text(&saved)), saved);
    }

    #[test]
    fn a_saved_name_replaces_the_tag() {
        let mut m = menu();
        assert_eq!(m.seat_tag(0), "1P");
        press(&mut m, 0, |k| k.name = true);
        assert_eq!(m.names.open[0], Some(Picker::List(0)));
        press(&mut m, 0, |k| k.down = true);
        press(&mut m, 0, |k| k.ok = true);
        assert_eq!((m.seat_tag(0), m.naming(0)), ("Bo".to_string(), false));
        // the last picked comes first, and goes to disc
        assert_eq!(m.names.saved, ["Bo", "ANNA"]);
        assert!(m.names.dirty);
        // 2P can't have it too
        press(&mut m, 1, |k| k.name = true);
        assert_eq!(m.names.open[1], Some(Picker::List(1)), "on the first free name");
        assert!(m.taken(1, 0));
        press(&mut m, 1, |k| k.up = true);
        press(&mut m, 1, |k| k.ok = true);
        assert_eq!(m.player_name(1), None);
        // the name follows the device to another seat
        m.seats.swap(0, 1);
        assert_eq!((m.seat_tag(0), m.seat_tag(1)), ("1P".to_string(), "Bo".to_string()));
    }

    /// The match gets the names in its own order; online, this machine's seats take its devices' names in turn and
    /// the others' come from their machines' live picks (a computer seat none).
    #[test]
    fn names_go_to_the_match_by_seat() {
        use crate::net::{Live, Pick, Room, Seat};
        let mut m = menu();
        assert!(m.name_flags(&[0, 1]).is_empty(), "nobody named: no flag");
        m.names.set(Dev::Pad(Entity::from_raw_u32(9).unwrap()), Some("Bo".into()));
        assert_eq!(m.name_flags(&[1, 0]), ["--names", "Bo,"]);
        let seat = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let live = |player: &str| Live { char: 0, costume: 0, ready: true, r#mod: None, name: String::new(), player: player.into(), ..Default::default() };
        let mut room = Room { doubles: true, seats: vec![seat("them"), seat("me"), seat(crate::net::CPU), seat("me")], ..default() };
        room.live.insert("them".into(), vec![live("ZED")]);
        // match order 1P 3P 2P 4P: ZED, the computer, this machine's keyboard (no name), its pad
        assert_eq!(m.online_name_flags(&room, "me"), ["--names", "ZED,,,Bo"]);
    }

    #[test]
    fn a_new_name_is_typed_and_no_name_clears_it() {
        let mut m = menu();
        press(&mut m, 1, |k| k.name = true);
        press(&mut m, 1, |k| k.up = true);
        press(&mut m, 1, |k| k.up = true);
        assert_eq!(m.names.open[1], Some(Picker::List(2)), "New name");
        press(&mut m, 1, |k| k.ok = true);
        // J (row 1, col 1), then Z (row 3, col 1)
        press(&mut m, 1, |k| k.down = true);
        press(&mut m, 1, |k| k.right = true);
        press(&mut m, 1, |k| k.ok = true);
        press(&mut m, 1, |k| k.down = true);
        press(&mut m, 1, |k| k.down = true);
        press(&mut m, 1, |k| k.ok = true);
        assert_eq!(m.names.open[1], Some(Picker::Typing("JZ".into(), 25)));
        // □ deletes
        press(&mut m, 1, |k| k.page = true);
        // down twice to the buttons (col 1: Back), right: OK
        press(&mut m, 1, |k| k.down = true);
        press(&mut m, 1, |k| k.down = true);
        press(&mut m, 1, |k| k.right = true);
        press(&mut m, 1, |k| k.ok = true);
        assert_eq!(m.seat_tag(1), "J");
        assert_eq!(m.names.saved, ["J", "ANNA", "Bo"]);
        assert_eq!(m.name_flags(&[0, 1]), ["--names", ",J"]);
        // No name, last on the list
        press(&mut m, 1, |k| k.name = true);
        press(&mut m, 1, |k| k.up = true);
        press(&mut m, 1, |k| k.ok = true);
        assert_eq!(m.seat_tag(1), "2P");
        assert!(m.name_flags(&[0, 1]).is_empty());
    }

    #[test]
    fn the_keyboard_types_into_its_entry() {
        let mut m = menu();
        m.names.open[0] = Some(Picker::Typing(String::new(), 0));
        // the comma is left out, the rest stops at six, then a Backspace
        for c in "Ann, Lee!xyz".chars() {
            m.type_name(0, Some(c));
        }
        m.type_name(0, None);
        assert_eq!(m.names.open[0], Some(Picker::Typing("Ann L".into(), 0)));
    }
}
