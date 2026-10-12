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
//! - **LAN** (M2j): every endpoint also advertises itself by mDNS (service [`MDNS`]), so bare IDs resolve on a shared
//!   network with no internet (a router, or one phone's hotspot with no mobile data); a host puts its listing in the
//!   mDNS record's user data too, and Find lists those first, before the DHT has answered (or with no internet at all).
//! - **Relay** for when hole punching fails: n0's public relays, your own `iroh-relay`, or none (LAN only).
//! - **Code / QR**: the host's [`EndpointTicket`] (ID + relay + direct addresses, its LAN ones included), needs no
//!   discovery at all.
//!
//! The lobby protocol (ALPN [`ALPN`]): the joiner opens a stream and sends [`Msg::Hello`] (build, its 1–3 local
//! players' picks); the host answers [`Msg::Room`] (seated) or [`Msg::Refused`], sends `Room` again on every
//! change and [`Msg::Start`] once every seat is taken. Messages are JSON, a u32 length before each.
//!
//! The app talks to this through [`Net`]: commands in, [`Event`]s out; the network runs on its own tokio thread (in the browser: a task on the page's event loop, relays only).
//! `hst net host|find|quick|join …` drives the same thing from a terminal (two machines, no menus).

// the browser build keeps the lobby types (menus) without the transport
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

use std::sync::mpsc;
use n0_future::time::{Duration, SystemTime};

use iroh::address_lookup::AddrFilter;
use iroh::endpoint::{Connection, RecvStream, SendStream, presets};
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMode, RelayUrl, SecretKey, Signature};
#[cfg(not(target_arch = "wasm32"))]
use iroh_mainline_address_lookup::DhtAddressLookup;
use iroh::endpoint_info::UserData;
#[cfg(not(target_arch = "wasm32"))]
use iroh_mdns_address_lookup::{DiscoveryEvent, MdnsAddressLookup};
use iroh_tickets::endpoint::EndpointTicket;
use n0_future::StreamExt;
#[cfg(not(target_arch = "wasm32"))]
use n0_mainline::{Dht, Id, MutableItem, SigningKey};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc as amp;

pub const ALPN: &[u8] = b"hst/lobby/1";
/// The lobby list's infohash (any 20 bytes; these spell it).
const TOPIC: &[u8; 20] = b"hst-remaster-lobby/1";
/// The listings' shared BEP 44 key (its secret is public on purpose: anyone may list; the endpoint signature inside
/// says who wrote a listing).
const BOARD: &[u8; 32] = b"hot-shots-tennis-remaster-lobby1";
/// The mDNS service name (`<id>._hst._udp.local`), so only our endpoints answer.
const MDNS: &str = "hst";
/// The LAN listing's prefix in the mDNS user data.
const LAN_TAG: &str = "hst1";
/// How long Find listens on the LAN before it shows what it has (mDNS answers come within a second or two).
const LAN_WAIT: Duration = Duration::from_secs(3);
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
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub doubles: bool,
    pub court: usize,
    pub sets: usize,
    pub games: i32,
    pub umpire: u8,
    pub seats: Vec<Option<Seat>>,
    /// Set by the host at the start (M2g): the match's seed (the weather generator's, `main.rs`) and the input delay
    /// in ticks, from the slowest guest's measured round trip.
    #[serde(default)]
    pub seed: u32,
    #[serde(default)]
    pub delay: u32,
    /// Lockstep (M2g) instead of rollback (M2h): the host's `HST_NET_LOCKSTEP`.
    #[serde(default)]
    pub lockstep: bool,
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

fn now() -> u64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_secs())
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

/// A listing as mDNS user data (≤ 245 bytes: the ID is the record's own, the time the finder's clock).
fn lan_data(l: &Listing) -> Option<UserData> {
    let json = serde_json::to_string(&Listing { id: String::new(), time: 0, ..l.clone() }).ok()?;
    format!("{LAN_TAG}{json}").parse().ok()
}

/// The listing in endpoint `id`'s mDNS user data, if it's hosting.
fn lan_listing(id: EndpointId, data: Option<&UserData>) -> Option<Listing> {
    let l: Listing = serde_json::from_str(data?.as_ref().strip_prefix(LAN_TAG)?).ok()?;
    Some(Listing { id: id.to_string(), time: now(), ..l })
}

