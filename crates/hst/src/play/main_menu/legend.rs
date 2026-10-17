//! M14: the main menus' button legend rows: what each screen takes and from whom (`acts`, `groups`, `fill`). The
//! legend's types and drawing are the match's (`play/legend.rs`, P61).

use bevy::prelude::*;
use std::sync::atomic::Ordering;

use super::super::controls::Bindings;
use super::{CONFIRM_ROWS, Dev, Menu, Screen, options};
pub(crate) use super::super::legend::*;
// over bevy's `Last` schedule (both globs)
pub(crate) use super::super::legend::Last;

// ---------------------------------------------------------------- the main menus' entries

/// What `dev`'s presses do on `m`'s screen as it stands (`Menu::step`): the legend lists exactly these (the test
/// probes `step` for them).
pub(super) fn acts(m: &Menu, dev: Dev) -> Vec<(Act, String)> {
    let mut v: Vec<(Act, &str)> = Vec::new();
    let host = if m.screen == Screen::Confirm { m.drives_foot(dev) } else { m.hosts(dev) };
    let s = m.sel;
    match m.screen {
        // B516: the splash's one press (none once 1P is taken)
        Screen::Press if m.p1.is_none() => v.push((Act::Ok, "Start")),
        Screen::Press => {}
        // M127: the pad takeover prompt: the asked pad answers, the keyboard's Esc dismisses it
        _ if m.takeover_asking() && m.takeover_asked(dev) => v.extend([(Act::Move, "Move"), (Act::Ok, "Choose"), (Act::Back, "Keep keyboard")]),
        _ if m.takeover_asking() && dev == Dev::Keys => v.push((Act::Back, "Keep keyboard")),
        _ if m.takeover_asking() => {}
        Screen::Main if m.add.ask.is_some() => v.extend([(Act::Move, "Move"), (Act::Ok, "Choose")]),
        Screen::Main if m.stale.0.is_some() => v.extend([(Act::Move, "Move"), (Act::Ok, "Choose"), (Act::Back, "Back")]),
        _ if m.mod_asking() => v.extend([(Act::UpDown, "Move"), (Act::Ok, "Choose"), (Act::Back, "Stand-in")]),
        _ if m.switch.is_some() => v.extend([(Act::Move, "Move"), (Act::Ok, "Choose"), (Act::Back, "Cancel")]),
        Screen::Main if m.main_rows().get(s) == Some(&super::home::Row::Update) => v.extend([(Act::UpDown, "Move"), (Act::Ok, crate::update::button()), (Act::Back, "Later")]),
        Screen::Main => v.extend([(Act::UpDown, "Move"), (Act::Ok, if m.main_rows().get(s) == Some(&super::home::Row::Quit) { if cfg!(target_arch = "wasm32") { "Launcher" } else { "Quit" } } else { "Choose" }), (Act::Back, "Back")]),
        Screen::Display | Screen::Graphics | Screen::Advanced | Screen::Sound | Screen::ModSettings | Screen::Mobile => {
            let open = options::page(m.screen).get(s).is_some_and(|o| !options::locked(*o));
            let listed = open && options::page(m.screen).get(s).is_some_and(|o| options::listed(*o));
            if m.listing.is_some() && listed {
                // B498: a dropdown open: ↑ ↓ move, ✕ picks, ○ closes it unchanged
                v.extend([(Act::UpDown, "Move"), (Act::Ok, "Select"), (Act::Back, "Close")]);
            } else if m.editing.is_some() && open && m.listing.is_none() && options::page(m.screen).get(s) != Some(&options::Opt::Advanced) {
                // B326: the row picked: ◀ ▶ change it, ✕ sets it, ○ puts it back
                v.extend([(Act::LeftRight, "Change"), (Act::Ok, "Select"), (Act::Back, "Cancel")]);
            } else {
                v.push((Act::UpDown, "Option"));
                if listed {
                    // B498: ◀ ▶ change a dropdown's row unopened
                    v.extend([(Act::LeftRight, "Change"), (Act::Ok, "Open list")]);
                } else if open {
                    // B411: Graphics' Advanced row opens its page
                    v.push((Act::Ok, if options::page(m.screen).get(s) == Some(&options::Opt::Advanced) { "Open" } else { "Change" }));
                }
                v.extend([(Act::Costume, "Page"), (Act::Back, "Back")]);
            }
        }
        // B291: the Keyboard & Mouse tab waits for a key, the Controller tab for a button (no keycap: any pad's)
        Screen::Controls if m.rebinding.is_some() && m.keys_tab => v.extend([(Act::Type, "Press the new key"), (Act::Cancel, "Cancel")]),
        Screen::Controls if m.rebinding.is_some() => v.extend([(Act::Key, "Press the new button"), (Act::Cancel, "Cancel")]),
        Screen::Controls => {
            v.extend([(Act::UpDown, "Action"), (Act::Ok, "Rebind"), (Act::Kind, if m.keys_tab { "Controller" } else { "Keyboard & Mouse" }), (Act::Costume, "Page"), (Act::Back, "Back")]);
        }
        Screen::Mode if host => v.extend([(Act::Move, "Singles / Doubles"), (Act::Ok, "Choose"), (Act::Back, "Back")]),
        Screen::Assign if host && m.keep.is_some() => v.extend([(Act::Move, "Move"), (Act::Ok, "Choose"), (Act::Back, "Cancel")]),
        Screen::Assign if host && m.held => v.extend([(Act::LeftRight, "Move controller"), (Act::Select, "Put down"), (Act::Back, "Put down")]),
        Screen::Assign if host => {
            v.push((if m.assign_per() < m.players() { Act::Move } else { Act::LeftRight }, "Move"));
            let p = m.sel.min(m.players() - 1);
            if m.pickable(p) {
                v.push((Act::Select, "Select"));
            }
            if m.droppable(m.sel.min(m.players() - 1)) {
                v.push((Act::Page, "Remove"));
            }
            if m.can_bot(m.sel.min(m.players() - 1)) {
                let p = m.sel.min(m.players() - 1);
                // a computer player's seat empties with Remove
                if !m.bots[p] {
                    v.push((Act::Card, "Add COM"));
                }
            }
            v.extend([(Act::Go, if m.paused { "Done" } else { "Start" }), (Act::Back, "Back")]);
        }
        Screen::Assign => {
            let n = m.players();
            let room = |q: usize| m.seats[q].is_none() && !(m.online() && q >= m.local_max());
            match m.seats[..n].iter().position(|d| *d == Some(dev)) {
                None if (0..if m.online() { m.local_max() } else { n }).any(|p| m.seats[p].is_none() && !m.bots[p]) => v.push((Act::Ok, "Join")),
                // B437: the device the driver moved onto 1P waits for it to leave
                None | Some(0) => {}
                Some(p) => {
                    if (1..n).any(|q| q != p && room(q)) {
                        v.push((Act::Move, "Seat"));
                    }
                    v.push((Act::Back, "Leave"));
                }
            }
        }
        Screen::Chars => {
            let Some(p) = m.driven(dev) else {
                return if m.can_join() { vec![(Act::Ok, "Join".into())] } else { vec![] };
            };
            // B371: a name picker open takes the device's presses
            let np = (0..m.players()).find(|&q| m.seats[q] == Some(dev) && m.naming(q)).unwrap_or(p);
            if let Some(v) = m.name_acts(np, dev) {
                return v.into_iter().map(|(a, l)| (a, l.to_string())).collect();
            }
            if m.can_swap(dev) || m.online_can_swap(dev) {
                v.push((Act::Page, "Singles / Doubles"));
            }
            if m.team_switch(dev).is_some() {
                v.push((Act::Page, "Switch team"));
            }
            // B522: online, a guest's □ moves its player to the other team
            if m.online_team(dev).is_some() {
                v.push((Act::Page, "Swap team"));
            }
            let me = m.players[p];
            if !me.ready {
                // a tile alone in its row (nothing open beside it): ↑↓ only
                v.extend([(if m.row_open(p) { Act::Move } else { Act::UpDown }, "Character"), (Act::Costume, "Costume"), (Act::Hand, "Hand")]);
                // B382: L2 / R2 jump between the grid's sections (with more than one)
                if m.sections().len() > 1 {
                    v.push((Act::Kind, "Section"));
                }
            }
            // B422 / B463 / B475: the device's own preview plays its win / loss (a pad's right stick up / down,
            // `previews::buttons`) and the match's emotes (`plate_emote.rs`), one hint (Random has no figure)
            if m.seats[p] == Some(dev) && !me.random {
                v.push((Act::Emote, "Emote"));
            }
            // B369 / B375: △ in or out of the favorites (not Random)
            if !me.ready && !me.random {
                v.push((Act::Card, "Favorite"));
            }
            // B371 / B375: Start names the player on the device's own seat (Stats / Picture is the card's pill)
            if (0..m.players()).any(|q| m.seats[q] == Some(dev)) {
                v.push((Act::Name, "Name"));
            }
            if !me.ready && !m.blocked(p) {
                v.push((Act::Ok, "Ready"));
            }
            if me.ready {
                v.push((Act::Back, "Cancel"));
            } else if m.online() && m.net.edit == Some(p) || m.seats[p].is_none() && m.players.iter().take(m.players()).any(|q| q.ready) {
                v.push((Act::Back, "Back"));
            } else if m.seats[p] == Some(dev) && m.droppable(p) {
                v.push((Act::Back, "Leave"));
            } else if p == 0 && m.seats[p].is_some() {
                v.push((Act::Back, "Back"));
            }
        }
        // B450: the court picker's rows
        Screen::Confirm if host && m.picking.is_some() && s == 1 => {
            let (rows, page) = super::court_picker::axes(m);
            v.push((Act::LeftRight, "Choose"));
            if rows {
                v.push((Act::UpDown, "Row"));
            }
            if page {
                v.push((Act::Costume, "Scroll"));
            }
            v.extend([(Act::Ok, "OK"), (Act::Back, "Cancel")]);
        }
        Screen::Confirm if host && m.picking.is_some() => {
            if s == 4 {
                // B451: the umpire picker's rows; △ only for an umpire with a voice to preview (B431)
                let (rows, page, voice) = super::umpire_picker::axes(m);
                v.push((Act::LeftRight, "Choose"));
                if rows {
                    v.push((Act::UpDown, "Row"));
                }
                if page {
                    v.push((Act::Costume, "Scroll"));
                }
                if voice {
                    v.push((Act::Card, "Hear voice"));
                }
                v.extend([(Act::Ok, "OK"), (Act::Back, "Cancel")]);
            } else {
                v.push((Act::LeftRight, ["", "Court", "Sets", "Games", "Umpire"].get(s).copied().unwrap_or("Change")));
                v.extend([(Act::Ok, "Select"), (Act::Back, "Cancel")]);
            }
        }
        Screen::Confirm if host => {
            // B449: about the screen; ✕ on a plate sends that player back to the select
            moves(&mut v, m.confirm_axes());
            let all = m.online() || (0..m.players()).all(|p| m.players[p].ready);
            let theirs = s >= CONFIRM_ROWS && !m.online() && !(m.seats[s - CONFIRM_ROWS] == Some(dev) || m.seats[s - CONFIRM_ROWS].is_none());
            if (s != 0 || all) && !theirs {
                v.push((Act::Ok, if s == 0 { "Start" } else if s < CONFIRM_ROWS || !m.online() { "Change" } else { "Back" }));
            }
            if m.confirm_swap() {
                v.push((Act::Page, "Singles / Doubles"));
            }
            // the name picker, for the device's own seat
            if (0..m.players()).any(|p| m.seats[p] == Some(dev)) {
                v.push((Act::Name, "Name"));
            }
            v.push((Act::Back, "Back"));
        }
        Screen::Online => {
            moves(&mut v, m.pill_axes());
            v.extend([(Act::Ok, "Choose"), (Act::Back, "Back")]);
        }
        Screen::Lobbies => {
            let n = m.net.list.len();
            // B496: □ Join by Code from anywhere on the list
            v.extend([(Act::UpDown, "Move"), (Act::Ok, if s < n { "Join" } else if s == n { "Refresh" } else if s == n + 1 { "Join by Code" } else { "Back" }), (Act::Page, "Join by Code"), (Act::Back, "Back")]);
        }
        // B441: the name's popup (the Mods page's keyboard)
        Screen::Code if m.net.naming && !m.net.url => v.extend(super::osk::legend(s, (!m.net.entry.trim().is_empty()).then_some("Save"))),
        Screen::Code => {
            let keys = m.keys().len();
            let ok = match s.checked_sub(keys) {
                None => Some("Type"),
                Some(0) => Some("Delete"),
                Some(1) => Some("Paste"),
                // Join a whole short code
                // B323: Save a relay URL that parses
                Some(2) if m.net.url => crate::net::relay_url(&m.net.entry).is_ok().then_some("Save"),
                Some(2) => crate::net::is_short(&m.net.entry).then_some("Join"),
                _ => Some("Back"),
            };
            // B207: typing on the phone's keyboard: no grid to move on or type from
            let grid = !super::online::touch_typing();
            if grid {
                v.push((Act::Move, "Move"));
            }
            v.extend(ok.map(|ok| (Act::Ok, ok)));
            // B239: □ deletes, ○ leaves
            v.extend([(Act::Type, "Type"), (Act::Page, "Delete"), (Act::Back, "Back")].into_iter().filter(|a| grid || a.0 == Act::Back));
        }
        Screen::Lobby if m.net.ask.is_none() && !m.net.keep.is_empty() => v.extend([(Act::UpDown, "Move"), (Act::Ok, "Choose"), (Act::Back, "Not this time")]),
        Screen::Lobby if m.net.ask.is_some() => v.extend([(Act::UpDown, "Move"), (Act::Ok, "Choose"), (Act::Back, "Leave")]),
        // B523: the host's kick prompt; a card carried to another
        Screen::Lobby if m.net.kick.is_some() => v.extend([(Act::UpDown, "Move"), (Act::Ok, "Choose"), (Act::Back, "Cancel")]),
        Screen::Lobby if let Some(from) = m.carrying() => {
            moves(&mut v, (true, true));
            v.extend([(Act::Ok, if m.target_seat() == Some(from) { "Put Back" } else { "Drop" }), (Act::Back, "Cancel")]);
        }
        Screen::Lobby => {
            moves(&mut v, m.pill_axes());
            if m.lobby_bot() {
                v.extend([(Act::Costume, "Costume"), (Act::Card, "Remove")]);
            }
            if m.kickable().is_some() {
                v.push((Act::Card, "Kick"));
            }
            // B396: this machine's own card: its costume and hand right there
            if m.lobby_own() {
                v.extend([(Act::Costume, "Costume"), (Act::Hand, "Hand")]);
            }
            if let Some(ok) = m.lobby_ok() {
                v.push((Act::Ok, ok));
            }
            // B449: the host's □ and Start, from anywhere
            if m.lobby_mode() {
                v.push((Act::Page, "Singles / Doubles"));
            }
            if m.lobby_team().is_some() {
                v.push((Act::Page, "Swap team"));
            }
            if m.net.hosting && m.net.room.is_some() && m.counting() == 0 {
                v.push((Act::Name, "Start"));
            }
            v.push((Act::Back, "Leave"));
        }
        // B373: a kind the store doesn't carry yet has nothing to move to or choose
        // (B383: the kind switch only while the row shows more than one kind)
        Screen::Mods if m.add.empty_kind() => v.extend(m.add.kind_switch().into_iter().chain([(Act::Back, "Back")])),
        // B372: Browse's grid says what its tiles' buttons do
        Screen::Mods if let Some(l) = super::mods_screen::browse_legend(m) => v.extend(l),
        Screen::Mods => {
            moves(&mut v, (true, m.add.sideways(m.sel)));
            v.push((Act::Ok, "Choose"));
            if super::mods_screen::show_keys(m) {
                v.push((Act::Costume, "Show"));
            }
            v.extend(m.add.kind_switch().into_iter().chain([(Act::Back, "Back")]));
        }
        Screen::Extras => v.extend([(Act::UpDown, "Move"), (Act::Ok, "Open Ko-fi"), (Act::Back, "Back")]),
        _ => {}
    }
    // B558: the Settings pages over the select and the lobby
    if m.settings_here(dev) {
        v.push((Act::Settings, "Settings"));
    }
    // B320: chat works in an online room's lobby and character select (`chat.rs`, unless Settings turned it off)
    if m.net.room.is_some() && matches!(m.screen, Screen::Lobby | Screen::Chars) && !crate::chat::HIDDEN.load(std::sync::atomic::Ordering::Relaxed) {
        v.push((Act::Chat, "Chat"));
    }
    v.into_iter().map(|(a, l)| (a, l.to_string())).collect()
}

