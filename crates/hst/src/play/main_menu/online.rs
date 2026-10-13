//! The Online screens (M2f, `net.rs`): Host, Find, Quickplay, Join by Code and the relay setting. Find lists every open
//! lobby first (singles and doubles together); Host, Quickplay and Join by Code ask singles/doubles. Then this
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
use crate::net::{Cmd, Event, Listing, Net, Online, Pick, RANDOM, Relay, Room, SHORT, SYMBOLS, Seat, Topic, is_short};

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
    /// B201a: the host is choosing what to put to a vote; this machine's ballot in the open vote.
    pub topics: bool,
    pub ballot: Option<u32>,
    /// Find: the chosen lobby's open seats, the most players this machine may bring (0: no lobby chosen).
    pub fit: usize,
    /// Joining or opening a lobby: its first line-up opens the character select (B195).
    pub pick_next: bool,
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
/// The lobby's buttons: the host's, a seated guest's (Ready shows "Not Ready" once ready), and with no lobby.
const HOST_BUTTONS: [&str; 6] = ["Start", "Vote", "Characters", "Court", "Copy Link", "Leave"];
/// What the host can put to a vote (B201a), and for how long.
const TOPICS: [(&str, Option<Topic>); 4] = [("Court", Some(Topic::Court)), ("Umpire", Some(Topic::Umpire)), ("Singles / Doubles", Some(Topic::Mode)), ("Cancel", None)];
const VOTE_SECS: u32 = 30;
const GUEST_BUTTONS: [&str; 3] = ["Ready", "Characters", "Leave"];
/// The lobby's buttons along the bottom: their pitch and width (six of the host's fit 4:3).
const BUTTON_STEP: f32 = 88.0;
const BUTTON_W: f32 = 82.0;