// ---------------------------------------------------------------- wire

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Msg {
    Hello { build: String, picks: Vec<Pick> },
    Room(Room),
    Refused(String),
    Start(Room),
    /// The host's round-trip probe (its clock in µs) and the guest's echo.
    Ping(u64),
    Pong(u64),
    /// In the match (`play/netplay.rs`): seats' pads for one tick (`Pad` as `[buttons lo, hi, rx, ry, lx, ly]`), a
    /// peer's state hash before a tick, a pause opened or closed, a peer ending the match (why).
    Pads { tick: u32, seats: Vec<(u8, [u8; 6])> },
    Hash { tick: u32, hash: u64 },
    Pause(bool),
    Bye(String),
}

impl Msg {
    /// A match message (relayed to every peer).
    fn in_match(&self) -> bool {
        matches!(self, Msg::Pads { .. } | Msg::Hash { .. } | Msg::Pause(_) | Msg::Bye(_))
    }
}

/// `HST_NET_LAG=ms`: every message waits this long before it goes out (simulated one-way latency, for tests).
pub static LAG_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The input delay for a round trip of `rtt`: enough ticks to cover one way, plus one for jitter (2..=15).
pub fn delay_for(rtt: Duration) -> u32 {
    ((rtt.as_secs_f64() / 2.0 * 60.0).ceil() as u32 + 1).clamp(2, 15)
}

/// Messages out through one stream, each after [`LAG_MS`], in order.
#[derive(Clone)]
struct Link(amp::UnboundedSender<(n0_future::time::Instant, Msg)>);

impl Link {
    fn new(mut tx: SendStream) -> Link {
        let (s, mut r) = amp::unbounded_channel::<(n0_future::time::Instant, Msg)>();
        n0_future::task::spawn(async move {
            while let Some((due, m)) = r.recv().await {
                n0_future::time::sleep_until(due).await;
                if send(&mut tx, &m).await.is_err() {
                    break;
                }
            }
        });
        Link(s)
    }

    fn send(&self, m: &Msg) {
        let lag = Duration::from_millis(LAG_MS.load(std::sync::atomic::Ordering::Relaxed));
        let _ = self.0.send((n0_future::time::Instant::now() + lag, m.clone()));
    }
}

async fn send(s: &mut SendStream, m: &Msg) -> Result<(), String> {
    let b = serde_json::to_vec(m).unwrap();
    s.write_all(&(b.len() as u32).to_le_bytes()).await.map_err(|e| e.to_string())?;
    s.write_all(&b).await.map_err(|e| e.to_string())
}

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
    /// Leave the lobby (or stop hosting, or leave the match).
    Leave,
    /// In the match: to every other peer.
    Send(Msg),
    /// In the match: a datagram (unreliable, unordered: rollback's messages) to one peer, through the host.
    Datagram(String, Vec<u8>),
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
    /// Every seat taken: the match's line-up and which peer this machine is (its seats' `peer`). The connections stay
    /// open for the match.
    Start(Room, String),
    Failed(String),
    /// In the match: another peer's message.
    Match(Msg),
    /// In the match: a peer is gone (why).
    Dropped(String),
    /// In the match: a peer's datagram (`Cmd::Datagram`), with who sent it.
    Datagram(String, Vec<u8>),
}

/// The network thread's handle (a Bevy resource in the menu).
#[derive(bevy::prelude::Resource)]
pub struct Net {
    cmd: amp::UnboundedSender<Cmd>,
    events: std::sync::Mutex<mpsc::Receiver<Event>>,
}

