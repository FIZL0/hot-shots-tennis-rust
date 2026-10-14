//! The Online screens (M2f, `net.rs`): Host, Find, Quickplay, Join by Code and the relay setting. Find lists every open
//! lobby first (singles and doubles together); Host and Quickplay ask singles/doubles, Join by Code and a link take the
//! lobby's (B225). Then this
//! machine's 1–3 players take their controllers (no more than the lobby's open seats) and join (or open) the lobby on
//! placeholder picks, which holds their seats while they pick on the same character select as Local (B195; no
//! computer players: the open seats are for the other peers). The lobby: its seats (who, their name, picking or
//! ready), the code and QR to share, Start (the open seats to the computer), Ready, Characters (pick again), Court
//! (the host's confirm screen: court, sets, games, umpire, until Start), Leave and taking a player out. The host adds a
//! computer player to an open seat and picks its character, costume and level on the select with its own cursor
//! (B216; △ removes it); its pick goes into the room like a person's. The player's
//! name (Name, `online.txt`'s `name`) is typed on Join by Code's keys.

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::pointer::{Act, OK};
use super::{Draw, Effect, Menu, Press, Screen, Tex, WHITE};
use crate::mods_online::{self as mo, Ask, Wire};
use crate::net::{Cmd, Event, Listing, Live, Msg, Net, Online, Pick, Relay, Room, SHORT, SYMBOLS, Seat, is_short};

/// What Online does once this machine's players are picked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Go {
    Host,
    Find,
    Quick,
    Code,
}

/// The Online screens' state, in `Menu`.
#[derive(Clone, Debug, Default)]
pub struct View {
    pub go: Option<Go>,
    pub cfg: Option<Online>,
    pub status: String,
    pub code: String,
    /// Hosting: the short code (B104; empty until it's on the DHT, or if it can't be).
    pub short: String,
    /// Join by Code's code to join (typed, pasted or from the link).
    pub paste: String,
    /// Join by Code's typed short code so far.
    pub entry: String,
    pub room: Option<Room>,
    pub list: Vec<Listing>,
    pub started: bool,
    /// Mods in the lobby (M2i), by their looks (pack hash): where this machine has them (`None`: a stand-in plays
    /// it), the ones asked for and not here yet, the one the prompt asks about, the line-up last reported settled.
    pub looks: HashMap<u64, Option<PathBuf>>,
    pub wanted: HashSet<u64>,
    pub ask: Option<u64>,
    pub ready: Option<u32>,
    /// B281: what this machine last told the host about still choosing about a mod.
    pub deciding: bool,
    /// This machine hosts the lobby; as a guest, it said it's ready.
    pub hosting: bool,
    pub prepared: bool,
    /// The code screen types this player's name, not a code.
    pub naming: bool,
    /// Find: the chosen lobby's open seats, the most players this machine may bring (0: no lobby chosen).
    pub fit: usize,
    /// Joining or opening a lobby: its first line-up opens the character select (B195).
    pub pick_next: bool,
    /// B201: this machine's peer name in the seats ("host", or a guest's endpoint ID; empty until known), and the
    /// select's places last sent.
    pub me: String,
    pub sent: Option<Vec<Live>>,
    /// B236: Find's lists still to come (two a search: the LAN's, then the DHT's), what the last failure stopped (its
    /// panel's title), and the spinner's clock (seconds).
    pub finding: u8,
    pub fail: Option<&'static str>,
    pub spin: f32,
    /// B216: the host's computer player (its seat) whose character it's picking on the select.
    pub edit: Option<usize>,
    /// The court or umpire select was opened from the lobby's settings row (closing it goes back there).
    pub venue: bool,
}

/// What an Online press asks of the app.
#[derive(Debug, PartialEq)]
pub enum Fx {
    Net(Cmd),
    /// Stop the network (back to the main menu, or the relay changed).
    Off,
    Paste,
    Copy,
    SaveCfg,
    /// Read `online.txt` (opening the Online screen).
    Load,
}

/// Join by Code's symbol keys (then its four buttons).
const KEYS: usize = SYMBOLS.len();
/// The name's keys on the same grid, and its longest.
const NAME_KEYS: &[u8; 40] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -.!";
const NAME_MAX: usize = 16;
const ROWS: [&str; 7] = ["Quickplay", "Host", "Find", "Join by Code", "Name", "Relay", "Back"];
/// The Online screen as the home screen (`home::columns`): Quickplay, Host, Find and Join by Code as its list's
/// pills, Name and Relay small under them; the preview card right of them (no store column: the card centred).
fn online_cols(extra: f32) -> (f32, f32, f32) {
    let (x, px, pw, store) = super::home::columns(extra);
    let shift = if store.is_some() { (16.0 + 170.0) / 2.0 } else { 0.0 };
    (x + shift, px + shift, pw)
}

/// Where `ROWS` are drawn (Back has none: the title tab and ○ are Back).
fn row_rects(extra: f32) -> Vec<[f32; 4]> {
    let (x, ..) = online_cols(extra);
    let mut v: Vec<[f32; 4]> = (0..4).map(|i| [x, 50.0 + 46.0 * i as f32, 200.0, 38.0]).collect();
    v.extend((0..2).map(|i| [x, 254.0 + 30.0 * i as f32, 200.0, 26.0]));
    v
}

/// The lobby's buttons: the host's, a seated guest's (Ready shows "Not Ready" once ready), and with no lobby. B214:
/// no Vote button (B226 removes lobby voting).
/// B213: "Public" is the host's Public/Private switch (labelled with what the lobby is).
const HOST_BUTTONS: [&str; 5] = ["Start", "Characters", "Copy Link", "Public", "Leave"];
const GUEST_BUTTONS: [&str; 3] = ["Ready", "Characters", "Leave"];

/// What a lobby selection is: a guest's seat (the host can take them out) or a button.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
    Seat(usize),
    /// the ✕ beside a computer player's seat: takes it off.
    Remove(usize),
    Button(&'static str),
    /// B201: the host's singles / doubles switch (the settings row's last pill).
    Mode,
    /// The host's settings row (`SETTINGS` of them): the court and umpire (✕ opens their select), sets, games, mods
    /// (←→ change them for everyone).
    Set(usize),
}
/// The settings row's pills before Mode: court, umpire, sets, games, mods.
const SETTINGS: usize = 5;
/// The prompt for a mod this machine doesn't have: fetch its looks or not, and whether to ask again.
const ASK_ROWS: [(&str, bool, Ask); 4] =
    [("Download, always", true, Ask::Always), ("Download this one", true, Ask::Each), ("Stand-in this time", false, Ask::Each), ("Never download", false, Ask::Never)];

impl Menu {
    pub(super) fn online(&self) -> bool {
        self.net.go.is_some()
    }

    /// The most players this machine may bring online.
    pub(super) fn local_max(&self) -> usize {
        let n = if self.doubles { 3 } else { 1 };
        if self.net.fit > 0 { n.min(self.net.fit) } else { n }
    }

    /// This machine's players' picks, in seat order.
    fn picks(&self) -> Vec<Pick> {
        (0..self.players()).filter(|&p| self.seats[p].is_some()).map(|p| self.wire(p)).collect()
    }

    /// Player `p`'s pick as it travels.
    fn wire(&self, p: usize) -> Pick {
        let random = self.players[p].random;
        let pick = match self.pick(p) {
            // a mod: its donor, and its gameplay data for every peer
            (super::Who::Mod(i), costume) => {
                let m = &self.mods[i];
                Pick { char: m.donor, costume, r#mod: Wire::of(m, costume).map_err(|e| warn!("{e}")).ok().map(Box::new), random: false }
            }
            (super::Who::Disc(c), costume) => Pick::disc(c, costume),
        };
        Pick { random, ..pick }
    }

    /// B216: online, the player `dev` drives on the select (it sits in seat `p`): the host's first device drives the
    /// computer player it's picking for, every other its own.
    pub(super) fn online_driven(&self, dev: super::Dev, p: usize) -> Option<usize> {
        let first = (0..4).find(|&q| self.seats[q].is_some()).and_then(|q| self.seats[q]);
        match self.net.edit {
            Some(i) if self.net.hosting && first == Some(dev) => Some(i),
            _ => Some(p),
        }
    }

    /// B216: the host's lobby into the select to pick seat `i`'s computer player (`new`: the one just added).
    fn edit_bot(&mut self, i: usize, new: Option<&Pick>, out: &mut Vec<Effect>) {
        self.align();
        if let Some(p) = new {
            self.players[i] = super::Player { cursor: super::GRID.iter().position(|g| g.0 == p.char).unwrap_or(0), costume: p.costume, ..default() };
            // B269: the cursor on the cell the last pick readied on
            if let Some(c) = self.last {
                self.put(i, c);
                self.skip_taken();
            }
        }
        self.players[i].ready = false;
        self.filter = 0;
        self.net.edit = Some(i);
        self.net.status.clear();
        self.go(Screen::Chars, out);
    }

    /// B216: the host's computer player in seat `p` ready on the select: its pick to the lobby, which carries it to
    /// everyone and into the handshake at Start like a person's.
    pub(super) fn online_bot_ready(&mut self, p: usize, out: &mut Vec<Effect>) {
        self.net.edit = None;
        self.go(Screen::Lobby, out);
        out.push(Effect::Online(Fx::Net(Cmd::Bot(p, Some(self.wire(p))))));
    }

    /// The lobby this machine would host: its players in the first seats, the rest open.
    fn room(&self) -> Room {
        let cfg = self.net.cfg.clone().unwrap_or_default();
        let mut seats: Vec<Option<Seat>> = vec![None; self.players()];
        for (s, pick) in seats.iter_mut().zip(self.picks()) {
            *s = Some(Seat { peer: "host".into(), pick });
        }
        let (court, umpire) = self.venue();
        let names = [("host".to_string(), cfg.name.clone())].into();
        let picking = ["host".to_string()].into();
        Room { name: cfg.name, doubles: self.doubles, court, sets: self.sets, games: self.games, umpire, seats, names, picking, private: cfg.private, ..Default::default() }
    }

    /// The code screen's keys: the short code's symbols, or the name's.
    pub(super) fn keys(&self) -> &'static [u8] {
        if self.net.naming { NAME_KEYS } else { SYMBOLS }
    }

    /// The lobby's selectable things: the guests' seats (host), then the buttons.
    fn items(&self) -> Vec<Item> {
        let room = self.net.room.as_ref();
        if self.net.hosting {
            // B201: every seat but its own: a guest's (take out), an open one (add a computer player), a computer's
            let seats = room.map_or(vec![], |r| (0..r.seats.len()).filter(|&i| r.seats[i].as_ref().is_none_or(|s| s.peer != "host")).collect());
            let bot = |i: usize| room.is_some_and(|r| r.seats[i].as_ref().is_some_and(|s| !s.human()));
            seats.into_iter().flat_map(|i| [Some(Item::Seat(i)), bot(i).then_some(Item::Remove(i))]).flatten().chain(HOST_BUTTONS.map(Item::Button)).chain((0..SETTINGS).map(Item::Set)).chain([Item::Mode]).collect()
        } else if room.is_some() && !self.net.started && !self.net.pick_next {
            GUEST_BUTTONS.map(Item::Button).to_vec()
        } else {
            vec![Item::Button("Leave")]
        }
    }

    /// B295: where `items` are drawn (640×448, `Grid::split`): a seat's row, a computer's ✕ at its end, the title
    /// line (singles / doubles), and on the side card Copy Link and Public by the code, Characters and Court under it,
    /// Start (a guest's Ready) big at its foot over Leave.
    pub(super) fn lobby_rects(&self) -> Vec<[f32; 4]> {
        let (col, [sx, sy, sw, sh]) = self.lobby_split();
        let doubles = self.net.room.as_ref().map_or(self.doubles, |r| r.doubles);
        let row = self.settings_row();
        self.items()
            .iter()
            .map(|it| match *it {
                Item::Seat(i) => super::seat_rect(col, doubles, i),
                Item::Remove(i) => {
                    let [x, y, w, _] = super::seat_rect(col, doubles, i);
                    [x + w - 32.0, y + 4.0, 26.0, 26.0]
                }
                Item::Mode => row[SETTINGS],
                Item::Set(i) => row[i],
                Item::Button("Start" | "Ready") => [sx, sy + sh - 78.0, sw, 44.0],
                Item::Button("Leave") => [sx + 24.0, sy + sh - 28.0, sw - 48.0, 26.0],
                Item::Button("Characters") => [sx, sy + 158.0, sw, 30.0],
                Item::Button("Copy Link") => [sx + 104.0, sy + 102.0, sw - 112.0, 18.0],
                Item::Button(_) => [sx + 104.0, sy + 124.0, sw - 112.0, 18.0],
            })
            .collect()
    }

    /// The lobby's seats column and side card (`Grid::split`), the side card beside the seats starting under the
    /// settings row.
    fn lobby_split(&self) -> ([f32; 4], [f32; 4]) {
        let (col, [sx, sy, sw, sh]) = self.grid.split();
        let top = self.grid.reach()[1] + 80.0;
        if sy < col[1] && sy < top { (col, [sx, top, sw, sh - (top - sy)]) } else { (col, [sx, sy, sw, sh]) }
    }

    /// The lobby's settings row under the header, across the screen (`Lobby.dc`): court, umpire, sets, games, mods,
    /// singles / doubles.
    fn settings_row(&self) -> [[f32; 4]; SETTINGS + 1] {
        let (col, [sx, sy, sw, _]) = self.grid.split();
        let [_, t, ..] = self.grid.reach();
        // the side card beside the seats: across both; under them (portrait): across the seats
        let (x, r) = if sy < col[1] { (col[0], sx + sw) } else { (col[0], col[0] + col[2]) };
        const PARTS: [f32; SETTINGS + 1] = [1.7, 1.15, 0.8, 0.8, 1.0, 1.05];
        let unit = (r - x - 6.0 * SETTINGS as f32) / PARTS.iter().sum::<f32>();
        let mut at = x;
        PARTS.map(|p| {
            let r = [at, t + 38.0, p * unit, 36.0];
            at += p * unit + 6.0;
            r
        })
    }

    /// The host's ←→ (or ✕) on settings pill `i` (2: sets, 3: games, 4: mods): the next value, sent to everyone.
    fn change_setting(&mut self, i: usize, d: isize, out: &mut Vec<Effect>) {
        let step = |v: usize, n: usize| (v as isize + d).rem_euclid(n as isize) as usize;
        match i {
            2 => self.sets = step(self.sets, super::SETS.len()),
            3 => self.games = super::GAMES[step(super::GAMES.iter().position(|&g| g >= self.games).unwrap_or(1), super::GAMES.len())],
            4 => {
                let vanilla = self.net.room.as_ref().is_some_and(|r| r.vanilla);
                return out.extend([Effect::Sound(0), Effect::Online(Fx::Net(Cmd::Vanilla(!vanilla)))]);
            }
            _ => return,
        }
        self.send_venue(out);
    }

    /// The host's court, sets, games and umpire to everyone.
    fn send_venue(&mut self, out: &mut Vec<Effect>) {
        let (court, umpire) = self.venue();
        out.extend([Effect::Sound(0), Effect::Online(Fx::Net(Cmd::Venue(court, self.sets, self.games, umpire)))]);
    }

    /// The lobby's Court or Umpire pill (`i` 0 / 1): their select (the confirm screen's picker), back to the lobby
    /// once it's kept or put back (`venue_picked`).
    fn pick_venue(&mut self, i: usize, out: &mut Vec<Effect>) {
        self.net.venue = true;
        self.screen = Screen::Confirm;
        self.sel = [1, 4][i];
        self.picking = Some(if i == 0 { self.court } else { self.umpire as usize });
        out.push(Effect::Sound(2));
    }

    /// The court or umpire select opened from the lobby closed: back on its pill, the venue sent if it changed.
    pub(super) fn venue_picked(&mut self, out: &mut Vec<Effect>) {
        if !std::mem::take(&mut self.net.venue) {
            return;
        }
        let pill = Item::Set(if self.sel == 1 { 0 } else { 1 });
        self.screen = Screen::Lobby;
        self.sel = self.items().iter().position(|it| *it == pill).unwrap_or(0);
        self.send_venue(out);
    }

    /// M14: the chosen lobby item is a computer player's seat (←→ and L1 / R1 change its pick).
    /// M14: on Online or the lobby, whether ↑↓ and ←→ reach another pill from the chosen one.
    pub(super) fn pill_axes(&self) -> (bool, bool) {
        // the settings row: ←→ change it (`lobby_setting`), ↑↓ go along it
        if self.screen == Screen::Lobby && self.net.ask.is_none() && matches!(self.items().get(self.sel), Some(Item::Mode | Item::Set(_))) {
            return (true, false);
        }
        let rects = if self.screen == Screen::Lobby { self.lobby_rects() } else { row_rects(self.grid.extra) };
        let to = |f: fn(&mut Press) -> &mut bool| {
            let mut k = Press::default();
            *f(&mut k) = true;
            pill_step(&rects, self.sel, k) != self.sel
        };
        (to(|k| &mut k.up) || to(|k| &mut k.down), to(|k| &mut k.left) || to(|k| &mut k.right))
    }