/// "Move" on whichever of ↑↓ and ←→ go somewhere.
fn moves(v: &mut Vec<(Act, &str)>, axes: (bool, bool)) {
    match axes {
        (true, true) => v.push((Act::Move, "Move")),
        (true, false) => v.push((Act::UpDown, "Move")),
        (false, true) => v.push((Act::LeftRight, "Move")),
        _ => {}
    }
}

/// The devices driving `m`'s screen and each one's player (None: not seated): the seated humans on the select and
/// seat assignment (and the last-used one there if it isn't seated: it can join), the host on Mode and Confirm, else
/// the last used.
fn drivers(m: &Menu, last: Dev) -> Vec<(Dev, Option<usize>)> {
    let seated = |m: &Menu| (0..m.players()).filter_map(|p| m.seats[p].map(|d| (d, Some(p)))).collect::<Vec<_>>();
    let seat = |d: Dev| (0..m.players()).find(|&p| m.seats[p] == Some(d));
    match m.screen {
        Screen::Chars | Screen::Assign => {
            let mut v = seated(m);
            // B437: the assignment's driver, taken off every seat until it leaves, still drives it
            if let Some(d) = m.lead().filter(|d| m.screen == Screen::Assign && !v.iter().any(|x| x.0 == *d)) {
                v.insert(0, (d, None));
            }
            v.dedup_by_key(|x| x.0);
            if (m.screen == Screen::Assign || m.can_join()) && !v.iter().any(|x| x.0 == last) {
                v.push((last, None));
            }
            v
        }
        Screen::Mode | Screen::Confirm if m.seats[0].is_some() => vec![(m.seats[0].unwrap(), Some(0))],
        _ => vec![(last, seat(last))],
    }
}

