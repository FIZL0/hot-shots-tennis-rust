//! Netplay transport and internet lobbies (M2f, `plan/M2.md` → *Hosting and finding games*).
//!
//! Peers are iroh endpoints (QUIC, encrypted, hole punching). No server of ours and none of n0's DNS:
//! - **Addresses** go on the mainline (BitTorrent) DHT as pkarr records (`iroh-mainline-address-lookup`), so a
//!   bare endpoint ID can be dialled.
//! - **Lobby list**: a host announces itself on the fixed infohash [`TOPIC`] (BEP 5, every DHT node supports it)
//!   with a random lobby number as the port, and puts its listing (endpoint ID, name, singles/doubles, court, seats,
//!   time) as a BEP 44 mutable item under the well-known [`BOARD`] key, salted with that number and signed again
//!   with its own endpoint key. Find = `get_peers(TOPIC)`, then each lobby number's listing; listings older than
//!   [`STALE`] are dropped. Republished every [`REPUBLISH`] and on change (the Halo CE port's 30 s / 90 s).
//! - **Relay** for when hole punching fails: n0's public relays, your own `iroh-relay`, or none (LAN only).
//! - **Code / QR**: the host's [`EndpointTicket`] (ID + relay + direct addresses), needs no discovery at all.
//!
//! The lobby protocol (ALPN [`ALPN`]): the joiner opens a stream and sends [`Msg::Hello`] (build, its 1–3 local
//! players' picks); the host answers [`Msg::Room`] (seated) or [`Msg::Refused`], sends `Room` again on every
//! change and [`Msg::Start`] once every seat is taken. Messages are JSON, a u32 length before each.
//!
//! The app talks to this through [`Net`]: commands in, [`Event`]s out; the network runs on its own tokio thread.
//! `hst net host|find|quick|join …` drives the same thing from a terminal (two machines, no menus).

// the browser build keeps the lobby types (menus) without the transport
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(not(target_arch = "wasm32"))]
use iroh::address_lookup::AddrFilter;
#[cfg(not(target_arch = "wasm32"))]
use iroh::endpoint::{Connection, RecvStream, SendStream, presets};
#[cfg(not(target_arch = "wasm32"))]
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMode, RelayUrl, SecretKey, Signature};
#[cfg(not(target_arch = "wasm32"))]
use iroh_mainline_address_lookup::DhtAddressLookup;
#[cfg(not(target_arch = "wasm32"))]
use iroh_tickets::endpoint::EndpointTicket;
#[cfg(not(target_arch = "wasm32"))]
use n0_future::StreamExt;
#[cfg(not(target_arch = "wasm32"))]
use n0_mainline::{Dht, Id, MutableItem, SigningKey};
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::mpsc as amp;

pub const ALPN: &[u8] = b"hst/lobby/1";
/// The lobby list's infohash (any 20 bytes; these spell it).
const TOPIC: &[u8; 20] = b"hst-remaster-lobby/1";
/// The listings' shared BEP 44 key (its secret is public on purpose: anyone may list; the endpoint signature inside
/// says who wrote a listing).
const BOARD: &[u8; 32] = b"hot-shots-tennis-remaster-lobby1";
const REPUBLISH: Duration = Duration::from_secs(30);
const STALE: u64 = 90;
/// The build peers must share (M2e will add the disc pack, rules and mods).
pub const BUILD: &str = concat!("hst ", env!("CARGO_PKG_VERSION"));

// ---------------------------------------------------------------- settings

/// Which relay to fall back on when hole punching fails.
#[derive(Clone, Debug, PartialEq)]
pub enum Relay {
    /// n0's free public relays.
    N0,
    /// Your own `iroh-relay` (README → *Online play*).
    Own(String),
    /// None: LAN only (and direct internet when both sides can hole punch).
    Off,
}

/// `online.txt` beside the disc image: `name = …` (the lobby name), `relay = n0 | own | off`, `relay_url = https://…`.
#[derive(Clone, Debug, PartialEq)]
pub struct Online {
    pub name: String,
    pub relay: Relay,
    pub relay_url: String,
}

impl Default for Online {
    fn default() -> Self {
        Online { name: "Tennis".into(), relay: Relay::N0, relay_url: String::new() }
    }
}