    /// The chosen lobby item is one of the host's settings row (←→ change it).
    pub(super) fn lobby_setting(&self) -> bool {
        self.screen == Screen::Lobby
            && self.net.ask.is_none()
            && match self.items().get(self.sel) {
                Some(Item::Set(i)) => *i >= 2,
                Some(Item::Mode) => self.can_switch(),
                _ => false,
            }
    }

    /// Singles / doubles can switch: singles only while the taken seats fit in two.
    fn can_switch(&self) -> bool {
        self.net.room.as_ref().is_some_and(|r| !r.doubles || r.seats.iter().flatten().count() <= 2)
    }

    pub(super) fn lobby_bot(&self) -> bool {
        matches!(self.items().get(self.sel), Some(Item::Seat(i)) if self.net.room.as_ref().is_some_and(|r| r.seats[*i].as_ref().is_some_and(|s| !s.human())))
    }

    /// M14: what ✕ does on the chosen lobby item, as `online_step` takes it (None: nothing).
    pub(super) fn lobby_ok(&self) -> Option<&'static str> {
        Some(match self.items().get(self.sel).copied().unwrap_or(Item::Button("Leave")) {
            Item::Seat(i) if self.guest(i).is_some() => "Take Out",
            Item::Seat(_) if self.lobby_bot() => "Pick",
            Item::Seat(i) => return self.bot_pick(i, 0, 0).map(|_| "Add Computer"),
            Item::Remove(_) => "Remove",
            Item::Mode if self.can_switch() => "Singles / Doubles",
            Item::Mode => return None,
            Item::Set(0 | 1) => "Choose",
            Item::Set(_) => "Change",
            Item::Button("Copy Link") => return shared(&self.net).map(|_| "Copy Link"),
            Item::Button("Start" | "Ready") if self.counting() > 0 => "Cancel",
            Item::Button("Ready") if self.net.prepared => "Not Ready",
            Item::Button("Public") if self.net.room.as_ref().is_some_and(|r| r.private) => "Make Public",
            Item::Button("Public") => "Make Private",
            Item::Button(b @ ("Start" | "Characters" | "Ready" | "Leave")) => b,
            Item::Button(_) => return None,
        })
    }

    /// B215: the start's countdown, seconds left (0: none).
    fn counting(&self) -> u32 {
        self.net.room.as_ref().map_or(0, |r| r.count)
    }

    /// A guest's seat's peer and name.
    fn guest(&self, seat: usize) -> Option<(String, String)> {
        let r = self.net.room.as_ref()?;
        let s = r.seats.get(seat)?.as_ref().filter(|s| s.human())?;
        Some((s.peer.clone(), name_of(r, &s.peer)))
    }

    /// B201: lobby seat `p` when another peer's player or the computer has it (this machine's players are in their
    /// own seats' panels on the select, `align`).
    pub(super) fn remote(&self, p: usize) -> Option<&Seat> {
        let r = self.net.room.as_ref().filter(|_| self.online() && !self.net.me.is_empty())?;
        r.seats.get(p)?.as_ref().filter(|s| s.peer != self.net.me)
    }

    /// B234: the name of the mod another machine's player in seat `p` is on, when this machine doesn't have it (the
    /// select shows its donor's looks); `None` for a disc character or a mod of this machine's own.
    pub(super) fn remote_mod_name(&self, p: usize) -> Option<&str> {
        let s = self.remote(p)?;
        let r = self.net.room.as_ref()?;
        let (id, name) = match r.live_of(p).filter(|_| s.human()) {
            Some(l) => (l.r#mod.as_deref()?, l.name.as_str()),
            None => s.pick.r#mod.as_deref().map(|w| (w.id.as_str(), w.name.as_str()))?,
        };
        (!self.mods.iter().any(|m| m.id == id) && !name.is_empty()).then_some(name)
    }

    /// B201: the select's panels as the lobby's seats: this machine's players moved into the seats it holds (in
    /// order), every other seat showing its player's live place (a computer's pick, ready; an open seat empty).
    pub(super) fn align(&mut self) {
        let Some(r) = self.net.room.clone().filter(|_| self.online() && !self.net.me.is_empty() && !self.net.started) else { return };
        let mine: Vec<usize> = (0..r.seats.len()).filter(|&i| r.seats[i].as_ref().is_some_and(|s| s.peer == self.net.me)).collect();
        let local: Vec<_> = (0..4).filter_map(|p| Some((self.seats[p]?, self.players[p]))).collect();
        if mine.len() != local.len() || r.seats.len() > 4 {
            return;
        }
        self.doubles = r.doubles;
        self.seats = [None; 4];
        for (&i, (d, pl)) in mine.iter().zip(local) {
            (self.seats[i], self.players[i]) = (Some(d), pl);
        }
        for i in (0..4).filter(|i| !mine.contains(i) && self.net.edit != Some(*i)) {
            let Some(s) = r.seats.get(i).cloned().flatten() else {
                self.players[i] = default();
                continue;
            };
            let (char, costume, ready, id) = match r.live_of(i).filter(|_| s.human()) {
                Some(l) => (l.char, l.costume, l.ready, l.r#mod.clone()),
                None => (s.pick.char, s.pick.costume, !r.picking.contains(&s.peer), s.pick.r#mod.as_ref().map(|w| w.id.clone())),
            };
            let mut pl = super::Player { cursor: super::GRID.iter().position(|g| g.0 == char).unwrap_or(0), costume, ready, ..default() };
            match id.map(|id| self.mods.iter().position(|m| m.id == id)) {
                Some(Some(m)) => (pl.modded, pl.pick) = (true, m),
                // ponytail: a mod this machine doesn't have shows as its donor's first costume
                Some(None) => pl.costume = 0,
                None => {}
            }
            // a computer added on Random stays on it when the host picks for it again
            pl.random = !s.human() && s.pick.random;
            self.players[i] = pl;
        }
    }

    /// B285: how many of `mods` are this machine's roster; after them come the lobby's mods it has the looks of but
    /// hasn't installed ([`Menu::borrow`]: online ones, with the lobby's skeleton), shown but never on the grid.
    pub(super) fn own_mods(&self) -> usize {
        self.mods.iter().position(|m| m.skeleton.is_some()).unwrap_or(self.mods.len())
    }

    /// B285: the lobby's mods whose looks this machine received (or kept from an earlier lobby) put after the roster
    /// in `mods` (their select text in `text`), so the lobby and the select draw a downloaded mod as itself; gone with
    /// the lobby.
    fn borrow(&mut self, text: &mut super::Text) {
        let own = self.own_mods();
        let mut want: Vec<(Wire, PathBuf)> = Vec::new();
        for (_, w) in self.net.room.iter().flat_map(|r| r.mods()) {
            if let Some(Some(d)) = self.net.looks.get(&w.looks)
                && !self.mods[..own].iter().any(|m| m.id == w.id)
                && !want.iter().any(|(x, _)| x.id == w.id)
            {
                want.push((w.clone(), d.clone()));
            }
        }
        if self.mods[own..].iter().map(|m| (&m.id, &m.dir)).eq(want.iter().map(|(w, d)| (&w.id, d))) {
            return;
        }
        self.mods.truncate(own);
        text.mods.truncate(own);
        for (w, d) in want {
            let mut m = mo::for_match(&w, Some(&d));
            if m.costumes.is_empty() {
                // its looks didn't read: the donor stands in, here and in the match
                self.net.looks.insert(w.looks, None);
                continue;
            }
            (m.id, m.costume_of) = (w.id, None);
            // ponytail: the grades are its base row's (its stat overrides need the disc's TParam, `mods::tparam`)
            text.mods.push((super::mod_style(&m, &text.style), text.grades[m.base]));
            self.mods.push(m);
        }
    }

    /// B201: this machine's players' places on the select, for the others to see.
    fn lives(&self) -> Vec<Live> {
        (0..4)
            .filter(|&p| self.seats[p].is_some())
            .map(|p| {
                let (who, costume) = self.pick(p);
                let (char, id, name) = match who {
                    super::Who::Disc(c) => (c, None, String::new()),
                    super::Who::Mod(i) => (self.mods[i].donor, Some(self.mods[i].id.clone()), self.mods[i].name.clone()),
                };
                Live { char, costume, ready: self.players[p].ready, r#mod: id, name }
            })
            .collect()
    }

    /// B201: the host's computer player in open seat `i` changed by `dc` characters (grid order) and `dk` costumes,
    /// on a disc character and costume nobody else has (B66); `None` if every one is.
    fn bot_pick(&self, i: usize, dc: isize, dk: isize) -> Option<Pick> {
        let r = self.net.room.as_ref()?;
        let taken = self.bot_taken(i)?;
        let now = r.seats.get(i)?.as_ref().map(|s| &s.pick);
        let (n, nk) = (super::CHARS as isize, super::COSTUMES as isize);
        let free = |c: usize, k: isize| Some((c, k.rem_euclid(nk) as usize)).filter(|ck| !taken.contains(ck)).map(|(c, k)| Pick::disc(c, k));
        if let Some(p) = now.filter(|_| dk != 0) {
            return (1..=nk).find_map(|j| free(p.char, p.costume as isize + dk * j));
        }
        // the next character that way (adding: the grid's first) on its first free costume
        let at = now.map_or(0, |p| super::GRID.iter().position(|g| g.0 == p.char).unwrap_or(0) as isize);
        let (step, from) = if dc == 0 { (1, 0) } else { (dc, 1) };
        (from..from + n).map(|j| super::GRID[(at + step * j).rem_euclid(n) as usize].0).find_map(|c| (0..nk).find_map(|k| free(c, k)))
    }

    /// B217: a computer player added to open seat `i` starts on Random: a disc character with a costume nobody else
    /// has, then one of its free costumes, each at random by `seed` (the host draws; the pick travels as any bot's).
    fn bot_random(&self, i: usize, seed: u64) -> Option<Pick> {
        let taken = self.bot_taken(i)?;
        // ponytail: disc characters only until bots can play mods online (B216)
        let open: Vec<(usize, Vec<usize>)> = (0..super::CHARS)
            .map(|c| (c, (0..super::COSTUMES).filter(|&k| !taken.contains(&(c, k))).collect::<Vec<_>>()))
            .filter(|(_, ks)| !ks.is_empty())
            .collect();
        let n = open.len() as u64;
        let (c, ks) = open.get((seed % n.max(1)) as usize)?;
        Some(Pick { random: true, ..Pick::disc(*c, ks[(seed / n % ks.len() as u64) as usize]) })
    }

    /// B66: the disc characters and costumes the lobby's other seats hold (the readied ones), as `bot_pick` skips them.
    fn bot_taken(&self, i: usize) -> Option<Vec<(usize, usize)>> {
        let r = self.net.room.as_ref()?;
        let taken: Vec<(usize, usize)> = (0..r.seats.len())
            .filter(|&j| j != i)
            .filter_map(|j| {
                let s = r.seats[j].as_ref()?;
                match r.live_of(j).filter(|_| s.human()) {
                    Some(l) if l.ready && l.r#mod.is_none() => Some((l.char, l.costume)),
                    Some(_) => None,
                    None => (s.pick.r#mod.is_none() && !r.picking.contains(&s.peer)).then_some((s.pick.char, s.pick.costume)),
                }
            })
            .collect();
        Some(taken)
    }

    /// This machine's players took their controllers: open or join the lobby on placeholder picks (holding the seats);
    /// its first line-up opens the character select (`heard`).
    pub(super) fn online_open(&mut self, out: &mut Vec<Effect>) {
        let go = self.net.go.unwrap();
        self.net.status.clear();
        self.net.room = None;
        self.net.started = false;
        (self.net.ask, self.net.ready, self.net.hosting, self.net.prepared, self.net.deciding) = (None, None, false, false, false);
        self.net.wanted.clear();
        self.net.pick_next = true;
        self.net.fail = None;
        (self.net.me, self.net.sent) = (if go == Go::Host { "host".into() } else { String::new() }, None);
        self.go(Screen::Lobby, out);
        let cmd = match go {
            Go::Host => {
                // ponytail: std's per-call hasher keys as the dice for a random court (not the match's RNG)
                self.roll = std::hash::BuildHasher::hash_one(&std::hash::RandomState::new(), 0u8);
                self.net.hosting = true;
                self.net.status = "Opening the lobby...".into();
                Cmd::Host(self.room())
            }
            // B213: a lobby Quickplay opens is always listed
            Go::Quick => Cmd::Quick(Room { private: false, ..self.room() }),
            Go::Find | Go::Code => Cmd::Join(self.net.paste.clone(), self.picks()),
        };
        out.push(Effect::Online(Fx::Net(cmd)));
    }

    /// Every one of this machine's players ready on the lobby's character select: their picks into the held seats.
    pub(super) fn online_ready(&mut self, out: &mut Vec<Effect>) {
        self.go(Screen::Lobby, out);
        if self.net.room.is_some() {
            out.push(Effect::Online(Fx::Net(Cmd::Picks(self.picks()))));
        }
    }

    /// The host's confirm screen's Start: the lobby's court, sets, games and umpire as chosen.
    pub(super) fn online_host(&mut self, out: &mut Vec<Effect>) {
        self.go(Screen::Lobby, out);
        let (court, umpire) = self.venue();
        out.push(Effect::Online(Fx::Net(Cmd::Venue(court, self.sets, self.games, umpire))));
    }

    /// One device's presses on the Online screens.
    pub(super) fn online_step(&mut self, k: Press, out: &mut Vec<Effect>) {
        use Effect::Sound;
        let rows = |n: usize, sel: &mut usize, out: &mut Vec<Effect>| {
            if k.up || k.down {
                *sel = if k.down { (*sel + 1) % n } else { (*sel + n - 1) % n };
                out.push(Sound(0));
            }
        };
        match self.screen {
            Screen::Online => {
                let to = pill_step(&row_rects(self.grid.extra), self.sel.min(5), k);
                if to != self.sel {
                    self.sel = to;
                    out.push(Sound(0));
                }
                let cfg = self.net.cfg.get_or_insert_with(Online::default);
                if self.sel == 5 && k.ok {
                    cfg.next_relay();
                    (self.net.list, self.net.finding) = (vec![], 0);
                    out.extend([Sound(0), Effect::Online(Fx::SaveCfg), Effect::Online(Fx::Off)]);
                } else if k.ok && (self.sel == 3 || self.sel == 4) {
                    self.net.naming = self.sel == 4;
                    self.net.entry = if self.net.naming { cfg.name.to_ascii_uppercase().chars().take(NAME_MAX).collect() } else { String::new() };
                    self.net.status.clear();
                    self.go(Screen::Code, out);
                } else if k.ok && self.sel == 2 {
                    // B195: every open lobby first, singles and doubles; the players after choosing one
                    self.net.go = Some(Go::Find);
                    self.net.list.clear();
                    self.net.fit = 0;
                    self.go(Screen::Lobbies, out);
                    (self.net.finding, self.net.fail) = (crate::net::FINDS, None);
                    out.push(Effect::Online(Fx::Net(Cmd::Find)));
                } else if k.ok && self.sel < 3 {
                    self.net.go = Some([Go::Quick, Go::Host, Go::Find][self.sel]);
                    self.net.fit = 0;
                    self.go(Screen::Mode, out);
                } else if (k.ok && self.sel == 6) || k.back {
                    self.net.go = None;
                    self.sel = self.main_at(super::home::Row::Online);
                    self.screen = Screen::Main;
                    out.extend([Sound(1), Effect::Online(Fx::Off)]);
                }
            }
            Screen::Lobbies => {
                let n = self.net.list.len();
                rows(n + 2, &mut self.sel, out);
                if k.ok && self.sel < n {
                    // the lobby's kind and room: this machine's controllers next, then join it
                    let l = &self.net.list[self.sel];
                    (self.net.paste, self.doubles, self.net.fit) = (l.dial().into(), l.doubles, l.open());
                    self.seats[1..].fill(None);
                    self.go(Screen::Assign, out);
                } else if k.ok && self.sel == n {
                    self.net.list.clear();
                    (self.net.finding, self.net.fail) = (self.net.finding.saturating_add(crate::net::FINDS), None);
                    out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::Find))]);
                } else if k.ok || k.back {
                    self.back_to_online(out);
                }
            }
            Screen::Code => {
                // 8 columns of symbols, 4 rows (the name's: 5), then Delete, Paste (Clear), Join (Save), Back (two
                // columns each)
                let (s, keys) = (self.sel, self.keys().len());
                let rows = keys / 8;
                if k.left || k.right {
                    self.sel = if s < keys { s / 8 * 8 + (s % 8 + if k.right { 1 } else { 7 }) % 8 } else { keys + (s - keys + if k.right { 1 } else { 3 }) % 4 };
                    out.push(Sound(0));
                }
                if k.up || k.down {
                    let (row, col) = if s < keys { (s / 8, s % 8) } else { (rows, (s - keys) * 2) };
                    let row = (row + if k.down { 1 } else { rows }) % (rows + 1);
                    self.sel = if row == rows { keys + col / 2 } else { row * 8 + col };
                    out.push(Sound(0));
                }
                if k.ok {
                    match self.sel.checked_sub(keys) {
                        None => {
                            self.type_char(self.keys()[self.sel] as char);
                            out.push(Sound(0));
                        }
                        Some(0) => out.push(Sound(if self.net.entry.pop().is_some() { 1 } else { 0 })),
                        Some(1) if self.net.naming => {
                            self.net.entry.clear();
                            out.push(Sound(1));
                        }
                        Some(1) => out.push(Effect::Online(Fx::Paste)),
                        Some(2) if self.net.naming && !self.net.entry.trim().is_empty() => {
                            self.net.cfg.get_or_insert_with(Online::default).name = self.net.entry.trim().to_string();
                            out.extend([Sound(2), Effect::Online(Fx::SaveCfg)]);
                            self.code_back(out);
                        }
                        Some(2) if !self.net.naming && is_short(&self.net.entry) => {
                            self.net.paste = self.net.entry.clone();
                            self.join_code(out);
                        }
                        Some(2) => out.push(Sound(1)),
                        _ => self.code_back(out),
                    }
                } else if k.back {
                    // B239: ○ (Esc) leaves the entry; □ deletes (Backspace too, `typing`)
                    self.code_back(out);
                } else if k.page {
                    out.push(Sound(if self.net.entry.pop().is_some() { 1 } else { 0 }));
                }
            }
            Screen::Lobby if let Some(h) = self.net.ask.filter(|_| !k.back) => {
                rows(ASK_ROWS.len(), &mut self.sel, out);
                if k.ok {
                    let (_, fetch, remember) = ASK_ROWS[self.sel];
                    self.net.ask = None;
                    if fetch {
                        self.net.wanted.insert(h);
                        out.push(Effect::Online(Fx::Net(Cmd::Want(h))));
                    } else {
                        self.net.looks.insert(h, None);
                    }
                    // B289: kept with the settings (Settings → Online Mod Downloads changes it later)
                    self.settings.4 = remember;
                    out.extend([Sound(2), Effect::SaveSettings]);
                }
            }
            Screen::Lobby => {
                let items = self.items();
                let bot = match items.get(self.sel) {
                    Some(Item::Seat(i)) if self.net.room.as_ref().is_some_and(|r| r.seats[*i].as_ref().is_some_and(|s| !s.human())) => Some(*i),
                    _ => None,
                };
                let rects = self.lobby_rects();
                // the settings row: ←→ change the value, ↑↓ go along it (off its ends: as the pills lie)
                let to = match items.get(self.sel) {
                    Some(Item::Mode | Item::Set(_)) if k.left || k.right => self.sel,
                    Some(Item::Set(i)) if k.up && *i > 0 => self.sel - 1,
                    Some(Item::Mode) if k.up => self.sel - 1,
                    Some(Item::Set(_)) if k.down => self.sel + 1,
                    _ => pill_step(&rects, self.sel, k),
                };
                if let (Some(Item::Set(i)), true) = (items.get(self.sel), k.left || k.right) {
                    self.change_setting(*i, if k.right { 1 } else { -1 }, out);
                }
                // B201: on a computer's seat L1 / R1 change its costume (and so its level); ←→ reach its ✕
                if let Some(i) = bot.filter(|_| k.prev || k.next) {
                    // a Random computer stays Random: a new costume doesn't show who it drew
                    let random = self.net.room.as_ref().is_some_and(|r| r.seats[i].as_ref().is_some_and(|s| s.pick.random));
                    match self.bot_pick(i, 0, k.next as isize - k.prev as isize).map(|p| Pick { random, ..p }) {
                        Some(p) => out.extend([Sound(0), Effect::Online(Fx::Net(Cmd::Bot(i, Some(p))))]),
                        None => out.push(Sound(1)),
                    }
                } else if to != self.sel {
                    self.sel = to;
                    self.net.status.clear();
                    out.push(Sound(0));
                }
                let item = items.get(self.sel).copied().unwrap_or(Item::Button("Leave"));
                if let Some(i) = bot.filter(|_| k.card) {
                    // B216: △ takes the computer player off its seat
                    out.extend([Sound(1), Effect::Online(Fx::Net(Cmd::Bot(i, None)))]);
                } else if let (true, Item::Remove(i)) = (k.ok, item) {
                    // back onto the seat, as the ✕ goes with the computer
                    self.sel -= 1;
                    out.extend([Sound(1), Effect::Online(Fx::Net(Cmd::Bot(i, None)))]);
                } else if k.back || (k.ok && item == Item::Button("Leave")) {
                    out.push(Effect::Online(Fx::Net(Cmd::Leave)));
                    self.back_to_online(out);
                } else if !k.ok && !(item == Item::Mode && to == self.sel && (k.left || k.right)) {
                } else if let Item::Seat(i) = item {
                    if let Some((peer, name)) = self.guest(i) {
                        self.net.status = format!("Taking {name} out...");
                        out.extend([Sound(1), Effect::Online(Fx::Net(Cmd::Kick(peer)))]);
                    } else if bot.is_some() {
                        // B216: its character, costume and level on the select
                        self.edit_bot(i, None, out);
                    } else if let Some(p) = self.bot_random(i, super::dice()) {
                        // B217's random pick, then B216's select on it
                        out.push(Effect::Online(Fx::Net(Cmd::Bot(i, Some(p.clone())))));
                        self.edit_bot(i, Some(&p), out);
                    }
                } else if let Item::Set(i @ (0 | 1)) = item {
                    self.pick_venue(i, out);
                } else if let Item::Set(i) = item {
                    // OK steps it on, as →
                    self.change_setting(i, 1, out);
                } else if item == Item::Mode {
                    let r = self.net.room.as_ref();
                    // singles only while the taken seats fit in two
                    if !self.can_switch() {
                        // the help line says why
                        out.push(Sound(1));
                    } else if let Some(r) = r {
                        out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::Doubles(!r.doubles)))]);
                    }
                } else if let Some(r) = self.net.room.as_ref().filter(|_| item == Item::Button("Public")) {
                    // B213: listed in Find and Quickplay or not; Host remembers it
                    let p = !r.private;
                    self.net.cfg.get_or_insert_with(Online::default).private = p;
                    out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::Private(p))), Effect::Online(Fx::SaveCfg)]);
                } else if item == Item::Button("Copy Link") && shared(&self.net).is_some() {
                    out.extend([Sound(2), Effect::Online(Fx::Copy)]);
                } else if matches!(item, Item::Button("Start" | "Ready")) && self.counting() > 0 {
                    // B215: stop the countdown for everyone
                    self.net.status.clear();
                    out.extend([Sound(1), Effect::Online(Fx::Net(Cmd::Cancel))]);
                } else if item == Item::Button("Start") {
                    // B215: a countdown, ready or not
                    self.net.status.clear();
                    out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::Start))]);
                } else if item == Item::Button("Characters") {
                    // pick again: not ready meanwhile
                    for p in 0..4 {
                        self.unready(p);
                    }
                    if self.net.prepared {
                        self.net.prepared = false;
                        out.push(Effect::Online(Fx::Net(Cmd::Prepared(false))));
                    }
                    self.net.status.clear();
                    self.go(Screen::Chars, out);
                } else if item == Item::Button("Ready") {
                    self.net.prepared = !self.net.prepared;
                    self.net.status.clear();
                    out.extend([Sound(if self.net.prepared { 2 } else { 1 }), Effect::Online(Fx::Net(Cmd::Prepared(self.net.prepared)))]);
                } else {
                    out.push(Sound(1));
                }
            }
            _ => {}
        }
    }

    fn code_back(&mut self, out: &mut Vec<Effect>) {
        (self.screen, self.sel) = (Screen::Online, if self.net.naming { 4 } else { 3 });
        self.net.naming = false;
        self.net.status.clear();
        out.push(Effect::Sound(1));
    }

    /// A symbol typed into Join by Code (any case; others ignored); the code full: the cursor to Join. Naming: any
    /// character the menu font has, up to `NAME_MAX`.
    pub(super) fn type_char(&mut self, c: char) {
        if self.net.naming {
            if self.net.entry.chars().count() < NAME_MAX && (c == ' ' || super::FONT.iter().any(|r| r.contains(c))) {
                self.net.entry.push(c);
            }
            return;
        }
        let c = c.to_ascii_uppercase();
        if self.net.entry.len() < SHORT && SYMBOLS.contains(&(c as u8)) {
            self.net.entry.push(c);
            if self.net.entry.len() == SHORT {
                self.sel = KEYS + 2;
            }
        }
    }

    pub(super) fn back_to_online(&mut self, out: &mut Vec<Effect>) {
        self.sel = [Go::Quick, Go::Host, Go::Find, Go::Code].iter().position(|g| Some(*g) == self.net.go).unwrap_or(0);
        self.net.go = None;
        self.net.code.clear();
        self.net.short.clear();
        self.net.room = None;
        (self.net.hosting, self.net.prepared, self.net.pick_next, self.net.fit) = (false, false, false, 0);
        (self.net.me, self.net.sent, self.net.edit) = (String::new(), None, None);
        self.screen = Screen::Online;
        out.push(Effect::Sound(1));
    }

    /// Join by Code's code chosen: no singles/doubles (the lobby has it, taken from its room once in, B225), straight to
    /// this machine's controllers, up to three until the lobby says (it refuses more than its open seats).
    fn join_code(&mut self, out: &mut Vec<Effect>) {
        (self.net.go, self.doubles, self.net.fit) = (Some(Go::Code), true, 0);
        self.seats[1..].fill(None);
        self.go(Screen::Assign, out);
    }

    /// The pasted text as Join by Code's code: on to the picks if it parses.
    pub(super) fn pasted(&mut self, text: Option<String>, out: &mut Vec<Effect>) {
        if let Some(code) = text.as_deref().and_then(crate::net::code_in) {
            self.net.paste = code;
            self.join_code(out);
        } else {
            self.net.status = if cfg!(target_arch = "wasm32") {
                "Paste a lobby link or code on the page first (Ctrl+V), then choose Paste."
            } else {
                "Copy a lobby link or code first, then choose Paste."
            }
            .into();
            out.push(Effect::Sound(1));
        }
    }

    /// A network event into the view.
    fn heard(&mut self, e: Event) {
        let v = &mut self.net;
        match e {
            // the Online screen's background search (`Fx::Load`): its card says how it goes
            Event::Status(_) if self.screen == Screen::Online => {}
            Event::Status(s) => v.status = s,
            // the lobby's gone (refused, taken out, the host left, or it couldn't open): only Leave is left
            // the Online screen's own search failing: its card says none found
            Event::Failed(_) if self.screen == Screen::Online && v.finding > 0 => v.finding = 0,
            Event::Failed(s) => {
                v.status = s;
                v.fail = Some(match v.go {
                    _ if v.finding > 0 => "Couldn't search for lobbies",
                    Some(Go::Host) if v.pick_next => "Couldn't create the lobby",
                    Some(Go::Quick) if v.pick_next => "Couldn't find or create a lobby",
                    _ if v.pick_next => "Couldn't join the lobby",
                    _ => "The lobby closed",
                });
                v.finding = 0;
                if !v.started {
                    (v.room, v.hosting, v.prepared, v.pick_next) = (None, false, false, false);
                    self.sel = 0;
                    // gone while picking: the lobby screen says why
                    if matches!(self.screen, Screen::Chars | Screen::Confirm) {
                        self.screen = Screen::Lobby;
                    }
                }
            }
            Event::Hosting => (v.hosting, v.me) = (true, "host".into()),
            Event::Me(me) => v.me = me,
            Event::Code(c) => v.code = c,
            Event::Short(c) => v.short = c,
            Event::Found(l) => {
                v.finding = v.finding.saturating_sub(1);
                // the Online screen's card shows them (the lobby count on its chip)
                if self.screen != Screen::Online {
                    v.status = match l.len() {
                    0 if v.finding > 0 => "None found yet: still looking...".into(),
                    0 => "No open lobbies found: Refresh to look again, or Host one from Online.".into(),
                        n => format!("{n} open lobb{}.", if n == 1 { "y" } else { "ies" }),
                    };
                }
                v.list = l;
            }
            Event::Room(r) => {
                // the lobby's singles or doubles is everyone's (a code join didn't choose)
                self.doubles = r.doubles;
                v.room = Some(r);
                // seated (or the lobby open): on to its character select, the seats held meanwhile
                if v.pick_next && self.screen == Screen::Lobby {
                    v.pick_next = false;
                    (self.screen, self.sel) = (Screen::Chars, 0);
                }
                self.align();
            }
            Event::Start(r, _) => {
                v.room = Some(r);
                v.started = true;
                v.status = "Starting the match...".into();
            }
            // B279: the host left the result screen after this machine: the lobby is open again
            Event::Match(Msg::Lobby(r)) => {
                (v.room, v.started) = (Some(r), false);
                v.status = "Back in the lobby.".into();
                self.align();
            }
            Event::Match(_) | Event::Dropped(_) | Event::Datagram(..) | Event::Pack(..) | Event::Chat { .. } => {}
        }
    }

    /// The lobby's mods, each to its looks: found here, fetched or a stand-in by the remembered answer, else asked
    /// (one at a time); once every one is settled, tell the host (the match waits for everyone).
    fn settle(&mut self, iso: &str, net: &Net) {
        let Some(room) = self.net.room.clone().filter(|_| !self.net.started) else { return };
        let cache = mo::cache(iso);
        // B281: the host's wait is over: the mod still asked about plays as its stand-in
        if room.stand_in
            && let Some(h) = self.net.ask.take()
        {
            self.net.looks.insert(h, None);
        }
        for (_, w) in room.mods() {
            let h = w.looks;
            if self.net.looks.contains_key(&h) || self.net.wanted.contains(&h) || self.net.ask == Some(h) {
                continue;
            }
            if let Some(d) = mo::find(w, &self.mods, &cache) {
                self.net.looks.insert(h, Some(d));
                continue;
            }
            match self.settings.4 {
                Ask::Always => {
                    self.net.wanted.insert(h);
                    net.send(Cmd::Want(h));
                }
                Ask::Never => drop(self.net.looks.insert(h, None)),
                _ if room.stand_in => drop(self.net.looks.insert(h, None)),
                _ if self.net.ask.is_none() => (self.net.ask, self.sel) = (Some(h), 0),
                _ => {}
            }
        }
        if self.net.ready != Some(room.ver) && room.mods().all(|(_, w)| self.net.looks.contains_key(&w.looks)) {
            self.net.ready = Some(room.ver);
            net.send(Cmd::Ready(room.ver));
        }
    }

    /// B281: a mod in the lobby this machine hasn't chosen to download or not (asked, or not asked yet).
    fn undecided(&self) -> bool {
        self.net.room.as_ref().filter(|_| !self.net.started).is_some_and(|r| r.mods().any(|(_, w)| !self.net.looks.contains_key(&w.looks) && !self.net.wanted.contains(&w.looks)))
    }

    /// A mod's looks arrived: checked and kept, or refused (a stand-in plays it).
    fn got_pack(&mut self, iso: &str, h: u64, pack: &[u8]) {
        if !self.net.wanted.remove(&h) {
            return;
        }
        let dir = mo::unpack(pack, h, &mo::cache(iso)).map(|m| m.dir).map_err(|e| self.net.status = format!("A mod's files were refused, a stand-in plays it: {e}")).ok();
        self.net.looks.insert(h, dir);
    }
}

