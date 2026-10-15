//! B369: the character select's category tabs, each player's own (plan/B368.md § Tabs and overflow): Favorites (only
//! while there are any), Hot Shots (the disc cast, Random first), then one per mod game (`Mod::game`, "Mods" for an
//! untagged mod) in name order, "Mods" last. No bumpers (L1 / R1 are the costume): → past a row's last tile goes on
//! to the next tab, ← past its first back to the previous one, wrapping, on the same row (clamped to the new tab's
//! last) at its first or last tile; ↑↓ stay in the tab. A tap on a tab switches to it. □ marks the highlighted
//! character a favorite (or not): saved in `favorites.txt` beside `settings.txt`, a star on its tile.
//!
//! B391: the tab row takes the cursor: ↑ from the grid's top row (or the pointer over a tab) focuses the player's
//! tab, ←→ there go straight onto the previous / next tab (wrapping, a tab with nothing open passed over), ↓ or ✕
//! back down onto the tile. The row keeps every tab at its own width and scrolls sideways (« » at a cut end) to show
//! the player's, never squeezing the tabs to fit the column.

use super::{COLOURS, Draw, Effect, GOLD, INK, Menu, Press, WHITE, Who, pointer};

/// A player's tab (`Player::tab`): Hot Shots is the default (0), so a fresh select (B346's Random) opens on it.
pub(super) const HOT_SHOTS: u8 = 0;
pub(super) const FAVORITES: u8 = 1;
/// Mod game `k` (in [`Menu::games`]' order) is tab id `GAMES + k`.
const GAMES: u8 = 2;

/// One tab: its id (`Player::tab`), its name, and its tiles (`None` the Random card).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Tab {
    pub id: u8,
    pub name: String,
    pub cells: Vec<Option<Who>>,
}

/// The favorite characters, by key ([`Menu::fav_key`]); sorted, so a `Menu`'s `Debug` is the same every time.
pub(super) type Favorites = std::collections::BTreeSet<String>;

/// Where the favorites are saved: beside `settings.txt`.
pub(super) fn path(iso: &str) -> std::path::PathBuf {
    super::Settings::path(iso).with_file_name("favorites.txt")
}