impl Online {
    pub fn path(iso: &str) -> std::path::PathBuf {
        std::path::Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()).join("online.txt")
    }

    pub fn load(iso: &str) -> Online {
        Online::parse(&std::fs::read_to_string(Online::path(iso)).unwrap_or_default())
    }

    pub fn parse(text: &str) -> Online {
        let mut o = Online::default();
        let mut relay = "n0".to_string();
        for (k, v) in text.lines().filter_map(|l| l.split('#').next()?.split_once('=')) {
            let v = v.trim().to_string();
            match k.trim() {
                "name" if !v.is_empty() => o.name = v.chars().take(24).collect(),
                "relay" => relay = v,
                "relay_url" => o.relay_url = v,
                _ => {}
            }
        }
        o.relay = match relay.as_str() {
            "off" => Relay::Off,
            "own" => Relay::Own(o.relay_url.clone()),
            _ => Relay::N0,
        };
        o
    }

    pub fn text(&self) -> String {
        let relay = match self.relay {
            Relay::N0 => "n0",
            Relay::Own(_) => "own",
            Relay::Off => "off",
        };
        format!("name = {}\nrelay = {relay}\nrelay_url = {}\n", self.name, self.relay_url)
    }

    /// The next relay choice (own only once a `relay_url` is set).
    pub fn next_relay(&mut self) {
        self.relay = match self.relay {
            Relay::N0 if !self.relay_url.is_empty() => Relay::Own(self.relay_url.clone()),
            Relay::N0 | Relay::Own(_) => Relay::Off,
            Relay::Off => Relay::N0,
        };
    }
}

// ---------------------------------------------------------------- lobby data

/// One local player's pick: disc character and costume.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pick {
    pub char: usize,
    pub costume: usize,
}

/// A taken seat: whose peer (endpoint ID) and their pick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Seat {
    pub peer: String,
    pub pick: Pick,
}

/// A lobby as the host keeps it: seats in menu order (1P..4P), `None` open.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub doubles: bool,
    pub court: usize,
    pub sets: usize,
    pub games: i32,
    pub umpire: u8,
    pub seats: Vec<Option<Seat>>,
}

impl Room {
    pub fn open(&self) -> usize {
        self.seats.iter().filter(|s| s.is_none()).count()
    }

    /// Seat `peer`'s picks in the first open seats; false (and no change) if they don't all fit.
    fn take(&mut self, peer: &str, picks: &[Pick]) -> bool {
        if picks.is_empty() || picks.len() > self.open() {
            return false;
        }
        let mut free = (0..self.seats.len()).filter(|&i| self.seats[i].is_none()).collect::<Vec<_>>().into_iter();
        for &pick in picks {
            self.seats[free.next().unwrap()] = Some(Seat { peer: peer.into(), pick });
        }
        true
    }

    fn leave(&mut self, peer: &str) {
        for s in &mut self.seats {
            if s.as_ref().is_some_and(|s| s.peer == peer) {
                *s = None;
            }
        }
    }
}

/// A lobby on the list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Listing {
    pub id: String,
    pub name: String,
    pub doubles: bool,
    pub court: usize,
    pub taken: usize,
    pub seats: usize,
    /// Unix seconds when the host wrote it.
    pub time: u64,
}

impl Listing {
    #[cfg(not(target_arch = "wasm32"))]
    fn of(id: EndpointId, room: &Room) -> Listing {
        let seats = room.seats.len();
        Listing { id: id.to_string(), name: room.name.clone(), doubles: room.doubles, court: room.court, taken: seats - room.open(), seats, time: now() }
    }

    pub fn open(&self) -> usize {
        self.seats - self.taken
    }
}

/// The Quickplay rule (the Halo CE port's Quick Match): the busiest open lobby that fits, newest first on a tie.
pub fn quick_pick(list: &[Listing], doubles: bool, need: usize) -> Vec<Listing> {
    let mut fit: Vec<_> = list.iter().filter(|l| l.doubles == doubles && l.open() >= need).cloned().collect();
    fit.sort_by_key(|l| (std::cmp::Reverse(l.taken), std::cmp::Reverse(l.time)));
    fit
}

#[cfg(not(target_arch = "wasm32"))]
fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(not(target_arch = "wasm32"))]
/// The salt of lobby number `n`.
fn salt(n: u16) -> Vec<u8> {
    format!("hst1/{n}").into_bytes()
}