fn relay_name(cfg: &Online) -> String {
    match &cfg.relay {
        Relay::N0 => "n0 (public)".into(),
        Relay::Own(url) => url.trim_start_matches("https://").trim_start_matches("http://").chars().take(18).collect(),
        Relay::Off => "Off (LAN)".into(),
    }
}

/// The name `peer`'s player gave (its ID's start if none).
fn name_of(r: &Room, peer: &str) -> String {
    r.names.get(peer).filter(|n| !n.is_empty()).cloned().unwrap_or_else(|| if peer == "host" { "Host".into() } else { peer.chars().take(6).collect() })
}

/// B281: after Start, who the match waits for to choose about a mod, and for how long.
fn waiting(r: &Room) -> String {
    let who: Vec<String> = r.deciding.iter().map(|p| name_of(r, p)).collect();
    format!("Waiting for {} to choose whether to download a mod: {} s, then a stand-in plays it.", who.join(", "), r.wait)
}

fn court_name(text: &super::Text, c: usize) -> String {
    format!("{} {}", text.msg(276 + 2 * c), text.msg(277 + 2 * c))
}

pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let text = d.text;
    match m.screen {
        Screen::Online => online_screen(d, m, hand_x),
        Screen::Lobbies => {
            d.frame(0, 5);
            // B195: every open lobby, singles and doubles together, a row each: host, kind, court, open seats
            let n = m.net.list.len();
            // ponytail: one page of 7 rows; scroll when lobbies get that many
            let first = m.sel.saturating_sub(6);
            let (x, w) = (80.0, 480.0);
            for (row, i) in (first..n + 2).take(7).enumerate() {
                let (y, on) = (76.0 + 44.0 * row as f32, i == m.sel);
                d.pill(x, y, w, if on { [128.0, 110.0, 40.0] } else { [70.0, 70.0, 80.0] });
                d.hit([x, y, w, 38.0], Act::Row(i, OK));
                let rgb = if on { WHITE } else { [100.0; 3] };
                let Some(l) = m.net.list.get(i) else {
                    d.centred(if i == n { "Refresh" } else { "Back" }, x + w / 2.0, y + 7.0, 24.0, rgb);
                    continue;
                };
                d.fitted(&l.name, x + 70.0, y + 9.0, 20.0, 120.0);
                d.centred(if l.doubles { "Doubles" } else { "Singles" }, x + 180.0, y + 9.0, 20.0, rgb);
                d.fitted(&court_name(text, l.court), x + 315.0, y + 9.0, 20.0, 170.0);
                d.centred(&format!("{} open", l.open()), x + 440.0, y + 9.0, 20.0, rgb);
            }
            d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x - 60.0 + hand_x, 76.0 + 44.0 * (m.sel - first) as f32 + 3.0, 64.0, 32.0], WHITE, 128.0);
            // B236: searching, nothing found or failed, under Refresh / Back; still searching with some listed, a
            // spinner up by the title
            match (n, m.net.finding > 0, m.net.fail) {
                (0, true, _) => busy(d, m.net.spin, 250.0, "Searching for lobbies...", "This network first, then the internet."),
                (0, false, Some(t)) => failed(d, 250.0, t, "Refresh to try again (the reason is below)."),
                (0, false, None) => notice(d, 250.0, "No open lobbies found", "Refresh to look again, or Host one from Online."),
                (_, true, _) => {
                    spinner(d, m.net.spin, 596.0, 44.0, 12.0);
                    d.outlined(|d| d.text("Searching...", 452.0, 34.0, 20.0, WHITE));
                }
                _ => {}
            }
            let info = match m.net.list.get(m.sel) {
                Some(l) => format!("{}'s {} lobby on {}: {} of {} seats open. OK joins, then you pick your characters.", l.name, if l.doubles { "doubles" } else { "singles" }, court_name(text, l.court), l.open(), l.seats),
                None if m.net.status.is_empty() => "Looking for lobbies...".into(),
                None => m.net.status.clone(),
            };
            d.info(&info);
        }
        Screen::Code => {
            d.frame(0, 5);
            const ON: [f32; 3] = [128.0, 110.0, 40.0];
            const OFF: [f32; 3] = [70.0, 70.0, 80.0];
            let keys = m.keys();
            let (top, rows) = if m.net.naming { (124.0, keys.len() / 8) } else { (132.0, 4) };
            if m.net.naming {
                // the name so far in one long pill, the cursor after it
                d.pill(320.0 - 232.0 + 3.0, 72.0, 458.0, [40.0, 40.0, 50.0]);
                let w = d.width(&m.net.entry, 26.0);
                d.centred(&m.net.entry, 320.0, 78.0, 26.0, WHITE);
                if m.net.entry.chars().count() < NAME_MAX {
                    d.text("_", 320.0 + w / 2.0 + 2.0, 78.0, 26.0, ON);
                }
            } else {
                // the code so far, a pill a symbol
                for i in 0..SHORT {
                    let x = 320.0 - 128.0 + 64.0 * i as f32 + 4.0;
                    d.pill(x, 76.0, 56.0, if i == m.net.entry.len() { ON } else { [40.0, 40.0, 50.0] });
                    if let Some(c) = m.net.entry.get(i..i + 1) {
                        d.centred(c, x + 28.0, 80.0, 30.0, WHITE);
                    }
                }
            }
            let grid = !touch_typing();
            for (i, c) in keys.iter().enumerate().filter(|_| grid) {
                let (x, y) = (320.0 - 232.0 + 58.0 * (i % 8) as f32 + 3.0, top + 44.0 * (i / 8) as f32);
                d.pill(x, y, 52.0, if m.sel == i { ON } else { OFF });
                d.hit([x, y, 52.0, 38.0], Act::Row(i, OK));
                let rgb = if m.sel == i { WHITE } else { [100.0; 3] };
                if *c == b' ' {
                    d.centred("Spc", x + 26.0, y + 11.0, 16.0, rgb);
                } else {
                    d.centred(&(*c as char).to_string(), x + 26.0, y + 7.0, 24.0, rgb);
                }
            }
            let buttons = if m.net.naming { ["Delete", "Clear", "Save", "Back"] } else { ["Delete", "Paste", "Join", "Back"] };
            for (i, l) in buttons.iter().enumerate().filter(|_| grid) {
                let (x, y, on) = (320.0 - 232.0 + 116.0 * i as f32 + 2.0, top + 44.0 * rows as f32 + 8.0, m.sel == keys.len() + i);
                let dim = i == 2 && if m.net.naming { m.net.entry.trim().is_empty() } else { !is_short(&m.net.entry) };
                d.pill(x, y, 112.0, if on { ON } else { OFF });
                d.hit([x, y, 112.0, 38.0], Act::Row(keys.len() + i, OK));
                d.centred(l, x + 56.0, y + 7.0, 24.0, if on && !dim { WHITE } else if dim { [60.0; 3] } else { [100.0; 3] });
            }
            let info = match () {
                _ if !m.net.status.is_empty() => m.net.status.clone(),
                _ if !grid && m.net.naming => "Tap to type your name; Return saves it.".into(),
                _ if !grid => "Tap to type the host's code (or paste their link or code); Return joins.".into(),
                _ if m.net.naming => "Type your name, or pick each letter; Save keeps it.".into(),
                _ => "Type the host's code (or pick each symbol), or Paste their link or code.".into(),
            };
            d.info(&info);
        }
        Screen::Lobby => {
            // B295 (ours, `mockups/Lobby`): the seats as rows by team with the help line under them; the side card
            // with the court, the QR and code, Characters / Court, and Start big over Leave
            d.frame(6, 8);
            let room = m.net.room.as_ref();
            let items = m.items();
            let rects = m.lobby_rects();
            let (col, [sx, sy, sw, _]) = m.lobby_split();
            let at = |it: Item| items.iter().position(|x| *x == it);
            // the lobby's name on the header's right
            if let Some(r) = room {
                d.anchor = super::Anchor::Sides;
                let w = d.width(&r.name, 18.0).min(200.0);
                d.outlined(|d| super::left(d, &r.name, 624.0 - w, 18.0, 18.0, 200.0, WHITE));
                d.anchor = super::Anchor::Centre;
            }
            settings_row(d, m, text, &items);
            if let Some(r) = room {
                super::team_boxes(d, col, r.doubles);
                for (i, s) in r.seats.iter().enumerate() {
                    let rect = super::seat_rect(col, r.doubles, i);
                    let k = at(Item::Seat(i));
                    if let Some(k) = k {
                        d.hit(rect, Act::Row(k, OK));
                    }
                    let on = m.net.ask.is_none() && k.is_some_and(|k| k == m.sel);
                    let Some(s) = s else {
                        let row = super::SeatRow { p: i, face: None, name: "Open seat", faint: true, who: "Start fills it with a computer player", chip: ("Open", None), filled: false, on };
                        super::seat_row(d, rect, &row, 0.0);
                        continue;
                    };
                    let live = r.live_of(i).filter(|l| s.human() && (!l.ready || r.picking.contains(&s.peer)));
                    let choosing = live.is_some() || r.picking.contains(&s.peer);
                    // the character: who they're on now while still choosing (B201), a mod's own name (B234), Random
                    // hidden until the match (B217)
                    let (c, id) = match live {
                        Some(l) => (l.char, l.r#mod.clone()),
                        None => (s.pick.char, s.pick.r#mod.as_ref().map(|w| w.id.clone())),
                    };
                    let local = id.as_ref().and_then(|id| m.mods.iter().position(|x| x.id == *id));
                    let name = match (&s.pick.r#mod, live) {
                        (_, Some(l)) if !l.name.is_empty() => l.name.clone(),
                        (_, Some(_)) => super::CHAR_NAMES[c.min(super::CHARS - 1)].to_string(),
                        _ if choosing => "Picking...".into(),
                        _ if s.pick.random => "Random".into(),
                        (Some(w), _) => w.name.clone(),
                        (None, _) => super::CHAR_NAMES[c.min(super::CHARS - 1)].to_string(),
                    };
                    let face = match local {
                        _ if choosing && live.is_none() => None,
                        _ if s.pick.random && !choosing => Some(super::cell_face(None)),
                        Some(j) => Some(super::cell_face(Some(super::Who::Mod(j)))),
                        None => Some(super::cell_face(Some(super::Who::Disc(c.min(super::CHARS - 1))))),
                    };
                    let (who, chip) = if !s.human() {
                        // B201: the computer's level, from its character and costume as the select's rank pill
                        let level = super::RANK_NAMES.get(super::com_rank(&text.ranks, s.pick.char, s.pick.costume, r.doubles)).copied().unwrap_or("COM");
                        (format!("COM #{} · {level}", s.pick.costume + 1), ("COM", Some(super::CHIP_COM)))
                    } else if r.deciding.contains(&s.peer) {
                        // B281: still choosing whether to download a mod
                        (name_of(r, &s.peer), ("Deciding", None))
                    } else if s.peer == "host" {
                        (name_of(r, "host"), ("Host", Some(super::CHIP_HOST)))
                    } else if r.ready.contains(&s.peer) {
                        (name_of(r, &s.peer), ("Ready", Some(super::CHIP_READY)))
                    } else {
                        (name_of(r, &s.peer), ("Waiting", None))
                    };
                    let remove = at(Item::Remove(i));
                    let row = super::SeatRow { p: i, face, name: &name, faint: choosing, who: &who, chip, filled: true, on };
                    super::seat_row(d, rect, &row, if remove.is_some() { 32.0 } else { 0.0 });
                    // B216: the host's ✕ that takes a computer player off (△ on its row, or ✕ on it)
                    if let Some(k) = remove {
                        let [x, y, w, h] = rects[k];
                        let ring = if m.sel == k { super::GOLD } else { WHITE };
                        d.pill_at([x, y, w, h], [super::INK, ring, [120.0, 30.0, 30.0]], 128.0);
                        d.centred("X", x + w / 2.0, y + 4.0, 18.0, WHITE);
                        d.hit([x, y, w, h], Act::Row(k, OK));
                    }
                }
            }
            if let Some(w) = m.net.ask.and_then(|h| room?.mods().find(|(_, w)| w.looks == h).map(|(_, w)| w)) {
                // a mod this machine doesn't have: its looks, or its donor recoloured, asked on the side card; the
                // gold pill is the cursor, and it has to be answered
                d.popup(None);
                d.box3([sx, sy, sw, 210.0], 14.0, [Some(super::INK), Some(WHITE), Some(super::CREAM)], 128.0);
                d.label(&format!("{} is a mod you don't have.", w.name), sx + sw / 2.0, sy + 14.0, 14.0, sw - 20.0, super::BROWN);
                for (i, r) in ASK_ROWS.iter().enumerate() {
                    let rect = [sx + 10.0, sy + 40.0 + 40.0 * i as f32, sw - 20.0, 32.0];
                    super::side_button(d, rect, r.0, m.sel == i, false);
                    d.hit(rect, Act::Row(i, OK));
                }
                let help = match room.filter(|r| r.wait > 0) {
                    Some(r) => format!("The match is waiting: choose in {} s, or it plays as its donor in odd colours.", r.wait),
                    None => "Get its looks, or play it as its donor in odd colours.".into(),
                };
                super::help_line(d, col, room.is_some_and(|r| r.doubles), &help);
                return;
            }
            if let Some(r) = room {
                // the side card: the court, the code as a QR (the texture `QrImage`, made when the code arrives) and
                // the short code, big enough to read out (the QR is its link)
                d.box3([sx, sy, sw, 150.0], 14.0, [Some(super::INK), Some(WHITE), Some(super::CREAM)], 128.0);
                super::court_band(d, [sx, sy, sw], &court_name(text, r.court));
                let (qx, qy) = (sx + 8.0, sy + 54.0);
                if shared(&m.net).is_some() {
                    d.q(Tex::Solid, [0.0; 4], [qx, qy, 88.0, 88.0], WHITE, 128.0);
                    d.q(Tex::Qr, [0.0, 0.0, 1.0, 1.0], [qx, qy, 88.0, 88.0], WHITE, 128.0);
                } else {
                    spinner(d, m.net.spin, qx + 44.0, qy + 44.0, 14.0);
                }
                let tx = sx + 104.0;
                d.text(if r.private { "PRIVATE CODE" } else { "JOIN CODE" }, tx, qy + 2.0, 11.0, [38.0, 51.0, 32.0]);
                let code = if m.net.short.is_empty() { "...." } else { m.net.short.as_str() };
                d.outlined(|d| super::left(d, code, tx, qy + 14.0, 32.0, sw - 112.0, super::GOLD));
            }
            if m.net.ask.is_none() {
                let open = room.map_or(0, Room::open);
                for (i, it) in items.iter().enumerate() {
                    let Item::Button(b) = *it else { continue };
                    let (r, on) = (rects[i], m.sel == i);
                    let label = match b {
                        "Start" | "Ready" if m.counting() > 0 => format!("Cancel ({})", m.counting()),
                        "Start" if open > 0 => format!("Start match +{open} COM"),
                        "Start" => "Start match".into(),
                        "Ready" if m.net.prepared => "Not Ready".into(),
                        "Copy Link" => "Copy link".into(),
                        "Public" if room.is_some_and(|r| r.private) => "Private".into(),
                        "Leave" => "Leave lobby".into(),
                        b => b.into(),
                    };
                    d.hit(r, Act::Row(i, OK));
                    match b {
                        // the small pills by the code: Copy link white (grey with nothing to copy), Public dark
                        "Copy Link" | "Public" => {
                            let dim = b == "Copy Link" && shared(&m.net).is_none();
                            let (fill, words) = if b == "Public" { (super::INK, WHITE) } else if dim { ([90.0; 3], [60.0; 3]) } else { (WHITE, super::INK) };
                            d.pill_at(r, [super::INK, if on { super::GOLD } else { fill }, if on { super::GOLD } else { fill }], 128.0);
                            d.label(&label, r[0] + r[2] / 2.0, r[1] + 3.0, 12.0, r[2] - 10.0, if on { WHITE } else { words });
                        }
                        // Leave: an outline only
                        "Leave" => {
                            d.pill_at(r, [super::INK, if on { super::GOLD } else { [60.0; 3] }, if on { [128.0, 110.0, 40.0] } else { [90.0, 104.0, 80.0] }], if on { 128.0 } else { 70.0 });
                            d.label(&label, r[0] + r[2] / 2.0, r[1] + 5.0, 15.0, r[2] - 16.0, if on { WHITE } else { super::INK });
                        }
                        _ => super::side_button(d, r, &label, on, matches!(b, "Start" | "Ready")),
                    }
                }
                // the hand on the chosen item's left end
                if let Some([x, y, _, h]) = rects.get(m.sel) {
                    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x - 22.0 + hand_x, y + h / 2.0 - 9.0, 36.0, 18.0], WHITE, 128.0);
                }
            }
            // B236: no lobby yet: what's happening, the spinner turning; or why it stopped
            if room.is_none() {
                let sub = "The steps show below; it gives up by itself if nothing answers.";
                match (m.net.pick_next, m.net.go) {
                    (true, _) if m.net.hosting => busy(d, m.net.spin, 230.0, "Creating lobby...", sub),
                    (true, Some(Go::Quick)) => busy(d, m.net.spin, 230.0, "Finding a lobby...", sub),
                    (true, _) => busy(d, m.net.spin, 230.0, "Joining...", sub),
                    _ => failed(d, 230.0, m.net.fail.unwrap_or("The lobby closed"), "Leave to go back (the reason is below)."),
                }
            }
            let info = match items.get(m.sel) {
                _ if let Some(r) = room.filter(|r| r.wait > 0) => waiting(r),
                _ if m.counting() > 0 => format!("The match starts in {}... Cancel stops it for everyone.", m.counting()),
                _ if !m.net.status.is_empty() => m.net.status.clone(),
                // B201: the host's open and computer seats, and the singles / doubles switch
                Some(Item::Seat(i)) if m.guest(*i).is_none() && room.is_some_and(|r| r.seats.get(*i).is_some_and(|s| s.is_some())) => {
                    "OK: pick its character, costume and level on the select (L1 / R1: costume here). Right or Triangle: remove it.".into()
                }
                Some(Item::Seat(i)) if m.guest(*i).is_none() => "OK: add a computer player to this seat and pick its character, costume and level. Left empty, Start fills it.".into(),
                Some(Item::Remove(_)) => "OK: take this computer player off its seat.".into(),
                Some(Item::Mode) if !m.can_switch() => "Singles has two seats: free some first.".into(),
                Some(Item::Mode) => "OK: switch between singles and doubles (everyone readies again).".into(),
                Some(Item::Seat(i)) => format!("OK takes {} out of the lobby.", m.guest(*i).map_or(String::new(), |g| g.1)),
                Some(Item::Button("Start")) => "Start a 4 second countdown, ready or not: open seats are played by the computer.".into(),
                Some(Item::Button("Copy Link")) => "Copy the lobby's link, or read out the code: friends type it in Join by Code.".into(),
                Some(Item::Button("Characters")) => "Pick your characters again (not ready meanwhile); your seats stay yours.".into(),
                Some(Item::Button("Public")) if room.is_some_and(|r| r.private) => "Private: Find and Quickplay don't show it, only the code joins. OK: list it again.".into(),
                Some(Item::Button("Public")) => "Public: Find and Quickplay show it. OK: make it private (only the code joins).".into(),
                Some(Item::Set(0)) => "OK: choose the court for everyone (Find lists it).".into(),
                Some(Item::Set(1)) => "OK: choose the umpire for everyone.".into(),
                Some(Item::Set(4)) if room.is_some_and(|r| r.vanilla) => "No mods: everyone plays disc characters. Left / Right: allow mods.".into(),
                Some(Item::Set(4)) => "Mods allowed: players may pick mod characters. Left / Right: a vanilla lobby (seated mods become their base character).".into(),
                Some(Item::Set(_)) => "Left / Right: change it for everyone.".into(),
                _ if m.net.pick_next => "Joining: your seats are held while you pick your characters.".into(),
                Some(Item::Button("Ready")) => "Ready up: when everyone in is, the match starts after 4 seconds, open seats played by the computer.".into(),
                _ if room.is_some() && !m.net.hosting => "Leave the lobby. The match starts once every seat is taken and everyone's ready (or the host starts it).".into(),
                _ if room.is_none() => "The lobby's closed.".into(),
                _ => "Leave the lobby.".into(),
            };
            // B295: under the seats; with no lobby, in the description bar under the panel
            if room.is_some() {
                super::help_line(d, col, room.is_some_and(|r| r.doubles), &info);
            } else {
                d.info(&info);
            }
        }
        _ => {}
    }
}

