//! B449: the confirm screen, v4 (ours, the approved board "Concept v4: Ready to play?", the lobby's frame,
//! `frame.rs`). The title and the Singles / Doubles button (□ switches, 1P's device or a tap) along the top; the
//! players' plates on the court, a team each side of the net (singles: near side, far side); the scoreboard strip
//! under it: Court (its picture; ✕ opens the court select), Umpire (her face; ✕ opens the umpire select), Sets and
//! Games (✕, then ←→, as before, B212); Handicap is hidden until the original's handicaps are in (B482, B40g1); along the bottom
//! ‹ Back and the big Start the match!. The arrows go about the screen as its parts lie; ✕ on a plate sends that
//! player back to the character select. The court and umpire selects (`court_screen`, `umpire_screen`) are as before.

use super::frame::{self, Cell, Chip, Plate, Spec, Thumb};
use super::{CHAR_NAMES, CONFIRM_ROWS, Draw, Menu, Press, RANK_NAMES, SETS, Text, UMPIRES, Who};
use super::pointer;
use bevy::prelude::{Entity, default};

impl Menu {
    pub(super) fn confirm_geo(&self) -> frame::Geo {
        let mut g = frame::layout(self.grid.reach(), Spec { n: self.players(), code: false, public: false, chat: false });
        // Handicap is hidden until handicaps work (B482; B40g1 brings it back)
        frame::spread(&mut g, 4);
        g
    }

    /// B449: where the arrows go from the confirm screen's chosen row (`Menu::sel`), as its parts lie.
    pub(super) fn confirm_step(&self, k: Press) -> usize {
        super::online::pill_step(&rects(self), self.sel, k)
    }

    /// Whether ↑↓ and ←→ reach another row from the chosen one.
    pub(super) fn confirm_axes(&self) -> (bool, bool) {
        let to = |f: fn(&mut Press) -> &mut bool| {
            let mut k = Press::default();
            *f(&mut k) = true;
            self.confirm_step(k) != self.sel
        };
        (to(|k| &mut k.up) || to(|k| &mut k.down), to(|k| &mut k.left) || to(|k| &mut k.right))
    }
}

/// The confirm screen's rows (`Menu::sel`): Start, the court, Sets, Games, the umpire (`CONFIRM_ROWS`), then a plate a
/// player.
pub(super) fn rects(m: &Menu) -> Vec<[f32; 4]> {
    let g = m.confirm_geo();
    let mut v = vec![g.start, g.cells[0], g.cells[2], g.cells[3], g.cells[1]];
    v.extend(g.plates);
    v
}

/// The confirm screen (the court and umpire selects open over it as their own screens).
pub(super) fn draw(d: &mut Draw, m: &Menu, text: &Text, pads: &[Entity], _hand_x: f32) {
    frame::ground(d);
    let g = m.confirm_geo();
    let picking = m.picking.is_some();
    let on = |row: usize| !picking && m.sel == row;
    // the top bar: the title (Back for the pointer), Singles / Doubles (1P's □ or a tap)
    frame::bar(d, &g, "Ready to play?", m.doubles, !m.online(), false);
    d.hit(g.title, pointer::press(|k| k.back = true));
    frame::stage(d, &g, if m.doubles { ["TEAM 1", "TEAM 2"] } else { ["NEAR SIDE", "FAR SIDE"] });
    let s = g.k * g.s;
    for (p, &rect) in g.plates.iter().enumerate() {
        plate(d, m, text, pads, p, rect, s, on(CONFIRM_ROWS + p));
        d.hit(rect, pointer::Act::Row(CONFIRM_ROWS + p, pointer::OK));
    }
    // the strip: Court, Umpire, Sets, Games (Handicap hidden, B482)
    let u = m.umpire as usize;
    let umpire = match (u, m.umpire_mod_of(u)) {
        (UMPIRES, _) => "Random".to_string(),
        (_, Some(x)) => x.name.clone(),
        _ => m.umpire_name(text, u),
    };
    let cells = [
        Cell { label: "COURT", value: m.court_title(text, m.court), arrows: false, sel: on(1), thumb: Thumb::Court(m.court) },
        Cell { label: "UMPIRE", value: umpire, arrows: false, sel: on(4), thumb: Thumb::Umpire(u) },
        Cell { label: "SETS", value: SETS[m.sets].to_string(), arrows: true, sel: on(2), thumb: Thumb::None },
        Cell { label: "GAMES", value: m.games.to_string(), arrows: true, sel: on(3), thumb: Thumb::None },
    ];
    frame::strip(d, m, &g, &cells);
    for (row, cell) in [(1, 0), (4, 1), (2, 2), (3, 3)] {
        d.hit(g.cells[cell], pointer::Act::Row(row, pointer::OK));
    }
    // the bottom row: ‹ Back, Start
    frame::back(d, &g, "< Back", false);
    d.hit(g.back, pointer::press(|k| k.back = true));
    frame::start(d, &g, "Start the match!", "Everyone ready", false, on(0));
    d.hit(g.start, pointer::Act::Row(0, pointer::OK));
    if picking {
        picked_cell(d, m, &g);
    }
}