#[cfg(not(target_arch = "wasm32"))]
/// A listing's DHT value: the endpoint's signature over salt + JSON, then the JSON.
fn seal(key: &SecretKey, n: u16, l: &Listing) -> Vec<u8> {
    let json = serde_json::to_vec(l).unwrap();
    let sig = key.sign(&[salt(n), json.clone()].concat());
    [sig.to_bytes().to_vec(), json].concat()
}

#[cfg(not(target_arch = "wasm32"))]
/// The listing in `value` if its endpoint signed it for lobby number `n`.
fn unseal(n: u16, value: &[u8]) -> Option<Listing> {
    let (sig, json) = value.split_at_checked(64)?;
    let l: Listing = serde_json::from_slice(json).ok()?;
    let id: EndpointId = l.id.parse().ok()?;
    id.verify(&[salt(n), json.to_vec()].concat(), &Signature::from_bytes(sig.try_into().ok()?)).ok()?;
    Some(l)
}

// ---------------------------------------------------------------- wire

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Msg {
    Hello { build: String, picks: Vec<Pick> },
    Room(Room),
    Refused(String),
    Start(Room),
}

#[cfg(not(target_arch = "wasm32"))]
async fn send(s: &mut SendStream, m: &Msg) -> Result<(), String> {
    let b = serde_json::to_vec(m).unwrap();
    s.write_all(&(b.len() as u32).to_le_bytes()).await.map_err(|e| e.to_string())?;
    s.write_all(&b).await.map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn recv(r: &mut RecvStream) -> Result<Msg, String> {
    let mut n = [0; 4];
    r.read_exact(&mut n).await.map_err(|e| e.to_string())?;
    let n = u32::from_le_bytes(n) as usize;
    if n > 64 << 10 {
        return Err("message too long".into());
    }
    let mut b = vec![0; n];
    r.read_exact(&mut b).await.map_err(|e| e.to_string())?;
    serde_json::from_slice(&b).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- the app's handle

/// What the app asks for.
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    /// Host `room` (the host's own picks already seated, peer "host").
    Host(Room),
    /// List the open lobbies.
    Find,
    /// Join the busiest open lobby that fits these picks, else host `room`.
    Quick(Room),
    /// Join by code (a ticket, or a bare endpoint ID looked up on the DHT) with these picks.
    Join(String, Vec<Pick>),
    /// Leave the lobby (or stop hosting).
    Leave,
}

/// What the network tells the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Status(String),
    /// Hosting: the code (ticket) to share.
    Code(String),
    Found(Vec<Listing>),
    /// The lobby as it is now (host and joiners alike).
    Room(Room),
    /// Every seat taken: the match's line-up. The connections stay open for the match (M2g).
    Start(Room),
    Failed(String),
}

/// The network thread's handle (a Bevy resource in the menu).
#[cfg(not(target_arch = "wasm32"))]
#[derive(bevy::prelude::Resource)]
pub struct Net {
    cmd: amp::UnboundedSender<Cmd>,
    events: std::sync::Mutex<mpsc::Receiver<Event>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Net {
    pub fn start(cfg: Online) -> Net {
        let (cmd, cmd_rx) = amp::unbounded_channel();
        let (ev, events) = mpsc::channel();
        std::thread::Builder::new()
            .name("net".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread().enable_all().worker_threads(2).build().unwrap();
                rt.block_on(run(cfg, cmd_rx, ev));
            })
            .unwrap();
        Net { cmd, events: events.into() }
    }

    pub fn send(&self, c: Cmd) {
        let _ = self.cmd.send(c);
    }

    pub fn poll(&self) -> Vec<Event> {
        self.events.lock().unwrap().try_iter().collect()
    }
}

// ---------------------------------------------------------------- the network thread