/// The main menus' legend: each driving device's entries, devices with the same ones in one group.
pub(super) fn groups(m: &Menu, last: Dev, bind: &Bindings) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    let mut seen = Vec::new();
    for (dev, p) in drivers(m, last) {
        if dev == Dev::Open || seen.contains(&dev) {
            continue;
        }
        seen.push(dev);
        let items: Vec<Item> =
            acts(m, dev).into_iter().map(|(act, label)| Item { act, keys: menu_keys(act, bind, m.screen == Screen::Chars), label, dim: kind_ends(m, act, p) }).collect();
        let id = match dev {
            Dev::Keys => None,
            Dev::Pad(e) => Some(e),
            Dev::Open => unreachable!(),
        };
        match out.iter_mut().find(|g| g.items == items) {
            Some(g) => g.devs.push(id),
            None => out.push(Group { tag: p.map(|p| format!("{}P", p + 1)), devs: vec![id], items }),
        }
    }
    out.retain(|g| !g.items.is_empty());
    if m.screen == Screen::Chars {
        out = shared(out);
    }
    if out.len() == 1 {
        out[0].tag = None;
    }
    out
}

/// B462: which half of the L2 / R2 hint (`act` [`Act::Kind`]) does nothing for player `p` on `m`'s screen: the
/// select's first section's first tile (L2) and last section (R2), the Mods page's first and last kind tab; B471:
/// L1 / R1's on the Mods page's first and last Show tab.
fn kind_ends(m: &Menu, act: Act, p: Option<usize>) -> [bool; 2] {
    match (act, m.screen, p) {
        (Act::Kind, Screen::Chars, Some(p)) => m.section_ends(p),
        (Act::Kind, Screen::Mods, _) => super::mods_kind::ends(m.add.kind, &m.add.kinds_shown()),
        (Act::Costume, Screen::Mods, _) => super::mods_screen::show_ends(&m.add),
        _ => [false; 2],
    }
}

