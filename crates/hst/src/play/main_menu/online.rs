//! The Online screens (M2f, `net.rs`): Host, Find, Quickplay, Join by Code and the relay setting, then the same
//! singles/doubles, controller and character screens as Local for this machine's 1–3 players (no computer
//! players: the open seats are for the other peers), then the lobby: its seats, the code and QR to share, and the
//! list of open lobbies for Find. The host's confirm screen (court, sets, games, umpire) opens the lobby.

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::{Draw, Effect, Menu, Press, Screen, Tex, WHITE};
use crate::mods_online::{self as mo, Ask, Wire};
use crate::net::{Cmd, Event, Listing, Net, Online, Pick, Relay, Room, SHORT, SYMBOLS, Seat, is_short};

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
const ROWS: [&str; 6] = ["Host", "Find", "Quickplay", "Join by Code", "Relay", "Back"];
/// The prompt for a mod this machine doesn't have: fetch its looks or not, and whether to ask again.
const ASK_ROWS: [(&str, bool, Ask); 4] =
    [("Download, always", true, Ask::Always), ("Download this one", true, Ask::Each), ("Stand-in this time", false, Ask::Each), ("Never download", false, Ask::Never)];

impl Menu {
    pub(super) fn online(&self) -> bool {
        self.net.go.is_some()
    }

    /// The most players this machine may bring online.
    pub(super) fn local_max(&self) -> usize {
        if self.doubles { 3 } else { 1 }
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
        Room { name: cfg.name, doubles: self.doubles, court, sets: self.sets, games: self.games, umpire, seats, ..Default::default() }
    }

    /// Every one of this machine's players ready on the select: on to the confirm screen (host) or the lobby.
    pub(super) fn online_ready(&mut self, out: &mut Vec<Effect>) {
        let go = self.net.go.unwrap();
        self.net.status.clear();
        self.net.room = None;
        self.net.started = false;
        (self.net.ask, self.net.ready) = (None, None);
        self.net.wanted.clear();
        match go {
            Go::Host => self.go(Screen::Confirm, out),
            Go::Find => {
                self.net.list.clear();
                self.go(Screen::Lobbies, out);
                out.push(Effect::Online(Fx::Net(Cmd::Find)));
            }
            Go::Quick => {
                self.go(Screen::Lobby, out);
                out.push(Effect::Online(Fx::Net(Cmd::Quick(self.room()))));
            }
            Go::Code => {
                self.go(Screen::Lobby, out);
                out.push(Effect::Online(Fx::Net(Cmd::Join(self.net.paste.clone(), self.picks()))));
            }
        }
    }

