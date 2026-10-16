//! B382: the character select's roster as one scrolling grid, each player's own view of it: every character under
//! a divider heading per section with its count — ★ Favorites (only while there are any), Hot Shots Tennis (the
//! Random card, then the disc cast), then one per mod game (`Mod::game`, "Mods" for an untagged mod) in name order,
//! "Mods" last. Each section starts on a row of its own. ←→ walk the tiles in order across the rows and sections,
//! stopping at the ends; ↑↓ a row, the same place across (the row's last if it's shorter), headings passed over;
//! L2 / R2 (keyboard 1 / 3; L1 / R1 stay the costume) jump to the previous / next section's first tile, wrapping;
//! a tap on a heading jumps there too. A favorite shows in Favorites and in its own section: `Player::tab` is the
//! section the cursor is in. □ marks the highlighted character a favorite (or not): saved in `favorites.txt` beside
//! `settings.txt`, a star on its tile. (B369's per-game tabs and B391's tab row are gone: squished, unclear.)

use super::{Draw, Effect, GOLD, Menu, Press, Who};
#[cfg(test)]
use super::pointer;

pub(super) const FAVORITES: u8 = 1;
pub(super) const HOT_SHOTS: u8 = 2;
/// The disc's own section's heading: a mod made from this game's own characters (its `source`) goes in it too,
/// after the disc cast.
const HST: &str = "Hot Shots Tennis";
/// Mod game `k` (in [`Menu::games`]' order) is section id `GAMES + k`.
const GAMES: u8 = 3;

/// One section: its id (`Player::tab`), its heading, and its tiles (`None` the Random card).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Section {
    pub id: u8,
    pub name: String,
    pub cells: Vec<Option<Who>>,
}

/// A line of the grid: a section's heading, or a row of its tiles from its `i`th.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Line {
    Head(usize),
    Tiles(usize, usize),
}

/// The grid's lines for `secs`, `cols` tiles a row.
pub(super) fn lines(secs: &[Section], cols: usize) -> Vec<Line> {
    let cols = cols.max(1);
    let mut out = Vec::new();
    for (s, sec) in secs.iter().enumerate() {
        out.push(Line::Head(s));
        out.extend((0..sec.cells.len()).step_by(cols).map(|i| Line::Tiles(s, i)));
    }
    out
}

/// Tile `i` of section `s` as a place in the whole grid: every section's tiles one after another (`Act::Cell`).
pub(super) fn flat(secs: &[Section], s: usize, i: usize) -> usize {
    secs[..s].iter().map(|t| t.cells.len()).sum::<usize>() + i
}

/// The other way: the section and tile at place `k`.
pub(super) fn unflat(secs: &[Section], mut k: usize) -> Option<(usize, usize)> {
    for (s, sec) in secs.iter().enumerate() {
        if k < sec.cells.len() {
            return Some((s, k));
        }
        k -= sec.cells.len();
    }
    None
}

/// The favorite characters, by key ([`Menu::fav_key`]); sorted, so a `Menu`'s `Debug` is the same every time.
pub(super) type Favorites = std::collections::BTreeSet<String>;

/// Where the favorites are saved: beside `settings.txt`.
pub(super) fn path(iso: &str) -> std::path::PathBuf {
    super::Settings::path(iso).with_file_name("favorites.txt")
}

/// The saved favorites (none if there's no file yet).
pub(super) fn load(iso: &str) -> Favorites {
    crate::store::read_to_string(path(iso))
        .map(|t| parse(&t))
        .unwrap_or_default()
}

fn parse(text: &str) -> Favorites {
    text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(String::from).collect()
}

pub(super) fn text(favs: &Favorites) -> String {
    favs.iter().map(|k| format!("{k}\n")).collect()
}

impl Menu {
    /// A favorite's key: a disc character by number, a mod by its folder (B248's unique id).
    fn fav_key(&self, w: Who) -> String {
        match w {
            Who::Disc(c) => format!("hst {c}"),
            Who::Mod(i) => format!("mod {}", self.mods[i].folder()),
        }
    }