#[cfg(not(target_arch = "wasm32"))]
struct Ctx {
    ep: Endpoint,
    dht: Dht,
    ev: mpsc::Sender<Event>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Ctx {
    fn say(&self, e: Event) {
        let _ = self.ev.send(e);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn endpoint(relay: &Relay) -> Result<Endpoint, String> {
    let mode = match relay {
        Relay::N0 => RelayMode::Default,
        Relay::Own(url) => RelayMode::Custom(url.parse::<RelayUrl>().map_err(|e| format!("relay_url {url}: {e}"))?.into()),
        Relay::Off => RelayMode::Disabled,
    };
    // the DHT record carries the direct addresses too, so a peer with relays off can still be dialled by ID
    let lookup = DhtAddressLookup::builder().addr_filter(AddrFilter::unfiltered());
    let mut b = Endpoint::builder(presets::Minimal).secret_key(SecretKey::generate()).alpns(vec![ALPN.to_vec()]).relay_mode(mode).address_lookup(lookup);
    if std::env::var_os("HST_RELAY_ONLY").is_some() {
        // testing: no direct UDP paths, everything through the relay
        b = b.clear_ip_transports();
    }
    let ep = b.bind().await.map_err(|e| e.to_string())?;
    if *relay != Relay::Off {
        // a home relay before the ticket, so the code works from anywhere (bounded: a dead relay still leaves LAN)
        let _ = tokio::time::timeout(Duration::from_secs(10), ep.online()).await;
    }
    Ok(ep)
}

#[cfg(not(target_arch = "wasm32"))]
async fn run(cfg: Online, mut cmds: amp::UnboundedReceiver<Cmd>, ev: mpsc::Sender<Event>) {
    let _ = ev.send(Event::Status("Connecting...".into()));
    let ep = match endpoint(&cfg.relay).await {
        Ok(ep) => ep,
        Err(e) => {
            let _ = ev.send(Event::Failed(e));
            return;
        }
    };
    // ponytail: a second DHT node beside the address lookup's (its builder makes its own); one socket if it matters
    let dht = match Dht::client() {
        Ok(d) => d,
        Err(e) => {
            let _ = ev.send(Event::Failed(format!("DHT: {e}")));
            return;
        }
    };
    let cx = Ctx { ep, dht, ev };
    cx.say(Event::Status("Online.".into()));
    while let Some(c) = cmds.recv().await {
        match c {
            Cmd::Host(room) => host(&cx, room, &mut cmds).await,
            Cmd::Find => {
                cx.say(Event::Status("Looking for lobbies...".into()));
                let list = find(&cx.dht).await;
                cx.say(Event::Found(list));
            }
            Cmd::Quick(room) => {
                cx.say(Event::Status("Looking for a lobby...".into()));
                let mine: Vec<Pick> = room.seats.iter().flatten().map(|s| s.pick).collect();
                let mut joined = false;
                for l in quick_pick(&find(&cx.dht).await, room.doubles, mine.len()) {
                    if join(&cx, &l.id, mine.clone(), &mut cmds).await {
                        joined = true;
                        break;
                    }
                }
                if !joined {
                    host(&cx, room, &mut cmds).await;
                }
            }
            Cmd::Join(code, picks) => {
                join(&cx, &code, picks, &mut cmds).await;
            }
            Cmd::Leave => {}
        }
    }
    cx.ep.close().await;
}

#[cfg(not(target_arch = "wasm32"))]
/// The open lobbies on the DHT now.
pub async fn find(dht: &Dht) -> Vec<Listing> {
    let mut ports = std::collections::BTreeSet::new();
    if let Ok(mut s) = dht.get_peers(Id::from_bytes(TOPIC).unwrap()).await {
        // the query ends by itself once the closest nodes answered; a cap in case it doesn't
        let _ = tokio::time::timeout(Duration::from_secs(30), async {
            while let Some(batch) = s.next().await {
                ports.extend(batch.iter().map(|a| a.port()));
            }
        })
        .await;
    }
    let board = SigningKey::from_bytes(BOARD).verifying_key().to_bytes();
    let mut gets = tokio::task::JoinSet::new();
    for n in ports {
        let dht = dht.clone();
        gets.spawn(async move { unseal(n, dht.get_mutable_most_recent(&board, Some(&salt(n))).await.ok().flatten()?.value()) });
    }
    let mut list: Vec<Listing> = gets.join_all().await.into_iter().flatten().filter(|l| now().saturating_sub(l.time) <= STALE && l.open() > 0).collect();
    list.sort_by(|a, b| (&a.id, b.time).cmp(&(&b.id, a.time)));
    list.dedup_by(|a, b| a.id == b.id);
    list
}

#[cfg(not(target_arch = "wasm32"))]
/// Put `room`'s listing on the DHT under lobby number `n`.
async fn announce(cx: &Ctx, n: u16, room: &Room) {
    let l = Listing::of(cx.ep.id(), room);
    let value = seal(cx.ep.secret_key(), n, &l);
    let item = MutableItem::new(&SigningKey::from_bytes(BOARD), &value, (now() * 1_000_000) as i64, Some(&salt(n)));
    if let Err(e) = cx.dht.put_mutable(item, None).await {
        bevy::log::warn!("lobby listing: {e}");
    }
    if let Err(e) = cx.dht.announce_peer(Id::from_bytes(TOPIC).unwrap(), Some(n)).await {
        bevy::log::warn!("lobby announce: {e}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// A joiner's side, inside the host task.
enum Guest {
    In { peer: String, picks: Vec<Pick>, build: String, tx: SendStream, conn: Connection },
    Gone(String),
}

#[cfg(not(target_arch = "wasm32"))]
async fn host(cx: &Ctx, mut room: Room, cmds: &mut amp::UnboundedReceiver<Cmd>) {
    let code = EndpointTicket::new(cx.ep.addr()).to_string();
    cx.say(Event::Code(code));
    cx.say(Event::Room(room.clone()));
    let n = lobby_number(cx.ep.id());
    let (gtx, mut guests) = amp::unbounded_channel::<Guest>();
    let mut joined: Vec<(String, SendStream, Connection)> = Vec::new();
    let mut tick = tokio::time::interval(REPUBLISH);
    let mut dirty = true;
    loop {
        if dirty {
            dirty = false;
            // listing in the background: a slow DHT put never holds up a join
            let l = room.clone();
            let cx2 = Ctx { ep: cx.ep.clone(), dht: cx.dht.clone(), ev: cx.ev.clone() };
            tokio::spawn(async move { announce(&cx2, n, &l).await });
        }
        tokio::select! {
            Some(inc) = cx.ep.accept() => {
                let gtx = gtx.clone();
                tokio::spawn(async move {
                    let Ok(conn) = async { inc.accept()?.await.map_err(Into::into) }.await.map_err(|e: Box<dyn std::error::Error + Send + Sync>| e) else { return };
                    let peer = conn.remote_id().to_string();
                    let Ok((tx, mut rx)) = conn.accept_bi().await else { return };
                    if let Ok(Msg::Hello { build, picks }) = recv(&mut rx).await {
                        let _ = gtx.send(Guest::In { peer: peer.clone(), picks, build, tx, conn: conn.clone() });
                        // the stream ends when they leave or drop
                        let _ = recv(&mut rx).await;
                        let _ = gtx.send(Guest::Gone(peer));
                    }
                });
            }
            Some(g) = guests.recv() => match g {
                Guest::In { peer, picks, build, mut tx, conn } => {
                    let why = if build != BUILD {
                        Some(format!("different version (host {BUILD}, you {build})"))
                    } else if !room.take(&peer, &picks) {
                        Some(format!("not enough open seats ({} open, you bring {})", room.open(), picks.len()))
                    } else {
                        None
                    };
                    if let Some(why) = why {
                        let _ = send(&mut tx, &Msg::Refused(why)).await;
                        let _ = tx.finish();
                        // let the refusal arrive before the connection goes
                        tokio::spawn(async move { tokio::time::sleep(Duration::from_secs(2)).await; drop(conn) });
                        continue;
                    }
                    cx.say(Event::Status(format!("{} joined.", short(&peer))));
                    joined.push((peer, tx, conn));
                    broadcast(&mut joined, &Msg::Room(room.clone())).await;
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                    if room.open() == 0 {
                        broadcast(&mut joined, &Msg::Start(room.clone())).await;
                        cx.say(Event::Start(room.clone()));
                        // ponytail: the lobby ends here; M2g takes the connections into the match
                        wait_leave(cmds).await;
                        return;
                    }
                }
                Guest::Gone(peer) => {
                    room.leave(&peer);
                    joined.retain(|(p, ..)| *p != peer);
                    cx.say(Event::Status(format!("{} left.", short(&peer))));
                    broadcast(&mut joined, &Msg::Room(room.clone())).await;
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                }
            },
            _ = tick.tick() => dirty = true,
            c = cmds.recv() => if matches!(c, Some(Cmd::Leave) | None) {
                for (_, _, conn) in &joined {
                    conn.close(0u32.into(), b"host left");
                }
                // an empty listing ages out in STALE; a full one isn't shown
                return;
            },
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn broadcast(to: &mut [(String, SendStream, Connection)], m: &Msg) {
    for (_, tx, _) in to.iter_mut() {
        let _ = send(tx, m).await;
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn wait_leave(cmds: &mut amp::UnboundedReceiver<Cmd>) {
    while let Some(c) = cmds.recv().await {
        if matches!(c, Cmd::Leave) {
            return;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// This host's lobby number, from its (random) endpoint ID.
fn lobby_number(id: EndpointId) -> u16 {
    // ponytail: 16 bits, two hosts on one number overwrite each other's listing (birthday odds, ~1 % at 36 hosts at
    // once); an IP-bound salt or an address index server if lobbies get that busy
    u16::from_le_bytes([id.as_bytes()[0], id.as_bytes()[1]]).max(1)
}

#[cfg(not(target_arch = "wasm32"))]
fn short(id: &str) -> &str {
    &id[..id.len().min(8)]
}

#[cfg(not(target_arch = "wasm32"))]
/// Join by code; true once seated (the lobby then runs until it starts or someone leaves).
async fn join(cx: &Ctx, code: &str, picks: Vec<Pick>, cmds: &mut amp::UnboundedReceiver<Cmd>) -> bool {
    let code = code.trim();
    let addr: EndpointAddr = match code.parse::<EndpointTicket>() {
        Ok(t) => t.into(),
        Err(_) => match code.parse::<EndpointId>() {
            Ok(id) => id.into(),
            Err(_) => {
                cx.say(Event::Failed("That code isn't a lobby code.".into()));
                return false;
            }
        },
    };
    cx.say(Event::Status(format!("Joining {}...", short(&addr.id.to_string()))));
    let conn = match tokio::time::timeout(Duration::from_secs(30), cx.ep.connect(addr, ALPN)).await {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => {
            cx.say(Event::Failed(format!("Couldn't reach the host: {e}")));
            return false;
        }
        Err(_) => {
            cx.say(Event::Failed("Couldn't reach the host (timed out).".into()));
            return false;
        }
    };
    let Ok((mut tx, mut rx)) = conn.open_bi().await else {
        cx.say(Event::Failed("The host closed the connection.".into()));
        return false;
    };
    if send(&mut tx, &Msg::Hello { build: BUILD.into(), picks }).await.is_err() {
        cx.say(Event::Failed("The host closed the connection.".into()));
        return false;
    }
    let mut seated = false;
    loop {
        tokio::select! {
            m = recv(&mut rx) => match m {
                Ok(Msg::Room(r)) => {
                    seated = true;
                    cx.say(Event::Room(r));
                }
                Ok(Msg::Start(r)) => {
                    cx.say(Event::Start(r));
                    wait_leave(cmds).await;
                    return true;
                }
                Ok(Msg::Refused(why)) => {
                    cx.say(Event::Failed(format!("Refused: {why}")));
                    return false;
                }
                Ok(Msg::Hello { .. }) | Err(_) => {
                    cx.say(Event::Failed(if seated { "The host left." } else { "The host closed the connection." }.into()));
                    return seated;
                }
            },
            c = cmds.recv() => if matches!(c, Some(Cmd::Leave) | None) {
                conn.close(0u32.into(), b"left");
                return true;
            },
        }
    }
}

// ---------------------------------------------------------------- terminal

#[cfg(not(target_arch = "wasm32"))]
/// `hst net host [doubles] [players] | find | quick [doubles] [players] | join <code> [players]`, with `online.txt`'s
/// relay (or `--relay n0|off|<url>`): the lobby flow without menus, for testing across machines. Prints the events.
pub fn cli(mut a: Vec<String>) {
    let mut cfg = Online::default();
    if let Some(i) = a.iter().position(|x| x == "--relay") {
        let v = a.get(i + 1).cloned().unwrap_or_default();
        cfg.relay = match v.as_str() {
            "off" => Relay::Off,
            "n0" => Relay::N0,
            url => Relay::Own(url.into()),
        };
        a.drain(i..(i + 2).min(a.len()));
    }
    let doubles = a.iter().any(|x| x == "doubles");
    let players = a.iter().filter_map(|x| x.parse::<usize>().ok()).next().unwrap_or(1);
    let picks: Vec<Pick> = (0..players).map(|i| Pick { char: i, costume: 0 }).collect();
    let mut room = Room { name: cfg.name.clone(), doubles, court: 0, sets: 0, games: 4, umpire: 4, seats: vec![None; if doubles { 4 } else { 2 }] };
    room.take("host", &picks);
    let net = Net::start(cfg);
    let cmd = match a.first().map(String::as_str) {
        Some("host") => Cmd::Host(room),
        Some("find") => Cmd::Find,
        Some("quick") => Cmd::Quick(room),
        Some("join") => Cmd::Join(a.get(1).cloned().unwrap_or_default(), picks),
        _ => {
            eprintln!("usage: hst net host|find|quick [doubles] [players] | join <code> [players]  [--relay n0|off|<url>]");
            return;
        }
    };
    let find = matches!(cmd, Cmd::Find);
    net.send(cmd);
    loop {
        for e in net.poll() {
            println!("{e:?}");
            match e {
                Event::Found(_) if find => return,
                Event::Start(_) | Event::Failed(_) => return,
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The browser build has no network yet (M3: iroh over its relays; the DHT needs UDP): every command fails.
#[cfg(target_arch = "wasm32")]
#[derive(bevy::prelude::Resource)]
pub struct Net;

#[cfg(target_arch = "wasm32")]
impl Net {
    pub fn start(_: Online) -> Net {
        Net
    }

    pub fn send(&self, _: Cmd) {}

    pub fn poll(&self) -> Vec<Event> {
        vec![Event::Failed("Online play isn't in the browser build yet.".into())]
    }
}

/// `code` is a join code: a host's ticket or a bare endpoint ID.
pub fn is_code(code: &str) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    return code.parse::<EndpointTicket>().is_ok() || code.parse::<EndpointId>().is_ok();
    #[cfg(target_arch = "wasm32")]
    {
        let _ = code;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(doubles: bool) -> Room {
        Room { name: "t".into(), doubles, court: 0, sets: 0, games: 4, umpire: 4, seats: vec![None; if doubles { 4 } else { 2 }] }
    }

    #[test]
    fn seats_fill_and_free() {
        let mut r = room(true);
        assert!(r.take("a", &[Pick { char: 1, costume: 0 }, Pick { char: 2, costume: 0 }]));
        assert!(!r.take("b", &[Pick { char: 3, costume: 0 }; 3]));
        assert!(r.take("b", &[Pick { char: 3, costume: 0 }]));
        assert_eq!(r.open(), 1);
        r.leave("a");
        assert_eq!(r.open(), 3);
        assert_eq!(r.seats[2].as_ref().unwrap().peer, "b");
    }

    #[test]
    fn quickplay_takes_the_busiest_that_fits() {
        let l = |id: &str, taken, seats, time| Listing { id: id.into(), name: String::new(), doubles: seats == 4, court: 0, taken, seats, time };
        let list = [l("a", 1, 4, 5), l("b", 2, 4, 1), l("c", 3, 4, 9), l("d", 1, 2, 9)];
        assert_eq!(quick_pick(&list, true, 2).iter().map(|l| l.id.as_str()).collect::<Vec<_>>(), ["b", "a"]);
        assert_eq!(quick_pick(&list, true, 1)[0].id, "c");
        assert_eq!(quick_pick(&list, false, 1)[0].id, "d");
    }

    #[test]
    fn listing_signature_binds_the_lobby_number() {
        let key = SecretKey::generate();
        let l = Listing::of(key.public(), &room(false));
        let v = seal(&key, 7, &l);
        assert_eq!(unseal(7, &v), Some(l));
        assert_eq!(unseal(8, &v), None);
        let mut bad = v.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert_eq!(unseal(7, &bad), None);
    }

    #[test]
    fn online_txt_round_trips() {
        let o = Online::parse("name = Court 1\nrelay = own\nrelay_url = https://relay.example.org\n");
        assert_eq!(o.relay, Relay::Own("https://relay.example.org".into()));
        assert_eq!(Online::parse(&o.text()), o);
        assert_eq!(Online::parse("").relay, Relay::N0);
    }

    /// The lobby list on a local DHT testnet: a host's announce + listing, found and verified.
    #[test]
    fn find_on_a_testnet() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let net = n0_mainline::Testnet::new(10).await.unwrap();
            let dht = |b: &[String]| Dht::builder().bootstrap(b).build().unwrap();
            let (a, b) = (dht(&net.bootstrap), dht(&net.bootstrap));
            let ep = Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled).bind().await.unwrap();
            let cx = Ctx { ep, dht: a, ev: mpsc::channel().0 };
            let mut r = room(true);
            r.take("host", &[Pick { char: 0, costume: 0 }]);
            announce(&cx, 4242, &r).await;
            let list = find(&b).await;
            assert_eq!(list.len(), 1);
            assert_eq!((list[0].id.clone(), list[0].taken, list[0].seats), (cx.ep.id().to_string(), 1, 4));
        });
    }
}