/// B348: the select's players all do the same things, so its legend is one bar for every device instead of a tagged
/// group per player: each action once, in the select's order, its labels where players differ joined ("Cancel /
/// Leave").
fn shared(groups: Vec<Group>) -> Vec<Group> {
    const ORDER: [Act; 7] = [Act::Move, Act::Costume, Act::Hand, Act::Page, Act::Ok, Act::Back, Act::Chat];
    let rank = |a: Act| ORDER.iter().position(|&o| o == a).unwrap_or(ORDER.len());
    if groups.is_empty() {
        return groups;
    }
    let mut one = Group { tag: None, devs: Vec::new(), items: Vec::new() };
    let mut labels: Vec<Vec<String>> = Vec::new();
    for g in groups {
        one.devs.extend(g.devs);
        for it in g.items {
            match one.items.iter().position(|x| x.act == it.act) {
                Some(i) => {
                    // B462: a half dims only where it does nothing for every player
                    let d = &mut one.items[i].dim;
                    *d = [d[0] && it.dim[0], d[1] && it.dim[1]];
                    if !labels[i].contains(&it.label) {
                        labels[i].push(it.label);
                    }
                }
                None => {
                    labels.push(vec![it.label.clone()]);
                    one.items.push(it);
                }
            }
        }
    }
    for (it, l) in one.items.iter_mut().zip(labels) {
        it.label = l.join(" / ");
    }
    one.items.sort_by_key(|it| rank(it.act));
    vec![one]
}