/// The saved favorites (none if there's no file yet). Before B394 the file was `favourites.txt`: read that if
/// there's no `favorites.txt` yet (the next save writes the new name).
pub(super) fn load(iso: &str) -> Favorites {
    crate::store::read_to_string(path(iso))
        .or_else(|_| crate::store::read_to_string(path(iso).with_file_name("favourites.txt")))
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

    /// The mod games with a tab, in tab order: by name, "Mods" (untagged) last.
    fn games(&self) -> Vec<&str> {
        let mut g: Vec<&str> = (0..self.own_mods()).map(|i| self.mods[i].game()).collect();
        g.sort_by_key(|&n| (n == "Mods", n));
        g.dedup();
        g
    }

    /// Every tab, in order.
    pub(super) fn tabs(&self) -> Vec<Tab> {
        let all = self.roster();
        let mut out = Vec::new();
        let favs: Vec<Option<Who>> = all.iter().copied().filter(|c| c.is_some_and(|w| self.favorite(w))).collect();
        if !favs.is_empty() {
            out.push(Tab { id: FAVORITES, name: "Favorites".into(), cells: favs });
        }
        out.push(Tab { id: HOT_SHOTS, name: "Hot Shots".into(), cells: all.iter().copied().filter(|c| !matches!(c, Some(Who::Mod(_)))).collect() });
        for (k, g) in self.games().into_iter().enumerate() {
            let cells = all.iter().copied().filter(|c| matches!(c, Some(Who::Mod(i)) if self.mods[*i].game() == g)).collect();
            out.push(Tab { id: GAMES + k as u8, name: g.to_string(), cells });
        }
        out
    }

    /// Player `p`'s tab in `tabs`: theirs, else (Favorites emptied, a game gone) Hot Shots.
    pub(super) fn tab_at(&self, p: usize, tabs: &[Tab]) -> usize {
        let id = self.players[p].tab;
        tabs.iter().position(|t| t.id == id).or_else(|| tabs.iter().position(|t| t.id == HOT_SHOTS)).unwrap_or(0)
    }

    /// Player `p`'s tab's tiles.
    pub(super) fn tab_cells(&self, p: usize) -> Vec<Option<Who>> {
        let mut tabs = self.tabs();
        let t = self.tab_at(p, &tabs);
        tabs.swap_remove(t).cells
    }

    /// The tiles across player `p`'s grid (B368: every column has as many).
    fn cols(&self, _p: usize) -> usize {
        self.columns().cols.max(1)
    }

    /// Player `p` (not ready) onto tab `t` of `tabs`: on the same tile if it's there, else its first open one.
    pub(super) fn set_tab(&mut self, p: usize, tabs: &[Tab], t: usize) -> bool {
        let tab = &tabs[t];
        let to = if tab.cells.contains(&self.cell(p)) { Some(self.cell(p)) } else { tab.cells.iter().copied().find(|&c| !self.full(p, c)) };
        let Some(c) = to else { return false };
        self.players[p].tab = tab.id;
        if c != self.cell(p) {
            self.put(p, c);
        }
        true
    }

    /// B369: a not-ready player's ↑↓←→ on their tab (B66: past a full tile the same way): ↑↓ a row, stopping at
    /// the ends; ←→ a tile, past the row's ends onto the previous / next tab (wrapping, a tab with nothing open
    /// passed over). Whether the cursor moved.
    pub(super) fn tab_move(&mut self, p: usize, k: Press) -> bool {
        let (dx, dy) = (k.right as i32 - k.left as i32, k.down as i32 - k.up as i32);
        if dx == 0 && dy == 0 {
            return false;
        }
        let tabs = self.tabs();
        if self.players[p].on_tabs {
            return self.row_move(p, &tabs, dx, dy);
        }
        let cols = self.cols(p);
        let mut t = self.tab_at(p, &tabs);
        if !tabs[t].cells.contains(&self.cell(p)) {
            // put off its tab (a pick carried over, a random draw): onto the tab it lives in
            self.players[p].tab = self.cell(p).map_or(HOT_SHOTS, |w| self.home_tab(w));
            t = self.tab_at(p, &tabs);
        }
        let cells = &tabs[t].cells;
        let Some(at) = cells.iter().position(|&c| c == self.cell(p)) else {
            return self.set_tab(p, &tabs, t);
        };
        let to = if dy != 0 {
            // B306's rows: stay put at the edge, past full tiles
            let mut to = at as i32;
            loop {
                to += dy * cols as i32;
                if to < 0 && tabs.len() > 1 {
                    // B391: ↑ off the top row onto the tab row (with another tab to go to)
                    self.players[p].on_tabs = true;
                    return true;
                }
                if !(0..cells.len() as i32).contains(&to) {
                    let edge = to.clamp(0, cells.len() as i32 - 1) as usize;
                    break if self.full(p, cells[edge]) { None } else { Some((t, edge)) };
                }
                if !self.full(p, cells[to as usize]) {
                    break Some((t, to as usize));
                }
            }
        } else {
            // the only tab: ←→ walk the whole of it, stopping at its ends (B306), as a narrow column has no tab to
            // overflow to
            let lone = tabs.len() == 1;
            let span = if lone { cells.len() } else { cols };
            let row = at / span;
            let end = ((row + 1) * span).min(cells.len());
            let mut to = at as i32 + dx;
            while (row * span..end).contains(&(to as usize)) && to >= 0 && self.full(p, cells[to as usize]) {
                to += dx;
            }
            if to >= 0 && (row * span..end).contains(&(to as usize)) {
                Some((t, to as usize))
            } else if lone {
                None
            } else {
                self.overflow(p, &tabs, t, row, dx > 0)
            }
        };
        match to {
            Some((t, i)) if (t, i) != (self.tab_at(p, &tabs), at) => {
                self.players[p].tab = tabs[t].id;
                self.put(p, tabs[t].cells[i]);
                true
            }
            _ => false,
        }
    }

    /// B391: a move on the tab row: ←→ onto the next tab that way with anything open (wrapping), ↓ back down onto
    /// the tile. Whether the cursor moved.
    fn row_move(&mut self, p: usize, tabs: &[Tab], dx: i32, dy: i32) -> bool {
        if dy > 0 {
            self.players[p].on_tabs = false;
            return true;
        }
        let n = tabs.len();
        if dx == 0 || n < 2 {
            return false;
        }
        let t = self.tab_at(p, tabs);
        (1..n).map(|s| if dx > 0 { (t + s) % n } else { (t + n - s) % n }).any(|u| self.set_tab(p, tabs, u))
    }

    /// B391: ✕ on the tab row: down onto the tab's tile (not a pick); `k` without its ✕ then.
    pub(super) fn tabs_ok(&mut self, p: usize, k: Press, out: &mut Vec<Effect>) -> Press {
        if !(k.ok && self.players[p].on_tabs && !self.players[p].ready) {
            return k;
        }
        self.players[p].on_tabs = false;
        self.follow(p);
        out.push(Effect::Sound(0));
        Press { ok: false, ..k }
    }

    /// Past row `row`'s end of tab `t` (`right`: its last tile): the next tab's same row (its last if it's shorter),
    /// first open tile from that end; tabs with none there passed over, wrapping.
    fn overflow(&self, p: usize, tabs: &[Tab], t: usize, row: usize, right: bool) -> Option<(usize, usize)> {
        let n = tabs.len();
        let cols = self.cols(p);
        (1..=n).map(|s| if right { (t + s) % n } else { (t + n - s) % n }).find_map(|u| {
            let cells = &tabs[u].cells;
            let r = row.min(cells.len().checked_sub(1)? / cols);
            let span = r * cols..((r + 1) * cols).min(cells.len());
            let i = if right { span.clone().find(|&i| !self.full(p, cells[i])) } else { span.clone().rev().find(|&i| !self.full(p, cells[i])) };
            i.map(|i| (u, i))
        })
    }

    /// B369: □: the highlighted character in or out of the favorites (saved). Taken off while on Favorites, the
    /// cursor stays at that place in the tab (or back on Hot Shots once it's empty).
    pub(super) fn toggle_favorite(&mut self, p: usize, out: &mut Vec<Effect>) {
        let Some(w) = self.cell(p) else { return };
        let key = self.fav_key(w);
        let tabs = self.tabs();
        let was = &tabs[self.tab_at(p, &tabs)];
        let at = was.cells.iter().position(|&c| c == Some(w)).unwrap_or(0);
        let on_favs = was.id == FAVORITES;
        if !self.favs.remove(&key) {
            self.favs.insert(key);
        }
        if on_favs {
            let tabs = self.tabs();
            let t = self.tab_at(p, &tabs);
            if tabs[t].id == FAVORITES {
                let cells = &tabs[t].cells;
                if let Some(c) = (at.min(cells.len() - 1)..cells.len()).chain(0..at).map(|i| cells[i]).find(|&c| !self.full(p, c)) {
                    self.put(p, c);
                }
            } else {
                self.players[p].tab = HOT_SHOTS;
                if !tabs[t].cells.contains(&Some(w)) {
                    self.players[p].tab = self.home_tab(w);
                }
            }
        }
        out.extend([Effect::Sound(0), Effect::SaveFavorites]);
    }

    /// The tab a character lives in (not Favorites).
    fn home_tab(&self, w: Who) -> u8 {
        match w {
            Who::Disc(_) => HOT_SHOTS,
            Who::Mod(i) => GAMES + self.games().iter().position(|&g| g == self.mods[i].game()).unwrap_or(0) as u8,
        }
    }
}