/// B236: dots in a ring, the bright one going round (once a second) with a fading tail, centred on (cx, cy).
/// The Online screen (home style): the list left, the chosen row's card right with its picture (the open lobbies
/// for Quickplay and Find), its words and three facts.
fn online_screen(d: &mut Draw, m: &Menu, hand_x: f32) {
    use super::INK;
    const ON: [f32; 3] = [128.0, 108.0, 38.0];
    const OFF: [f32; 3] = [56.0, 56.0, 67.0];
    const PAPER: [f32; 3] = [128.0; 3];
    const MUTED: [f32; 3] = [45.0, 40.0, 52.0];
    const SMALL: [f32; 3] = [84.0, 96.0, 110.0];
    let fit = super::home::fit_text;
    d.frame(8, 5);
    d.anchor = super::Anchor::Span;
    d.outlined(|d| d.text("<", 10.0, 5.0, 30.0, WHITE));
    d.anchor = super::Anchor::Centre;
    let cfg = m.net.cfg.clone().unwrap_or_default();
    let relay = relay_name(&cfg);
    let sel = m.sel.min(5);
    let rects = row_rects(m.grid.extra);
    for (i, &r) in rects.iter().enumerate() {
        let on = i == sel;
        let [x, y, w, h] = r;
        if i < 4 {
            d.pill_at(r, [INK, WHITE, if on { ON } else { OFF }], 128.0);
            d.text(ROWS[i], x + 20.0, y + 7.0, 24.0, if on { INK } else { [100.0; 3] });
            if on {
                d.text(">", x + w - 28.0, y + 7.0, 24.0, INK);
            }
        } else {
            d.pill_at(r, [INK, WHITE, if on { ON } else { SMALL }], 128.0);
            let value = if i == 4 { cfg.name.clone() } else { relay.clone() };
            d.text(ROWS[i], x + 12.0, y + 5.0, 15.0, INK);
            let vw = d.width(&value, 15.0).min(w - 80.0);
            fit(d, &value, x + w - 12.0 - vw, y + 5.0, 15.0, w - 80.0, INK);
        }
        d.hit(r, Act::Row(i, OK));
        if on {
            d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x - 60.0 + hand_x, y + h / 2.0 - 16.0, 64.0, 32.0], WHITE, 128.0);
        }
    }
    d.outlined(|d| d.text("YOU", rects[4][0] + 6.0, rects[4][1] - 17.0, 13.0, WHITE));

    // the card
    let (_, x, w) = online_cols(m.grid.extra);
    // over the help line (`Draw::info`, from 404)
    let (y, h, pic_h) = (50.0, 346.0, 160.0);
    d.box3([x, y, w, h], 16.0, [Some(INK), Some(WHITE), Some(PAPER)], 118.0);
    let pic = [x + 3.0, y + 3.0, w - 6.0, pic_h];
    d.box3(pic, 13.0, [None, None, Some([92.0, 108.0, 122.0])], 128.0);
    d.q(Tex::Solid, [0.0; 4], [x, y + 3.0 + pic_h, w, 2.5], INK, 128.0);
    let lobbies = |d: &mut Draw| {
        d.outlined(|d| d.text("OPEN LOBBIES NOW", x + 18.0, y + 12.0, 13.0, WHITE));
        if m.net.list.is_empty() {
            let line = if m.net.finding > 0 { "Looking for lobbies..." } else { "None open right now" };
            d.outlined(|d| d.text(line, x + 18.0, y + 40.0, 18.0, WHITE));
        }
        for (k, l) in m.net.list.iter().take(3).enumerate() {
            let r = [x + 16.0, y + 32.0 + 30.0 * k as f32, w - 32.0, 26.0];
            d.box3(r, 9.0, [Some(INK), Some(PAPER), Some(PAPER)], 110.0);
            let count = format!("{} / {}", l.taken, l.seats);
            let cw = d.width(&count, 15.0);
            fit(d, &l.name, r[0] + 10.0, r[1] + 5.0, 15.0, r[2] * 0.5 - 14.0, INK);
            d.text(if l.doubles { "Doubles" } else { "Singles" }, r[0] + r[2] * 0.5, r[1] + 5.0, 15.0, MUTED);
            d.text(&count, r[0] + r[2] - 10.0 - cw, r[1] + 5.0, 15.0, INK);
        }
    };
    let big = |d: &mut Draw, s: &str| {
        let k = ((w - 60.0) / d.width(s, 40.0)).min(1.0);
        d.outlined(|d| d.centred(s, x + w / 2.0, y + 50.0, 40.0 * k, WHITE));
    };
    match sel {
        0 | 2 => lobbies(d),
        1 => {
            // the courts the host can choose (`Confirmation_02`'s tiles, one row)
            let (tw, th) = (92.0 * 0.8, 68.0 * 0.8);
            let n = (((w - 20.0) / (tw + 8.0)) as usize).min(super::COURTS);
            let x0 = x + (w - n as f32 * (tw + 8.0) + 8.0) / 2.0;
            for k in 0..n {
                let tile = if k < 7 { 2 * k } else { 2 * (k - 7) + 1 };
                d.q(super::C02, [96.0 * (tile % 2) as f32, 72.0 * (tile / 2) as f32, 92.0, 68.0], [x0 + (tw + 8.0) * k as f32, y + 30.0, tw, th], WHITE, 128.0);
            }
        }
        3 => big(d, "_  _  _  _"),
        4 => big(d, &cfg.name),
        _ => big(d, &relay),
    }
    d.outlined(|d| d.text(ROWS[sel], x + 18.0, y + pic_h - 34.0, 34.0, WHITE));
    let words = match sel {
        1 => "Open a lobby for players over the internet or on this network; start when you like: open seats are played by the computer.".to_string(),
        2 => "List the open lobbies: this network's first (no internet needed), then the BitTorrent DHT's.".into(),
        0 => "Join the fullest open lobby that has room for your players, or host one and wait.".into(),
        3 => "Type the host's 4-character code, or paste their link or code.".into(),
        4 => format!("Your name on your lobby seats (now {}).", cfg.name),
        _ if cfg!(target_arch = "wasm32") => "Relay: n0's public relays or your own (online.txt) for the internet; Off for this network only (no internet needed): codes are scanned both ways.".into(),
        _ => "Relay for when a direct connection fails: n0's public relays, your own (online.txt), or none.".into(),
    };
    para(d, &words, x + 18.0, y + pic_h + 16.0, w - 36.0, 17.0, 4, INK);
    let open = match m.net.list.len() {
        0 if m.net.finding > 0 => "Searching...".to_string(),
        0 => "None open".into(),
        n => format!("{n} open"),
    };
    let players = m.seats.iter().filter(|s| s.is_some()).count().max(1);
    let facts = [("Lobbies", open), ("Your players", players.to_string()), ("Relay", relay.clone())];
    let tw = (w - 36.0 - 20.0) / 3.0;
    for (i, (k, v)) in facts.iter().enumerate() {
        let (tx, ty) = (x + 18.0 + (tw + 10.0) * i as f32, y + h - 16.0 - 44.0);
        d.box3([tx, ty, tw, 44.0], 10.0, [Some(INK), Some(PAPER), Some(PAPER)], 128.0);
        fit(d, &k.to_uppercase(), tx + 10.0, ty + 6.0, 12.0, tw - 20.0, MUTED);
        fit(d, v, tx + 10.0, ty + 21.0, 17.0, tw - 20.0, INK);
    }
    d.info(if m.net.status.is_empty() { &words } else { &m.net.status });
}