    pub(super) fn favorite(&self, w: Who) -> bool {
        self.favs.contains(&self.fav_key(w))
    }

    /// The mod games with a section, in order: by name, "Mods" (untagged) last.
    fn games(&self) -> Vec<&str> {
        let mut g: Vec<&str> = (0..self.own_mods()).map(|i| self.mods[i].game()).filter(|&g| g != HST).collect();
        g.sort_by_key(|&n| (n == "Mods", n));
        g.dedup();
        g
    }

    /// Every section, in order.
    pub(super) fn sections(&self) -> Vec<Section> {
        let all = self.roster();
        let mut out = Vec::new();
        let favs: Vec<Option<Who>> = all.iter().copied().filter(|c| c.is_some_and(|w| self.favorite(w))).collect();
        if !favs.is_empty() {
            out.push(Section { id: FAVORITES, name: "Favorites".into(), cells: favs });
        }
        out.push(Section { id: HOT_SHOTS, name: HST.into(), cells: all.iter().copied().filter(|c| !matches!(c, Some(Who::Mod(i)) if self.mods[*i].game() != HST)).collect() });
        for (k, g) in self.games().into_iter().enumerate() {
            let cells = all.iter().copied().filter(|c| matches!(c, Some(Who::Mod(i)) if self.mods[*i].game() == g)).collect();
            out.push(Section { id: GAMES + k as u8, name: g.to_string(), cells });
        }
        out
    }

    /// The section a character lives in (never Favorites; the Random card's is Hot Shots Tennis).
    fn home(&self, c: Option<Who>) -> u8 {
        match c {
            None | Some(Who::Disc(_)) => HOT_SHOTS,
            Some(Who::Mod(i)) if self.mods[i].game() == HST => HOT_SHOTS,
            Some(Who::Mod(i)) => GAMES + self.games().iter().position(|&g| g == self.mods[i].game()).unwrap_or(0) as u8,
        }
    }

    /// Player `p`'s section in `secs`: theirs (`Player::tab`) while their tile is in it, else the tile's own.
    pub(super) fn section_at(&self, p: usize, secs: &[Section]) -> usize {
        let c = self.cell(p);
        let id = self.players[p].tab;
        let home = self.home(c);
        secs.iter().position(|s| s.id == id && s.cells.contains(&c)).or_else(|| secs.iter().position(|s| s.id == home)).unwrap_or(0)
    }

    /// Player `p`'s place in the grid ([`flat`]), if their tile is on it.
    pub(super) fn place(&self, p: usize, secs: &[Section]) -> Option<usize> {
        let s = self.section_at(p, secs);
        secs[s].cells.iter().position(|&c| c == self.cell(p)).map(|i| flat(secs, s, i))
    }

    /// Player `p` onto place `k` (a new character on its first costume).
    pub(super) fn put_at(&mut self, p: usize, secs: &[Section], k: usize) {
        let Some((s, i)) = unflat(secs, k) else { return };
        self.players[p].tab = secs[s].id;
        let c = secs[s].cells[i];
        if c != self.cell(p) {
            self.put(p, c);
        }
    }

    /// Player `p` (not ready) onto section `s`'s first open tile; whether there was one.
    pub(super) fn jump(&mut self, p: usize, secs: &[Section], s: usize) -> bool {
        let Some(i) = secs[s].cells.iter().position(|&c| !self.full(p, c)) else { return false };
        self.put_at(p, secs, flat(secs, s, i));
        true
    }

    /// The tiles across player `p`'s grid (B368: every column has as many).
    fn cols(&self) -> usize {
        self.columns().cols.max(1)
    }