/// What a lobby selection is: a guest's seat (the host can take them out) or a button.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
    Seat(usize),
    Button(&'static str),
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
            let guests = room.map_or(vec![], |r| (0..r.seats.len()).filter(|&i| r.seats[i].as_ref().is_some_and(|s| s.peer != "host" && s.human())).collect());
            guests.into_iter().map(Item::Seat).chain(HOST_BUTTONS.map(Item::Button)).collect()
        } else if room.is_some() && !self.net.started && !self.net.pick_next {
            GUEST_BUTTONS.map(Item::Button).to_vec()
        } else {
            vec![Item::Button("Leave")]
        }
    }

    /// A guest's seat's peer and name.
    fn guest(&self, seat: usize) -> Option<(String, String)> {
        let r = self.net.room.as_ref()?;
        let s = r.seats.get(seat)?.as_ref()?;
        Some((s.peer.clone(), name_of(r, &s.peer)))
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
                rows(ROWS.len(), &mut self.sel, out);
                let cfg = self.net.cfg.get_or_insert_with(Online::default);
                if self.sel == 5 && (k.ok || k.left || k.right) {
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
                            self.net.go = Some(Go::Code);
                            self.go(Screen::Mode, out);
                        }
                        Some(2) => out.push(Sound(1)),
                        _ => self.code_back(out),
                    }
                } else if k.back && self.net.entry.pop().is_some() {
                    out.push(Sound(1));
                } else if k.back {
                    self.code_back(out);
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
            Screen::Lobby if self.net.topics && !k.back => {
                rows(TOPICS.len(), &mut self.sel, out);
                if k.ok {
                    self.net.topics = false;
                    let topic = TOPICS[self.sel].1;
                    self.sel = 0;
                    match topic {
                        Some(t) => out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::OpenVote(t, vote_options(t), VOTE_SECS)))]),
                        None => out.push(Sound(1)),
                    }
                }
            }
            Screen::Lobby if let Some(v) = self.net.room.as_ref().and_then(|r| r.vote.as_ref()).filter(|_| !k.back) => {
                // the options, then the host's End Vote
                let n = v.options.len();
                rows(n + self.net.hosting as usize, &mut self.sel, out);
                if k.ok && self.sel < n {
                    let o = v.options[self.sel];
                    self.net.ballot = Some(o);
                    out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::Vote(o)))]);
                } else if k.ok {
                    out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::EndVote))]);
                }
            }
            Screen::Lobby => {
                let items = self.items();
                if k.up || k.down || k.left || k.right {
                    let n = items.len();
                    self.sel = if k.down || k.right { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
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
                    }
                } else if item == Item::Button("Vote") {
                    (self.net.topics, self.sel) = (true, 0);
                    out.push(Sound(2));
                } else if item == Item::Button("Copy Link") && !self.net.code.is_empty() {
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
        self.screen = Screen::Online;
        out.push(Effect::Sound(1));
    }

    /// The pasted text as Join by Code's code: on to the picks if it parses.
    pub(super) fn pasted(&mut self, text: Option<String>, out: &mut Vec<Effect>) {
        if let Some(code) = text.as_deref().and_then(crate::net::code_in) {
            self.net.paste = code;
            self.net.go = Some(Go::Code);
            self.go(Screen::Mode, out);
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
            Event::Hosting => v.hosting = true,
            Event::Code(c) => v.code = c,
            Event::Short(c) => v.short = c,
            Event::Found(l) => {
                v.status = format!("{} open lobb{}.", l.len(), if l.len() == 1 { "y" } else { "ies" });
                v.list = l;
            }
            Event::Room(r) => {
                // a vote opened or closed: the cursor to its first row
                if r.vote.is_some() != v.room.as_ref().is_some_and(|o| o.vote.is_some()) {
                    (self.sel, v.ballot) = (0, None);
                }
                v.room = Some(r);
                // seated (or the lobby open): on to its character select, the seats held meanwhile
                if v.pick_next && self.screen == Screen::Lobby {
                    v.pick_next = false;
                    (self.screen, self.sel) = (Screen::Chars, 0);
                }
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

/// How many peers can vote.
fn r_names(r: Option<&Room>) -> usize {
    r.map_or(0, |r| r.names.len())
}

/// The first guest in `r` not ready yet, by name (None: everyone is).
fn waiting_for(r: &Room) -> Option<String> {
    let s = r.seats.iter().flatten().find(|s| s.peer != "host" && s.human() && !r.ready.contains(&s.peer))?;
    Some(name_of(r, &s.peer))
}

/// What a vote on `t` offers, in the confirm screen's order (courts as its strip), random last.
fn vote_options(t: Topic) -> Vec<u32> {
    match t {
        Topic::Court => super::STRIP.iter().map(|&c| c as u32).chain([RANDOM]).collect(),
        Topic::Umpire => (0..super::UMPIRES as u32).chain([RANDOM]).collect(),
        Topic::Mode => vec![0, 1],
    }
}

/// A vote option's name.
fn option_name(text: &super::Text, t: Topic, o: u32) -> String {
    match (t, o) {
        (Topic::Court, RANDOM) => "Random Court".into(),
        (Topic::Court, c) => court_name(text, c as usize),
        (Topic::Umpire, RANDOM) => "Random Umpire".into(),
        (Topic::Umpire, u) => text.msg(669 + 2 * u as usize).trim_start_matches("Umpire ").to_string(),
        (Topic::Mode, m) => if m == 1 { "Doubles" } else { "Singles" }.into(),
    }
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
            d.rows(&labels, m.sel, 170.0, 84.0, 300.0, hand_x);
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
            d.outlined(|d| d.text(&title, 40.0, 76.0, 26.0, [128.0, 110.0, 40.0]));
            const DIM: [f32; 3] = [100.0; 3];
            const GOLD: [f32; 3] = [128.0, 110.0, 40.0];
            let items = m.items();
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
                    match &s.pick.r#mod {
                        // B195: still choosing, the seat held
                        _ if r.picking.contains(&s.peer) => d.centred("Picking...", x0 + 110.0, y + 9.0, 20.0, DIM),
                        // the name from the select's name sheet (a mod's in text)
                        Some(w) => d.fitted(&w.name, x0 + 110.0, y + 9.0, 20.0, 100.0),
                        None => d.q(super::NAMES, [23.0, 24.0 * s.pick.char as f32, 105.0, 24.0], [x0 + 58.0, y + 7.0, 105.0, 24.0], WHITE, 128.0),
                    }
                    let (who, state, rgb) = if !s.human() {
                        ("Computer".to_string(), "COM", DIM)
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
                if let (None, false, None, Some(Item::Seat(i))) = (m.net.ask, m.net.topics, r.vote.as_ref(), items.get(m.sel)) {
                    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [x0 - 60.0 + hand_x, 140.0 + 44.0 * *i as f32 + 3.0, 64.0, 32.0], WHITE, 128.0);
                }
            }
            if m.net.ask.is_none() && m.net.topics {
                // B221: a tap off the topics is their Cancel
                d.popup(Some(Act::Row(TOPICS.len() - 1, OK)));
                d.outlined(|d| d.text("Put to a vote:", 410.0, 120.0, 18.0, WHITE));
                for (i, (t, _)) in TOPICS.iter().enumerate() {
                    let y = 156.0 + 44.0 * i as f32;
                    d.pill(420.0, y, 200.0, if m.sel == i { GOLD } else { [70.0, 70.0, 80.0] });
                    d.hit([420.0, y, 200.0, 38.0], Act::Row(i, OK));
                    d.centred(t, 520.0, y + 8.0, 22.0, if m.sel == i { WHITE } else { DIM });
                }
                d.info("Everyone in the lobby votes; it closes when all have voted, after 30 s, or when you end it.");
                return;
            }
            if let (None, Some(v)) = (m.net.ask, room.and_then(|r| r.vote.as_ref())) {
                // the options with their ballots so far, live; the host's End Vote last
                let what = TOPICS.iter().find(|t| t.1 == Some(v.topic)).map_or("", |t| t.0);
                d.outlined(|d| d.text(&format!("Vote: {what}  {} s", v.left), 410.0, 82.0, 18.0, WHITE));
                let tally = v.tally();
                let rows = v.options.iter().map(|&o| Some(o)).chain(m.net.hosting.then_some(None));
                for (i, o) in rows.enumerate() {
                    let y = 104.0 + 18.0 * i as f32;
                    let on = m.sel == i;
                    let fill = if on { GOLD } else if o.is_some() && o == m.net.ballot { [60.0, 90.0, 60.0] } else { [70.0, 70.0, 80.0] };
                    d.pill_at([410.0, y, 220.0, 17.0], [[30.0; 3], WHITE, fill], 128.0);
                    d.hit([410.0, y, 220.0, 18.0], Act::Row(i, OK));
                    let rgb = if on { WHITE } else { DIM };
                    match o {
                        Some(o) => {
                            d.fitted(&option_name(text, v.topic, o), 505.0, y + 2.0, 13.0, 170.0);
                            d.centred(&tally[i].to_string(), 612.0, y + 2.0, 13.0, rgb);
                        }
                        None => d.centred("End Vote", 520.0, y + 2.0, 13.0, rgb),
                    }
                }
                let voted = v.ballots.len();
                d.info(&format!("Choose with OK (you can change it): {voted} of {} voted. Most votes wins; a tie is drawn.", r_names(room)));
                return;
            }
            if m.net.ask.is_none() {
                // the buttons along the bottom
                let buttons: Vec<(usize, &str)> = items.iter().enumerate().filter_map(|(i, it)| if let Item::Button(b) = it { Some((i, *b)) } else { None }).collect();
                let open = room.map_or(0, Room::open);
                for (k, (i, b)) in buttons.iter().enumerate() {
                    let (x, on) = (60.0 + BUTTON_STEP * k as f32, m.sel == *i);
                    let label = match *b {
                        "Start" if open > 0 => format!("Start +{open} COM"),
                        "Ready" if m.net.prepared => "Not Ready".to_string(),
                        b => b.to_string(),
                    };
                    let dim = (*b == "Start" && room.and_then(waiting_for).is_some()) || (*b == "Copy Link" && m.net.code.is_empty());
                    d.pill(x, 330.0, BUTTON_W, if on { GOLD } else { [70.0, 70.0, 80.0] });
                    d.hit([x, 330.0, BUTTON_W, 38.0], Act::Row(*i, OK));
                    let k = ((BUTTON_W - 20.0) / d.width(&label, 20.0)).min(1.0);
                    d.centred(&label, x + BUTTON_W / 2.0, 339.0 + 10.0 * (1.0 - k), 20.0 * k, if on && !dim { WHITE } else if dim { [60.0; 3] } else { DIM });
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
            if !m.net.code.is_empty() {
                // the code as a QR beside the seats (the texture `QrImage`, made when the code arrives)
                d.q(Tex::Qr, [0.0, 0.0, 1.0, 1.0], [430.0, 100.0, 170.0, 170.0], WHITE, 128.0);
                if m.net.short.is_empty() {
                    d.outlined(|d| d.centred("Getting a short code...", 515.0, 280.0, 18.0, WHITE));
                } else {
                    // the short code, big enough to read out (the QR is its link)
                    d.outlined(|d| d.centred(&m.net.short, 515.0, 272.0, 46.0, GOLD));
                }
            } else if m.net.hosting {
                d.outlined(|d| d.centred("Getting the code...", 515.0, 176.0, 18.0, WHITE));
            }
            let info = match items.get(m.sel) {
                _ if !m.net.status.is_empty() => m.net.status.clone(),
                Some(Item::Seat(i)) => format!("OK takes {} out of the lobby.", m.guest(*i).map_or(String::new(), |g| g.1)),
                Some(Item::Button("Start")) => "Start the match now: open seats are played by the computer.".into(),
                Some(Item::Button("Vote")) => "Let everyone vote on the court, the umpire, or singles or doubles.".into(),
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
    }
    let code = if menu.net.short.is_empty() { menu.net.code.clone() } else { crate::net::link(&menu.net.short) };
    let code = &code;
    if !menu.net.code.is_empty()
        && made.is_none_or(|m| m.1 != *code)
        && let Some(img) = qr(code)
    {
        commands.insert_resource(QrImage(images.add(img), code.clone()));
    }
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
    *m = Menu { screen: Screen::Assign, net, settings: m.settings.clone(), mods: std::mem::take(&mut m.mods), ..default() };
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
    // B201a: the court vote open, two ballots in (this machine's on the second row); the host's topic list
    if menu == "lobby-vote" {
        let ballots = [("host".to_string(), super::STRIP[1] as u32), (peers[0].to_string(), RANDOM)].into();
        m.net.room.as_mut().unwrap().vote = Some(crate::net::Vote { topic: Topic::Court, options: vote_options(Topic::Court), ballots, left: 21 });
        (m.net.ballot, m.sel) = (Some(super::STRIP[1] as u32), 1);
    }
    m.net.topics = menu == "lobby-topics";
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
        assert_eq!((m.screen, m.net.go), (Screen::Mode, Some(Go::Code)));
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
        assert_eq!((m.screen, m.net.go), (Screen::Mode, Some(Go::Code)));
        m.step(dev, ok);
        let fx = m.step(dev, ok);
        assert!(fx.contains(&Effect::Online(Fx::Net(Cmd::Join("K7QX".into(), m.picks())))), "{fx:?}");
        // Back deletes, then leaves
        let mut m = Menu { screen: Screen::Code, net: View { entry: "AB".into(), ..default() }, ..default() };
        m.step(dev, Press { back: true, ..default() });
        assert_eq!((m.screen, m.net.entry.as_str()), (Screen::Code, "A"));
        m.step(dev, Press { back: true, ..default() });
        m.step(dev, Press { back: true, ..default() });
        assert_eq!((m.screen, m.sel), (Screen::Online, 3));
        // a pasted link joins its code
        let mut out = Vec::new();
        m.pasted(Some(crate::net::link("K7QX")), &mut out);
        assert_eq!((m.screen, m.net.paste.as_str()), (Screen::Mode, "K7QX"));
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
        // the host: seats 1, 2 (guests), then Start, Copy Link, Leave along the bottom
        assert_eq!(at(&m, 200.0, 140.0 + 44.0 + 10.0), Some(0));
        assert_eq!(at(&m, 200.0, 150.0), None, "the host's own seat");
        assert_eq!(at(&m, 70.0, 340.0), Some(2), "Start");
        assert_eq!(at(&m, 60.0 + 5.0 * BUTTON_STEP + 50.0, 360.0), Some(7), "Leave");
        assert_eq!(at(&m, 60.0 + BUTTON_W + 4.0, 340.0), Some(3), "between pills: the nearer");
        m.net.hosting = false;
        assert_eq!(at(&m, 70.0, 340.0), Some(0), "a guest's Ready");
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
        assert_eq!(m.items(), vec![Item::Seat(1), Item::Seat(2), Item::Button("Start"), Item::Button("Vote"), Item::Button("Characters"), Item::Button("Court"), Item::Button("Copy Link"), Item::Button("Leave")]);
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Kick("k51qzi5uqu5d".into())));
        m.sel = 2;
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

    /// B201a: the host puts the court to a vote, everyone votes on the panel, the host can end it.
    #[test]
    fn lobby_vote() {
        let dev = super::super::Dev::Keys;
        let (ok, down) = (Press { ok: true, ..default() }, Press { down: true, ..default() });
        let net = |fx: &[Effect]| fx.iter().find_map(|e| if let Effect::Online(Fx::Net(c)) = e { Some(c.clone()) } else { None });
        let mut m = Menu { screen: Screen::Lobby, ..default() };
        sample(&mut m);
        m.net.hosting = true;
        m.sel = 3;
        assert_eq!(net(&m.step(dev, ok)), None);
        assert!(m.net.topics);
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::OpenVote(Topic::Court, vote_options(Topic::Court), VOTE_SECS)));
        let mut r = m.net.room.clone().unwrap();
        r.vote = Some(crate::net::Vote { topic: Topic::Court, options: vote_options(Topic::Court), ballots: Default::default(), left: 30 });
        m.sel = 5;
        m.heard(Event::Room(r.clone()));
        assert_eq!(m.sel, 0, "the cursor to the vote's first row");
        m.step(dev, down);
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::Vote(super::super::STRIP[1] as u32)));
        m.sel = 12;
        assert_eq!(net(&m.step(dev, ok)), Some(Cmd::EndVote));
        r.vote = None;
        m.heard(Event::Room(r));
        assert_eq!((m.sel, m.net.ballot), (0, None));
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