/// Sets or Games picked (B212): its cell lit blue with ‹ › (each gone at its end, as the original's); the cell's
/// thirds step it, the middle keeps it, a tap off puts it back (as ○).
fn picked_cell(d: &mut Draw, m: &Menu, g: &frame::Geo) {
    if !matches!(m.sel, 2 | 3) {
        return;
    }
    let (at, n) = if m.sel == 2 { (m.sets, SETS.len()) } else { (super::GAMES.iter().position(|&n| n >= m.games).unwrap_or(1), super::GAMES.len()) };
    let (cap, value) = if m.sel == 2 { ("SETS", SETS[m.sets].to_string()) } else { ("GAMES", m.games.to_string()) };
    let [x, y, w, h] = g.cells[m.sel];
    let lit = [x, y + g.y(2.0), w, h - g.y(4.0)];
    frame::rounded(d, lit, g.x(6.0), frame::hex(0x2f5f9d), 128.0);
    frame::ringed(d, lit, g.x(6.0), g.x(3.0), super::WHITE, None);
    let (ls, vs) = (g.y(11.0), g.y(20.0));
    let top = y + (h - ls - g.y(2.0) - vs) / 2.0;
    let (l, r) = (x + g.x(14.0), x + w - g.x(14.0));
    super::left(d, cap, l, top, ls, r - l, frame::CAPTION);
    let vy = top + ls + g.y(2.0);
    if at > 0 {
        d.text("<", l, vy, vs, super::WHITE);
    }
    if at + 1 < n {
        d.text(">", r - d.width(">", vs), vy, vs, super::WHITE);
    }
    d.label(&value, (l + r) / 2.0, vy, vs, r - l - g.x(40.0), super::WHITE);
    d.popup(Some(pointer::press(|k| k.back = true)));
    d.hit([x, y, w, h], pointer::Act::Row(m.sel, pointer::OK));
    d.hit([x, y, w / 3.0, h], pointer::Act::Row(m.sel, Press { left: true, ..default() }));
    d.hit([x + w * 2.0 / 3.0, y, w / 3.0, h], pointer::Act::Row(m.sel, Press { right: true, ..default() }));
}

/// Player `p`'s plate: who plays it (the device, a computer, online the player's name), the character, its style and
/// hand, Ready or the computer's COM · rank.
#[allow(clippy::too_many_arguments)]
fn plate(d: &mut Draw, m: &Menu, text: &Text, pads: &[Entity], p: usize, rect: [f32; 4], s: f32, focus: bool) {
    let pl = m.players[p];
    let (who, costume) = m.pick(p);
    let remote = m.remote(p);
    let human = m.seats[p].is_some() || remote.is_some_and(|s| s.human());
    let donor = match who {
        Who::Mod(i) => m.mods[i].donor,
        Who::Disc(c) => c,
    };
    let chip_words = if human {
        "Ready".to_string()
    } else {
        let rank = super::com_rank(&text.ranks, donor, costume, m.doubles);
        format!("COM · {}", RANK_NAMES.get(rank).copied().unwrap_or("COM").trim_start_matches("Tennis "))
    };
    let random = pl.random;
    let name = match who {
        _ if random => "Random".to_string(),
        Who::Mod(i) => m.mods[i].name.clone(),
        Who::Disc(c) => m.remote_mod_name(p).unwrap_or(CHAR_NAMES[c]).to_string(),
    };
    let owner = match remote.filter(|s| s.human()) {
        Some(s) => m.net.room.as_ref().and_then(|r| r.names.get(&s.peer)).map_or("Online".to_string(), |n| n.clone()),
        None if human => super::dev_name(m.seats[p], pads),
        None => "Computer".into(),
    };
    let sub = if random { "Picked at the start".to_string() } else { format!("{} · {}", frame::style_short(text, who), if m.hand(text, p) < 0.0 { "Left" } else { "Right" }) };
    let mod_note = m.remote_mod_name(p).map(|n| format!("{n} mod: you'll see a stand-in"));
    let plate = Plate {
        p,
        owner: &owner,
        slow: false,
        name: &name,
        sub: &sub,
        chip: if human { Chip::Ready } else { Chip::Com },
        chip_words: &chip_words,
        dashed: !human,
        remove: false,
        focus,
        remove_focus: false,
        random,
        hide: false,
        empty: false,
        mod_note,
    };
    frame::plate(d, &plate, rect, s);
}

#[cfg(test)]
mod tests {
    use super::super::{Grid, Screen};
    use super::*;

    /// Every row reachable with the arrows from Start, at any window.
    #[test]
    fn arrows_reach_every_row() {
        for aspect in [0.5, 0.75, 1.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0] {
            for doubles in [false, true] {
                let m = Menu { screen: Screen::Confirm, doubles, grid: Grid::fit(aspect), ..default() };
                let n = rects(&m).len();
                let mut seen = vec![false; n];
                let mut todo = vec![0];
                while let Some(s) = todo.pop() {
                    if std::mem::replace(&mut seen[s], true) {
                        continue;
                    }
                    for f in [|k: &mut Press| k.up = true, |k: &mut Press| k.down = true, |k: &mut Press| k.left = true, |k: &mut Press| k.right = true] {
                        let mut k = Press::default();
                        f(&mut k);
                        todo.push(Menu { sel: s, ..m.clone() }.confirm_step(k));
                    }
                }
                assert!(seen.iter().all(|&s| s), "{aspect} {doubles}: {seen:?}");
            }
        }
    }
}