    /// B382: a not-ready player's ↑↓←→ and L2 / R2 on the grid (B66: past a full tile the same way): ←→ the next
    /// tile in order, across rows and sections, stopping at the ends; ↑↓ a row, headings passed over; L2 this
    /// section's first tile (the previous section's from there), R2 the next section's, wrapping. Whether the cursor
    /// moved.
    pub(super) fn grid_move(&mut self, p: usize, k: Press) -> bool {
        let (dx, dy) = (k.right as i32 - k.left as i32, k.down as i32 - k.up as i32);
        let jump = k.kind_next as i32 - k.kind_prev as i32;
        if dx == 0 && dy == 0 && jump == 0 {
            return false;
        }
        let secs = self.sections();
        let Some(at) = self.place(p, &secs) else { return false };
        let (s, i) = unflat(&secs, at).unwrap_or((0, 0));
        if jump != 0 && secs.len() < 2 {
            return false;
        }
        if jump != 0 {
            let first = secs[s].cells.iter().position(|&c| !self.full(p, c));
            if jump < 0 && first.is_some_and(|f| f < i) {
                return self.jump(p, &secs, s);
            }
            let n = secs.len();
            return (1..n).map(|d| if jump > 0 { (s + d) % n } else { (s + n - d) % n }).any(|u| self.jump(p, &secs, u));
        }
        let to = if dy != 0 {
            self.row_step(p, &secs, s, i, dy)
        } else {
            let n = secs.iter().map(|t| t.cells.len()).sum::<usize>() as i32;
            let cell = |k: i32| unflat(&secs, k as usize).map(|(s, i)| secs[s].cells[i]);
            let mut to = at as i32 + dx;
            while (0..n).contains(&to) && cell(to).is_some_and(|c| self.full(p, c)) {
                to += dx;
            }
            (0..n).contains(&to).then_some(to as usize)
        };
        match to {
            Some(t) if t != at => {
                self.put_at(p, &secs, t);
                true
            }
            _ => false,
        }
    }

    /// The tile `dy` rows down from section `s`'s `i`th (up for -1): the same place across, the row's last if it's
    /// shorter, past rows whose tile there has nothing open; past the grid's ends its last (first) tile, if open (B306).
    fn row_step(&self, p: usize, secs: &[Section], s: usize, i: usize, dy: i32) -> Option<usize> {
        let cols = self.cols();
        let rows: Vec<(usize, usize)> = lines(secs, cols).into_iter().filter_map(|l| if let Line::Tiles(s, i) = l { Some((s, i)) } else { None }).collect();
        let col = i % cols;
        let mut r = rows.iter().position(|&row| row == (s, i - col))? as i32;
        loop {
            r += dy;
            let Some(&(rs, ri)) = usize::try_from(r).ok().and_then(|r| rows.get(r)) else {
                let (s, i) = if dy > 0 { (secs.len() - 1, secs[secs.len() - 1].cells.len().checked_sub(1)?) } else { (0, 0) };
                return (!self.full(p, secs[s].cells[i])).then(|| flat(secs, s, i));
            };
            let i = ri + col.min((secs[rs].cells.len() - ri).min(cols) - 1);
            if !self.full(p, secs[rs].cells[i]) {
                return Some(flat(secs, rs, i));
            }
        }
    }

    /// B369: □: the highlighted character in or out of the favorites (saved). Taken off while in Favorites, the
    /// cursor stays at that place in it (or goes to the character's own section once it's empty). B382: Favorites
    /// grows or shrinks over every column's grid: each one's view keeps its place.
    pub(super) fn toggle_favorite(&mut self, p: usize, out: &mut Vec<Effect>) {
        let Some(w) = self.cell(p) else { return };
        let key = self.fav_key(w);
        let secs = self.sections();
        let cols = self.cols();
        let fav_lines = |secs: &[Section]| secs.iter().find(|s| s.id == FAVORITES).map_or(0, |s| 1 + s.cells.len().div_ceil(cols));
        let was = fav_lines(&secs);
        let s = self.section_at(p, &secs);
        let at = secs[s].cells.iter().position(|&c| c == Some(w)).unwrap_or(0);
        let on_favs = secs[s].id == FAVORITES;
        if !self.favs.remove(&key) {
            self.favs.insert(key);
        }
        let secs = self.sections();
        let now = fav_lines(&secs);
        for q in 0..self.players() {
            if self.scroll[q] >= was {
                self.scroll[q] = (self.scroll[q] + now).saturating_sub(was);
            }
        }
        if on_favs {
            match secs.iter().position(|s| s.id == FAVORITES) {
                Some(t) => {
                    let cells = &secs[t].cells;
                    if let Some(i) = (at.min(cells.len() - 1)..cells.len()).chain(0..at).find(|&i| !self.full(p, cells[i])) {
                        self.put_at(p, &secs, flat(&secs, t, i));
                    }
                }
                None => self.players[p].tab = self.home(Some(w)),
            }
        }
        for q in 0..self.players() {
            self.follow(q);
        }
        out.extend([Effect::Sound(0), Effect::SaveFavorites]);
    }
}