/// `s` word-wrapped to `w` at `size` from (x, y), at most `lines` lines (as the home card's).
fn para(d: &mut Draw, s: &str, x: f32, y: f32, w: f32, size: f32, lines: usize, rgb: [f32; 3]) {
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
    for (i, l) in out.iter().take(lines).enumerate() {
        super::home::fit_text(d, l, x, y + size * 1.35 * i as f32, size, w, rgb);
    }
}

/// The lobby's settings row (`Lobby.dc`): court (its tile), umpire, sets, games, mods, singles / doubles, each a
/// small caption over its value; the host's are items (the chosen one gold), with "< >" where ←→ change them, a
/// guest's only show them.
fn settings_row(d: &mut Draw, m: &Menu, text: &super::Text, items: &[Item]) {
    use super::INK;
    let Some(r) = m.net.room.as_ref() else { return };
    // the host's own choice (Random stays Random), a guest's the lobby's
    let (court, sets, games, umpire) = if m.net.hosting { (m.court, m.sets, m.games, m.umpire as usize) } else { (r.court, r.sets, r.games, r.umpire as usize) };
    let values = [
        if court == super::COURTS { "Random".into() } else { text.msg(276 + 2 * court).to_string() },
        if umpire == super::UMPIRES { "Random".into() } else { text.msg(669 + 2 * umpire).trim_start_matches("Umpire ").to_string() },
        super::SETS.get(sets).copied().unwrap_or(1).to_string(),
        games.to_string(),
        if r.vanilla { "None" } else { "Allowed" }.to_string(),
        if r.doubles { "Doubles" } else { "Singles" }.to_string(),
    ];
    const CAPS: [&str; SETTINGS + 1] = ["COURT", "UMPIRE", "SETS", "GAMES", "MODS", "MODE"];
    for (i, rect) in m.settings_row().into_iter().enumerate() {
        let it = if i == SETTINGS { Item::Mode } else { Item::Set(i) };
        let k = items.iter().position(|x| *x == it);
        let on = k.is_some_and(|k| k == m.sel) && m.net.ask.is_none();
        let [x, y, w, h] = rect;
        let fill = if on { [128.0, 108.0, 37.0] } else { [62.0, 60.0, 75.0] };
        d.pill_at(rect, [INK, WHITE, fill], 128.0);
        let rgb = if on { INK } else { [107.0, 105.0, 114.0] };
        d.text(CAPS[i], x + 16.0, y + 4.0, 10.0, rgb);
        // "< value >" where ←→ change it (the host's sets, games, mods, mode); court and umpire open their select
        let arrows = k.is_some() && i >= 2;
        let size = 16.0;
        let (mut l, rr) = (x + 14.0, x + w - 14.0);
        if arrows {
            d.text("<", l, y + 15.0, size, rgb);
            d.text(">", rr - d.width(">", size), y + 15.0, size, rgb);
            l += 12.0;
        }
        let r_end = if arrows { rr - 12.0 } else { rr };
        if i == 0 {
            // the court's tile (`Confirmation_02`: courts 0–6 down its left column, 7–10 its right), "?" for Random
            let src = if court == super::COURTS {
                super::RANDOM_TILE
            } else {
                let tile = if court < 7 { 2 * court } else { 2 * (court - 7) + 1 };
                [96.0 * (tile % 2) as f32, 72.0 * (tile / 2) as f32, 92.0, 68.0]
            };
            d.q(super::C02, src, [l, y + 15.0, 24.0, 17.0], WHITE, 128.0);
            l += 28.0;
        }
        let vw = r_end - l;
        let tw = d.width(&values[i], size).min(vw);
        let vx = if i == 0 { l } else { l + (vw - tw) / 2.0 };
        super::home::fit_text(d, &values[i], vx, y + 15.0, size, vw, rgb);
        if let Some(k) = k {
            d.hit(rect, Act::Row(k, OK));
            if arrows {
                // the pill's halves step it back and on
                d.hit([x, y, w / 2.0, h], Act::Row(k, Press { left: true, ..default() }));
                d.hit([x + w / 2.0, y, w / 2.0, h], Act::Row(k, Press { right: true, ..default() }));
            }
        }
    }
}

fn spinner(d: &mut Draw, t: f32, cx: f32, cy: f32, r: f32) {
    let lead = (t * 8.0).floor() as i32;
    let dot = r * 0.32;
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::FRAC_PI_4;
        let age = (lead - i).rem_euclid(8) as f32;
        let (x, y) = (cx + r * a.sin(), cy - r * a.cos());
        d.box3([x - dot, y - dot, 2.0 * dot, 2.0 * dot], dot, [None, None, Some(super::GOLD)], 128.0 * (1.0 - age / 9.0));
    }
}

/// B236: a box across the middle at `y` (its centre) with a title and a line under it (`room` above them for the
/// spinner).
fn panel(d: &mut Draw, y: f32, title: &str, rgb: [f32; 3], line: &str, room: bool) {
    let top = if room { 56.0 } else { 36.0 };
    d.box3([120.0, y - top, 400.0, top + 56.0], 16.0, [Some(super::INK), Some(WHITE), Some([40.0, 40.0, 50.0])], 120.0);
    d.outlined(|d| d.label(title, 320.0, y - 10.0, 26.0, 360.0, rgb));
    d.label(line, 320.0, y + 24.0, 16.0, 372.0, [100.0; 3]);
}

/// Something under way: its words, the spinner turning above them.
fn busy(d: &mut Draw, t: f32, y: f32, title: &str, line: &str) {
    panel(d, y, title, WHITE, line, true);
    spinner(d, t, 320.0, y - 30.0, 14.0);
}

fn notice(d: &mut Draw, y: f32, title: &str, line: &str) {
    panel(d, y, title, super::GOLD, line, false);
}

fn failed(d: &mut Draw, y: f32, title: &str, line: &str) {
    panel(d, y, title, [128.0, 50.0, 40.0], line, false);
}

/// What the lobby's QR and Copy Link share: the short code's link, or the browser's LAN offer (scanned, nothing else
/// reaches it). Never the long iroh ticket (B224: useless to type or scan); nothing until the short code is there.
fn shared(v: &View) -> Option<String> {
    if !v.short.is_empty() {
        Some(crate::net::link(&v.short))
    } else {
        crate::net::rtc::is_offer(&v.code).then(|| v.code.clone())
    }
}

/// The lobby code's QR, drawn as `Tex::Qr`.
#[derive(Resource)]
pub struct QrImage(pub Handle<Image>, String);

/// `code` as a QR: black modules on white, 4 modules of quiet zone, one texel a module (drawn with nearest
/// sampling would be sharper; the menu's linear filter blurs edges a little at small sizes).
fn qr(code: &str) -> Option<Image> {
    let q = qrcode::QrCode::new(code.as_bytes()).ok()?;
    let n = q.width();
    let side = n + 8;
    let mut rgba = vec![255u8; side * side * 4];
    for (i, c) in q.to_colors().iter().enumerate() {
        if *c == qrcode::Color::Dark {
            let (x, y) = (i % n + 4, i / n + 4);
            rgba[(y * side + x) * 4..][..3].fill(0);
        }
    }
    let mut img = Image::new(Extent3d { width: side as u32, height: side as u32, depth_or_array_layers: 1 }, TextureDimension::D2, rgba, TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::RENDER_WORLD);
    img.sampler = bevy::image::ImageSampler::nearest();
    Some(img)
}