impl Net {
    pub fn start(cfg: Online) -> Net {
        if let Some(ms) = std::env::var("HST_NET_LAG").ok().and_then(|v| v.parse().ok()) {
            LAG_MS.store(ms, std::sync::atomic::Ordering::Relaxed);
        }
        let (cmd, cmd_rx) = amp::unbounded_channel();
        let (ev, events) = mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::Builder::new()
            .name("net".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread().enable_all().worker_threads(2).build().unwrap();
                rt.block_on(run(cfg, cmd_rx, ev));
            })
            .unwrap();
        // the browser: a task on the page's event loop
        #[cfg(target_arch = "wasm32")]
        n0_future::task::spawn(run(cfg, cmd_rx, ev));
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

struct Ctx {
    ep: Endpoint,
    dht: Dht,
    /// The lobbies heard on the LAN, kept current by mDNS.
    lan: Lan,
    ev: mpsc::Sender<Event>,
}

type Lan = std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<EndpointId, Listing>>>;

#[cfg(not(target_arch = "wasm32"))]
/// Keep `lan` to the hosts mDNS hears now (a host's record updates when its seats change; gone when it stops).
fn watch_lan(mdns: MdnsAddressLookup) -> Lan {
    let lan = Lan::default();
    let l = lan.clone();
    n0_future::task::spawn(async move {
        let mut events = mdns.subscribe().await;
        while let Some(e) = events.next().await {
            let mut l = l.lock().unwrap();
            match e {
                DiscoveryEvent::Discovered { endpoint_info: i, .. } => match lan_listing(i.endpoint_id, i.data.user_data()) {
                    Some(listing) => l.insert(i.endpoint_id, listing),
                    None => l.remove(&i.endpoint_id),
                },
                DiscoveryEvent::Expired { endpoint_id } => l.remove(&endpoint_id),
                _ => None,
            };
        }
    });
    lan
}

/// The open lobbies on the LAN.
fn lan_list(lan: &Lan) -> Vec<Listing> {
    lan.lock().unwrap().values().filter(|l| l.open() > 0).cloned().collect()
}

impl Ctx {
    fn say(&self, e: Event) {
        let _ = self.ev.send(e);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn endpoint(relay: &Relay) -> Result<(Endpoint, MdnsAddressLookup), String> {
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
    // LAN: bare IDs resolve by mDNS and hosts' listings ride on it (no internet needed)
    let mdns = MdnsAddressLookup::builder().service_name(MDNS).build(ep.id()).map_err(|e| format!("mDNS: {e}"))?;
    ep.address_lookup().map_err(|e| e.to_string())?.add(mdns.clone());
    if *relay != Relay::Off {
        // a home relay before the ticket, so the code works from anywhere (bounded: a dead relay still leaves LAN)
        let _ = n0_future::time::timeout(Duration::from_secs(10), ep.online()).await;
    }
    Ok((ep, mdns))
}

// The browser has no UDP: no DHT and no mDNS, only the relays. A code (ticket) carries the host's relay, so joining by
// code works; the lobby list doesn't (Find shows none, Quickplay hosts).
#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
struct Dht;

#[cfg(target_arch = "wasm32")]
impl Dht {
    fn client() -> Result<Dht, String> {
        Ok(Dht)
    }
}

#[cfg(target_arch = "wasm32")]
fn watch_lan(_: ()) -> Lan {
    Lan::default()
}

#[cfg(target_arch = "wasm32")]
async fn find_all(_: &Ctx) -> Vec<Listing> {
    vec![]
}

#[cfg(target_arch = "wasm32")]
async fn announce(_: &Ctx, _: u16, _: &Room) {}

#[cfg(target_arch = "wasm32")]
pub async fn endpoint(relay: &Relay) -> Result<(Endpoint, ()), String> {
    let mode = match relay {
        Relay::N0 => RelayMode::Default,
        Relay::Own(url) => RelayMode::Custom(url.parse::<RelayUrl>().map_err(|e| format!("relay_url {url}: {e}"))?.into()),
        Relay::Off => return Err("The browser can only play online through a relay (Settings: relay).".into()),
    };
    let ep = Endpoint::builder(presets::Minimal).secret_key(SecretKey::generate()).alpns(vec![ALPN.to_vec()]).relay_mode(mode).bind().await.map_err(|e| e.to_string())?;
    // the code needs the home relay in it: nothing else reaches a browser
    let _ = n0_future::time::timeout(Duration::from_secs(10), ep.online()).await;
    Ok((ep, ()))
}

async fn run(cfg: Online, mut cmds: amp::UnboundedReceiver<Cmd>, ev: mpsc::Sender<Event>) {
    let _ = ev.send(Event::Status("Connecting...".into()));
    let (ep, mdns) = match endpoint(&cfg.relay).await {
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
    let cx = Ctx { ep, dht, lan: watch_lan(mdns), ev };
    cx.say(Event::Status("Online.".into()));
    while let Some(c) = cmds.recv().await {
        match c {
            Cmd::Host(room) => host(&cx, room, &mut cmds).await,
            Cmd::Find => {
                cx.say(Event::Status("Looking for lobbies...".into()));
                // the LAN's first (all there is with no internet), then the DHT's added
                n0_future::time::sleep(LAN_WAIT).await;
                cx.say(Event::Found(lan_list(&cx.lan)));
                let list = find_all(&cx).await;
                cx.say(Event::Found(list));
            }
            Cmd::Quick(room) => {
                cx.say(Event::Status("Looking for a lobby...".into()));
                let mine: Vec<Pick> = room.seats.iter().flatten().map(|s| s.pick).collect();
                let mut joined = false;
                n0_future::time::sleep(LAN_WAIT).await;
                for l in quick_pick(&find_all(&cx).await, room.doubles, mine.len()) {
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
            Cmd::Leave | Cmd::Send(_) | Cmd::Datagram(..) => {}
        }
    }
    cx.ep.close().await;
}

#[cfg(not(target_arch = "wasm32"))]
/// The open lobbies on the LAN and the DHT (the LAN's copy of one listed on both).
async fn find_all(cx: &Ctx) -> Vec<Listing> {
    let mut list = lan_list(&cx.lan);
    let dht = find(&cx.dht).await;
    list.extend(dht.into_iter().filter(|l| !cx.lan.lock().unwrap().keys().any(|id| id.to_string() == l.id)));
    list
}

#[cfg(not(target_arch = "wasm32"))]
/// The open lobbies on the DHT now.
pub async fn find(dht: &Dht) -> Vec<Listing> {
    let mut ports = std::collections::BTreeSet::new();
    if let Ok(mut s) = dht.get_peers(Id::from_bytes(TOPIC).unwrap()).await {
        // the query ends by itself once the closest nodes answered; a cap in case it doesn't
        let _ = n0_future::time::timeout(Duration::from_secs(30), async {
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

/// A joiner's side, inside the host task.
enum Guest {
    In { peer: String, picks: Vec<Pick>, build: String, tx: SendStream, conn: Connection },
    Said(String, Msg),
    Gone(String),
    /// A datagram from the first peer for the second.
    Datagram(String, String, Vec<u8>),
}

/// The host's clock for the round-trip probes, in µs.
fn micros(t0: n0_future::time::Instant) -> u64 {
    t0.elapsed().as_micros() as u64
}

async fn host(cx: &Ctx, mut room: Room, cmds: &mut amp::UnboundedReceiver<Cmd>) {
    let code = EndpointTicket::new(cx.ep.addr()).to_string();
    cx.say(Event::Code(code));
    cx.say(Event::Room(room.clone()));
    let n = lobby_number(cx.ep.id());
    let (gtx, mut guests) = amp::unbounded_channel::<Guest>();
    let mut joined: Vec<(String, Link, Connection)> = Vec::new();
    let mut tick = n0_future::time::interval(REPUBLISH);
    let mut dirty = true;
    // the round trips: each guest's last, and (full) the guests not yet answered since the start's probe, until when
    let t0 = n0_future::time::Instant::now();
    let mut rtt: std::collections::HashMap<String, Duration> = Default::default();
    // a host with every seat (`HST_NET_SYNCTEST`) starts at once
    let mut starting: Option<(Vec<String>, n0_future::time::Instant)> = room.seats.iter().all(Option::is_some).then(|| (Vec::new(), t0));
    let mut ping = n0_future::time::interval(Duration::from_secs(1));
    loop {
        if dirty {
            dirty = false;
            // listing in the background: a slow DHT put never holds up a join
            let l = room.clone();
            cx.ep.set_user_data_for_address_lookup(lan_data(&Listing::of(cx.ep.id(), &l)));
            let cx2 = Ctx { ep: cx.ep.clone(), dht: cx.dht.clone(), lan: cx.lan.clone(), ev: cx.ev.clone() };
            n0_future::task::spawn(async move { announce(&cx2, n, &l).await });
        }
        if let Some((waiting, until)) = &starting
            && (waiting.is_empty() || n0_future::time::Instant::now() >= *until)
        {
            // ponytail: no probe answer in time counts as 250 ms; a slower peer just stalls the match more
            let worst = joined.iter().map(|(p, ..)| rtt.get(p).copied().unwrap_or(Duration::from_millis(250))).max().unwrap_or_default();
            room.delay = delay_for(worst);
            room.lockstep = std::env::var_os("HST_NET_LOCKSTEP").is_some();
            room.seed = (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_nanos()) as u32) | 1;
            broadcast(&joined, &Msg::Start(room.clone()));
            cx.say(Event::Status(format!("Round trip {} ms, input delay {} ticks.", worst.as_millis(), room.delay)));
            cx.say(Event::Start(room.clone(), "host".into()));
            cx.ep.set_user_data_for_address_lookup(None);
            return relay(cx, joined, gtx, guests, cmds).await;
        }
        tokio::select! {
            Some(inc) = cx.ep.accept() => {
                let gtx = gtx.clone();
                n0_future::task::spawn(async move {
                    let Ok(conn) = async { inc.accept()?.await.map_err(Into::into) }.await.map_err(|e: Box<dyn std::error::Error + Send + Sync>| e) else { return };
                    let peer = conn.remote_id().to_string();
                    let Ok((tx, mut rx)) = conn.accept_bi().await else { return };
                    if let Ok(Msg::Hello { build, picks }) = recv(&mut rx).await {
                        let _ = gtx.send(Guest::In { peer: peer.clone(), picks, build, tx, conn: conn.clone() });
                        // everything they say, until they leave or drop
                        while let Ok(m) = recv(&mut rx).await {
                            let _ = gtx.send(Guest::Said(peer.clone(), m));
                        }
                        let _ = gtx.send(Guest::Gone(peer));
                    }
                });
            }
            Some(g) = guests.recv() => match g {
                Guest::In { peer, picks, build, mut tx, conn } => {
                    let why = if build != BUILD {
                        Some(format!("different version (host {BUILD}, you {build})"))
                    } else if starting.is_some() || !room.take(&peer, &picks) {
                        Some(format!("not enough open seats ({} open, you bring {})", room.open(), picks.len()))
                    } else {
                        None
                    };
                    if let Some(why) = why {
                        let _ = send(&mut tx, &Msg::Refused(why)).await;
                        let _ = tx.finish();
                        // let the refusal arrive before the connection goes
                        n0_future::task::spawn(async move { n0_future::time::sleep(Duration::from_secs(2)).await; drop(conn) });
                        continue;
                    }
                    cx.say(Event::Status(format!("{} joined.", short(&peer))));
                    let link = Link::new(tx);
                    link.send(&Msg::Ping(micros(t0)));
                    joined.push((peer, link, conn));
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                    if room.open() == 0 {
                        // a fresh round trip from everyone before the input delay is picked (3 s at most)
                        broadcast(&joined, &Msg::Ping(micros(t0)));
                        starting = Some((joined.iter().map(|(p, ..)| p.clone()).collect(), n0_future::time::Instant::now() + Duration::from_secs(3)));
                    }
                }
                Guest::Said(peer, Msg::Pong(sent)) => {
                    rtt.insert(peer.clone(), Duration::from_micros(micros(t0).saturating_sub(sent)));
                    if let Some((waiting, _)) = &mut starting {
                        waiting.retain(|p| *p != peer);
                    }
                }
                Guest::Said(..) | Guest::Datagram(..) => {}
                Guest::Gone(peer) => {
                    room.leave(&peer);
                    joined.retain(|(p, ..)| *p != peer);
                    starting = None;
                    cx.say(Event::Status(format!("{} left.", short(&peer))));
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                }
            },
            _ = ping.tick() => broadcast(&joined, &Msg::Ping(micros(t0))),
            _ = tick.tick() => dirty = true,
            c = cmds.recv() => if matches!(c, Some(Cmd::Leave) | None) {
                for (_, _, conn) in &joined {
                    conn.close(0u32.into(), b"host left");
                }
                cx.ep.set_user_data_for_address_lookup(None);
                // an empty listing ages out in STALE; a full one isn't shown
                return;
            },
        }
    }
}

/// The host in the match: every guest's match messages go to the app and the other guests, the app's to every guest;
/// a guest dropping ends the match for everyone.
async fn relay(cx: &Ctx, joined: Vec<(String, Link, Connection)>, gtx: amp::UnboundedSender<Guest>, mut guests: amp::UnboundedReceiver<Guest>, cmds: &mut amp::UnboundedReceiver<Cmd>) {
    for (peer, _, conn) in &joined {
        let (peer, conn, gtx) = (peer.clone(), conn.clone(), gtx.clone());
        n0_future::task::spawn(async move {
            while let Ok(d) = conn.read_datagram().await {
                if let Some((to, data)) = undatagram(&d) {
                    let _ = gtx.send(Guest::Datagram(peer.clone(), to, data.to_vec()));
                }
            }
        });
    }
    let to = |peer: &str| joined.iter().find(|(p, ..)| p == peer).map(|(_, _, c)| c);
    loop {
        tokio::select! {
            Some(g) = guests.recv() => match g {
                Guest::Said(peer, m) if m.in_match() => {
                    joined.iter().filter(|(p, ..)| *p != peer).for_each(|(_, l, _)| l.send(&m));
                    cx.say(Event::Match(m));
                }
                Guest::Gone(peer) => {
                    let why = format!("Player {} left.", short(&peer));
                    joined.iter().filter(|(p, ..)| *p != peer).for_each(|(_, l, _)| l.send(&Msg::Bye(why.clone())));
                    cx.say(Event::Dropped(why));
                }
                // a join while the match runs: no seat
                Guest::In { mut tx, .. } => {
                    let _ = send(&mut tx, &Msg::Refused("the match has started".into())).await;
                }
                Guest::Datagram(from, to_, data) if to_ == "host" => cx.say(Event::Datagram(from, data)),
                Guest::Datagram(from, to_, data) => {
                    if let Some(c) = to(&to_) {
                        send_datagram(c, &from, &data);
                    }
                }
                Guest::Said(..) => {}
            },
            c = cmds.recv() => match c {
                Some(Cmd::Send(m)) => broadcast(&joined, &m),
                Some(Cmd::Datagram(peer, data)) => {
                    if let Some(c) = to(&peer) {
                        send_datagram(c, "host", &data);
                    }
                }
                Some(Cmd::Leave) | None => {
                    // the goodbyes go out with the lag, then the connections close
                    n0_future::time::sleep(Duration::from_millis(LAG_MS.load(std::sync::atomic::Ordering::Relaxed) + 200)).await;
                    for (_, _, conn) in &joined {
                        conn.close(0u32.into(), b"host left");
                    }
                    return;
                }
                Some(_) => {}
            },
        }
    }
}

/// A datagram on the wire: the other end's peer name (the sender's from the host, the receiver's to it), then the data.
fn datagram(peer: &str, data: &[u8]) -> Vec<u8> {
    let mut d = vec![peer.len() as u8];
    d.extend_from_slice(peer.as_bytes());
    d.extend_from_slice(data);
    d
}

fn undatagram(d: &[u8]) -> Option<(String, &[u8])> {
    let n = *d.first()? as usize;
    Some((String::from_utf8(d.get(1..1 + n)?.to_vec()).ok()?, d.get(1 + n..)?))
}

/// Sends after [`LAG_MS`] (simulated latency); a datagram that doesn't fit or finds the connection gone is lost, as
/// datagrams are.
fn send_datagram(c: &Connection, peer: &str, data: &[u8]) {
    let d = datagram(peer, data);
    let lag = LAG_MS.load(std::sync::atomic::Ordering::Relaxed);
    if lag == 0 {
        let _ = c.send_datagram(d.into());
    } else {
        let c = c.clone();
        n0_future::task::spawn(async move {
            n0_future::time::sleep(Duration::from_millis(lag)).await;
            let _ = c.send_datagram(d.into());
        });
    }
}

fn broadcast(to: &[(String, Link, Connection)], m: &Msg) {
    for (_, l, _) in to {
        l.send(m);
    }
}

async fn wait_leave(cmds: &mut amp::UnboundedReceiver<Cmd>) {
    while let Some(c) = cmds.recv().await {
        if matches!(c, Cmd::Leave) {
            return;
        }
    }
}

/// This host's lobby number, from its (random) endpoint ID.
fn lobby_number(id: EndpointId) -> u16 {
    // ponytail: 16 bits, two hosts on one number overwrite each other's listing (birthday odds, ~1 % at 36 hosts at
    // once); an IP-bound salt or an address index server if lobbies get that busy
    u16::from_le_bytes([id.as_bytes()[0], id.as_bytes()[1]]).max(1)
}

fn short(id: &str) -> &str {
    &id[..id.len().min(8)]
}

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
    let conn = match n0_future::time::timeout(Duration::from_secs(30), cx.ep.connect(addr, ALPN)).await {
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
    let link = Link::new(tx);
    // its own task: `recv` isn't cancel-safe, so it can't sit in the `select!` (a command mid-read lost bytes)
    let (got, mut inbox) = tokio::sync::mpsc::unbounded_channel();
    n0_future::task::spawn(async move {
        loop {
            let m = recv(&mut rx).await;
            let end = m.is_err();
            if got.send(m).is_err() || end {
                return;
            }
        }
    });
    let mut seated = false;
    let mut playing = false;
    loop {
        tokio::select! {
            m = inbox.recv() => match m.unwrap_or_else(|| Err("closed".into())) {
                Ok(Msg::Room(r)) => {
                    seated = true;
                    cx.say(Event::Room(r));
                }
                Ok(Msg::Start(r)) => {
                    playing = true;
                    cx.say(Event::Start(r, cx.ep.id().to_string()));
                    let (conn, ev) = (conn.clone(), cx.ev.clone());
                    n0_future::task::spawn(async move {
                        while let Ok(d) = conn.read_datagram().await {
                            if let Some((from, data)) = undatagram(&d) {
                                let _ = ev.send(Event::Datagram(from, data.to_vec()));
                            }
                        }
                    });
                }
                Ok(Msg::Ping(t)) => link.send(&Msg::Pong(t)),
                Ok(m) if playing && m.in_match() => cx.say(Event::Match(m)),
                Ok(Msg::Refused(why)) => {
                    cx.say(Event::Failed(format!("Refused: {why}")));
                    return false;
                }
                Ok(Msg::Hello { .. }) | Err(_) => {
                    if playing {
                        cx.say(Event::Dropped("The host left.".into()));
                        wait_leave(cmds).await;
                    } else {
                        cx.say(Event::Failed(if seated { "The host left." } else { "The host closed the connection." }.into()));
                    }
                    return seated;
                }
                Ok(_) => {}
            },
            c = cmds.recv() => match c {
                Some(Cmd::Send(m)) => link.send(&m),
                Some(Cmd::Datagram(to, data)) => send_datagram(&conn, &to, &data),
                Some(Cmd::Leave) | None => {
                    n0_future::time::sleep(Duration::from_millis(if playing { LAG_MS.load(std::sync::atomic::Ordering::Relaxed) + 200 } else { 0 })).await;
                    conn.close(0u32.into(), b"left");
                    return true;
                }
                Some(_) => {}
            },
        }
    }
}

// ---------------------------------------------------------------- terminal

#[cfg(not(target_arch = "wasm32"))]
/// `hst net host [doubles] [players] | find | quick [doubles] [players] | join <code> [players]`, with `online.txt`'s
/// relay (or `--relay n0|off|<url>`): the lobby flow without menus, for testing across machines. Prints the events.
/// With `play`, returns once everyone's in: the lobby, the room and this machine's peer name, for the match.
pub fn cli(mut a: Vec<String>, play: bool) -> Option<(Net, Room, String)> {
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
    let mut games = 4;
    if let Some(i) = a.iter().position(|x| x == "--games") {
        games = a.get(i + 1).and_then(|g| g.parse().ok()).unwrap_or(games);
        a.drain(i..(i + 2).min(a.len()));
    }
    let doubles = a.iter().any(|x| x == "doubles");
    let players = a.iter().filter_map(|x| x.parse::<usize>().ok()).next().unwrap_or(1);
    let picks: Vec<Pick> = (0..players).map(|i| Pick { char: i, costume: 0 }).collect();
    let mut room = Room { name: cfg.name.clone(), doubles, court: 0, sets: 0, games, umpire: 4, seats: vec![None; if doubles { 4 } else { 2 }], ..Default::default() };
    room.take("host", &picks);
    let net = Net::start(cfg);
    let cmd = match a.first().map(String::as_str) {
        Some("host") => Cmd::Host(room),
        Some("find") => Cmd::Find,
        Some("quick") => Cmd::Quick(room),
        Some("join") => Cmd::Join(a.get(1).cloned().unwrap_or_default(), picks),
        _ => {
            eprintln!("usage: hst net host|find|quick [doubles] [players] | join <code> [players]  [--relay n0|off|<url>] [--games N] [--play [disc] [flags]]");
            return None;
        }
    };
    // Find answers twice: the LAN's list, then with the DHT's
    let mut finds = if matches!(cmd, Cmd::Find) { 2 } else { 0 };
    net.send(cmd);
    loop {
        for e in net.poll() {
            println!("{e:?}");
            match e {
                Event::Found(_) if finds > 0 => {
                    finds -= 1;
                    if finds == 0 {
                        return None;
                    }
                }
                Event::Start(r, me) if play => return Some((net, r, me)),
                Event::Start(..) | Event::Failed(_) => return None,
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// `code` is a join code: a host's ticket or a bare endpoint ID.
pub fn is_code(code: &str) -> bool {
    code.parse::<EndpointTicket>().is_ok() || code.parse::<EndpointId>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(doubles: bool) -> Room {
        Room { name: "t".into(), doubles, court: 0, sets: 0, games: 4, umpire: 4, seats: vec![None; if doubles { 4 } else { 2 }], ..Default::default() }
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
            let cx = Ctx { ep, dht: a, lan: Lan::default(), ev: mpsc::channel().0 };
            let mut r = room(true);
            r.take("host", &[Pick { char: 0, costume: 0 }]);
            announce(&cx, 4242, &r).await;
            let list = find(&b).await;
            assert_eq!(list.len(), 1);
            assert_eq!((list[0].id.clone(), list[0].taken, list[0].seats), (cx.ep.id().to_string(), 1, 4));
        });
    }

    #[test]
    fn lan_listing_fits_mdns() {
        let mut r = room(true);
        // the longest name: 24 four-byte characters
        r.name = "🎾".repeat(24);
        r.take("host", &[Pick { char: 0, costume: 0 }]);
        let id = SecretKey::generate().public();
        let data = lan_data(&Listing::of(id, &r)).expect("fits in 245 bytes");
        let l = lan_listing(id, Some(&data)).unwrap();
        assert_eq!((l.id, l.name, l.taken, l.seats), (id.to_string(), r.name, 1, 4));
        assert_eq!(lan_listing(id, Some(&"other app".parse().unwrap())), None);
    }

    /// LAN play with relays off and no DHT: a host's lobby heard by mDNS, then joined by bare ID (mDNS lookup) and
    /// by its code (whose direct addresses need no lookup at all).
    #[test]
    fn lan_lobby_by_mdns() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (host, _hm) = endpoint(&Relay::Off).await.unwrap();
            let (guest, gm) = endpoint(&Relay::Off).await.unwrap();
            let lan = watch_lan(gm);
            let mut r = room(false);
            r.take("host", &[Pick { char: 0, costume: 0 }]);
            host.set_user_data_for_address_lookup(lan_data(&Listing::of(host.id(), &r)));
            let t = tokio::time::Instant::now();
            while lan_list(&lan).is_empty() && t.elapsed() < Duration::from_secs(15) {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            let list = lan_list(&lan);
            assert_eq!(list.len(), 1, "no LAN listing in 15 s (is multicast blocked?)");
            assert_eq!((list[0].id.clone(), list[0].open()), (host.id().to_string(), 1));
            let accept = tokio::spawn({
                let host = host.clone();
                async move {
                    for _ in 0..2 {
                        let c = host.accept().await.unwrap().await.unwrap();
                        tokio::spawn(async move { c.closed().await });
                    }
                }
            });
            // by ID: the address from mDNS
            let c = guest.connect(list[0].id.parse::<EndpointId>().unwrap(), ALPN).await.unwrap();
            c.close(0u32.into(), b"");
            // by code: the ticket carries the LAN addresses
            let ticket = EndpointTicket::new(host.addr());
            assert!(host.addr().ip_addrs().next().is_some(), "no direct address in the code");
            // (an endpoint with no address lookup at all)
            let fresh = Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled).bind().await.unwrap();
            let c = fresh.connect(EndpointAddr::from(ticket), ALPN).await.unwrap();
            c.close(0u32.into(), b"");
            accept.await.unwrap();
            // stopped hosting: off the list once its record updates
            host.set_user_data_for_address_lookup(None);
        });
    }
}