/// Player `p`'s footer: the section and where the tile is in it (`Golf   3 of 7`).
pub(super) fn foot(m: &Menu, p: usize) -> String {
    let secs = m.sections();
    let s = m.section_at(p, &secs);
    let at = secs[s].cells.iter().position(|&c| c == m.cell(p)).unwrap_or(0) + 1;
    format!("{}   {at} of {}", secs[s].name, secs[s].cells.len())
}

/// A favorite's star in a tile's top-right corner (the font's `*`, gold over a dark ring).
pub(super) fn star(d: &mut Draw, [x, y, w, _]: [f32; 4]) {
    let s = (w * 0.34).clamp(12.0, 22.0);
    d.outlined(|d| d.centred("*", x + w - s * 0.45, y + 1.0, s, GOLD));
}

#[cfg(test)]
mod tests {
    use super::super::{CHARS, Dev, GRID, Grid, Screen, default};
    use super::*;

    fn press(f: impl FnOnce(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    fn a_mod(i: usize, game: &str) -> crate::mods::Mod {
        crate::mods::Mod {
            dir: format!("mods/m{i}").into(),
            id: format!("m{i}"),
            name: format!("M{i}"),
            source: game.into(),
            costumes: vec!["model/c00.glb".into()],
            donor: i % CHARS,
            hand: 1.0,
            base: i % CHARS,
            overrides: vec![],
            ai_row: i % CHARS,
            texture_face: false,
            no_face: false,
            voice: "voice/".into(),
            motions: None,
            costume_of: None,
            skeleton: None,
        }
    }

    /// A mod from the disc's own game: in Hot Shots Tennis, after the cast, no section of its own.
    #[test]
    fn the_games_own_mods_join_its_section() {
        let mut m = menu();
        m.mods.push(a_mod(12, "Hot Shots Tennis"));
        assert_eq!(names(&m)[..2], [("Hot Shots Tennis".into(), 2 + CHARS), ("Get a Grip".into(), 2)]);
        assert_eq!(m.sections()[0].cells.last(), Some(&Some(Who::Mod(12))));
        m.put(0, Some(Who::Mod(12)));
        assert_eq!(at(&m), ("Hot Shots Tennis".into(), 1 + CHARS));
    }

    /// 1P on the keyboard, singles, 5 tiles a row; mods: 3 untagged, 7 from Golf, 2 from Get a Grip.
    fn menu() -> Menu {
        let mods = (0..12).map(|i| a_mod(i, if i < 3 { "" } else if i < 10 { "Golf" } else { "Get a Grip" })).collect();
        let mut m = Menu { screen: Screen::Chars, mods, seats: [Some(Dev::Keys), None, None, None], ..default() };
        m.players[0] = super::super::Player::fresh();
        // B368: a column narrow enough for 5 a row
        m.grid = Grid::fit(4.0 / 3.0);
        while m.columns().cols != 5 {
            m.grid.view[2] += if m.columns().cols < 5 { 1.0 } else { -1.0 };
        }
        assert_eq!(m.columns().cols, 5);
        m
    }

    fn names(m: &Menu) -> Vec<(String, usize)> {
        m.sections().into_iter().map(|t| (t.name, t.cells.len())).collect()
    }

    /// Where 1P is: their section's name and the tile's place in it.
    fn at(m: &Menu) -> (String, usize) {
        let secs = m.sections();
        let s = m.section_at(0, &secs);
        (secs[s].name.clone(), secs[s].cells.iter().position(|&c| c == m.cell(0)).unwrap())
    }

    #[test]
    fn sections_in_order_with_counts() {
        let mut m = menu();
        assert_eq!(names(&m), [("Hot Shots Tennis".into(), 1 + CHARS), ("Get a Grip".into(), 2), ("Golf".into(), 7), ("Mods".into(), 3)]);
        // B390: the select opens on the Random card, Hot Shots Tennis' first
        assert_eq!(at(&m), ("Hot Shots Tennis".into(), 0));
        assert_eq!(m.sections()[0].cells[..2], [None, Some(Who::Disc(GRID[0].0))]);
        // each section on rows of its own under its heading
        let l = lines(&m.sections(), 5);
        assert_eq!(l[..5], [Line::Head(0), Line::Tiles(0, 0), Line::Tiles(0, 5), Line::Tiles(0, 10), Line::Head(1)]);
        assert_eq!(l.len(), 4 + (1 + CHARS).div_ceil(5) + 1 + 2 + 1);
        // a favorite puts Favorites first; the cursor stays in the character's own section
        m.put(0, Some(Who::Mod(4)));
        let mut out = Vec::new();
        m.toggle_favorite(0, &mut out);
        assert_eq!(out, [Effect::Sound(0), Effect::SaveFavorites]);
        assert_eq!(names(&m)[0], ("Favorites".into(), 1));
        assert_eq!(at(&m), ("Golf".into(), 1));
    }

    /// ←→ walk every tile in order across rows and sections, stopping at the grid's ends; ↑↓ keep the place across
    /// (the row's last on a shorter row), passing the headings.
    #[test]
    fn moves_cross_rows_and_sections() {
        let mut m = menu();
        m.players[0].random = false;
        let k = |m: &mut Menu, f: fn(&mut Press) -> &mut bool| {
            m.step(Dev::Keys, press(f));
            at(m)
        };
        let hst = "Hot Shots Tennis".to_string();
        let last = CHARS; // the Random card first
        m.put(0, m.sections()[0].cells[last]);
        assert_eq!(k(&mut m, |p| &mut p.right), ("Get a Grip".into(), 0));
        assert_eq!(k(&mut m, |p| &mut p.left), (hst.clone(), last));
        // ↓ from Hot Shots' last tile (its last row's 5th) to Get a Grip's last (two tiles), then Golf's 2nd, its 7th
        assert_eq!(last % 5, 4);
        assert_eq!(k(&mut m, |p| &mut p.down), ("Get a Grip".into(), 1), "the shorter row's last");
        assert_eq!(k(&mut m, |p| &mut p.down), ("Golf".into(), 1));
        assert_eq!(k(&mut m, |p| &mut p.down), ("Golf".into(), 6));
        assert_eq!(k(&mut m, |p| &mut p.up), ("Golf".into(), 1));
        assert_eq!(k(&mut m, |p| &mut p.up), ("Get a Grip".into(), 1));
        assert_eq!(k(&mut m, |p| &mut p.up), (hst.clone(), last - 3), "over the heading, the same place across");
        // the far ends: ↓ / ↑ to the grid's last / first tile, then stay put
        m.put(0, Some(Who::Mod(0)));
        assert_eq!(k(&mut m, |p| &mut p.down), ("Mods".into(), 2));
        assert_eq!(k(&mut m, |p| &mut p.right), ("Mods".into(), 2));
        assert_eq!(k(&mut m, |p| &mut p.down), ("Mods".into(), 2));
        m.put(0, m.sections()[0].cells[3]);
        assert_eq!(k(&mut m, |p| &mut p.up), (hst.clone(), 0));
        assert_eq!(k(&mut m, |p| &mut p.left), (hst.clone(), 0));
        assert_eq!(k(&mut m, |p| &mut p.up), (hst, 0));
    }

    /// L2 / R2: the next section's first tile, wrapping; L2 inside a section its own first first.
    #[test]
    fn l2_r2_jump_sections() {
        let mut m = menu();
        let k = |m: &mut Menu, f: fn(&mut Press) -> &mut bool| {
            m.step(Dev::Keys, press(f));
            at(m)
        };
        assert_eq!(k(&mut m, |p| &mut p.kind_next), ("Get a Grip".into(), 0));
        assert_eq!(k(&mut m, |p| &mut p.kind_next), ("Golf".into(), 0));
        assert_eq!(k(&mut m, |p| &mut p.right), ("Golf".into(), 1));
        assert_eq!(k(&mut m, |p| &mut p.kind_prev), ("Golf".into(), 0), "back to its own heading first");
        assert_eq!(k(&mut m, |p| &mut p.kind_prev), ("Get a Grip".into(), 0));
        assert_eq!(k(&mut m, |p| &mut p.kind_prev), ("Hot Shots Tennis".into(), 0));
        assert_eq!(k(&mut m, |p| &mut p.kind_prev), ("Mods".into(), 0), "wraps");
        assert_eq!(k(&mut m, |p| &mut p.kind_next), ("Hot Shots Tennis".into(), 0));
        // L1 / R1 stay the costume: no move
        m.players[0].random = false;
        m.put(0, Some(Who::Mod(5)));
        assert_eq!(k(&mut m, |p| &mut p.next), ("Golf".into(), 2));
        // a favorite: Favorites first, its tile there and in its own section; jumps reach both
        let mut out = Vec::new();
        m.toggle_favorite(0, &mut out);
        assert_eq!(k(&mut m, |p| &mut p.kind_next), ("Mods".into(), 0));
        assert_eq!(k(&mut m, |p| &mut p.kind_next), ("Favorites".into(), 0));
        assert_eq!(m.cell(0), Some(Who::Mod(5)));
        assert_eq!(k(&mut m, |p| &mut p.right), ("Hot Shots Tennis".into(), 0), "on past Favorites' end");
    }

    /// B66: a tile with nothing open is passed over, the jumps' first tile too.
    #[test]
    fn moves_pass_full_tiles() {
        let mut m = menu();
        m.seats[1] = Some(Dev::Open);
        // 2P ready on Get a Grip's first mod: its only costume is closed to 1P
        m.put(1, Some(Who::Mod(10)));
        m.players[1].ready = true;
        m.players[0].random = false;
        m.put(0, m.sections()[0].cells[CHARS]);
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_eq!(m.cell(0), Some(Who::Mod(11)));
        m.put(0, None);
        m.step(Dev::Keys, press(|p| &mut p.kind_next));
        assert_eq!(m.cell(0), Some(Who::Mod(11)));
    }

    #[test]
    fn favorites_persist() {
        let mut m = menu();
        m.players[0].random = false;
        m.put(0, Some(Who::Disc(3)));
        m.step(Dev::Keys, press(|p| &mut p.card));
        m.put(0, Some(Who::Mod(7)));
        let out = m.step(Dev::Keys, press(|p| &mut p.card));
        assert!(out.contains(&Effect::SaveFavorites));
        assert!(m.favorite(Who::Disc(3)) && m.favorite(Who::Mod(7)));
        let saved = text(&m.favs);
        assert_eq!(saved, "hst 3\nmod m7\n");
        let mut n = menu();
        n.players[0].random = false;
        n.favs = parse(&saved);
        assert_eq!(n.sections()[0].cells, [Some(Who::Disc(3)), Some(Who::Mod(7))]);
        // off again, from Favorites: the cursor stays in it, then goes to the character's own section once it's empty
        n.players[0].tab = FAVORITES;
        n.put(0, Some(Who::Disc(3)));
        assert_eq!(at(&n), ("Favorites".into(), 0));
        n.step(Dev::Keys, press(|p| &mut p.card));
        assert_eq!((n.cell(0), at(&n)), (Some(Who::Mod(7)), ("Favorites".into(), 0)));
        n.step(Dev::Keys, press(|p| &mut p.card));
        assert!(n.favs.is_empty());
        assert_eq!((n.cell(0), at(&n)), (Some(Who::Mod(7)), ("Golf".into(), 4)), "its own section, the same tile");
        // Random has no favorite; a ready player's △ does nothing
        n.put(0, None);
        assert!(n.step(Dev::Keys, press(|p| &mut p.card)).is_empty());
    }

    /// The grid scrolls to keep the cursor on view, its section's heading too on its first row; a favorite added
    /// above keeps another column's view where it was.
    #[test]
    fn the_grid_follows_the_cursor() {
        let mut m = menu();
        m.seats[1] = Some(Dev::Pad(bevy::prelude::Entity::from_raw_u32(9).unwrap()));
        m.players[0].random = false;
        let v = m.view(0);
        assert_eq!(v.top, 0);
        assert!(v.end < v.lines.len(), "more lines than the view");
        // R2 to the last section: its heading is the top line on view or above it, its first row on view
        for _ in 0..3 {
            m.step(Dev::Keys, press(|p| &mut p.kind_next));
        }
        assert_eq!(at(&m), ("Mods".into(), 0));
        let v = m.view(0);
        let row = v.lines.iter().position(|&l| l == Line::Tiles(3, 0)).unwrap();
        assert!((v.top..v.end).contains(&row) && (v.top..v.end).contains(&(row - 1)), "{v:?}");
        assert_eq!(m.view(1).top, 0, "2P's column didn't move");
        // a favorite: one heading and one row more over every view past the top
        let top = v.top;
        m.step(Dev::Keys, press(|p| &mut p.card));
        assert_eq!(m.view(0).top, top + 2);
        assert_eq!(m.view(1).top, 2, "2P's view keeps Hot Shots Tennis' heading on top");
        assert_eq!(m.view(1).lines[2], Line::Head(1));
    }

    /// B428: 1P readied from Favorites: the computer player 1P picks for next opens in Favorites too, on the same
    /// tile (not the character's own section); readied from its own section, the COM's opens there.
    #[test]
    fn the_com_opens_in_the_section_1p_readied_in() {
        let ok = press(|p| &mut p.ok);
        let section = |m: &Menu, q: usize| {
            let secs = m.sections();
            let s = m.section_at(q, &secs);
            (secs[s].name.clone(), secs[s].cells.iter().position(|&c| c == m.cell(q)).unwrap())
        };
        for (tab, want) in [(FAVORITES, ("Favorites".to_string(), 0)), (HOT_SHOTS, ("Golf".to_string(), 1))] {
            let mut m = menu();
            m.put(0, Some(Who::Mod(4)));
            m.toggle_favorite(0, &mut Vec::new());
            m.players[0].tab = tab;
            assert_eq!(section(&m, 0), want);
            m.step(Dev::Keys, ok);
            assert_eq!(m.driven(Dev::Keys), Some(1));
            assert_eq!((m.cell(1), section(&m, 1)), (Some(Who::Mod(4)), want.clone()));
            // → walks on from there
            if tab == FAVORITES {
                m.step(Dev::Keys, press(|p| &mut p.right));
                assert_eq!(section(&m, 1), ("Hot Shots Tennis".into(), 0), "on past Favorites' end");
            }
        }
    }

    #[test]
    fn tap_a_heading() {
        let mut m = menu();
        let secs = m.sections();
        m.point(Dev::Keys, pointer::Act::Section(0, 2), false);
        assert_eq!(at(&m), ("Hot Shots Tennis".into(), 0), "hovering a heading moves nothing");
        m.point(Dev::Keys, pointer::Act::Section(0, 2), true);
        assert_eq!((m.players[0].tab, m.cell(0)), (secs[2].id, Some(Who::Mod(3))));
        // a tile's place across the sections
        m.point(Dev::Keys, pointer::Act::Cell(0, flat(&secs, 1, 1)), false);
        assert_eq!(m.cell(0), Some(Who::Mod(11)));
    }

    /// B426: a grid whose last whole line is a section's heading shows the top of that section's first row in
    /// what's left under it, cut at the grid's foot (a tall phone held upright, two players), and a tap there picks it.
    #[test]
    fn a_heading_at_the_foot_shows_its_rows_top() {
        let mut m = menu();
        m.seats[1] = Some(Dev::Keys);
        m.players[1] = super::super::Player::fresh();
        m.mods.truncate(3);
        m.mods.extend((0..8).map(|i| a_mod(i, "Hot Shots Golf Fore! 5")));
        m.grid = Grid::fit_at(0.68, 1.2);
        let v = m.view(0);
        assert_eq!(v.lines[v.end - 1], Line::Head(1));
        let c = m.columns();
        let [_, gy, _, gh] = c.parts(0).grid;
        let first = flat(&v.secs, 1, 0);
        let (_, tiles) = c.laid(0, &v);
        let peek: Vec<_> = tiles.iter().filter(|t| t.0 >= first).collect();
        assert_eq!(peek.len(), c.cols);
        assert!(peek.iter().all(|t| t.2[1] < gy + gh && (t.2[1] + t.2[3] - (gy + gh)).abs() < 1e-3 && t.2[3] < c.tile[1]), "{peek:?}");
        // drawn cut at the foot, the hit too
        let t = super::super::pointer::tests::text_with_mods(m.mods.len());
        let d = super::super::drawn(&m, &t, &super::super::Bindings::default(), &[], 0, 0.0);
        let hit = d.hits.iter().find(|h| h.act == super::super::pointer::Act::Cell(0, first)).expect("the peeking tile's hit");
        assert!((hit.rect[1] + hit.rect[3] - (gy + gh)).abs() < 1e-3, "{:?}", hit.rect);
        let [x, y, w, _] = peek[0].2;
        let mine: Vec<_> = d.out.iter().filter(|q| q.dst[0] >= x && q.dst[0] < x + w && q.dst[1] >= y && q.dst[1] < gy + gh).collect();
        assert!(mine.len() >= 2);
        assert!(mine.into_iter().all(|q| q.dst[1] + q.dst[3] <= gy + gh + 1e-3));
    }

    /// B423: lines off view get a pill: "▼ N more: …" naming the sections below while the grid isn't at its end,
    /// "▲ …" naming those above once scrolled; a tap on one pages that way (only for the column's own device).
    #[test]
    fn the_more_pills_page_the_grid() {
        use super::super::pointer::Act;
        let mut m = menu();
        let t = super::super::pointer::tests::text_with_mods(m.mods.len());
        let pages = |m: &Menu| super::super::drawn(m, &t, &super::super::Bindings::default(), &[], 0, 0.0).hits.iter().filter_map(|h| if let Act::Page(p, d) = h.act { Some((p, d)) } else { None }).collect::<Vec<_>>();
        let c = m.columns();
        let v = m.view(0);
        assert!(v.end < v.lines.len(), "{v:?}");
        let (above, more, below) = super::super::roster::off_view(&v, c.cols);
        assert!(above.is_empty());
        // every tile not wholly on view, and the sections it's in
        let shown: usize = v.lines[v.top..v.end].iter().map(|&l| if let Line::Tiles(s, i) = l { v.secs[s].cells[i..].len().min(c.cols) } else { 0 }).sum();
        assert_eq!(more + shown, v.secs.iter().map(|s| s.cells.len()).sum::<usize>());
        assert_eq!(below.last(), Some(&(v.secs.len() - 1)));
        assert_eq!(pages(&m), [(0, 1)]);
        // another device doesn't page 1P's grid
        let pad = Dev::Pad(bevy::prelude::Entity::from_raw_u32(9).unwrap());
        assert!(m.point(pad, Act::Page(0, 1), true).is_empty() && m.scroll[0] == 0);
        // hovering does nothing, a tap moves a page: the first line not wholly shown comes to the top
        assert!(m.point(Dev::Keys, Act::Page(0, 1), false).is_empty() && m.scroll[0] == 0);
        assert!(!m.point(Dev::Keys, Act::Page(0, 1), true).is_empty());
        assert_eq!(m.scroll[0], v.end.min(c.last_top(&v.lines)));
        let v2 = m.view(0);
        let (above, ..) = super::super::roster::off_view(&v2, c.cols);
        assert_eq!(above.first(), Some(&0));
        assert!(pages(&m).contains(&(0, -1)));
        while !m.point(Dev::Keys, Act::Page(0, 1), true).is_empty() {}
        assert_eq!(pages(&m), [(0, -1)], "at the end: only the way back");
        while !m.point(Dev::Keys, Act::Page(0, -1), true).is_empty() {}
        assert_eq!(m.scroll[0], 0);
    }
}