/// Fills the legend from the main menu (and its Settings screens over a paused match).
pub(super) fn fill(menu: Res<Menu>, last: Res<Last>, bind: Res<Bindings>, mut legend: ResMut<Legend>) {
    let dev = match *last {
        Last::Pad(e) => Dev::Pad(e),
        _ => Dev::Keys,
    };
    let style = dev_style(last.dev());
    LAST_STYLE.store(Style::ALL.iter().position(|s| *s == style).unwrap_or(0) as u8, Ordering::Relaxed);
    legend.0 = groups(&menu, dev, &bind);
}

#[cfg(test)]
mod tests {
    use super::super::{Effect, Press, online};
    use super::*;

    const DIRS: [fn(&mut Press) -> &mut bool; 4] = [|p| &mut p.up, |p| &mut p.down, |p| &mut p.left, |p| &mut p.right];

    fn press(f: fn(&mut Press) -> &mut bool) -> Press {
        let mut p = Press::default();
        *f(&mut p) = true;
        p
    }

    /// Whether `dev`'s press does anything on `m`: the menu ends up unlike a step with nothing pressed (which fills
    /// in what a screen sets up lazily), or it asks for more than the refusal sound.
    fn handled(m: &Menu, dev: Dev, f: fn(&mut Press) -> &mut bool) -> bool {
        let (mut idle, mut n) = (m.clone(), m.clone());
        idle.step(dev, Press::default());
        let fx = n.step(dev, press(f));
        format!("{n:?}") != format!("{idle:?}") || fx.iter().any(|e| *e != Effect::Sound(1))
    }