    /// The host's confirm screen's Start: open the lobby.
    pub(super) fn online_host(&mut self, out: &mut Vec<Effect>) {
        self.go(Screen::Lobby, out);
        out.push(Effect::Online(Fx::Net(Cmd::Host(self.room()))));
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
                if self.sel == 4 && (k.ok || k.left || k.right) {
                    cfg.next_relay();
                    out.extend([Sound(0), Effect::Online(Fx::SaveCfg), Effect::Online(Fx::Off)]);
                } else if k.ok && self.sel == 3 {
                    self.net.entry.clear();
                    self.net.status.clear();
                    self.go(Screen::Code, out);
                } else if k.ok && self.sel < 3 {
                    self.net.go = Some([Go::Host, Go::Find, Go::Quick][self.sel]);
                    self.go(Screen::Mode, out);
                } else if (k.ok && self.sel == 5) || k.back {
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
                    let id = self.net.list[self.sel].id.clone();
                    self.go(Screen::Lobby, out);
                    out.push(Effect::Online(Fx::Net(Cmd::Join(id, self.picks()))));
                } else if k.ok && self.sel == n {
                    self.net.list.clear();
                    out.extend([Sound(2), Effect::Online(Fx::Net(Cmd::Find))]);
                } else if k.ok || k.back {
                    self.back_to_online(out);
                }
            }
            Screen::Code => {
                // 8 columns of symbols, 4 rows, then Delete, Paste, Join, Back (two columns each)
                let s = self.sel;
                if k.left || k.right {
                    self.sel = if s < KEYS { s / 8 * 8 + (s % 8 + if k.right { 1 } else { 7 }) % 8 } else { KEYS + (s - KEYS + if k.right { 1 } else { 3 }) % 4 };
                    out.push(Sound(0));
                }
                if k.up || k.down {
                    let (row, col) = if s < KEYS { (s / 8, s % 8) } else { (4, (s - KEYS) * 2) };
                    let row = (row + if k.down { 1 } else { 4 }) % 5;
                    self.sel = if row == 4 { KEYS + col / 2 } else { row * 8 + col };
                    out.push(Sound(0));
                }
                if k.ok {
                    match self.sel {
                        s if s < KEYS => {
                            self.type_char(SYMBOLS[s] as char);
                            out.push(Sound(0));
                        }
                        32 => out.push(Sound(if self.net.entry.pop().is_some() { 1 } else { 0 })),
                        33 => out.push(Effect::Online(Fx::Paste)),
                        34 if is_short(&self.net.entry) => {
                            self.net.paste = self.net.entry.clone();
                            self.net.go = Some(Go::Code);
                            self.go(Screen::Mode, out);
                        }
                        34 => out.push(Sound(1)),
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
            Screen::Lobby => {
                if k.ok && !self.net.code.is_empty() {
                    out.extend([Sound(2), Effect::Online(Fx::Copy)]);
                } else if k.back {
                    out.push(Effect::Online(Fx::Net(Cmd::Leave)));
                    self.back_to_online(out);
                }
            }
            _ => {}
        }
    }

    fn code_back(&mut self, out: &mut Vec<Effect>) {
        (self.screen, self.sel) = (Screen::Online, 3);
        self.net.status.clear();
        out.push(Effect::Sound(1));
    }

    /// A symbol typed into Join by Code (any case; others ignored); the code full: the cursor to Join.
    pub(super) fn type_char(&mut self, c: char) {
        let c = c.to_ascii_uppercase();
        if self.net.entry.len() < SHORT && SYMBOLS.contains(&(c as u8)) {
            self.net.entry.push(c);
            if self.net.entry.len() == SHORT {
                self.sel = KEYS + 2;
            }
        }
    }

    pub(super) fn back_to_online(&mut self, out: &mut Vec<Effect>) {
        self.sel = [Go::Host, Go::Find, Go::Quick, Go::Code].iter().position(|g| Some(*g) == self.net.go).unwrap_or(0);
        self.net.go = None;
        self.net.code.clear();
        self.net.short.clear();
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
            Event::Failed(s) => v.status = s,
            Event::Code(c) => v.code = c,
            Event::Short(c) => v.short = c,
            Event::Found(l) => {
                v.status = format!("{} open lobb{}.", l.len(), if l.len() == 1 { "y" } else { "ies" });
                v.list = l;
            }
            Event::Room(r) => v.room = Some(r),
            Event::Start(r, _) => {
                v.room = Some(r);
                v.started = true;
                v.status = "Everyone's in.".into();
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
                0 => "Open a lobby for players over the internet or on this network; the match starts when every seat is taken.".into(),
                1 => "List the open lobbies: this network's first (no internet needed), then the BitTorrent DHT's.".into(),
                2 => "Join the fullest open lobby that has room for your players, or host one and wait.".into(),
                3 => "Type the host's 4-character code, or paste their link or code.".into(),
                4 if cfg!(target_arch = "wasm32") => "Relay: n0's public relays or your own (online.txt) for the internet; Off for this network only (no internet needed): codes are scanned both ways.".into(),
                4 => "Relay for when a direct connection fails: n0's public relays, your own (online.txt), or none.".into(),
                _ => "Back to the main menu.".into(),
            };
            d.info(&info);
        }
        Screen::Lobbies => {
            d.frame(0, 5);
            let mut labels: Vec<String> =
                m.net.list.iter().map(|l| format!("{}  {}  {}/{}", l.name, if l.doubles { "Doubles" } else { "Singles" }, l.taken, l.seats)).collect();
            labels.extend(["Refresh".into(), "Back".into()]);
            // ponytail: one page of 7 rows; scroll when lobbies get that many
            let first = m.sel.saturating_sub(6);
            let shown: Vec<String> = labels.iter().skip(first).take(7).cloned().collect();
            d.rows(&shown, m.sel - first, 110.0, 76.0, 420.0, hand_x);
            let info = match m.net.list.get(m.sel) {
                Some(l) => format!("{}: {}, {} open.", l.name, court_name(text, l.court), l.open()),
                None if m.net.status.is_empty() => "Looking for lobbies...".into(),
                None => m.net.status.clone(),
            };
            d.info(&info);
        }
        Screen::Code => {
            d.frame(0, 5);
            const ON: [f32; 3] = [128.0, 110.0, 40.0];
            const OFF: [f32; 3] = [70.0, 70.0, 80.0];
            // the code so far, a pill a symbol
            for i in 0..SHORT {
                let x = 320.0 - 128.0 + 64.0 * i as f32 + 4.0;
                d.pill(x, 76.0, 56.0, if i == m.net.entry.len() { ON } else { [40.0, 40.0, 50.0] });
                if let Some(c) = m.net.entry.get(i..i + 1) {
                    d.centred(c, x + 28.0, 80.0, 30.0, WHITE);
                }
            }
            for (i, c) in SYMBOLS.iter().enumerate() {
                let (x, y) = (320.0 - 232.0 + 58.0 * (i % 8) as f32 + 3.0, 132.0 + 44.0 * (i / 8) as f32);
                d.pill(x, y, 52.0, if m.sel == i { ON } else { OFF });
                d.centred(&(*c as char).to_string(), x + 26.0, y + 7.0, 24.0, if m.sel == i { WHITE } else { [100.0; 3] });
            }
            for (i, l) in ["Delete", "Paste", "Join", "Back"].iter().enumerate() {
                let (x, y, on) = (320.0 - 232.0 + 116.0 * i as f32 + 2.0, 132.0 + 44.0 * 4.0 + 8.0, m.sel == KEYS + i);
                let dim = i == 2 && !is_short(&m.net.entry);
                d.pill(x, y, 112.0, if on { ON } else { OFF });
                d.centred(l, x + 56.0, y + 7.0, 24.0, if on && !dim { WHITE } else if dim { [60.0; 3] } else { [100.0; 3] });
            }
            let info = if m.net.status.is_empty() { "Type the host's code (or pick each symbol), or Paste their link or code.".to_string() } else { m.net.status.clone() };
            d.info(&info);
        }
        Screen::Lobby => {
            d.frame(4, 8);
            let room = m.net.room.as_ref();
            let title = room.map_or("Lobby".to_string(), |r| format!("{} - {}", r.name, if r.doubles { "Doubles" } else { "Singles" }));
            d.text(&title, 40.0, 76.0, 26.0, [128.0, 110.0, 40.0]);
            if let Some(r) = room {
                d.text(&court_name(text, r.court), 40.0, 106.0, 20.0, WHITE);
                for (i, s) in r.seats.iter().enumerate() {
                    let y = 140.0 + 48.0 * i as f32;
                    d.pill(40.0, y, 320.0, if s.is_some() { super::COLOURS[i] } else { [60.0; 3] });
                    d.text(&format!("{}P", i + 1), 56.0, y + 8.0, 20.0, WHITE);
                    match s {
                        // the name from the select's name sheet, and whose machine (the host, or the peer's ID)
                        Some(s) if let Some(w) = &s.pick.r#mod => {
                            d.text(&w.name, 100.0, y + 8.0, 20.0, WHITE);
                            let whose = if s.peer == "host" { "host".to_string() } else { s.peer.chars().take(6).collect() };
                            d.text(&whose, 240.0, y + 8.0, 18.0, WHITE);
                        }
                        Some(s) => {
                            d.q(super::NAMES, [23.0, 24.0 * s.pick.char as f32, 105.0, 24.0], [100.0, y + 4.0, 105.0, 24.0], WHITE, 128.0);
                            let whose = if s.peer == "host" { "host".to_string() } else { s.peer.chars().take(6).collect() };
                            d.text(&whose, 240.0, y + 8.0, 18.0, WHITE);
                        }
                        None => d.text("open", 100.0, y + 8.0, 20.0, WHITE),
                    }
                }
            }
            if let Some(w) = m.net.ask.and_then(|h| room?.mods().find(|(_, w)| w.looks == h).map(|(_, w)| w)) {
                // a mod this machine doesn't have: its looks, or its donor recoloured
                d.text(&format!("{} is a mod you don't have.", w.name), 380.0, 120.0, 18.0, WHITE);
                let labels: Vec<String> = ASK_ROWS.iter().map(|r| r.0.to_string()).collect();
                d.rows(&labels, m.sel, 400.0, 156.0, 220.0, hand_x);
                d.info("Get its looks, or play it as its donor in odd colours.");
                return;
            }
            if !m.net.code.is_empty() {
                // the code as a QR beside the seats (the texture `QrImage`, made when the code arrives)
                d.q(Tex::Qr, [0.0, 0.0, 1.0, 1.0], [400.0, 120.0, 200.0, 200.0], WHITE, 128.0);
                if m.net.short.is_empty() {
                    d.centred("Scan it, or OK copies the code", 500.0, 326.0, 18.0, WHITE);
                } else {
                    // the short code, big enough to read out (the QR is its link)
                    d.centred(&m.net.short, 500.0, 322.0, 46.0, [128.0, 110.0, 40.0]);
                    d.centred("OK: Copy Link", 500.0, 372.0, 18.0, WHITE);
                }
            }
            let info = if m.net.status.is_empty() { "Waiting for players...".to_string() } else { m.net.status.clone() };
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

/// Started from a lobby link (the browser's `?code=`, or `HST_CODE`): straight to this machine's character pick
/// for that lobby, as a singles pick (one player; the first device to press anything is it).
pub(super) fn linked(m: &mut Menu) {
    #[cfg(target_arch = "wasm32")]
    let text = crate::web::pasted();
    #[cfg(not(target_arch = "wasm32"))]
    let text = std::env::var("HST_CODE").ok();
    let Some(code) = text.as_deref().and_then(crate::net::code_in) else { return };
    let net = View { paste: code, go: Some(Go::Code), ..default() };
    *m = Menu { screen: Screen::Chars, net, settings: m.settings.clone(), mods: std::mem::take(&mut m.mods), ..default() };
}

/// Join by Code's keyboard: typed symbols and Backspace go into the code, not to the menu's key bindings (the
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
    let seats = vec![Some(Seat { peer: "host".into(), pick: pick(6) }), Some(Seat { peer: "k51qzi5uqu5d".into(), pick: pick(1) }), None, None];
    m.net.room = Some(Room { name: "Player".into(), doubles: true, court: 0, sets: 1, games: 4, umpire: 4, seats, ..Default::default() });
    if std::env::var("HST_MENU").as_deref() == Ok("lobby-ask") {
        // 2P brought a mod this machine doesn't have: the prompt
        let w = Wire { id: "x".into(), name: "Emi".into(), donor: 1, hand: 1.0, base: 1, overrides: vec![], ai_row: 1, names: vec![], parent: vec![], rest: vec![], looks: 7, dir: None };
        m.net.room.as_mut().unwrap().seats[1].as_mut().unwrap().pick.r#mod = Some(Box::new(w));
        m.net.ask = Some(7);
    }
    (m.net.short, m.net.entry) = ("K7QX".into(), "K7".into());
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
    let base = args.clone();
    commands.queue(move |w: &mut World| {
        let Some(net) = w.remove_resource::<Net>() else { return };
        let back = crate::mode::Mode::new(move |app| {
            app.insert_resource(base);
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
            Some(n) => n.send(c),
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
                if let Err(e) = std::fs::write(&path, cfg.text()) {
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
        // Online → Host → singles/doubles → controllers → select → confirm → lobby
        m.step(dev, ok);
        assert_eq!(m.screen, Screen::Online);
        m.step(dev, ok);
        assert_eq!((m.screen, m.net.go), (Screen::Mode, Some(Go::Host)));
        m.step(dev, Press { down: true, ..default() });
        m.step(dev, ok);
        assert_eq!(m.screen, Screen::Assign);
        m.step(dev, ok);
        assert_eq!(m.screen, Screen::Chars);
        let fx = m.step(dev, ok);
        assert_eq!(m.screen, Screen::Confirm, "{fx:?}: one local player ready is everyone here");
        let fx = m.step(dev, ok);
        assert_eq!(m.screen, Screen::Lobby);
        let Some(Effect::Online(Fx::Net(Cmd::Host(r)))) = fx.into_iter().find(|e| matches!(e, Effect::Online(_))) else { panic!() };
        assert_eq!((r.doubles, r.open()), (true, 3));
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
}