/// The network's events into the menu, and the QR of the menu's code whenever the code changes.
pub(super) fn poll(
    net: Option<Res<Net>>,
    mut menu: ResMut<Menu>,
    mut images: ResMut<Assets<Image>>,
    made: Option<Res<QrImage>>,
    gamepads: Query<(Entity, &Gamepad)>,
    args: Res<crate::Args>,
    time: Res<Time>,
    mut commands: Commands,
    art: Option<ResMut<super::Art>>,
) {
    menu.net.spin = (menu.net.spin + time.delta_secs()) % 60.0;
    if let Some(net) = net {
        for e in net.poll() {
            if let Event::Start(r, me) = &e {
                launch(&menu, r.clone(), me.clone(), &gamepads, &args, &mut commands);
            }
            if let Event::Pack(h, b) = &e {
                menu.got_pack(&args.iso, *h, b);
            }
            menu.heard(e);
        }
        // B281: once the host's wait is over, settled on any screen (stand-ins: no prompt, no cursor moved)
        if menu.screen == Screen::Lobby || menu.net.room.as_ref().is_some_and(|r| r.stand_in) {
            menu.settle(&args.iso, &net);
        }
        let d = menu.undecided();
        if d != menu.net.deciding {
            menu.net.deciding = d;
            net.send(Cmd::Deciding(d));
        }
        if let Some(mut art) = art {
            menu.borrow(&mut art.2);
        }
        // B201: everyone's places on the select into its panels, and this machine's out when they move
        menu.align();
        if menu.net.room.is_some() && !menu.net.me.is_empty() && !menu.net.started && matches!(menu.screen, Screen::Chars | Screen::Lobby | Screen::Confirm) {
            let lives = menu.lives();
            if menu.net.sent.as_ref() != Some(&lives) {
                net.send(Cmd::Live(lives.clone()));
                menu.net.sent = Some(lives);
            }
        }
    }
    if let Some(code) = &shared(&menu.net)
        && made.is_none_or(|m| m.1 != *code)
        && let Some(img) = qr(code)
    {
        commands.insert_resource(QrImage(images.add(img), code.clone()));
    }
}

/// B214: a pill grid's cursor moved by ←→↑↓: the nearest pill that way (sideways distance counted double, as the
/// select's `grid_step`; ←→ only along its row); none that way, it stays (the grids don't wrap).
pub(super) fn pill_step(rects: &[[f32; 4]], from: usize, k: Press) -> usize {
    let (dx, dy) = ((k.right as i32 - k.left as i32) as f32, (k.down as i32 - k.up as i32) as f32);
    let centre = |[x, y, w, h]: [f32; 4]| (x + w / 2.0, y + h / 2.0);
    let Some(&f) = rects.get(from) else { return 0 };
    let (fx, fy) = centre(f);
    let cost = |&(i, &r): &(usize, &[f32; 4])| {
        let (ax, ay) = (centre(r).0 - fx, centre(r).1 - fy);
        let along = ax * dx + ay * dy;
        // ←→ keep to the row
        (along > 1.0 && (dx == 0.0 || ay.abs() < r[3])).then_some((along + 2.0 * (ax * dy - ay * dx).abs(), i))
    };
    rects.iter().enumerate().filter_map(|e| cost(&e)).min_by(|a, b| a.0.total_cmp(&b.0)).map_or(from, |c| c.1)
}

/// Started from a lobby link (the browser's `?code=`, or `HST_CODE`): straight to joining that lobby as one player
/// (the first device to press anything is it, `Menu::step`), then its character select.
pub(super) fn linked(m: &mut Menu) {
    #[cfg(target_arch = "wasm32")]
    let text = crate::web::pasted();
    #[cfg(not(target_arch = "wasm32"))]
    let text = std::env::var("HST_CODE").ok();
    let Some(code) = text.as_deref().and_then(crate::net::code_in) else { return };
    let net = View { paste: code, go: Some(Go::Code), ..default() };
    *m = Menu { screen: Screen::Assign, doubles: true, net, settings: m.settings.clone(), mods: std::mem::take(&mut m.mods), ..default() };
}