/// The tab row of player `p`'s column (its colour), in `[x, y, w, h]`: a pill per tab with its count, theirs filled
/// (the column's footer names the tab and its neighbours), ringed gold while the cursor is on the row (B391). B391:
/// every pill at its own width; too many for the column, the row shows a run of whole tabs round theirs, a « / » at
/// a cut end (a tap there goes on to the tab past it).
pub(super) fn row(d: &mut Draw, m: &Menu, p: usize, [x, y, w, h]: [f32; 4]) {
    let tabs = m.tabs();
    let t = m.tab_at(p, &tabs);
    let size = h * 0.62;
    let gap = 4.0;
    let labels: Vec<String> = tabs.iter().map(|tab| format!("{} {}", tab.name, tab.cells.len())).collect();
    let want: Vec<f32> = labels.iter().map(|l| d.width(l, size) + h * 0.6).collect();
    let arrow = h * 0.8;
    let (lo, hi, room) = shown(&want, t, w, gap, arrow);
    let focused = m.players[p].on_tabs && !m.players[p].ready;
    let mut px = x;
    if lo > 0 {
        d.label("«", x + arrow / 2.0, y + (h - size) / 2.0, size, arrow, WHITE);
        d.hit([x, y, arrow, h], pointer::Act::Category(p, lo - 1));
    }
    if (lo, hi) != (0, tabs.len()) {
        px += arrow + gap;
    }
    for i in lo..hi {
        let pw = want[i].min(room);
        let on = i == t;
        let fill = if on { COLOURS[p] } else { [24.0, 22.0, 30.0] };
        let ring = if on && focused { GOLD } else if on { WHITE } else { [70.0; 3] };
        if on && focused {
            d.pill_at([px - 2.0, y - 2.0, pw + 4.0, h + 4.0], [GOLD; 3], 128.0);
        }
        d.pill_at([px, y, pw, h], [INK, ring, fill], 128.0);
        d.label(&labels[i], px + pw / 2.0, y + (h - size) / 2.0, size, pw - h * 0.5, if on { WHITE } else { [100.0; 3] });
        d.hit([px, y, pw, h], pointer::Act::Category(p, i));
        px += pw + gap;
    }
    if hi < tabs.len() {
        d.label("»", x + w - arrow / 2.0, y + (h - size) / 2.0, size, arrow, WHITE);
        d.hit([x + w - arrow, y, arrow, h], pointer::Act::Category(p, hi));
    }
}

