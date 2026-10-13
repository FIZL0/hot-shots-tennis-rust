//! The Online screens (M2f, `net.rs`): Host, Find, Quickplay, Join by Code and the relay setting. Find lists every open
//! lobby first (singles and doubles together); Host and Quickplay ask singles/doubles, Join by Code and a link take the
//! lobby's (B225). Then this
//! machine's 1–3 players take their controllers (no more than the lobby's open seats) and join (or open) the lobby on
//! placeholder picks, which holds their seats while they pick on the same character select as Local (B195; no
//! computer players: the open seats are for the other peers). The lobby: its seats (who, their name, picking or
//! ready), the code and QR to share, Start (the open seats to the computer), Ready, Characters (pick again), Court
//! (the host's confirm screen: court, sets, games, umpire, until Start), Leave and taking a player out. The player's
//! name (Name, `online.txt`'s `name`) is typed on Join by Code's keys.

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::pointer::{Act, OK};
use super::{Draw, Effect, Menu, Press, Screen, Tex, WHITE};
use crate::mods_online::{self as mo, Ask, Wire};
use crate::net::{Cmd, Event, Listing, Live, Net, Online, Pick, Relay, Room, SHORT, SYMBOLS, Seat, is_short};

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
    /// Remember the answer to "get a remote mod's looks?".
    Ask(Ask),
}

/// Join by Code's symbol keys (then its four buttons).
const KEYS: usize = SYMBOLS.len();
/// The name's keys on the same grid, and its longest.
const NAME_KEYS: &[u8; 40] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -.!";
const NAME_MAX: usize = 16;
const ROWS: [&str; 7] = ["Quickplay", "Host", "Find", "Join by Code", "Name", "Relay", "Back"];
/// B214: the Online screen's pills as the confirm screen's grid: Quickplay wide on top, then two columns.
const ROW_RECTS: [[f32; 4]; 7] = [
    [170.0, 84.0, 300.0, 38.0],
    [100.0, 140.0, 210.0, 38.0],
    [330.0, 140.0, 210.0, 38.0],
    [100.0, 184.0, 210.0, 38.0],
    [330.0, 184.0, 210.0, 38.0],
    [100.0, 228.0, 210.0, 38.0],
    [330.0, 228.0, 210.0, 38.0],
];
/// The lobby's buttons: the host's, a seated guest's (Ready shows "Not Ready" once ready), and with no lobby. B214:
/// no Vote button (B226 removes lobby voting).
const HOST_BUTTONS: [&str; 5] = ["Start", "Characters", "Court", "Copy Link", "Leave"];
const GUEST_BUTTONS: [&str; 3] = ["Ready", "Characters", "Leave"];
/// B214: the lobby's buttons as a grid under the seats, three to a row: their pitch, width and rows' top and pitch.
const BUTTON_STEP: f32 = 176.0;
const BUTTON_W: f32 = 168.0;
const BUTTON_Y: f32 = 322.0;
const BUTTON_ROW: f32 = 42.0;