    /// Every screen's legend (each state reachable with the d-pad from a start, on that screen) lists exactly the
    /// presses `Menu::step` takes there: a direction axis, ✕, ○, △, L1/R1, □, Select.
    #[test]
    fn legend_lists_what_each_screen_handles() {
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let base = |screen| {
            let mut m = Menu { screen, doubles: true, seats: [Some(Dev::Keys), None, None, None], ..default() };
            m.players[0].cursor = 6;
            m
        };
        let online = |screen| {
            let mut m = base(screen);
            online::sample(&mut m);
            m
        };
        let mut starts: Vec<(Menu, Dev)> = vec![(Menu::default(), Dev::Keys)];
        for s in [Screen::Display, Screen::Graphics, Screen::Advanced, Screen::Sound, Screen::ModSettings, Screen::Controls, Screen::Mode, Screen::Assign, Screen::Chars, Screen::Confirm, Screen::Online, Screen::Mods, Screen::Extras] {
            starts.push((base(s), Dev::Keys));
        }
        let mut m = base(Screen::Main);
        m.add.ask = Some(0);
        starts.push((m, Dev::Keys));
        // B373: a Mods kind tab the store doesn't carry yet
        let mut m = base(Screen::Mods);
        m.add.kind = crate::modstore::Kind::Umpire;
        starts.push((m, Dev::Keys));
        // B326: a Settings row picked, on every row of every page (B411: Advanced, which opens its page, never is)
        for s in [Screen::Display, Screen::Graphics, Screen::Advanced, Screen::Sound, Screen::ModSettings] {
            for sel in (0..options::page(s).len()).filter(|&i| options::page(s)[i] != options::Opt::Advanced) {
                let mut m = base(s);
                (m.sel, m.editing) = (sel, Some(m.settings));
                starts.push((m.clone(), Dev::Keys));
                // B498: its dropdown open
                if options::listed(options::page(s)[sel]) {
                    m.listing = Some(1);
                    starts.push((m, Dev::Keys));
                }
            }
        }
        let mut m = base(Screen::Controls);
        m.rebinding = Some(0);
        starts.push((m, Dev::Keys));
        // seat assignment: a pad not yet in, and in
        starts.push((base(Screen::Assign), pad));
        let mut m = base(Screen::Assign);
        m.seats[1] = Some(pad);
        starts.push((m.clone(), pad));
        // the select: two humans, one or both ready; the court / umpire pickers
        m.screen = Screen::Chars;
        starts.push((m.clone(), pad));
        m.players[1].ready = true;
        starts.push((m.clone(), pad));
        m.players[0].ready = true;
        starts.push((m, pad));
        // B371: 2P's name picker: the list (a saved name, New name, No name), then typing one
        let mut m = base(Screen::Assign);
        (m.screen, m.seats[1]) = (Screen::Chars, Some(pad));
        m.names.saved = vec!["ANNA".into()];
        m.names.open[1] = Some(super::super::names::Picker::List(0));
        starts.push((m.clone(), pad));
        m.names.open[1] = Some(super::super::names::Picker::Typing("AB".into(), 0));
        starts.push((m, pad));
        for sel in [1, 4] {
            let mut m = base(Screen::Confirm);
            (m.sel, m.picking) = (sel, Some(0));
            starts.push((m, Dev::Keys));
        }
        for s in [Screen::Lobbies, Screen::Lobby, Screen::Code] {
            starts.push((online(s), Dev::Keys));
        }
        let mut m = online(Screen::Lobby);
        m.net.ask = Some(7);
        starts.push((m, Dev::Keys));
        // B484: the prompt over the select and the confirm
        for s in [Screen::Chars, Screen::Confirm] {
            let mut m = online(s);
            m.net.ask = Some(7);
            starts.push((m, Dev::Keys));
        }
        let mut m = online(Screen::Lobby);
        m.net.hosting = false;
        starts.push((m, Dev::Keys));
        let mut m = online(Screen::Code);
        (m.net.naming, m.net.entry) = (true, String::new());
        starts.push((m, Dev::Keys));

        let mut checked = 0;
        for (mut start, last) in starts {
            // as every step leaves it: nobody not ready on a taken costume
            start.skip_taken();
            let screen = start.screen;
            let mut seen = std::collections::HashSet::new();
            let mut todo = vec![start];
            while let Some(m) = todo.pop() {
                if !seen.insert(format!("{m:?}")) || seen.len() > 400 {
                    continue;
                }
                for f in DIRS {
                    let mut n = m.clone();
                    n.step(last, press(f));
                    if n.screen == screen {
                        todo.push(n);
                    }
                }
                for (dev, _) in drivers(&m, last) {
                    let acts = acts(&m, dev);
                    let has = |a: &[Act]| acts.iter().any(|(x, _)| a.contains(x));
                    let h = |f| handled(&m, dev, f);
                    // ✕ there reloads the game, opens a file picker or reaches the store: taken as handled
                    let ok = matches!(m.screen, Screen::Mods) && !m.add.empty_kind() || m.screen == Screen::Main && (m.add.ask.is_some() || m.stale.0 == Some(0)) || h(|p| &mut p.ok);
                    let got = [
                        h(|p| &mut p.up) || h(|p| &mut p.down),
                        h(|p| &mut p.left) || h(|p| &mut p.right),
                        ok,
                        h(|p| &mut p.back),
                        h(|p| &mut p.card),
                        h(|p| &mut p.prev) || h(|p| &mut p.next),
                        h(|p| &mut p.page),
                        h(|p| &mut p.hand),
                        h(|p| &mut p.kind_prev) || h(|p| &mut p.kind_next),
                        h(|p| &mut p.name),
                        h(|p| &mut p.settings),
                    ];
                    let want = [
                        has(&[Act::Move, Act::UpDown]),
                        has(&[Act::Move, Act::LeftRight]),
                        has(&[Act::Ok, Act::Select]),
                        has(&[Act::Back]),
                        has(&[Act::Card]),
                        has(&[Act::Costume]),
                        has(&[Act::Page]),
                        has(&[Act::Hand]),
                        has(&[Act::Kind]),
                        has(&[Act::Name, Act::Go]),
                        has(&[Act::Settings]),
                    ];
                    assert_eq!(got, want, "{:?} sel {} {dev:?}: [↕ ↔ ✕ ○ △ L1R1 □ Select L2R2 Start R3] handled vs legend {acts:?}", m.screen, m.sel);
                    checked += 1;
                }
            }
        }
        assert!(checked > 200, "{checked}");
    }