/// B391: the run of tabs `lo..hi` the row shows (pill widths `want`, `gap` apart, in `w`), round tab `t` (grown a tab
/// each way in turn while they fit), and the most one pill may take; a cut row keeps `arrow` (+ `gap`) at each end.
fn shown(want: &[f32], t: usize, w: f32, gap: f32, arrow: f32) -> (usize, usize, f32) {
    let n = want.len();
    if want.iter().sum::<f32>() + gap * n.saturating_sub(1) as f32 <= w {
        return (0, n, w);
    }
    let room = w - 2.0 * (arrow + gap);
    let (mut lo, mut hi, mut used) = (t, t + 1, want[t]);
    loop {
        let mut grew = false;
        if hi < n && used + gap + want[hi] <= room {
            used += gap + want[hi];
            hi += 1;
            grew = true;
        }
        if lo > 0 && used + gap + want[lo - 1] <= room {
            used += gap + want[lo - 1];
            lo -= 1;
            grew = true;
        }
        if !grew {
            break (lo, hi, room);
        }
    }
}

/// Player `p`'s footer: `« previous   n of N   next »` (the font has no ◂ ▸ or ·), `n` the tile's place in the tab.
pub(super) fn foot(m: &Menu, p: usize) -> String {
    let tabs = m.tabs();
    let t = m.tab_at(p, &tabs);
    let n = tabs.len();
    let at = tabs[t].cells.iter().position(|&c| c == m.cell(p)).unwrap_or(0) + 1;
    let k = format!("{}   {at} of {}", tabs[t].name, tabs[t].cells.len());
    if n > 1 { format!("« {}   {k}   {} »", tabs[(t + n - 1) % n].name, tabs[(t + 1) % n].name) } else { k }
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
        m.tabs().into_iter().map(|t| (t.name, t.cells.len())).collect()
    }

    #[test]
    fn tabs_in_order_with_counts() {
        let mut m = menu();
        assert_eq!(names(&m), [("Hot Shots".into(), 1 + CHARS), ("Get a Grip".into(), 2), ("Golf".into(), 7), ("Mods".into(), 3)]);
        assert_eq!(m.tab_cells(0)[..2], [None, Some(Who::Disc(GRID[0].0))]);
        // a favorite puts Favorites first
        m.put(0, Some(Who::Mod(4)));
        let mut out = Vec::new();
        m.toggle_favorite(0, &mut out);
        assert_eq!(out, [Effect::Sound(0), Effect::SaveFavorites]);
        assert_eq!(names(&m)[0], ("Favorites".into(), 1));
        assert_eq!(m.tabs()[0].cells, [Some(Who::Mod(4))]);
    }

    /// → past the row's end: the next tab's same row at its first tile; ← back: the previous tab's row at its last;
    /// a shorter tab clamps the row; the ends wrap.
    #[test]
    fn overflow_wraps_and_clamps_the_row() {
        let mut m = menu();
        let k = |m: &mut Menu, f: fn(&mut Press) -> &mut bool| {
            m.step(Dev::Keys, press(f));
            let tabs = m.tabs();
            let t = m.tab_at(0, &tabs);
            (tabs[t].name.clone(), tabs[t].cells.iter().position(|&c| c == m.cell(0)).unwrap())
        };
        // Hot Shots row 1 (tiles 5..10): to its last tile, then on to Get a Grip, whose only row is 0
        m.put(0, m.tab_cells(0)[9]);
        assert_eq!(k(&mut m, |p| &mut p.right), ("Get a Grip".into(), 0));
        assert_eq!(k(&mut m, |p| &mut p.right), ("Get a Grip".into(), 1));
        assert_eq!(k(&mut m, |p| &mut p.right), ("Golf".into(), 0));
        // Golf row 1 (5, 6), right past 6 → Mods (one row): its row 0
        assert_eq!(k(&mut m, |p| &mut p.down), ("Golf".into(), 5));
        assert_eq!(k(&mut m, |p| &mut p.right), ("Golf".into(), 6));
        assert_eq!(k(&mut m, |p| &mut p.right), ("Mods".into(), 0));
        // wraps: Mods → Hot Shots, row 0 at its first tile (Random)
        assert_eq!(k(&mut m, |p| &mut p.right), ("Mods".into(), 1));
        assert_eq!(k(&mut m, |p| &mut p.right), ("Mods".into(), 2));
        assert_eq!(k(&mut m, |p| &mut p.right), ("Hot Shots".into(), 0));
        assert_eq!(m.cell(0), None);
        // ← past the first: the previous tab (wrapping), its row's last tile
        assert_eq!(k(&mut m, |p| &mut p.left), ("Mods".into(), 2));
        // ↓ stays in the tab (Mods has one row), ↑ too
        assert_eq!(k(&mut m, |p| &mut p.down), ("Mods".into(), 2));
        assert_eq!(k(&mut m, |p| &mut p.left), ("Mods".into(), 1));
        // Hot Shots' last row (tiles 10..15) → ← from its first goes to the previous tab's (Mods') row 0, at its last
        m.players[0].tab = HOT_SHOTS;
        m.put(0, m.tab_cells(0)[10]);
        assert_eq!(k(&mut m, |p| &mut p.left), ("Mods".into(), 2));
    }

    /// B66: a tile with nothing open is passed over, a tab with nothing open on that row too.
    #[test]
    fn overflow_passes_full_tiles() {
        let mut m = menu();
        m.seats[1] = Some(Dev::Open);
        // 2P ready on Get a Grip's first mod: its only costume is closed to 1P
        m.put(1, Some(Who::Mod(10)));
        m.players[1].ready = true;
        m.put(0, m.tab_cells(0)[4]);
        m.step(Dev::Keys, press(|p| &mut p.right));
        assert_eq!(m.cell(0), Some(Who::Mod(11)));
    }

    #[test]
    fn favorites_persist() {
        let mut m = menu();
        m.put(0, Some(Who::Disc(3)));
        m.step(Dev::Keys, press(|p| &mut p.card));
        m.put(0, Some(Who::Mod(7)));
        let out = m.step(Dev::Keys, press(|p| &mut p.card));
        assert!(out.contains(&Effect::SaveFavorites));
        assert!(m.favorite(Who::Disc(3)) && m.favorite(Who::Mod(7)));
        let saved = text(&m.favs);
        assert_eq!(saved, "hst 3\nmod m7\n");
        let mut n = menu();
        n.favs = parse(&saved);
        assert_eq!(n.tabs()[0].cells, [Some(Who::Disc(3)), Some(Who::Mod(7))]);
        // off again, from the Favorites tab: the cursor stays on the tab, then Hot Shots once it's empty
        n.players[0].tab = FAVORITES;
        n.put(0, Some(Who::Disc(3)));
        n.step(Dev::Keys, press(|p| &mut p.card));
        assert_eq!((n.cell(0), n.tabs()[0].name.as_str()), (Some(Who::Mod(7)), "Favorites"));
        n.step(Dev::Keys, press(|p| &mut p.card));
        assert!(n.favs.is_empty());
        assert_eq!((n.players[0].tab, n.cell(0)), (GAMES + 1, Some(Who::Mod(7))), "Golf, where it lives");
        // Random has no favorite; a ready player's △ does nothing
        n.put(0, None);
        assert!(n.step(Dev::Keys, press(|p| &mut p.card)).is_empty());
    }

    /// B391: ↑ off the top row focuses the tab row; ←→ there switch tabs (wrapping), ✕ or ↓ go back to the tile.
    #[test]
    fn the_tab_row_takes_the_cursor() {
        let mut m = menu();
        m.players[0].random = false;
        m.put(0, m.tab_cells(0)[2]);
        let k = |m: &mut Menu, f: fn(&mut Press) -> &mut bool| {
            m.step(Dev::Keys, press(f));
            let tabs = m.tabs();
            (tabs[m.tab_at(0, &tabs)].name.clone(), m.players[0].on_tabs)
        };
        assert_eq!(k(&mut m, |p| &mut p.up), ("Hot Shots".into(), true));
        // on the row: → straight onto Get a Grip's first tile, ← back (the same tile kept), ← wraps to Mods
        assert_eq!(k(&mut m, |p| &mut p.right), ("Get a Grip".into(), true));
        assert_eq!(m.cell(0), Some(Who::Mod(10)));
        assert_eq!(k(&mut m, |p| &mut p.left), ("Hot Shots".into(), true));
        assert_eq!(k(&mut m, |p| &mut p.left), ("Mods".into(), true));
        assert_eq!(k(&mut m, |p| &mut p.up), ("Mods".into(), true));
        // ✕ goes down, not ready; ↓ too
        assert_eq!(k(&mut m, |p| &mut p.ok), ("Mods".into(), false));
        assert!(!m.players[0].ready);
        assert_eq!(k(&mut m, |p| &mut p.up), ("Mods".into(), true));
        assert_eq!(k(&mut m, |p| &mut p.down), ("Mods".into(), false));
        assert_eq!(m.cell(0), Some(Who::Mod(0)));
        // the pointer over a tab focuses it and puts the player on it; over a tile, back on the grid
        m.point(Dev::Keys, pointer::Act::Category(0, 2), false);
        assert_eq!((m.players[0].tab, m.players[0].on_tabs), (GAMES + 1, true));
        m.point(Dev::Keys, pointer::Act::Cell(0, 1), false);
        assert_eq!((m.players[0].on_tabs, m.cell(0)), (false, Some(Who::Mod(4))));
    }

    /// B391: tabs too wide for the row keep their widths: a run round the player's tab, « » room at the ends.
    #[test]
    fn the_row_scrolls_instead_of_squeezing() {
        let want = [50.0; 6];
        assert_eq!(shown(&want, 3, 400.0, 4.0, 10.0), (0, 6, 400.0));
        let (lo, hi, room) = shown(&want, 5, 200.0, 4.0, 10.0);
        assert_eq!((lo, hi, room), (3, 6, 172.0));
        assert_eq!(shown(&want, 0, 200.0, 4.0, 10.0).0..shown(&want, 0, 200.0, 4.0, 10.0).1, 0..3);
        assert_eq!(&shown(&want, 2, 200.0, 4.0, 10.0).0, &1);
        // one pill wider than the row: just it, squeezed to the room
        assert_eq!(shown(&[500.0, 50.0], 0, 200.0, 4.0, 10.0), (0, 1, 172.0));
    }

    #[test]
    fn tap_a_tab() {
        let mut m = menu();
        let tabs = m.tabs();
        assert!(m.set_tab(0, &tabs, 2));
        assert_eq!((m.players[0].tab, m.cell(0)), (GAMES + 1, Some(Who::Mod(3))));
    }
}