/// What a lobby selection is: a guest's seat (the host can take them out) or a button.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
    Seat(usize),
    Button(&'static str),
    /// B201: the host's singles / doubles switch (the title).
    Mode,
}
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
        let pick = |p: usize| {
            match self.pick(p) {
                // a mod: its donor, and its gameplay data for every peer
                (super::Who::Mod(i), costume) => {
                    let m = &self.mods[i];
                    Pick { char: m.donor, costume, r#mod: Wire::of(m, costume).map_err(|e| warn!("{e}")).ok().map(Box::new) }
                }
                (super::Who::Disc(c), costume) => Pick::disc(c, costume),
            }
        };
        (0..self.players()).filter(|&p| self.seats[p].is_some()).map(pick).collect()
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
        Room { name: cfg.name, doubles: self.doubles, court, sets: self.sets, games: self.games, umpire, seats, names, picking, ..Default::default() }
    }

    /// The code screen's keys: the short code's symbols, or the name's.
    fn keys(&self) -> &'static [u8] {
        if self.net.naming { NAME_KEYS } else { SYMBOLS }
    }

    /// The lobby's selectable things: the guests' seats (host), then the buttons.
    fn items(&self) -> Vec<Item> {
        let room = self.net.room.as_ref();
        if self.net.hosting {
            // B201: every seat but its own: a guest's (take out), an open one (add a computer player), a computer's
            let seats = room.map_or(vec![], |r| (0..r.seats.len()).filter(|&i| r.seats[i].as_ref().is_none_or(|s| s.peer != "host")).collect());
            seats.into_iter().map(Item::Seat).chain(HOST_BUTTONS.map(Item::Button)).chain([Item::Mode]).collect()
        } else if room.is_some() && !self.net.started && !self.net.pick_next {
            GUEST_BUTTONS.map(Item::Button).to_vec()
        } else {
            vec![Item::Button("Leave")]
        }
    }

    /// B214: where `items` are drawn (640×448): a guest's seat on its row, the buttons three to a row under the seats.
    fn lobby_rects(&self) -> Vec<[f32; 4]> {
        let mut b = 0;
        self.items()
            .iter()
            .map(|it| match it {
                Item::Seat(i) => [60.0, 140.0 + 44.0 * *i as f32, 340.0, 38.0],
                Item::Mode => [32.0, 72.0, 300.0, 38.0],
                Item::Button(_) => {
                    b += 1;
                    [60.0 + BUTTON_STEP * ((b - 1) % 3) as f32, BUTTON_Y + BUTTON_ROW * ((b - 1) / 3) as f32, BUTTON_W, 38.0]
                }
            })
            .collect()
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
        for i in (0..4).filter(|i| !mine.contains(i)) {
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
            self.players[i] = pl;
        }
    }

    /// B201: this machine's players' places on the select, for the others to see.
    fn lives(&self) -> Vec<Live> {
        (0..4)
            .filter(|&p| self.seats[p].is_some())
            .map(|p| {
                let (who, costume) = self.pick(p);
                let (char, id) = match who {
                    super::Who::Disc(c) => (c, None),
                    super::Who::Mod(i) => (self.mods[i].donor, Some(self.mods[i].id.clone())),
                };
                Live { char, costume, ready: self.players[p].ready, r#mod: id }
            })
            .collect()
    }

    /// B201: the host's computer player in open seat `i` changed by `dc` characters (grid order) and `dk` costumes,
    /// on a disc character and costume nobody else has (B66); `None` if every one is.
    fn bot_pick(&self, i: usize, dc: isize, dk: isize) -> Option<Pick> {
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

    /// This machine's players took their controllers: open or join the lobby on placeholder picks (holding the seats);
    /// its first line-up opens the character select (`heard`).
    pub(super) fn online_open(&mut self, out: &mut Vec<Effect>) {
        let go = self.net.go.unwrap();
        self.net.status.clear();
        self.net.room = None;
        self.net.started = false;
        (self.net.ask, self.net.ready, self.net.hosting, self.net.prepared) = (None, None, false, false);
        self.net.wanted.clear();
        self.net.pick_next = true;
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
            Go::Quick => Cmd::Quick(self.room()),
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
                let to = pill_step(&ROW_RECTS, self.sel, k);
                if to != self.sel {
                    self.sel = to;
                    out.push(Sound(0));
                }
                let cfg = self.net.cfg.get_or_insert_with(Online::default);
                if self.sel == 5 && k.ok {
                    cfg.next_relay();
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
                    out.push(Effect::Online(Fx::Net(Cmd::Find)));
                } else if k.ok && self.sel < 3 {
                    self.net.go = Some([Go::Quick, Go::Host, Go::Find][self.sel]);
                    self.net.fit = 0;
                    self.go(Screen::Mode, out);
                } else if (k.ok && self.sel == 6) || k.back {
                    self.net.go = None;
                    self.sel = 1;
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
                    (self.net.paste, self.doubles, self.net.fit) = (l.id.clone(), l.doubles, l.open());
                    self.seats[1..].fill(None);
                    self.go(Screen::Assign, out);
                } else if k.ok && self.sel == n {
                    self.net.list.clear();
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
                    out.extend([Sound(2), Effect::Online(Fx::Ask(remember))]);
                }
            }
            Screen::Lobby => {
                let items = self.items();
                let bot = match items.get(self.sel) {
                    Some(Item::Seat(i)) if self.net.room.as_ref().is_some_and(|r| r.seats[*i].as_ref().is_some_and(|s| !s.human())) => Some(*i),
                    _ => None,
                };
                let to = pill_step(&self.lobby_rects(), self.sel, k);
                // B201: on a computer's seat ←→ change its character and L1 / R1 its costume (and so its level)
                if let Some(i) = bot.filter(|_| k.left || k.right || k.prev || k.next) {
                    let (dc, dk) = ((k.right as isize - k.left as isize), (k.next as isize - k.prev as isize));
                    match self.bot_pick(i, dc, dk) {
                        Some(p) => out.extend([Sound(0), Effect::Online(Fx::Net(Cmd::Bot(i, Some(p))))]),
                        None => out.push(Sound(1)),
                    }
                } else if to != self.sel {
                    self.sel = to;
                    self.net.status.clear();
                    out.push(Sound(0));
                }
                let item = items.get(self.sel).copied().unwrap_or(Item::Button("Leave"));
                if k.back || (k.ok && item == Item::Button("Leave")) {
                    out.push(Effect::Online(Fx::Net(Cmd::Leave)));
                    self.back_to_online(out);
                } else if !k.ok {
                } else if let Item::Seat(i) = item {
                    if let Some((peer, name)) = self.guest(i) {
                        self.net.status = format!("Taking {name} out...");
                        out.extend([Sound(1), Effect::Online(Fx::Net(Cmd::Kick(peer)))]);
                    } else if bot.is_some() {
                        out.extend([Sound(1), Effect::Online(Fx::Net(Cmd::Bot(i, None)))]);
                    } else if let Some(p) = self.bot_pick(i, 0, 0) {
                        out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::Bot(i, Some(p))))]);
                    }
                } else if item == Item::Mode {
                    let r = self.net.room.as_ref();
                    // singles only while the taken seats fit in two
                    if r.is_some_and(|r| r.doubles && r.seats.iter().flatten().count() > 2) {
                        self.net.status = "Singles has two seats: free some first.".into();
                        out.push(Sound(1));
                    } else if let Some(r) = r {
                        out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::Doubles(!r.doubles)))]);
                    }
                } else if item == Item::Button("Copy Link") && shared(&self.net).is_some() {
                    out.extend([Sound(2), Effect::Online(Fx::Copy)]);
                } else if item == Item::Button("Start") {
                    match self.net.room.as_ref().map(waiting_for) {
                        Some(None) => {
                            self.net.status = "Starting...".into();
                            out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::Start))]);
                        }
                        Some(Some(who)) => {
                            self.net.status = format!("Waiting for {who} to get ready.");
                            out.push(Sound(1));
                        }
                        None => out.push(Sound(1)),
                    }
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
                } else if item == Item::Button("Court") {
                    self.net.status.clear();
                    self.go(Screen::Confirm, out);
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
        (self.net.me, self.net.sent) = (String::new(), None);
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
            Event::Status(s) => v.status = s,
            // the lobby's gone (refused, taken out, the host left, or it couldn't open): only Leave is left
            Event::Failed(s) => {
                v.status = s;
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
                v.status = format!("{} open lobb{}.", l.len(), if l.len() == 1 { "y" } else { "ies" });
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
            Event::Match(_) | Event::Dropped(_) | Event::Datagram(..) | Event::Pack(..) => {}
        }
    }

    /// The lobby's mods, each to its looks: found here, fetched or a stand-in by the remembered answer, else asked
    /// (one at a time); once every one is settled, tell the host (the match waits for everyone).
    fn settle(&mut self, iso: &str, net: &Net) {
        let Some(room) = self.net.room.clone().filter(|_| !self.net.started) else { return };
        let cache = mo::cache(iso);
        for (_, w) in room.mods() {
            let h = w.looks;
            if self.net.looks.contains_key(&h) || self.net.wanted.contains(&h) || self.net.ask == Some(h) {
                continue;
            }
            if let Some(d) = mo::find(w, &self.mods, &cache) {
                self.net.looks.insert(h, Some(d));
                continue;
            }
            match Ask::load(&cache) {
                Some(Ask::Always) => {
                    self.net.wanted.insert(h);
                    net.send(Cmd::Want(h));
                }
                Some(Ask::Never) => drop(self.net.looks.insert(h, None)),
                _ if self.net.ask.is_none() => (self.net.ask, self.sel) = (Some(h), 0),
                _ => {}
            }
        }
        if self.net.ready != Some(room.ver) && room.mods().all(|(_, w)| self.net.looks.contains_key(&w.looks)) {
            self.net.ready = Some(room.ver);
            net.send(Cmd::Ready(room.ver));
        }
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

/// The first guest in `r` not ready yet, by name (None: everyone is).
fn waiting_for(r: &Room) -> Option<String> {
    let s = r.seats.iter().flatten().find(|s| s.peer != "host" && s.human() && !r.ready.contains(&s.peer))?;
    Some(name_of(r, &s.peer))
}

fn court_name(text: &super::Text, c: usize) -> String {
    format!("{} {}", text.msg(276 + 2 * c), text.msg(277 + 2 * c))
}

pub(super) fn layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    let text = d.text;
    match m.screen {
        Screen::Online => {
            d.frame(0, 5);
            let relay = relay_name(m.net.cfg.as_ref().unwrap_or(&Online::default()));
            let labels: Vec<String> = ROWS.iter().map(|r| if *r == "Relay" { format!("Relay: {relay}") } else { r.to_string() }).collect();
            for (i, (l, [x, y, w, _])) in labels.iter().zip(ROW_RECTS).enumerate() {
                let on = i == m.sel;
                d.pill(x, y, w, if on { [128.0, 110.0, 40.0] } else { [70.0, 70.0, 80.0] });
                let k = ((w - 24.0) / d.width(l, 24.0)).min(1.0);
                d.centred(l, x + w / 2.0, y + 7.0 + 12.0 * (1.0 - k), 24.0 * k, if on { WHITE } else { [100.0; 3] });
            }
            let [x, y, ..] = ROW_RECTS[m.sel.min(6)];
            d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x - 60.0 + hand_x, y + 3.0, 64.0, 32.0], WHITE, 128.0);
            let info = match m.sel {
                _ if !m.net.status.is_empty() => m.net.status.clone(),
                1 => "Open a lobby for players over the internet or on this network; start when you like: open seats are played by the computer.".into(),
                2 => "List the open lobbies: this network's first (no internet needed), then the BitTorrent DHT's.".into(),
                0 => "Join the fullest open lobby that has room for your players, or host one and wait.".into(),
                3 => "Type the host's 4-character code, or paste their link or code.".into(),
                4 => format!("Your name on your lobby seats (now {}).", m.net.cfg.as_ref().map_or("Player", |c| c.name.as_str())),
                5 if cfg!(target_arch = "wasm32") => "Relay: n0's public relays or your own (online.txt) for the internet; Off for this network only (no internet needed): codes are scanned both ways.".into(),
                5 => "Relay for when a direct connection fails: n0's public relays, your own (online.txt), or none.".into(),
                _ => "Back to the main menu.".into(),
            };
            d.info(&info);
        }
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
            for (i, c) in keys.iter().enumerate() {
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
            for (i, l) in buttons.iter().enumerate() {
                let (x, y, on) = (320.0 - 232.0 + 116.0 * i as f32 + 2.0, top + 44.0 * rows as f32 + 8.0, m.sel == keys.len() + i);
                let dim = i == 2 && if m.net.naming { m.net.entry.trim().is_empty() } else { !is_short(&m.net.entry) };
                d.pill(x, y, 112.0, if on { ON } else { OFF });
                d.hit([x, y, 112.0, 38.0], Act::Row(keys.len() + i, OK));
                d.centred(l, x + 56.0, y + 7.0, 24.0, if on && !dim { WHITE } else if dim { [60.0; 3] } else { [100.0; 3] });
            }
            let info = match () {
                _ if !m.net.status.is_empty() => m.net.status.clone(),
                _ if m.net.naming => "Type your name, or pick each letter; Save keeps it.".into(),
                _ => "Type the host's code (or pick each symbol), or Paste their link or code.".into(),
            };
            d.info(&info);
        }
        Screen::Lobby => {
            d.frame(4, 8);
            let room = m.net.room.as_ref();
            let title = room.map_or("Lobby".to_string(), |r| format!("{} - {}", r.name, if r.doubles { "Doubles" } else { "Singles" }));
            const DIM: [f32; 3] = [100.0; 3];
            const GOLD: [f32; 3] = [128.0, 110.0, 40.0];
            let items = m.items();
            // B201: the host's singles / doubles switch is the title
            if items.get(m.sel) == Some(&Item::Mode) && m.net.ask.is_none() {
                d.pill(32.0, 72.0, d.width(&title, 26.0) + 16.0, [70.0, 70.0, 80.0]);
            }
            if let Some(k) = items.iter().position(|it| *it == Item::Mode) {
                d.hit([32.0, 72.0, 300.0, 38.0], Act::Row(k, OK));
            }
            d.outlined(|d| d.text(&title, 40.0, 76.0, 26.0, [128.0, 110.0, 40.0]));
            if let Some(r) = room {
                d.outlined(|d| d.text(&court_name(text, r.court), 40.0, 106.0, 20.0, WHITE));
                // a seat a row: who (1P..), their character, their name, ready; the columns centred on the pill's middle
                let (x0, w) = (60.0, 340.0);
                for (i, s) in r.seats.iter().enumerate() {
                    let y = 140.0 + 44.0 * i as f32;
                    d.pill(x0, y, w, if s.is_some() { super::COLOURS[i] } else { [60.0; 3] });
                    if let Some(k) = items.iter().position(|it| *it == Item::Seat(i)) {
                        d.hit([x0, y, w, 38.0], Act::Row(k, OK));
                    }
                    d.centred(&format!("{}P", i + 1), x0 + 30.0, y + 9.0, 20.0, WHITE);
                    let Some(s) = s else {
                        d.centred("open", x0 + 110.0, y + 9.0, 20.0, DIM);
                        d.centred("COM", x0 + 292.0, y + 10.0, 18.0, DIM);
                        continue;
                    };
                    let live = r.live_of(i).filter(|l| s.human() && (!l.ready || r.picking.contains(&s.peer)));
                    match &s.pick.r#mod {
                        // B201: still choosing: who they're on now, dimmed
                        _ if live.is_some_and(|l| l.r#mod.is_none()) => {
                            let c = live.unwrap().char;
                            d.q(super::NAMES, [23.0, 24.0 * c as f32, 105.0, 24.0], [x0 + 58.0, y + 7.0, 105.0, 24.0], DIM, 128.0);
                        }
                        // B195: still choosing, the seat held
                        _ if live.is_some() || r.picking.contains(&s.peer) => d.centred("Picking...", x0 + 110.0, y + 9.0, 20.0, DIM),
                        // the name from the select's name sheet (a mod's in text)
                        Some(w) => d.fitted(&w.name, x0 + 110.0, y + 9.0, 20.0, 100.0),
                        None => d.q(super::NAMES, [23.0, 24.0 * s.pick.char as f32, 105.0, 24.0], [x0 + 58.0, y + 7.0, 105.0, 24.0], WHITE, 128.0),
                    }
                    let level;
                    let (who, state, rgb) = if !s.human() {
                        // B201: the computer's level, from its character and costume as the select's rank pill
                        level = super::RANK_NAMES.get(super::com_rank(&text.ranks, s.pick.char, s.pick.costume, r.doubles)).copied().unwrap_or("COM");
                        (format!("COM #{}", s.pick.costume + 1), level, DIM)
                    } else if s.peer == "host" {
                        (name_of(r, "host"), "Host", GOLD)
                    } else if r.ready.contains(&s.peer) {
                        (name_of(r, &s.peer), "Ready", GOLD)
                    } else {
                        (name_of(r, &s.peer), "Waiting", DIM)
                    };
                    d.fitted(&who, x0 + 200.0, y + 9.0, 20.0, 96.0);
                    d.centred(state, x0 + 292.0, y + 10.0, 18.0, rgb);
                }
                // the host's hand on the guest seat it would take out
                if let (None, Some(Item::Seat(i))) = (m.net.ask, items.get(m.sel)) {
                    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x0 - 60.0 + hand_x, 140.0 + 44.0 * *i as f32 + 3.0, 64.0, 32.0], WHITE, 128.0);
                }
            }
            if m.net.ask.is_none() {
                // the buttons along the bottom
                let rects = m.lobby_rects();
                let buttons: Vec<(usize, &str)> = items.iter().enumerate().filter_map(|(i, it)| if let Item::Button(b) = it { Some((i, *b)) } else { None }).collect();
                let open = room.map_or(0, Room::open);
                for (i, b) in &buttons {
                    let ([x, y, ..], on) = (rects[*i], m.sel == *i);
                    let label = match *b {
                        "Start" if open > 0 => format!("Start +{open} COM"),
                        "Ready" if m.net.prepared => "Not Ready".to_string(),
                        b => b.to_string(),
                    };
                    let dim = (*b == "Start" && room.and_then(waiting_for).is_some()) || (*b == "Copy Link" && shared(&m.net).is_none());
                    d.pill(x, y, BUTTON_W, if on { GOLD } else { [70.0, 70.0, 80.0] });
                    d.hit([x, y, BUTTON_W, 38.0], Act::Row(*i, OK));
                    let k = ((BUTTON_W - 20.0) / d.width(&label, 20.0)).min(1.0);
                    d.centred(&label, x + BUTTON_W / 2.0, y + 9.0 + 10.0 * (1.0 - k), 20.0 * k, if on && !dim { WHITE } else if dim { [60.0; 3] } else { DIM });
                }
            }
            if let Some(w) = m.net.ask.and_then(|h| room?.mods().find(|(_, w)| w.looks == h).map(|(_, w)| w)) {
                // a mod this machine doesn't have: its looks, or its donor recoloured
                d.outlined(|d| d.text(&format!("{} is a mod you don't have.", w.name), 410.0, 120.0, 18.0, WHITE));
                // the gold pill is the cursor (a hand would sit on the seats); it has to be answered
                d.popup(None);
                for (i, r) in ASK_ROWS.iter().enumerate() {
                    let y = 156.0 + 44.0 * i as f32;
                    d.pill(420.0, y, 200.0, if m.sel == i { GOLD } else { [70.0, 70.0, 80.0] });
                    d.hit([420.0, y, 200.0, 38.0], Act::Row(i, OK));
                    let k = (176.0 / d.width(r.0, 22.0)).min(1.0);
                    d.centred(r.0, 520.0, y + 8.0 + 11.0 * (1.0 - k), 22.0 * k, if m.sel == i { WHITE } else { DIM });
                }
                d.info("Get its looks, or play it as its donor in odd colours.");
                return;
            }
            if shared(&m.net).is_some() {
                // the code as a QR beside the seats (the texture `QrImage`, made when the code arrives)
                d.q(Tex::Qr, [0.0, 0.0, 1.0, 1.0], [430.0, 100.0, 170.0, 170.0], WHITE, 128.0);
                // the short code, big enough to read out (the QR is its link)
                d.outlined(|d| d.centred(&m.net.short, 515.0, 272.0, 46.0, GOLD));
            } else if m.net.hosting {
                d.outlined(|d| d.centred("Getting the code...", 515.0, 176.0, 18.0, WHITE));
            }
            let info = match items.get(m.sel) {
                _ if !m.net.status.is_empty() => m.net.status.clone(),
                // B201: the host's open and computer seats, and the singles / doubles switch
                Some(Item::Seat(i)) if m.guest(*i).is_none() && room.is_some_and(|r| r.seats.get(*i).is_some_and(|s| s.is_some())) => {
                    "Left / right: its character, L1 / R1: its costume (its level). OK: open the seat again.".into()
                }
                Some(Item::Seat(i)) if m.guest(*i).is_none() => "OK: a computer player in this seat (you choose its character and level).".into(),
                Some(Item::Mode) => "OK: switch between singles and doubles (everyone readies again).".into(),
                Some(Item::Seat(i)) => format!("OK takes {} out of the lobby.", m.guest(*i).map_or(String::new(), |g| g.1)),
                Some(Item::Button("Start")) => "Start the match now: open seats are played by the computer.".into(),
                Some(Item::Button("Copy Link")) => "Copy the lobby's link, or read out the code: friends type it in Join by Code.".into(),
                Some(Item::Button("Characters")) => "Pick your characters again (not ready meanwhile); your seats stay yours.".into(),
                Some(Item::Button("Court")) => "Change the court, sets, games and umpire: everyone sees it, and Find lists the court.".into(),
                _ if m.net.pick_next => "Joining: your seats are held while you pick your characters.".into(),
                Some(Item::Button("Ready")) => "Tell the host you're ready: they start when everyone in is, open seats played by the computer.".into(),
                _ if room.is_some() && !m.net.hosting => "Leave the lobby. The host starts when everyone in is ready: open seats are played by the computer.".into(),
                _ if room.is_none() => "The lobby's closed.".into(),
                _ => "Leave the lobby.".into(),
            };
            d.info(&info);
        }
        _ => {}
    }
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
    mut commands: Commands,
) {
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
        if menu.screen == Screen::Lobby {
            menu.settle(&args.iso, &net);
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
fn pill_step(rects: &[[f32; 4]], from: usize, k: Press) -> usize {
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
pub(super) fn typing(mut menu: ResMut<Menu>, mut keys: ResMut<ButtonInput<KeyCode>>, mut typed: MessageReader<bevy::input::keyboard::KeyboardInput>, mut was: Local<bool>) {
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
    #[cfg(target_arch = "wasm32")]
    for c in crate::web::typed().chars() {
        if c == '\u{8}' {
            menu.net.entry.pop();
        } else {
            menu.type_char(c);
        }
    }
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
    m.net.naming = menu == "name";
    if m.net.naming {
        m.net.entry = "SUNNY DAY".into();
    }
    // B201: ACE-42 on Kaoru's second costume, live; as 2P (Sunny Day) on the shared select, the host ready beside them
    let r = m.net.room.as_mut().unwrap();
    r.live.insert(peers[1].into(), vec![Live { char: 4, costume: 1, ready: false, r#mod: None }]);
    if menu == "chars-online" {
        r.live.insert("host".into(), vec![Live { char: 6, costume: 0, ready: true, r#mod: None }]);
        r.ready.clear();
        (m.net.hosting, m.net.me, m.net.go) = (false, peers[0].into(), Some(Go::Code));
        m.align();
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
    m.net.list = (0..3).map(|i| Listing { id: format!("{i}"), name: format!("Lobby {i}"), doubles: i != 1, court: i, taken: 1 + i % 2, seats: if i != 1 { 4 } else { 2 }, time: 0 }).collect();
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
            if let Some((net, room)) = crate::play::netplay::LOBBY.lock().unwrap().take() {
                let mut m = kept;
                (m.screen, m.sel) = (super::Screen::Lobby, 0);
                (m.net.room, m.net.started, m.net.prepared, m.net.ready) = (Some(room), false, false, None);
                m.net.status = "Back in the lobby.".into();
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
        }
        Fx::Ask(a) => a.save(&mo::cache(iso)),
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
        // the host's Court: the confirm screen, its Start back to the lobby with the venue
        m.sel = m.items().iter().position(|i| *i == Item::Button("Court")).unwrap();
        m.step(dev, ok);
        assert_eq!(m.screen, Screen::Confirm);
        let fx = m.step(dev, ok);
        assert_eq!(m.screen, Screen::Lobby);
        assert!(fx.iter().any(|e| matches!(e, Effect::Online(Fx::Net(Cmd::Venue(..))))), "{fx:?}");
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
        // the host: seats 1, 2 (guests), 3 (open), then Start, Characters, Court / Copy Link, Leave under them, the title last
        assert_eq!(at(&m, 200.0, 140.0 + 44.0 + 10.0), Some(0));
        assert_eq!(at(&m, 200.0, 150.0), None, "the host's own seat");
        assert_eq!(at(&m, 70.0, BUTTON_Y + 10.0), Some(3), "Start");
        assert_eq!(at(&m, 60.0 + BUTTON_STEP + 50.0, BUTTON_Y + BUTTON_ROW + 10.0), Some(7), "Leave");
        assert_eq!(at(&m, 100.0, 80.0), Some(8), "singles / doubles");
        assert_eq!(at(&m, 60.0 + BUTTON_W + 2.0, BUTTON_Y + 10.0), Some(3), "between pills: the nearer");
        m.net.hosting = false;
        assert_eq!(at(&m, 70.0, BUTTON_Y + 10.0), Some(0), "a guest's Ready");
    }

    #[test]
    fn lobby_start_ready_kick_and_name() {
        let dev = super::super::Dev::Keys;
        let ok = Press { ok: true, ..default() };
        let net = |fx: &[Effect]| fx.iter().find_map(|e| if let Effect::Online(Fx::Net(c)) = e { Some(c.clone()) } else { None });
        // the host: a guest's seat first (OK takes them out), then Start, which waits for them to be ready
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        m.net.hosting = true;
        assert_eq!(m.items(), vec![Item::Seat(1), Item::Seat(2), Item::Seat(3), Item::Button("Start"), Item::Button("Characters"), Item::Button("Court"), Item::Button("Copy Link"), Item::Button("Leave"), Item::Mode]);
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Kick("k51qzi5uqu5d".into())));
        m.sel = 3;
        assert_eq!(net(&m.step(dev, ok)), None, "ACE-42 isn't ready");
        assert!(m.net.status.contains("ACE-42"), "{}", m.net.status);
        m.net.room.as_mut().unwrap().ready.insert("r8mxw2pa0c1e".into());
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Start));
        // a guest: Ready toggles; Leave leaves
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        m.net.hosting = false;
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Prepared(true)));
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Prepared(false)));
        // Characters: unready (it was), back to the select
        m.net.prepared = true;
        m.step(dev, Press { right: true, ..default() });
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
        r.live.insert("host".into(), vec![Live { char: 6, costume: 2, ready: true, r#mod: None }]);
        r.live.insert("r8mxw2pa0c1e".into(), vec![Live { char: 4, costume: 1, ready: false, r#mod: None }]);
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
        assert_eq!(m.lives(), vec![Live { char: 6, costume: 2, ready: false, r#mod: None }]);
        // the host: OK on the open seat adds a computer player nobody has, ←→ / L1 R1 change it, OK clears it
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        (m.net.me, m.sel) = ("host".into(), 2);
        let Some(Cmd::Bot(3, Some(p))) = net(&m.step(dev, ok)) else { panic!() };
        assert_eq!(p.char, super::super::GRID[0].0);
        m.net.room.as_mut().unwrap().seats[3] = Some(Seat { peer: crate::net::CPU.into(), pick: Pick::disc(6, 0) });
        let Some(Cmd::Bot(3, Some(p))) = net(&m.step(dev, Press { next: true, ..default() })) else { panic!() };
        assert_eq!((p.char, p.costume, m.sel), (6, 1, 2), "the host's costume 0 of the same character taken");
        let Some(Cmd::Bot(3, Some(p))) = net(&m.step(dev, Press { right: true, ..default() })) else { panic!() };
        assert_ne!(p.char, 6);
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Bot(3, None)));
        // singles only once the taken seats fit
        m.sel = m.items().iter().position(|it| *it == Item::Mode).unwrap();
        assert_eq!(net(&m.step(dev, ok)), None);
        assert!(m.net.status.starts_with("Singles"), "{}", m.net.status);
        m.net.room.as_mut().unwrap().seats[2] = None;
        m.net.room.as_mut().unwrap().seats[3] = None;
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
        // Quickplay; Host Find; Join by Code Name; Relay Back
        let mut m = Menu { screen: Screen::Online, sel: 0, net: View { cfg: Some(Online::default()), ..default() }, ..default() };
        walk(&mut m, &[(l, 0), (d, 1), (r, 2), (r, 2), (d, 4), (l, 3), (d, 5), (r, 6), (d, 6), (u, 4), (u, 2), (u, 0)]);
        let relay = m.net.cfg.clone();
        m.sel = 5;
        m.step(dev, r);
        assert_eq!((m.sel, &m.net.cfg), (6, &relay), "←→ move off Relay, they don't change it");
        // the host's lobby: seats 1–3 (items 0–2); Start Characters Court (3–5); Copy Link Leave (6, 7); the title (8)
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        (m.net.hosting, m.sel) = (true, 0);
        walk(&mut m, &[(d, 1), (d, 2), (d, 3), (r, 4), (r, 5), (r, 5), (d, 7), (l, 6), (u, 3), (u, 2), (u, 1), (u, 0), (u, 8), (u, 8)]);
        // a guest: Ready Characters Leave in a row
        m.net.hosting = false;
        m.sel = 0;
        walk(&mut m, &[(r, 1), (r, 2), (d, 2), (l, 1)]);
    }

    /// B195: Find lists every open lobby before any pick; choosing one takes this machine's controllers (no more
    /// than its open seats), joins on placeholder picks, and the seat opens the character select.
    #[test]
    fn find_first_then_pick_in_the_lobby() {
        let dev = super::super::Dev::Keys;
        let ok = Press { ok: true, ..default() };
        let mut m = Menu { screen: Screen::Online, sel: 2, seats: [Some(dev), None, None, None], net: View { cfg: Some(Online::default()), ..default() }, ..default() };
        let fx = m.step(dev, ok);
        assert_eq!(m.screen, Screen::Lobbies);
        assert!(fx.contains(&Effect::Online(Fx::Net(Cmd::Find))), "{fx:?}");
        // a singles and a doubles lobby side by side
        let l = |id: &str, doubles, taken| Listing { id: id.into(), name: id.into(), doubles, court: 1, taken, seats: if doubles { 4 } else { 2 }, time: 0 };
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
}