/// Join by Code's keyboard: typed symbols, Backspace and Enter (finish) go into the code, not to the menu's key bindings (the
/// browser's on-screen keyboard too, `web.rs`).
pub(super) fn typing(
    mut menu: ResMut<Menu>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut typed: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut was: Local<bool>,
    gamepads: Query<&Gamepad>,
    mut pending: Option<ResMut<super::pointer::Pending>>,
) {
    use bevy::input::keyboard::Key;
    let on = menu.screen == Screen::Code;
    if on != *was {
        *was = on;
        #[cfg(target_arch = "wasm32")]
        crate::web::code_entry(on);
    }
    if !on {
        typed.clear();
        return;
    }
    for e in typed.read().filter(|e| e.state.is_pressed()) {
        match &e.logical_key {
            Key::Character(c) => {
                if !e.repeat {
                    c.chars().for_each(|c| menu.type_char(c));
                }
                keys.clear_just_pressed(e.key_code);
            }
            Key::Backspace => {
                menu.net.entry.pop();
                keys.clear_just_pressed(e.key_code);
            }
            // B159: Enter (either) finishes the entry: the cursor onto Save / Join, which the menu's OK then takes
            Key::Enter => menu.sel = menu.keys().len() + 2,
            _ => {}
        }
    }
    let save = menu.keys().len() + 2;
    // B207: Start on a pad finishes the entry from any key, as Enter: the cursor onto Save / Join, which its OK takes
    if gamepads.iter().any(|g| super::controls::readable(g) && g.just_pressed(GamepadButton::Start)) {
        menu.sel = save;
    }
    #[cfg(target_arch = "wasm32")]
    for c in crate::web::typed().chars() {
        match c {
            '\u{8}' => _ = menu.net.entry.pop(),
            // B207: the phone keyboard's Return / Go: a tap on Save / Join
            '\n' => {
                menu.sel = save;
                if let Some(p) = pending.as_mut() {
                    p.0.push((Act::Row(save, OK), true));
                }
            }
            c => menu.type_char(c),
        }
    }
    // B207: typing on the phone's keyboard only: the cursor stays on Save / Join (the touch pad's ✕ takes it)
    if touch_typing() {
        menu.sel = save;
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &mut pending;
}

/// B207: a touch page with no controller types on the phone's own keyboard: no letter grid or its buttons.
pub(super) fn touch_typing() -> bool {
    #[cfg(target_arch = "wasm32")]
    return crate::web::touched() && !crate::web::real_pad();
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// A made-up lobby and lobby list for `HST_MENU=lobby|lobbies` shots: two seats taken, a code (for the QR).
pub(super) fn sample(m: &mut Menu) {
    let pick = |char| Pick::disc(char, 0);
    let menu = std::env::var("HST_MENU").unwrap_or_default();
    let peers = ["k51qzi5uqu5d", "r8mxw2pa0c1e"];
    let seats = vec![Some(Seat { peer: "host".into(), pick: pick(6) }), Some(Seat { peer: peers[0].into(), pick: pick(1) }), Some(Seat { peer: peers[1].into(), pick: pick(10) }), None];
    let names = [("host", "MAYA"), (peers[0], "Sunny Day"), (peers[1], "ACE-42")].map(|(p, n)| (p.to_string(), n.to_string())).into();
    let ready = [peers[0].to_string()].into();
    // B195: ACE-42 still on the character select, the seat held
    let picking = [peers[1].to_string()].into();
    m.net.room = Some(Room { name: "MAYA".into(), doubles: true, court: 0, sets: 1, games: 4, umpire: 4, seats, names, ready, picking, ..Default::default() });
    m.net.hosting = menu != "lobby-guest";
    // the host's settings as its lobby has them
    (m.court, m.sets, m.games, m.umpire) = (0, 1, 4, 4);
    m.net.naming = menu == "name";
    if m.net.naming {
        m.net.entry = "SUNNY DAY".into();
    }
    // B201: ACE-42 on Kaoru's second costume, live; as 2P (Sunny Day) on the shared select, the host ready beside them
    let r = m.net.room.as_mut().unwrap();
    r.live.insert(peers[1].into(), vec![Live { char: 4, costume: 1, ready: false, r#mod: None, name: String::new() }]);
    // B216: a computer player on 4P, the host's hand on it
    if menu == "lobby-bot" {
        r.seats[3] = Some(Seat { peer: crate::net::CPU.into(), pick: Pick::disc(9, 4) });
        m.sel = 2;
    }
    if menu == "chars-online" {
        r.live.insert("host".into(), vec![Live { char: 6, costume: 0, ready: true, r#mod: None, name: String::new() }]);
        r.ready.clear();
        (m.net.hosting, m.net.me, m.net.go) = (false, peers[0].into(), Some(Go::Code));
        m.align();
    }
    // B281: the host pressed Start; Sunny Day is still choosing about a mod
    if menu == "lobby-wait" {
        let r = m.net.room.as_mut().unwrap();
        r.deciding.insert(peers[0].into());
        r.wait = 7;
    }
    if std::env::var("HST_MENU").as_deref() == Ok("lobby-ask") {
        // 2P brought a mod this machine doesn't have: the prompt
        let w = Wire { id: "x".into(), name: "Emi".into(), donor: 1, hand: 1.0, base: 1, overrides: vec![], ai_row: 1, names: vec![], parent: vec![], rest: vec![], looks: 7, dir: None };
        m.net.room.as_mut().unwrap().seats[1].as_mut().unwrap().pick.r#mod = Some(Box::new(w));
        m.net.ask = Some(7);
    }
    m.net.short = "K7QX".into();
    if !m.net.naming {
        m.net.entry = "K7".into();
    }
    m.net.code = "endpointaabbccddeeffgghhiijjkkllmmnnooppqqrrssttuuvvwwxxyyzz223344556677".into();
    m.net.list = (0..3).map(|i| Listing { id: format!("{i}"), name: format!("Lobby {i}"), doubles: i != 1, court: i, taken: 1 + i % 2, seats: if i != 1 { 4 } else { 2 }, time: 0, code: String::new() }).collect();
    // B236: the loading, empty and failed states
    match menu.as_str() {
        "lobbies-searching" | "lobbies-none" | "lobbies-failed" => m.net.list.clear(),
        "online" => m.net.room = None,
        "lobby-hosting" | "lobby-joining" | "lobby-failed" => (m.net.room, m.net.short, m.net.go, m.net.pick_next) = (None, String::new(), Some(Go::Code), true),
        _ => {}
    }
    match menu.as_str() {
        "lobbies-searching" | "lobbies-more" => (m.net.finding, m.net.status) = (1, "Looking for lobbies...".into()),
        "lobby-hosting" => (m.net.go, m.net.status) = (Some(Go::Host), "Connecting...".into()),
        "lobby-joining" => (m.net.hosting, m.net.status) = (false, "Looking up K7QX...".into()),
        "lobbies-failed" | "lobby-failed" => {
            m.net.finding = (menu == "lobbies-failed") as u8;
            m.heard(Event::Failed("Couldn't reach the host (timed out).".into()));
        }
        "lobbies-none" => m.heard(Event::Found(vec![])),
        _ => {}
    }
}

/// Everyone's in: the online match (`netplay`), this machine's players on their devices; it ends back in the menu.
fn launch(menu: &Menu, room: Room, me: String, gamepads: &Query<(Entity, &Gamepad)>, args: &crate::Args, commands: &mut Commands) {
    let mut pads: Vec<_> = gamepads.iter().filter(|(_, g)| super::controls::readable(g)).map(|(e, _)| e).collect();
    pads.sort();
    let devices: Vec<_> = (0..menu.players())
        .filter_map(|p| menu.seats[p])
        .map(|d| match d {
            super::Dev::Keys => Some(super::Seat::Keys),
            super::Dev::Pad(e) => pads.iter().position(|x| *x == e).map(super::Seat::Pad),
            super::Dev::Open => None,
        })
        .collect();
    let flags = crate::play::netplay::flags(&room, &me, &devices, &menu.settings.flags());
    // the mods by match slot (as `netplay::flags` orders the seats): the wire's gameplay, the looks this machine has
    let order: &[usize] = if room.doubles { &[0, 2, 1, 3] } else { &[0, 1] };
    let mut mm = crate::mods::MatchMod::default();
    for (k, &s) in order.iter().enumerate() {
        if let Some(w) = room.seats.get(s).and_then(|x| x.as_ref()?.pick.r#mod.as_deref()) {
            mm.set(k, mo::for_match(w, menu.net.looks.get(&w.looks).cloned().flatten().as_deref()));
        }
    }
    info!("starting the online match: {}", flags.join(" "));
    let (base, kept) = (args.clone(), menu.clone());
    commands.queue(move |w: &mut World| {
        let Some(net) = w.remove_resource::<Net>() else { return };
        let back = crate::mode::Mode::new(move |app| {
            app.insert_resource(base);
            // B196: the match ended for a player who didn't come back: this lobby again, with whoever is still in
            if let Some((net, room, wait)) = crate::play::netplay::LOBBY.lock().unwrap().take() {
                let mut m = kept;
                (m.screen, m.sel) = (super::Screen::Lobby, 0);
                // B279: a guest ahead of its host holds as started (only Leave) until the host's `Msg::Lobby`
                (m.net.room, m.net.started, m.net.prepared, m.net.ready, m.net.sent) = (Some(room), wait, false, None, None);
                m.net.status = if wait { "Waiting for the host to leave the results..." } else { "Back in the lobby." }.into();
                app.insert_resource(net).insert_resource(super::Kept(m));
            }
            crate::menu_mode(app);
        });
        let iso = w.resource::<crate::Args>().iso.clone();
        let mode = crate::play::netplay::mode(&iso, &flags, back, net, room, me);
        let mode = if mm.0.is_empty() { mode } else { mode.and(move |app| {
            app.insert_resource(mm);
        }) };
        w.insert_resource(crate::mode::Next(Some(mode)));
    });
}

/// An Online effect (from `Menu::step`).
/// Returns the sounds to play.
pub(super) fn apply(fx: Fx, menu: &mut Menu, commands: &mut Commands, net: Option<&Net>, clipboard: &mut bevy::clipboard::Clipboard, iso: &str) -> Vec<u8> {
    let mut out = Vec::new();
    match fx {
        Fx::Net(c) => match net {
            Some(n) => {
                // the name may have changed since the network started
                if matches!(c, Cmd::Host(_) | Cmd::Join(..) | Cmd::Quick(_)) {
                    n.send(Cmd::Name(menu.net.cfg.as_ref().map_or_else(String::new, |c| c.name.clone())));
                }
                n.send(c)
            }
            None => {
                let mut cfg = menu.net.cfg.clone().unwrap_or_else(|| Online::load(iso));
                // a browser's LAN code (M2k) needs no relay: none is dialled (none would be reachable with no internet)
                if matches!(&c, Cmd::Join(code, _) if crate::net::rtc::is_offer(code)) {
                    cfg.relay = Relay::Off;
                }
                let n = Net::start(cfg);
                n.send(c);
                commands.insert_resource(n);
            }
        },
        Fx::Off => commands.remove_resource::<Net>(),
        Fx::Paste => {
            // ponytail: desktop clipboards answer at once; the browser's is async and would need polling
            let text = clipboard.fetch_text().poll_result().and_then(Result::ok);
            // the browser: what was pasted on the page, or the URL's `?code=`
            #[cfg(target_arch = "wasm32")]
            let text = text.or_else(crate::web::pasted);
            menu.pasted(text, &mut out);
        }
        Fx::Copy if !menu.net.short.is_empty() => {
            let link = crate::net::link(&menu.net.short);
            let ok = clipboard.set_text(link.clone()).is_ok();
            menu.net.status = if ok { "Link copied: send it to a friend (it opens this lobby), or they type the code in Join by Code.".into() } else { link };
        }
        Fx::Copy => {
            // only the browser's LAN offer gets here (`shared`): the long iroh ticket is never offered (B224)
            let ok = clipboard.set_text(menu.net.code.clone()).is_ok();
            menu.net.status = if ok { "Code copied: send it to the other players (they paste it in Join by Code).".into() } else { format!("Code: {}", menu.net.code) };
        }
        Fx::Load => {
            menu.net.cfg.get_or_insert_with(|| Online::load(iso));
            // the Online screen's card lists the open lobbies: look for them at once
            if net.is_none() {
                menu.net.list.clear();
                menu.net.finding = crate::net::FINDS;
                return apply(Fx::Net(Cmd::Find), menu, commands, net, clipboard, iso);
            }
        }
        Fx::SaveCfg => {
            if let Some(cfg) = &menu.net.cfg {
                let path = Online::path(iso);
                if let Err(e) = crate::store::write(&path, cfg.text()) {
                    warn!("{}: {e}", path.display());
                }
            }
        }
    }
    out.into_iter().filter_map(|e| if let Effect::Sound(s) = e { Some(s) } else { None }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B269: a computer player the host adds opens on the cell the last pick readied on (Random here).
    #[test]
    fn an_added_bot_starts_on_the_last_pick() {
        let mut m = Menu { doubles: true, last: Some(None), ..default() };
        m.edit_bot(2, Some(&Pick::disc(6, 0)), &mut vec![]);
        assert!(m.players[2].random && !m.players[2].ready);
        m.last = Some(Some(super::super::Who::Disc(4)));
        m.edit_bot(3, Some(&Pick::disc(6, 0)), &mut vec![]);
        assert_eq!((m.players[3].char(), m.players[3].random), (4, false));
    }

    #[test]
    fn online_flow_host_and_code() {
        let mut m = Menu { sel: 1, net: View { cfg: Some(Online::default()), ..default() }, ..default() };
        let dev = super::super::Dev::Keys;
        let ok = Press { ok: true, ..default() };
        // B195: Online → Host → singles/doubles → controllers → the lobby (no character yet) → its select → lobby
        m.step(dev, ok);
        assert_eq!((m.screen, m.sel), (Screen::Online, 0), "Quickplay first");
        m.step(dev, Press { down: true, ..default() });
        m.step(dev, ok);
        assert_eq!((m.screen, m.net.go), (Screen::Mode, Some(Go::Host)));
        m.step(dev, Press { down: true, ..default() });
        m.step(dev, ok);
        assert_eq!(m.screen, Screen::Assign);
        let fx = m.step(dev, ok);
        assert_eq!(m.screen, Screen::Lobby);
        let Some(Effect::Online(Fx::Net(Cmd::Host(r)))) = fx.into_iter().find(|e| matches!(e, Effect::Online(_))) else { panic!() };
        assert_eq!((r.doubles, r.open(), r.picking.contains("host")), (true, 3, true));
        m.heard(Event::Room(r));
        assert_eq!(m.screen, Screen::Chars, "the lobby open: its character select");
        let fx = m.step(dev, ok);
        assert_eq!(m.screen, Screen::Lobby, "{fx:?}: one local player ready is everyone here");
        assert!(fx.contains(&Effect::Online(Fx::Net(Cmd::Picks(m.picks())))), "{fx:?}");
        // the host's settings row: court and umpire open their select (back on the pill, sent to everyone), ←→ change
        // sets, games (wrapping) and mods, ↑↓ go along it
        let (l, r, d) = (Press { left: true, ..default() }, Press { right: true, ..default() }, Press { down: true, ..default() });
        let venue = |fx: &[Effect]| fx.iter().find_map(|e| if let Effect::Online(Fx::Net(Cmd::Venue(c, s, g, u))) = e { Some((*c, *s, *g, *u)) } else { None });
        (m.screen, m.net.hosting, m.court, m.sets, m.games, m.umpire) = (Screen::Lobby, true, 0, 0, 4, 1);
        m.sel = m.items().iter().position(|i| *i == Item::Set(0)).unwrap();
        assert_eq!(venue(&m.step(dev, r)), None, "the court has no < >");
        m.step(dev, ok);
        assert_eq!((m.screen, m.sel, m.picking), (Screen::Confirm, 1, Some(0)), "Select Court");
        m.court = 1;
        assert_eq!(venue(&m.step(dev, ok)), Some((1, 0, 4, 1)), "kept: the court to everyone");
        assert_eq!((m.screen, m.items()[m.sel]), (Screen::Lobby, Item::Set(0)));
        m.step(dev, d);
        m.step(dev, ok);
        assert_eq!((m.screen, m.sel), (Screen::Confirm, 4), "Select Umpire");
        m.step(dev, Press { back: true, ..default() });
        assert_eq!((m.screen, m.items()[m.sel], m.umpire), (Screen::Lobby, Item::Set(1), 1), "put back");
        m.step(dev, d);
        assert_eq!(venue(&m.step(dev, l)), Some((1, 2, 4, 1)), "one set back: five");
        m.step(dev, d);
        assert_eq!(venue(&m.step(dev, r)), Some((1, 2, 6, 1)));
        assert_eq!(venue(&m.step(dev, r)), Some((1, 2, 2, 1)), "games wrap");
        m.step(dev, d);
        assert!(m.step(dev, r).contains(&Effect::Online(Fx::Net(Cmd::Vanilla(true)))), "mods off: a vanilla lobby");
        assert_eq!(m.items()[m.sel], Item::Set(4));
        // a bad paste stays put
        let mut m = Menu { screen: Screen::Online, sel: 3, ..default() };
        let mut out = Vec::new();
        m.pasted(Some("hello".into()), &mut out);
        assert_eq!(m.screen, Screen::Online);
        m.pasted(Some(iroh::SecretKey::generate().public().to_string()), &mut out);
        assert_eq!((m.screen, m.net.go), (Screen::Assign, Some(Go::Code)));
    }

    #[test]
    fn join_by_typing_a_short_code() {
        let mut m = Menu { screen: Screen::Online, sel: 3, seats: [Some(super::super::Dev::Keys), None, None, None], ..default() };
        let dev = super::super::Dev::Keys;
        let ok = Press { ok: true, ..default() };
        m.step(dev, ok);
        assert_eq!(m.screen, Screen::Code);
        // "k" from the keyboard, the rest picked on the grid: 7 (row 0, column 5), Q, X
        m.type_char('k');
        m.type_char('0');
        for c in ['7', 'Q', 'X'] {
            let i = SYMBOLS.iter().position(|&s| s as char == c).unwrap();
            m.sel = 0;
            for _ in 0..i / 8 {
                m.step(dev, Press { down: true, ..default() });
            }
            for _ in 0..i % 8 {
                m.step(dev, Press { right: true, ..default() });
            }
            m.step(dev, ok);
        }
        assert_eq!((m.net.entry.as_str(), m.sel), ("K7QX", KEYS + 2), "full: on Join");
        m.step(dev, ok);
        // B225: no singles/doubles, straight to the controllers; Back returns to the code
        assert_eq!((m.screen, m.net.go), (Screen::Assign, Some(Go::Code)));
        m.step(dev, Press { back: true, ..default() });
        assert_eq!((m.screen, m.net.entry.as_str()), (Screen::Code, "K7QX"));
        m.sel = KEYS + 2;
        m.step(dev, ok);
        let fx = m.step(dev, ok);
        assert!(fx.contains(&Effect::Online(Fx::Net(Cmd::Join("K7QX".into(), m.picks())))), "{fx:?}");
        // the lobby's mode, whatever this machine had: a singles lobby's select is singles
        assert_eq!(m.screen, Screen::Lobby);
        m.heard(Event::Room(Room { doubles: false, seats: vec![None; 2], ..Default::default() }));
        assert_eq!((m.screen, m.doubles, m.players()), (Screen::Chars, false, 2));
        // B239: □ deletes one character, ○ leaves at once
        let mut m = Menu { screen: Screen::Code, net: View { entry: "AB".into(), ..default() }, ..default() };
        m.step(dev, Press { page: true, ..default() });
        assert_eq!((m.screen, m.net.entry.as_str()), (Screen::Code, "A"));
        m.step(dev, Press { back: true, ..default() });
        assert_eq!((m.screen, m.sel), (Screen::Online, 3));
        // a pasted link joins its code
        let mut out = Vec::new();
        m.pasted(Some(crate::net::link("K7QX")), &mut out);
        assert_eq!((m.screen, m.net.paste.as_str()), (Screen::Assign, "K7QX"));
    }

    #[test]
    fn lobby_taps_hit_items() {
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        m.net.hosting = true;
        let at = |m: &Menu, x: f32, y: f32| match super::super::pointer::tests::act_at(m, x, y) {
            Some(Act::Row(i, _)) => Some(i),
            _ => None,
        };
        // the host: seats 1, 2 (guests), 3 (open), then Start, Characters, Copy Link, Public, Leave on the side card,
        // the settings row last: each one's middle hits it
        let rects = m.lobby_rects();
        assert_eq!(rects.len(), 14);
        for (k, [x, y, w, h]) in rects.iter().enumerate() {
            assert_eq!(at(&m, x + w / 2.0, y + h / 2.0), Some(k), "{:?}", m.items()[k]);
        }
        let [x, y, ..] = super::super::seat_rect(m.grid.split().0, true, 0);
        assert_eq!(at(&m, x + 100.0, y + 10.0), None, "the host's own seat");
        // the side card beside the seats, every item inside the 4:3 screen
        assert!(rects.iter().all(|r| r[0] >= 0.0 && r[0] + r[2] <= 640.0 && r[1] >= 0.0 && r[1] + r[3] <= 448.0), "{rects:?}");
        m.net.hosting = false;
        let [x, y, w, h] = m.lobby_rects()[0];
        assert_eq!(at(&m, x + w / 2.0, y + h / 2.0), Some(0), "a guest's Ready");
    }

    #[test]
    fn lobby_start_ready_kick_and_name() {
        let dev = super::super::Dev::Keys;
        let ok = Press { ok: true, ..default() };
        let net = |fx: &[Effect]| fx.iter().find_map(|e| if let Effect::Online(Fx::Net(c)) = e { Some(c.clone()) } else { None });
        // the host: a guest's seat first (OK takes them out), then Start (a countdown, ready or not), which Cancel stops
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        m.net.hosting = true;
        assert_eq!(m.items(), vec![Item::Seat(1), Item::Seat(2), Item::Seat(3), Item::Button("Start"), Item::Button("Characters"), Item::Button("Copy Link"), Item::Button("Public"), Item::Button("Leave"), Item::Set(0), Item::Set(1), Item::Set(2), Item::Set(3), Item::Set(4), Item::Mode]);
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Kick("k51qzi5uqu5d".into())));
        m.sel = 3;
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Start), "ACE-42 isn't ready: B215 starts the countdown anyway");
        m.net.room.as_mut().unwrap().count = 4;
        assert_eq!((m.lobby_ok(), net(&m.step(dev, ok))), (Some("Cancel"), Some(Cmd::Cancel)));
        // B213: Public / Private, remembered for the next Host
        m.sel = 6;
        assert_eq!(m.lobby_ok(), Some("Make Private"));
        let fx = m.step(dev, ok);
        assert_eq!(net(&fx), Some(Cmd::Private(true)));
        assert!(fx.contains(&Effect::Online(Fx::SaveCfg)) && m.net.cfg.as_ref().is_some_and(|c| c.private));
        assert!(m.room().private);
        // a guest: Ready toggles; Leave leaves
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        m.net.hosting = false;
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Prepared(true)));
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Prepared(false)));
        // B215: a countdown running: Ready is Cancel
        m.net.room.as_mut().unwrap().count = 3;
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Cancel));
        m.net.room.as_mut().unwrap().count = 0;
        // Characters: unready (it was), back to the select
        m.net.prepared = true;
        m.step(dev, Press { up: true, ..default() });
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Prepared(false)));
        assert_eq!(m.screen, Screen::Chars);
        m.screen = Screen::Lobby;
        m.sel = 2;
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Leave));
        assert_eq!((m.screen, m.net.room.is_none()), (Screen::Online, true));
        // taken out: only Leave is left
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        m.net.hosting = false;
        m.heard(Event::Failed("the host took you out of the lobby".into()));
        assert_eq!(m.items(), vec![Item::Button("Leave")]);
        // Name: typed, saved to online.txt's name
        let mut m = Menu { screen: Screen::Online, sel: 4, net: View { cfg: Some(Online::default()), ..default() }, ..default() };
        m.step(dev, ok);
        assert_eq!((m.screen, m.net.naming, m.net.entry.as_str()), (Screen::Code, true, "PLAYER"));
        m.net.entry.clear();
        "Sunny Day~".chars().for_each(|c| m.type_char(c));
        m.sel = NAME_KEYS.len() + 2;
        let fx = m.step(dev, ok);
        assert!(fx.contains(&Effect::Online(Fx::SaveCfg)), "{fx:?}");
        assert_eq!((m.screen, m.sel, m.net.cfg.unwrap().name.as_str()), (Screen::Online, 4, "Sunny Day"));
    }

    /// B234: a mod pick is named by the mod, not its donor, on the peer that has it and on one that doesn't (the
    /// name rides along with the live place and the seat's pick).
    #[test]
    fn mod_name_not_donor() {
        let w = Wire { id: "emi".into(), name: "Emi".into(), donor: 4, hand: 1.0, base: 4, overrides: vec![], ai_row: 4, names: vec![], parent: vec![], rest: vec![], looks: 7, dir: None };
        // the guest with the mod, on it: its live place carries the name
        let mut a = Menu { screen: Screen::Chars, ..default() };
        a.mods = vec![crate::mods_online::for_match(&w, None)];
        a.seats[0] = Some(super::super::Dev::Keys);
        (a.players[0].modded, a.players[0].pick) = (true, 0);
        let live = a.lives();
        assert_eq!((live[0].char, live[0].r#mod.as_deref(), live[0].name.as_str()), (4, Some("emi"), "Emi"));
        // the host without it: still choosing, then ready with the mod's pick in the seat
        let mut b = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut b);
        (b.net.me, b.net.go) = ("host".into(), Some(Go::Code));
        let r = b.net.room.as_mut().unwrap();
        r.live.insert("k51qzi5uqu5d".into(), live);
        assert_eq!(b.remote_mod_name(1), Some("Emi"));
        let r = b.net.room.as_mut().unwrap();
        r.live.clear();
        r.seats[1].as_mut().unwrap().pick = Pick { r#mod: Some(Box::new(w.clone())), ..Pick::disc(4, 0) };
        assert_eq!((b.remote_mod_name(1), b.remote_mod_name(2)), (Some("Emi"), None), "a disc pick keeps the sheet's name");
        // with the mod installed the select draws it as this machine's own (its name from its pack)
        b.mods = vec![crate::mods_online::for_match(&w, None)];
        assert_eq!(b.remote_mod_name(1), None);
        b.seats[0] = Some(super::super::Dev::Keys);
        b.align();
        assert_eq!(b.pick(1), (super::super::Who::Mod(0), 0));
    }

    /// B285: a mod pick shows as the mod for its owner (its pick carries it, its looks are its own folder) and, in the
    /// lobby and the select, for a peer that downloaded it (kept after the roster, off the grid); a stand-in otherwise.
    #[test]
    fn own_and_downloaded_mods_show_as_themselves() {
        let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/mods/test_pc00"));
        if !dir.is_dir() {
            return eprintln!("no context/mods/test_pc00, skipped");
        }
        let m = crate::mods::read(dir).unwrap();
        let tmp = std::env::temp_dir().join(format!("hst-b285-{}", std::process::id()));
        // the owner: the pick carries the mod, and its looks are found in the roster
        let mut a = Menu { screen: Screen::Chars, mods: vec![m.clone()], ..default() };
        a.seats[0] = Some(super::super::Dev::Keys);
        (a.players[0].modded, a.players[0].pick) = (true, 0);
        let w = *a.wire(0).r#mod.expect("the owner's pick carries its mod");
        assert_eq!(mo::find(&w, &a.mods, &tmp), Some(m.dir.clone()));
        // a guest that downloaded it: drawn as the mod in its seat, not on its own roster
        let got = mo::unpack(&mo::pack(dir).unwrap(), w.looks, &tmp).unwrap();
        let mut b = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut b);
        (b.net.me, b.net.go) = ("host".into(), Some(Go::Code));
        let r = b.net.room.as_mut().unwrap();
        r.live.clear();
        r.picking.clear();
        r.seats[1].as_mut().unwrap().pick = Pick { r#mod: Some(Box::new(w.clone())), ..Pick::disc(w.donor, 0) };
        let mut text = super::super::Text { msg: Vec::new(), glyphs: default(), grades: [[0; 5]; 14], style: [0; 14], hands: [1.0; 14], ranks: Vec::new(), mods: Vec::new() };
        b.borrow(&mut text);
        assert!(b.mods.is_empty(), "no looks yet: the donor stands in");
        b.net.looks.insert(w.looks, Some(got.dir));
        b.borrow(&mut text);
        assert_eq!((b.mods.len(), text.mods.len(), b.own_mods(), b.mods[0].id.as_str()), (1, 1, 0, w.id.as_str()));
        assert!(!b.roster().contains(&Some(super::super::Who::Mod(0))));
        b.seats[0] = Some(super::super::Dev::Keys);
        b.align();
        assert_eq!((b.pick(1), b.remote_mod_name(1)), ((super::super::Who::Mod(0), 0), None));
        // gone with the lobby
        b.net.room = None;
        b.borrow(&mut text);
        assert!(b.mods.is_empty() && text.mods.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Every lobby item reachable with ↑↓←→ from every other, whatever the seats hold (host and guest).
    #[test]
    fn arrows_reach_every_lobby_item() {
        let dev = super::super::Dev::Keys;
        let cpu = || Some(Seat { peer: crate::net::CPU.into(), pick: Pick::disc(9, 4) });
        let guest = || Some(Seat { peer: "k51qzi5uqu5d".into(), pick: Pick::disc(1, 0) });
        let host = || Some(Seat { peer: "host".into(), pick: Pick::disc(6, 0) });
        for doubles in [false, true] {
            for mask in 0..if doubles { 27 } else { 3 } {
                for hosting in [true, false] {
                    let mut m = Menu { screen: Screen::Lobby, ..default() };
                    sample(&mut m);
                    (m.net.me, m.net.go, m.seats[0], m.doubles, m.net.hosting) = ("host".into(), Some(Go::Host), Some(dev), doubles, hosting);
                    let r = m.net.room.as_mut().unwrap();
                    r.doubles = doubles;
                    r.seats = vec![host()];
                    let mut k = mask;
                    for _ in 1..if doubles { 4 } else { 2 } {
                        r.seats.push([None, cpu(), guest()][k % 3].clone());
                        k /= 3;
                    }
                    let n = m.items().len();
                    for from in 0..n {
                        let mut seen = vec![false; n];
                        let mut todo = vec![from];
                        while let Some(i) = todo.pop() {
                            if std::mem::replace(&mut seen[i], true) {
                                continue;
                            }
                            for f in [|k: &mut Press| k.up = true, |k: &mut Press| k.down = true, |k: &mut Press| k.left = true, |k: &mut Press| k.right = true] {
                                let mut c = m.clone();
                                c.sel = i;
                                let mut k = Press::default();
                                f(&mut k);
                                c.step(dev, k);
                                assert_eq!((c.screen, c.net.room.as_ref().map(|r| r.seats.len())), (Screen::Lobby, m.net.room.as_ref().map(|r| r.seats.len())));
                                todo.push(c.sel);
                            }
                        }
                        assert!(seen.iter().all(|&s| s), "doubles {doubles} mask {mask} hosting {hosting}: from {:?} not {:?}", m.items()[from], m.items().iter().zip(&seen).filter(|p| !p.1).collect::<Vec<_>>());
                    }
                }
            }
        }
    }

    /// B201: the select's panels are the lobby's seats (this machine's player in the one it holds, the others' live
    /// places beside it, their ready costumes taken); the host's computer players and singles / doubles.
    #[test]
    fn shared_select_and_host_bots() {
        let dev = super::super::Dev::Keys;
        let ok = Press { ok: true, ..default() };
        let net = |fx: &[Effect]| fx.iter().find_map(|e| if let Effect::Online(Fx::Net(c)) = e { Some(c.clone()) } else { None });
        // a guest holding seat 1: its player moves there, the host's and ACE-42's live places fill the others
        let mut m = Menu { screen: Screen::Chars, ..default() };
        sample(&mut m);
        let r = m.net.room.as_mut().unwrap();
        r.live.insert("host".into(), vec![Live { char: 6, costume: 2, ready: true, r#mod: None, name: String::new() }]);
        r.live.insert("r8mxw2pa0c1e".into(), vec![Live { char: 4, costume: 1, ready: false, r#mod: None, name: String::new() }]);
        (m.net.hosting, m.net.me, m.net.go, m.seats[0], m.doubles) = (false, "k51qzi5uqu5d".into(), Some(Go::Code), Some(dev), false);
        m.players[0].cursor = 3;
        m.align();
        assert_eq!((m.doubles, m.seats), (true, [None, Some(dev), None, None]));
        assert_eq!(m.players[1].cursor, 3, "this machine's own place kept");
        assert_eq!((m.pick(0), m.players[0].ready), ((super::super::Who::Disc(6), 2), true));
        assert_eq!((m.pick(2), m.players[2].ready), ((super::super::Who::Disc(4), 1), false));
        assert_eq!((m.players[3].ready, m.remote(3).is_none(), m.remote(1).is_none()), (false, true, true));
        // B66: the host's ready costume is passed over; this machine's place goes out
        m.players[1].cursor = super::super::GRID.iter().position(|g| g.0 == 6).unwrap();
        m.players[1].costume = 2;
        assert!(m.blocked(1));
        assert_eq!(m.lives(), vec![Live { char: 6, costume: 2, ready: false, r#mod: None, name: String::new() }]);
        // the host: OK on the open seat adds a computer player nobody has and opens the select on it (B216): the
        // host's device drives its cursor; ready, its pick goes to the lobby
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        (m.net.me, m.sel, m.net.go, m.seats[0]) = ("host".into(), 2, Some(Go::Host), Some(dev));
        let Some(Cmd::Bot(3, Some(p))) = net(&m.step(dev, ok)) else { panic!() };
        assert!(!m.bot_taken(3).unwrap().contains(&(p.char, p.costume)));
        // B217: added on Random: every character and costume comes up, never a taken one
        let taken = m.bot_taken(3).unwrap();
        assert!(!taken.is_empty());
        let drawn: HashSet<(usize, usize)> = (0..20_000u64).map(|s| m.bot_random(3, s.wrapping_mul(0x9e37_79b9_7f4a_7c15))).map(|p| p.map(|p| (p.char, p.costume)).unwrap()).collect();
        assert_eq!(drawn.len(), super::super::CHARS * super::super::COSTUMES - taken.len());
        assert!(drawn.iter().all(|t| !taken.contains(t)));
        assert_eq!((m.screen, m.net.edit, m.driven(dev), m.players[3].ready), (Screen::Chars, Some(3), Some(3), false));
        m.align();
        assert_eq!(m.players[3].char(), p.char, "the seat being picked isn't overwritten by the room");
        // from the grid's first card (the add drew a random one), so the steps below land on known picks
        (m.players[3].cursor, m.players[3].costume) = (0, 0);
        m.step(dev, Press { right: true, ..default() });
        m.step(dev, Press { next: true, ..default() });
        let c = m.players[3].char();
        let Some(Cmd::Bot(3, Some(p))) = net(&m.step(dev, ok)) else { panic!() };
        // Cody: Sunny Day is ready on costume 1, so it landed on 2 and R1 took it to 3
        assert_eq!((p.char, p.costume, m.screen, m.net.edit), (c, 2, Screen::Lobby, None));
        m.net.room.as_mut().unwrap().seats[3] = Some(Seat { peer: crate::net::CPU.into(), pick: Pick::disc(6, 0) });
        // OK on a computer's seat: its pick on the select again; ○ back to the lobby as it was
        m.sel = 2;
        assert_eq!(net(&m.step(dev, ok)), None);
        assert_eq!((m.screen, m.players[3].char(), m.players[3].ready), (Screen::Chars, 6, false));
        assert_eq!(net(&m.step(dev, Press { back: true, ..default() })), None);
        assert_eq!((m.screen, m.net.edit), (Screen::Lobby, None));
        m.sel = 2;
        let Some(Cmd::Bot(3, Some(p))) = net(&m.step(dev, Press { next: true, ..default() })) else { panic!() };
        assert_eq!((p.char, p.costume, m.sel), (6, 1, 2), "the host's costume 0 of the same character taken");
        m.net.room.as_mut().unwrap().seats[3].as_mut().unwrap().pick.random = true;
        let Some(Cmd::Bot(3, Some(p))) = net(&m.step(dev, Press { next: true, ..default() })) else { panic!() };
        assert!(p.random, "a new costume keeps a Random computer hidden");
        // ←→ don't change its character: → reaches its ✕, OK on that takes it off
        assert_eq!(net(&m.step(dev, Press { right: true, ..default() })), None);
        assert_eq!(m.items()[m.sel], Item::Remove(3));
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Bot(3, None)));
        assert_eq!((m.items()[m.sel], net(&m.step(dev, Press { card: true, ..default() }))), (Item::Seat(3), Some(Cmd::Bot(3, None))));
        // singles only once the taken seats fit
        m.sel = m.items().iter().position(|it| *it == Item::Mode).unwrap();
        assert_eq!(net(&m.step(dev, ok)), None);
        m.net.room.as_mut().unwrap().seats[2] = None;
        m.net.room.as_mut().unwrap().seats[3] = None;
        m.sel = m.items().iter().position(|it| *it == Item::Mode).unwrap();
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Doubles(false)));
    }

    /// B214: ←→↑↓ walk the Online screen's grid and the host's lobby grid (seats, then the buttons three to a row).
    #[test]
    fn cursor_walks_the_grids() {
        let dev = super::super::Dev::Keys;
        let [l, r, u, d] = [0, 1, 2, 3].map(|i| Press { left: i == 0, right: i == 1, up: i == 2, down: i == 3, ..default() });
        let walk = |m: &mut Menu, steps: &[(Press, usize)]| {
            for (n, (k, to)) in steps.iter().enumerate() {
                m.step(dev, *k);
                assert_eq!(m.sel, *to, "step {n} on {:?}", m.screen);
            }
        };
        // Quickplay, Host, Find, Join by Code, then Name and Relay: one column
        let mut m = Menu { screen: Screen::Online, sel: 0, net: View { cfg: Some(Online::default()), ..default() }, ..default() };
        walk(&mut m, &[(l, 0), (d, 1), (r, 1), (d, 2), (d, 3), (d, 4), (d, 5), (d, 5), (u, 4), (u, 3), (u, 2), (u, 1), (u, 0), (u, 0)]);
        let relay = m.net.cfg.clone();
        m.sel = 5;
        m.step(dev, r);
        assert_eq!((m.sel, &m.net.cfg), (5, &relay), "←→ don't change Relay");
        // the host's lobby: seats 1–3 (items 0–2); Start Characters (3, 4); Copy Link Public Leave (5–7); the settings
        // row: Court, Umpire, Sets, Games, Mods, Mode (8–13), ↑↓ along it, off Mode's end onto the side card
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        (m.net.hosting, m.sel) = (true, 0);
        walk(&mut m, &[(d, 1), (d, 2), (d, 3), (u, 4), (r, 4), (u, 6), (u, 5), (l, 5), (u, 13), (u, 12), (u, 11), (u, 10), (u, 9), (d, 10), (d, 11), (d, 12), (d, 13), (d, 5), (d, 6)]);
        // a guest: Characters over Ready over Leave
        m.net.hosting = false;
        m.sel = 0;
        walk(&mut m, &[(u, 1), (u, 1), (d, 0), (d, 2), (u, 0)]);
    }

    /// B195: Find lists every open lobby before any pick; choosing one takes this machine's controllers (no more
    /// than its open seats), joins on placeholder picks, and the seat opens the character select.
    #[test]
    fn loading_states_end() {
        // B236: Find spins until both lists are in (or it fails); a join spins until seated or refused
        let dev = super::super::Dev::Keys;
        let ok = Press { ok: true, ..default() };
        let mut m = Menu { screen: Screen::Online, sel: 2, seats: [Some(dev), None, None, None], net: View { cfg: Some(Online::default()), ..default() }, ..default() };
        m.step(dev, ok);
        // B267: one list per lookup (LAN, DHT board, slots), each as it answers
        assert_eq!(m.net.finding, crate::net::FINDS);
        for left in (0..crate::net::FINDS).rev() {
            m.heard(Event::Found(vec![]));
            assert_eq!(m.net.finding, left);
        }
        assert_eq!((m.net.finding, m.net.fail), (0, None));
        assert!(m.net.status.starts_with("No open lobbies"), "{}", m.net.status);
        m.net.finding = 2;
        m.heard(Event::Failed("DHT: down".into()));
        assert_eq!((m.net.finding, m.net.fail), (0, Some("Couldn't search for lobbies")));
        (m.net.go, m.net.pick_next, m.screen) = (Some(Go::Code), true, Screen::Lobby);
        m.heard(Event::Failed("timed out".into()));
        assert_eq!((m.net.pick_next, m.net.fail), (false, Some("Couldn't join the lobby")));
    }

    /// B279: a guest back from the results ahead of its host has only Leave until the host's lobby comes.
    #[test]
    fn waits_for_the_hosts_lobby() {
        let room = Room { seats: vec![None, None], ..default() };
        let mut m = Menu { screen: Screen::Lobby, net: View { room: Some(room.clone()), me: "g".into(), started: true, ..default() }, ..default() };
        assert_eq!(m.items(), [Item::Button("Leave")]);
        m.heard(Event::Match(Msg::Lobby(Room { ver: 7, ..room })));
        assert_eq!((m.net.started, m.net.room.as_ref().map(|r| r.ver)), (false, Some(7)));
        assert_eq!(m.items(), GUEST_BUTTONS.map(Item::Button));
    }

    fn find_first_then_pick_in_the_lobby() {
        let dev = super::super::Dev::Keys;
        let ok = Press { ok: true, ..default() };
        let mut m = Menu { screen: Screen::Online, sel: 2, seats: [Some(dev), None, None, None], net: View { cfg: Some(Online::default()), ..default() }, ..default() };
        let fx = m.step(dev, ok);
        assert_eq!(m.screen, Screen::Lobbies);
        assert!(fx.contains(&Effect::Online(Fx::Net(Cmd::Find))), "{fx:?}");
        // a singles and a doubles lobby side by side
        let l = |id: &str, doubles, taken| Listing { id: id.into(), name: id.into(), doubles, court: 1, taken, seats: if doubles { 4 } else { 2 }, time: 0, code: String::new() };
        m.heard(Event::Found(vec![l("S", false, 1), l("D", true, 2)]));
        assert_eq!(m.net.list.len(), 2);
        for (sel, id, doubles, fit) in [(0, "S", false, 1), (1, "D", true, 2)] {
            m.screen = Screen::Lobbies;
            m.sel = sel;
            m.step(dev, ok);
            assert_eq!((m.screen, m.doubles, m.local_max()), (Screen::Assign, doubles, fit));
            let fx = m.step(dev, ok);
            assert_eq!(m.screen, Screen::Lobby);
            let Some(Effect::Online(Fx::Net(Cmd::Join(code, picks)))) = fx.into_iter().find(|e| matches!(e, Effect::Online(_))) else { panic!() };
            assert_eq!((code.as_str(), picks.len()), (id, 1));
            // seated: the lobby's character select; ready there: the picks into the held seat
            let mut r = Room { doubles, seats: vec![None; if doubles { 4 } else { 2 }], ..Default::default() };
            r.seats[1] = Some(Seat { peer: "me".into(), pick: picks[0].clone() });
            r.picking.insert("me".into());
            m.heard(Event::Room(r));
            assert_eq!(m.screen, Screen::Chars);
            let fx = m.step(dev, ok);
            assert_eq!(m.screen, Screen::Lobby);
            assert!(fx.iter().any(|e| matches!(e, Effect::Online(Fx::Net(Cmd::Picks(p))) if p.len() == 1)), "{fx:?}");
            // a later line-up doesn't reopen the select
            m.heard(Event::Room(m.net.room.clone().unwrap()));
            assert_eq!(m.screen, Screen::Lobby);
        }
        // turned away while picking: the lobby screen says why
        m.screen = Screen::Chars;
        m.heard(Event::Failed("Refused: full".into()));
        assert_eq!(m.screen, Screen::Lobby);
    }

    /// B159: Enter on the keyboard finishes the entry, it doesn't type the hovered key.
    #[test]
    fn enter_finishes_entry() {
        use bevy::input::{ButtonState, keyboard::{Key, KeyboardInput}};
        let mut app = App::new();
        app.add_message::<KeyboardInput>().init_resource::<ButtonInput<KeyCode>>().add_systems(Update, typing);
        let mut m = Menu { screen: Screen::Code, net: View { naming: true, entry: "SUNNY".into(), cfg: Some(Online::default()), ..default() }, ..default() };
        m.sel = 0; // hovering "A"
        app.insert_resource(m);
        for code in [KeyCode::Enter, KeyCode::NumpadEnter] {
            app.world_mut().resource_mut::<Menu>().sel = 0;
            app.world_mut().write_message(KeyboardInput { key_code: code, logical_key: Key::Enter, state: ButtonState::Pressed, text: None, repeat: false, window: Entity::PLACEHOLDER });
            app.update();
            assert_eq!(app.world().resource::<Menu>().sel, NAME_KEYS.len() + 2, "{code:?}: on Save");
        }
        let mut m = app.world_mut().remove_resource::<Menu>().unwrap();
        m.step(super::super::Dev::Keys, Press { ok: true, ..default() });
        assert_eq!((m.screen, m.net.cfg.unwrap().name.as_str()), (Screen::Online, "SUNNY"), "saved without the hovered A");
    }

    /// B207: Start on a pad finishes the entry from any hovered key, as Enter.
    #[test]
    fn start_finishes_entry() {
        use bevy::input::keyboard::KeyboardInput;
        let mut app = App::new();
        app.add_message::<KeyboardInput>().init_resource::<ButtonInput<KeyCode>>().add_systems(Update, typing);
        let mut m = Menu { screen: Screen::Code, net: View { naming: true, entry: "SUNNY".into(), cfg: Some(Online::default()), ..default() }, ..default() };
        m.sel = 5; // hovering "F"
        app.insert_resource(m);
        let mut g = Gamepad::default();
        g.digital_mut().press(GamepadButton::Start);
        app.world_mut().spawn(g);
        app.update();
        let mut m = app.world_mut().remove_resource::<Menu>().unwrap();
        assert_eq!(m.sel, NAME_KEYS.len() + 2, "on Save");
        // the same Start is the pad's OK
        m.step(super::super::Dev::Keys, Press { ok: true, ..default() });
        assert_eq!((m.screen, m.net.cfg.unwrap().name.as_str()), (Screen::Online, "SUNNY"), "saved without the hovered F");
    }
}