    /// B463: the select's keyboard caps: Q / E the sections (as Settings' tabs), Z / X the costume, 1-4 the emotes (a
    /// pad's right stick, B475: one hint for its four ways); the Mods page's kind tabs stay 1 / 3.
    #[test]
    fn select_keys() {
        let mut m = Menu { screen: Screen::Chars, seats: [Some(Dev::Keys), None, None, None], ..default() };
        m.players[0].cursor = 1;
        let bind = Bindings::default();
        let g = groups(&m, Dev::Keys, &bind);
        let keys = |a: Act| g[0].items.iter().find(|it| it.act == a).map(|it| it.keys.clone());
        assert_eq!(menu_keys(Act::Kind, &bind, true), ["Q", "E"]);
        assert_eq!(keys(Act::Costume), Some(vec!["Z".into(), "X".into()]));
        assert_eq!(keys(Act::Emote), Some(vec!["1-4".into()]));
        // B475: one Emote hint for the right stick's four ways and the emote keys, no R / F
        assert_eq!(g[0].items.iter().filter(|it| it.label.contains("Emote") || it.label.contains("Win")).count(), 1);
        assert_eq!(button_names(None, Act::Costume, &bind), ["Z", "X"]);
        assert_eq!(glyphs(Style::Xbox, &item(Act::Emote, "", &[])), [Glyph::Stick]);
        assert_eq!(menu_keys(Act::Kind, &bind, false), ["1", "3"]);
    }

    /// B348: the select's legend is one untagged bar for every player's device, each action once.
    #[test]
    fn select_legend_is_one_shared_bar() {
        let pad = Dev::Pad(Entity::from_raw_u32(9).unwrap());
        let mut m = Menu { screen: Screen::Chars, doubles: true, seats: [Some(Dev::Keys), Some(pad), None, None], ..default() };
        m.players[0].cursor = 6;
        let bind = Bindings::default();
        for ready in [false, true] {
            m.players[1].ready = ready;
            let g = groups(&m, Dev::Keys, &bind);
            assert_eq!(g.len(), 1, "{g:?}");
            assert_eq!(g[0].tag, None);
            assert_eq!(g[0].devs.len(), 2);
            let shown: Vec<Act> = g[0].items.iter().map(|it| it.act).collect();
            let mut once = shown.clone();
            once.dedup();
            assert_eq!(shown, once, "{shown:?}");
            for dev in [Dev::Keys, pad] {
                for (a, l) in acts(&m, dev) {
                    let it = g[0].items.iter().find(|it| it.act == a).expect("every player's action");
                    assert!(it.label.contains(&l), "{l} in {}", it.label);
                }
            }
        }
    }
}
