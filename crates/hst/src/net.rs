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
//! - **Board slots** (B116), which the browser reaches too: [`SLOTS`] well-known keys (public seeds, like the short
//!   codes') each hold one listing with its short code, read and written on the DHT natively and through the pkarr
//!   relay in the browser. A host takes the first slot that's free, stale or its own; Find reads them all.
//! - **Relay** for when hole punching fails: n0's public relays, your own `iroh-relay`, or none (LAN only).
//! - **Code / QR**: the host's [`EndpointTicket`] (ID + relay + direct addresses, its LAN ones included), needs no
//!   discovery at all.
//!
//! The lobby protocol (ALPN [`ALPN`]): the joiner opens a stream and sends [`Msg::Hello`] (build, its 1–3 local
//! players' picks, its player's name); the host answers [`Msg::Room`] (seated) or [`Msg::Refused`], sends `Room` again
//! on every change (a guest's [`Msg::Prepared`] marks it ready) and [`Msg::Start`] at the end of a countdown (B215,
//! `Room::count`: every guest ready, or the host's Start; a peer's [`Msg::Cancel`] or un-ready stops it), the open
//! seats then to the computer ([`CPU`]). Mid-match, a joiner with the same build takes a seat whose
//! peer went silent ([`Msg::Away`]) and gets `Start` with `rejoin` set (B196, `play/recover.rs`). Messages are JSON, a u32 length before each.
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

pub mod rtc;

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

/// `online.txt` beside the disc image: `name = …` (this player's name, shown on their lobby seats; a host's lobby is
/// listed under it), `relay = n0 | own | off`, `relay_url = https://…`, `private = yes | no` (B213: Host's last choice).
#[derive(Clone, Debug, PartialEq)]
pub struct Online {
    pub name: String,
    pub relay: Relay,
    pub relay_url: String,
    /// B213: the host's last Public/Private choice (Host opens the lobby with it).
    pub private: bool,
}

impl Default for Online {
    fn default() -> Self {
        Online { name: "Player".into(), relay: Relay::N0, relay_url: String::new(), private: false }
    }
}

impl Online {
    pub fn path(iso: &str) -> std::path::PathBuf {
        std::path::Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref()).join("online.txt")
    }

    pub fn load(iso: &str) -> Online {
        Online::parse(&crate::store::read_to_string(Online::path(iso)).unwrap_or_default())
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
                "private" => o.private = v == "yes",
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
        format!("name = {}\nrelay = {relay}\nrelay_url = {}\nprivate = {}\n", self.name, self.relay_url, if self.private { "yes" } else { "no" })
    }

    /// The next relay choice: n0, Off, then own only once a `relay_url` is set (the menu's Custom... comes after the
    /// last of them, B323).
    pub fn next_relay(&mut self) {
        self.relay = match self.relay {
            Relay::N0 => Relay::Off,
            Relay::Off if !self.relay_url.is_empty() => Relay::Own(self.relay_url.clone()),
            Relay::Off | Relay::Own(_) => Relay::N0,
        };
    }

    /// B323: this player's own relay from now on (`url` checked by [`relay_url`]).
    pub fn set_own(&mut self, url: String) {
        self.relay = Relay::Own(url.clone());
        self.relay_url = url;
    }
}

/// B323: a typed relay URL as the endpoint will dial it, or why it can't be one: http(s) with a host (iroh's
/// `RelayUrl` parsing, so what's accepted here binds).
pub fn relay_url(s: &str) -> Result<String, String> {
    let u = s.trim().parse::<RelayUrl>().map_err(|e| e.to_string())?;
    match u.scheme() {
        "http" | "https" if u.host_str().is_some_and(|h| !h.is_empty()) => Ok(u.to_string()),
        "http" | "https" => Err("no host".into()),
        s => Err(format!("{s}: not http or https")),
    }
}

// ---------------------------------------------------------------- lobby data

/// One local player's pick: disc character (a mod's donor) and costume, and a mod's gameplay data (M2i).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pick {
    pub char: usize,
    pub costume: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#mod: Option<Box<crate::mods_online::Wire>>,
    /// Drawn by the Random card: the lobby shows "Random", not who it drew (the mod still streams as any).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub random: bool,
    /// B370: the select's hand toggle (Select / −): the match plays the character with the other hand
    /// (`--swap-hands`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub swap: bool,
}

impl Pick {
    pub fn disc(char: usize, costume: usize) -> Pick {
        Pick { char, costume, r#mod: None, random: false, swap: false }
    }
}

/// A taken seat: whose peer (endpoint ID) and their pick.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Seat {
    pub peer: String,
    pub pick: Pick,
}

/// The peer name of a seat the computer plays (the open seats when the host starts).
pub const CPU: &str = "cpu";

impl Seat {
    /// A person plays it (on some peer), not the computer.
    pub fn human(&self) -> bool {
        self.peer != CPU
    }
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
    /// M8f3: the host's umpire mod (id, name); `umpire` is its donor, whose word lengths time the calls on every
    /// peer. A peer with the mod installed hears it (`--umpire-mod-id`), one without hears the donor. None in a
    /// vanilla lobby (`netplay::flags`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub umpire_mod: Option<(String, String)>,
    /// Set by the host at the start (M2g): the match's seed (the weather generator's, `main.rs`) and the input delay
    /// in ticks, from the slowest guest's measured round trip.
    #[serde(default)]
    pub seed: u32,
    #[serde(default)]
    pub delay: u32,
    /// Lockstep (M2g) instead of rollback (M2h): the host's `HST_NET_LOCKSTEP`.
    #[serde(default)]
    pub lockstep: bool,
    /// Counts the line-up's changes: a peer's [`Msg::Ready`] is for one line-up.
    #[serde(default)]
    pub ver: u32,
    /// Each peer's player name (its Hello's), and the guests who said they're ready (the host always is).
    #[serde(default)]
    pub names: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub ready: std::collections::BTreeSet<String>,
    /// B195: peers seated but still on the lobby's character select (their seats hold placeholder picks).
    #[serde(default)]
    pub picking: std::collections::BTreeSet<String>,
    /// B196: this machine comes back into a match under way (its state follows in `Msg::Restart`).
    #[serde(default)]
    pub rejoin: bool,
    /// B201: each peer's players on the lobby's character select as they choose, in the order of the seats it holds.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub live: std::collections::BTreeMap<String, Vec<Live>>,
    /// B215: the start's countdown, seconds left (0: none running), as the host counts it.
    #[serde(default)]
    pub count: u32,
    /// B213: not listed (no DHT listing, no mDNS listing, so Find and Quickplay never see it): only a code joins.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub private: bool,
    /// B281: peers with a mod they haven't chosen to download or not; after Start, the seconds left for them to
    /// choose (0: no wait running), and the wait being over (a mod not chosen plays as its stand-in).
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub deciding: std::collections::BTreeSet<String>,
    #[serde(default)]
    pub wait: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stand_in: bool,
    /// A vanilla lobby: the host allows no mods, so every seat plays a disc character (mod picks are refused).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub vanilla: bool,
    /// B322: the host's short code once it's online (empty until then, or with none), so a guest's lobby shows the
    /// same code and QR as the host's and can pass the lobby on. Never the long ticket (B224).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub short: String,
    /// B370: each guest's last round trip to the host (ms, by peer), for the lobby's cards.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub ping: std::collections::BTreeMap<String, u32>,
}

/// B281: how long the host's Start waits for a player still choosing whether to download a mod (ms; tests shorten it).
pub static DECIDE_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(10_000);

/// B201: one player's place on the character select, live: the disc character hovered (a mod's donor, with the mod's
/// id and name), the costume, and whether they're ready on it. Only for showing; the match takes the seats' picks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Live {
    pub char: usize,
    pub costume: usize,
    pub ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#mod: Option<String>,
    /// B234: the mod's own name, for peers that don't have it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// B370: the hand toggle and the Random card, as they change (the lobby's cards follow them).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub swap: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub random: bool,
    /// B371: the player's own name (`main_menu::names`), "" for none: shown in place of "1P".."4P".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub player: String,
}

impl Room {
    pub fn open(&self) -> usize {
        self.seats.iter().filter(|s| s.is_none()).count()
    }

    /// B201: seat `i`'s player's live place on the select (its peer's k-th held seat's), if its peer sent one.
    pub fn live_of(&self, i: usize) -> Option<&Live> {
        let peer = &self.seats.get(i)?.as_ref()?.peer;
        let k = self.seats[..i].iter().flatten().filter(|s| s.peer == *peer).count();
        self.live.get(peer)?.get(k)
    }

    /// B201: the host's computer player in open (or computer) seat `i`, or the seat opened again (`None`); false (no
    /// change) for a person's seat.
    fn bot(&mut self, i: usize, pick: Option<Pick>) -> bool {
        if self.seats.get(i).is_none_or(|s| s.as_ref().is_some_and(Seat::human)) {
            return false;
        }
        self.seats[i] = pick.map(|pick| Seat { peer: CPU.into(), pick });
        self.ver += 1;
        true
    }

    /// B201: singles or doubles, the taken seats (people and computer) first in order; false (no change) if they
    /// don't fit.
    fn set_doubles(&mut self, doubles: bool) -> bool {
        let taken: Vec<Seat> = self.seats.iter().flatten().cloned().collect();
        let n = if doubles { 4 } else { 2 };
        if doubles == self.doubles || taken.len() > n {
            return false;
        }
        self.doubles = doubles;
        self.seats = taken.into_iter().map(Some).chain(std::iter::repeat(None)).take(n).collect();
        self.ver += 1;
        true
    }

    /// Seat `peer`'s picks in the first open seats; false (and no change) if they don't all fit.
    fn take(&mut self, peer: &str, picks: &[Pick]) -> bool {
        if picks.is_empty() || picks.len() > self.open() {
            return false;
        }
        let mut free = (0..self.seats.len()).filter(|&i| self.seats[i].is_none()).collect::<Vec<_>>().into_iter();
        for pick in picks {
            self.seats[free.next().unwrap()] = Some(Seat { peer: peer.into(), pick: pick.clone() });
        }
        self.ver += 1;
        true
    }

    /// B195: `peer`'s chosen picks into the seats it holds (same count), its placeholders replaced; false (no
    /// change) if the count differs.
    fn repick(&mut self, peer: &str, picks: &[Pick]) -> bool {
        let mut held: Vec<&mut Seat> = self.seats.iter_mut().flatten().filter(|s| s.peer == peer).collect();
        if held.len() != picks.len() || picks.is_empty() {
            return false;
        }
        for (s, p) in held.iter_mut().zip(picks) {
            s.pick = p.clone();
        }
        self.picking.remove(peer);
        self.ready.remove(peer);
        self.ver += 1;
        true
    }

    /// Picks this lobby takes: playable mods (`picks_ok`), and none in a vanilla lobby.
    fn allowed(&self, p: &[Pick]) -> Result<(), String> {
        match self.vanilla && p.iter().any(|p| p.r#mod.is_some()) {
            true => Err("this lobby plays without mods".into()),
            false => picks_ok(p),
        }
    }

    /// Mods allowed or not (vanilla); going vanilla puts each seated mod's donor in its place. False: no change.
    fn set_vanilla(&mut self, vanilla: bool) -> bool {
        if vanilla == self.vanilla {
            return false;
        }
        self.vanilla = vanilla;
        for s in self.seats.iter_mut().flatten() {
            if let Some(w) = s.pick.r#mod.take() {
                s.pick = Pick::disc(w.donor, 0);
            }
        }
        self.ver += 1;
        true
    }

    /// The mods' looks (pack hashes) in the seats, and who brought each.
    pub fn mods(&self) -> impl Iterator<Item = (&Seat, &crate::mods_online::Wire)> {
        self.seats.iter().flatten().filter_map(|s| Some((s, s.pick.r#mod.as_deref()?)))
    }

    /// Every mod pick one this game can play.
    fn check(&self) -> Result<(), String> {
        self.mods().try_for_each(|(_, w)| w.check())
    }

    /// Every guest in the seats has said they're ready.
    pub fn all_ready(&self) -> bool {
        self.seats.iter().flatten().all(|s| s.peer == "host" || !s.human() || self.ready.contains(&s.peer))
    }

    /// B215: a guest in, every person's seat picked and ready: the countdown starts by itself. B292: only with no
    /// seat open (otherwise the host's Start, the open seats going to the computer).
    fn all_set(&self) -> bool {
        self.open() == 0 && self.picking.is_empty() && self.all_ready() && self.seats.iter().flatten().any(|s| s.peer != "host" && s.human())
    }

    /// The open seats to the computer: disc characters nobody picked, in grid order.
    fn bots(&mut self) {
        let taken: Vec<usize> = self.seats.iter().flatten().map(|s| s.pick.char).collect();
        let mut free = (0..14).filter(|c| !taken.contains(c));
        for s in self.seats.iter_mut().filter(|s| s.is_none()) {
            *s = Some(Seat { peer: CPU.into(), pick: Pick::disc(free.next().unwrap_or(0), 0) });
        }
    }

    /// B196: player `old` is back as peer `new` (named `name`): their seats and name move over.
    pub fn rejoined(&mut self, old: &str, new: &str, name: &str) {
        for s in self.seats.iter_mut().flatten().filter(|s| s.peer == old) {
            s.peer = new.into();
        }
        self.names.remove(old);
        self.names.insert(new.into(), name.into());
    }

    /// B196: the lobby after the match, for the players still in: the computer's seats and `away`'s open again,
    /// everyone to say they're ready again.
    pub fn back_to_lobby(&mut self, away: &[String]) {
        for s in &mut self.seats {
            if s.as_ref().is_some_and(|s| s.peer == CPU || away.contains(&s.peer)) {
                *s = None;
            }
        }
        self.names.retain(|p, _| !away.contains(p));
        self.picking.retain(|p| !away.contains(p));
        self.deciding.retain(|p| !away.contains(p));
        self.ready.clear();
        (self.rejoin, self.ver) = (false, self.ver + 1);
    }

    /// B279: the lobby again after a finished match: everyone's seats, picks and the court as they were, nobody ready.
    pub fn again(&mut self) {
        self.ready.clear();
        (self.rejoin, self.ver) = (false, self.ver + 1);
    }

    fn leave(&mut self, peer: &str) {
        self.ver += 1;
        self.names.remove(peer);
        self.ready.remove(peer);
        self.picking.remove(peer);
        self.deciding.remove(peer);
        self.live.remove(peer);
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
    /// Its short code, on a lobby only the board slots list (B116: a browser host has no DHT or mDNS address, so
    /// it's joined by the code); empty where the ID is dialled.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub code: String,
}

impl Listing {
    fn of(id: EndpointId, room: &Room) -> Listing {
        let seats = room.seats.len();
        Listing { id: id.to_string(), name: room.name.clone(), doubles: room.doubles, court: room.court, taken: seats - room.open(), seats, time: now(), code: String::new() }
    }

    pub fn open(&self) -> usize {
        self.seats - self.taken
    }

    /// What to join it by: its ID, or a slot-only lobby's (a browser's) short code (B116).
    pub fn dial(&self) -> &str {
        if self.code.is_empty() { &self.id } else { &self.code }
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
    let json = serde_json::to_string(&Listing { id: String::new(), time: 0, code: String::new(), ..l.clone() }).ok()?;
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
    Hello {
        build: String,
        picks: Vec<Pick>,
        #[serde(default)]
        name: String,
    },
    Room(Room),
    Refused(String),
    Start(Room),
    /// The host's round-trip probe (its clock in µs) and the guest's echo.
    Ping(u64),
    Pong(u64),
    /// In the match (`play/netplay.rs`): seats' pads for one tick (`Pad` as `[buttons lo, hi, rx, ry, lx, ly]`), a
    /// peer's state hash before a tick, a pause opened or closed, a peer ending the match (why).
    Pads { tick: u32, seats: Vec<(u8, [u8; 6])> },
    /// The court intro's ticks before the match (B42e), as `Pads` (a seat's ✕ skips it for everyone).
    Intro { tick: u32, seats: Vec<(u8, [u8; 6])> },
    Hash { tick: u32, hash: u64 },
    Pause(bool),
    Bye(String),
    /// The lobby with mods (M2i): a peer asks the host for a mod's looks (its pack hash; the host asks the mod's
    /// owner in turn), and a peer has every mod's looks or stand-in settled for line-up `ver`.
    Want(u64),
    Ready(u32),
    /// B281: a peer has (or no longer has) a mod it hasn't chosen to download or not.
    Deciding(bool),
    /// In the lobby: a guest is ready to play (or not any more).
    Prepared(bool),
    /// B215: a guest stops the start's countdown.
    Cancel,
    /// In the lobby (B195): a guest's chosen picks for the seats it holds (it joined on placeholders).
    Picks(Vec<Pick>),
    /// B201: a guest's players' places on the select, as they move (`Room::live`).
    Live(Vec<Live>),
    /// After a desync (B154b, `play/resync.rs`): the host's match state before a tick in pieces (`last` ends it; a
    /// browser's data channel takes ~256 KB a message), and a peer having taken it (the tick it's in sync from).
    State { tick: u32, data: Vec<u8>, last: bool },
    Resynced(u32),
    /// A peer gone silent (B196, `play/recover.rs`): every peer's keep-alive; the host's word on who is away (peer
    /// names; empty: nobody), its countdown, and whether it asks what to do now; a peer's answer (true: a bot plays
    /// on, false: the match ends); the match going on from the host's state before `tick` with the line-up `room`
    /// (a bot in a seat, or a player back), in pieces (`end` closes it), `last` every seat's pads the tick before.
    Alive(String),
    Away { who: Vec<String>, left: u32, ask: bool },
    Choose(bool),
    Restart { round: u8, tick: u32, room: Room, last: [[u8; 6]; 4], data: Vec<u8>, end: bool },
    /// The host's network to its match only: player `old` came back as peer `new`.
    Rejoined { old: String, new: String, name: String },
    /// The players left chose to end it: everyone still in goes back to the lobby `Room` together (B196).
    Lobby(Room),
    /// B200: a peer's match is past its loading screen (the match starts once every peer's is).
    Loaded(String),
    /// M20: a chat line (`chat.rs`), in the lobby and the match alike; never in the match's input. A guest's goes to
    /// the host with `from` and `name` empty; the host fills them in (the sender's peer and lobby name), cleans and
    /// rate-limits it and passes it to everyone else.
    Chat { from: String, name: String, text: String },
}

impl Msg {
    /// A match message (relayed to every peer).
    fn in_match(&self) -> bool {
        matches!(
            self,
            Msg::Pads { .. } | Msg::Intro { .. } | Msg::Hash { .. } | Msg::Pause(_) | Msg::Bye(_) | Msg::State { .. } | Msg::Resynced(_) | Msg::Alive(_) | Msg::Away { .. } | Msg::Choose(_) | Msg::Restart { .. } | Msg::Lobby(_) | Msg::Loaded(_)
        )
    }
}

/// `HST_NET_LAG=ms`: every message waits this long before it goes out (simulated one-way latency, for tests).
pub static LAG_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// B215: the start's countdown (ms; tests shorten it).
pub static COUNTDOWN_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(4000);

/// The input delay for a round trip of `rtt`: enough ticks to cover one way, plus one for jitter (2..=15).
pub fn delay_for(rtt: Duration) -> u32 {
    ((rtt.as_secs_f64() / 2.0 * 60.0).ceil() as u32 + 1).clamp(2, 15)
}

/// Messages out through one stream (or the browser's data channel, `rtc.rs`), each after [`LAG_MS`], in order.
#[derive(Clone)]
struct Link(amp::UnboundedSender<(n0_future::time::Instant, Msg)>);

/// A peer connection on either transport: datagrams out, and closing it (its messages go through a [`Link`]).
#[derive(Clone)]
enum Conn {
    Quic(Connection),
    /// The browser on the LAN (M2k).
    #[cfg(target_arch = "wasm32")]
    Rtc(rtc::Peer),
}

impl Conn {
    fn send_datagram(&self, d: Vec<u8>) {
        match self {
            Conn::Quic(c) => drop(c.send_datagram(d.into())),
            #[cfg(target_arch = "wasm32")]
            Conn::Rtc(p) => p.send_datagram(&d),
        }
    }

    fn close(&self, why: &[u8]) {
        match self {
            Conn::Quic(c) => c.close(0u32.into(), why),
            #[cfg(target_arch = "wasm32")]
            Conn::Rtc(p) => p.close(),
        }
    }
}

/// What arrives on a connection: messages (an `Err` ends it) and datagrams.
struct Inbox {
    msgs: amp::UnboundedReceiver<Result<Msg, String>>,
    dgrams: amp::UnboundedReceiver<Vec<u8>>,
}

/// A QUIC connection with its stream and datagrams read on their own tasks (`recv` isn't cancel-safe, so it can't sit
/// in a `select!`: a command mid-read lost bytes).
fn quic(conn: Connection, tx: SendStream, mut rx: RecvStream) -> (Conn, Link, Inbox) {
    let (mt, msgs) = amp::unbounded_channel();
    n0_future::task::spawn(async move {
        loop {
            let m = recv(&mut rx).await;
            let end = m.is_err();
            if mt.send(m).is_err() || end {
                return;
            }
        }
    });
    let (dt, dgrams) = amp::unbounded_channel();
    let c = conn.clone();
    n0_future::task::spawn(async move {
        while let Ok(d) = c.read_datagram().await {
            if dt.send(d.to_vec()).is_err() {
                return;
            }
        }
    });
    (Conn::Quic(conn), Link::new(tx), Inbox { msgs, dgrams })
}

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
    // a lobby message with mods carries their skeletons (~30 KB a mod)
    if n > 1 << 20 {
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
    /// This player's name for the lobby (sent in the Hello; before Host/Join/Quick).
    Name(String),
    /// Leave the lobby (or stop hosting, or leave the match).
    Leave,
    /// In the match: to every other peer.
    Send(Msg),
    /// In the match: a datagram (unreliable, unordered: rollback's messages) to one peer, through the host.
    Datagram(String, Vec<u8>),
    /// In the lobby: a mod's looks wanted (its pack hash), and this machine settled for the room's line-up `ver`.
    Want(u64),
    Ready(u32),
    /// B281: this machine has (or no longer has) a mod it hasn't chosen to download or not.
    Deciding(bool),
    /// In the lobby: a guest ready or not; the host starts the match (the open seats played by the computer) or takes a
    /// peer's players out.
    Prepared(bool),
    Start,
    Kick(String),
    /// B215: stop the start's countdown (Start begins it; the host's or a guest's).
    Cancel,
    /// In the lobby (B195): this machine's chosen picks for the seats it holds; the host's court, sets, games, umpire.
    Picks(Vec<Pick>),
    Venue(usize, usize, i32, u8),
    /// M8f3: the host's umpire mod (id, name) alongside `Venue`'s umpire (its donor), or none.
    UmpireMod(Option<(String, String)>),
    /// B201: this machine's players' places on the select; the host's computer player in a seat (`None` opens it);
    /// the host's singles (false) or doubles.
    Live(Vec<Live>),
    Bot(usize, Option<Pick>),
    Doubles(bool),
    /// B213: the host's lobby private (unlisted) or public.
    Private(bool),
    /// The host's lobby vanilla (no mods) or not.
    Vanilla(bool),
    /// M20: a chat line to everyone else in the lobby or match.
    Chat(String),
}

/// What the network tells the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Status(String),
    /// This machine hosts the lobby (Quickplay found none to join).
    Hosting,
    /// Hosting: the code (ticket) to share.
    Code(String),
    /// B201: this machine's peer name in the lobby's seats (a guest's endpoint ID; the host is "host").
    Me(String),
    /// Hosting: the short code that also finds the lobby (B104).
    Short(String),
    Found(Vec<Listing>),
    /// The lobby as it is now (host and joiners alike).
    Room(Room),
    /// The host started: the match's line-up and which peer this machine is (its seats' `peer`). The connections stay
    /// open for the match.
    Start(Room, String),
    Failed(String),
    /// In the match: another peer's message.
    Match(Msg),
    /// In the match: a peer is gone (why).
    Dropped(String),
    /// In the match: a peer's datagram (`Cmd::Datagram`), with who sent it.
    Datagram(String, Vec<u8>),
    /// In the lobby: a mod's looks (`Cmd::Want`): its hash and the pack, as received (unchecked).
    Pack(u64, Vec<u8>),
    /// M20: another peer's chat line (sender's peer, name, text), cleaned by the host. `Net::poll` keeps these for
    /// `Net::chats`.
    Chat { from: String, name: String, text: String },
}

/// The network thread's handle (a Bevy resource in the menu).
#[derive(bevy::prelude::Resource)]
pub struct Net {
    cmd: amp::UnboundedSender<Cmd>,
    events: std::sync::Mutex<mpsc::Receiver<Event>>,
    /// M20: the chat lines `poll` took out, for `chats`; whether this machine is in a lobby or match (chat is open).
    chats: std::sync::Mutex<Vec<(String, String, String)>>,
    in_room: std::sync::atomic::AtomicBool,
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
        Net { cmd, events: events.into(), chats: Default::default(), in_room: Default::default() }
    }

    /// A `Net` with no network behind it: what it's told to send comes out of the receiver (tests).
    #[cfg(test)]
    pub fn wired() -> (Net, amp::UnboundedReceiver<Cmd>) {
        let (cmd, rx) = amp::unbounded_channel();
        let (_, events) = mpsc::channel();
        (Net { cmd, events: events.into(), chats: Default::default(), in_room: Default::default() }, rx)
    }

    /// This test `Net` in a lobby (`in_room`), as a `Room` or `Hosting` event would put it.
    #[cfg(test)]
    pub fn enter_room(&self) {
        self.in_room.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn send(&self, c: Cmd) {
        if matches!(c, Cmd::Leave | Cmd::Find | Cmd::Join(..) | Cmd::Quick(_)) {
            self.in_room.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        let _ = self.cmd.send(c);
    }

    /// M20: the chat lines heard since the last call (`poll` takes them out of the events).
    pub fn chats(&self) -> Vec<(String, String, String)> {
        std::mem::take(&mut self.chats.lock().unwrap())
    }

    /// M20: this machine is in a lobby or its match (seated or hosting).
    pub fn in_room(&self) -> bool {
        self.in_room.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// What came in, up to a `Start` (what follows it is the match's: B196, a rejoin's state right behind it).
    pub fn poll(&self) -> Vec<Event> {
        let rx = self.events.lock().unwrap();
        let mut v = Vec::new();
        while let Ok(e) = rx.try_recv() {
            let start = matches!(e, Event::Start(..));
            match &e {
                Event::Room(_) | Event::Start(..) | Event::Hosting => self.in_room.store(true, std::sync::atomic::Ordering::Relaxed),
                Event::Failed(_) | Event::Dropped(_) => self.in_room.store(false, std::sync::atomic::Ordering::Relaxed),
                _ => {}
            }
            if let Event::Chat { from, name, text } = e {
                self.chats.lock().unwrap().push((from, name, text));
                continue;
            }
            v.push(e);
            if start {
                break;
            }
        }
        v
    }
}

// ---------------------------------------------------------------- the network thread

struct Ctx {
    /// None: the browser on the LAN, by WebRTC (`rtc.rs`).
    ep: Option<Endpoint>,
    dht: Dht,
    /// The lobbies heard on the LAN, kept current by mDNS.
    lan: Lan,
    ev: mpsc::Sender<Event>,
    /// This player's name (`Cmd::Name`).
    name: String,
    /// The relays are on (n0 or own): the codes need one, as browsers reach peers only through relays (B175).
    relays: bool,
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
        if let Event::Failed(why) = &e {
            bevy::log::info!("online: {why}");
        }
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
    // B224: no waiting for the home relay here (up to 10 s before Host could show a code): the short code's ticket
    // and a join wait for it themselves
    let ep = b.bind().await.map_err(|e| e.to_string())?;
    // LAN: bare IDs resolve by mDNS and hosts' listings ride on it (no internet needed)
    let mdns = MdnsAddressLookup::builder().service_name(MDNS).build(ep.id()).map_err(|e| format!("mDNS: {e}"))?;
    ep.address_lookup().map_err(|e| e.to_string())?.add(mdns.clone());
    Ok((ep, mdns))
}

// The browser has no UDP: no DHT and no mDNS, only the relays. A code (ticket) carries the host's relay, so joining by
// code works; Find and Quickplay see lobbies through the slot listings (`find_slots`), not the DHT board.
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
async fn announce(_: &Ctx, _: u16, _: &Room) {}

#[cfg(target_arch = "wasm32")]
pub async fn endpoint(relay: &Relay) -> Result<(Endpoint, ()), String> {
    let mode = match relay {
        Relay::N0 => RelayMode::Default,
        Relay::Own(url) => RelayMode::Custom(url.parse::<RelayUrl>().map_err(|e| format!("relay_url {url}: {e}"))?.into()),
        Relay::Off => unreachable!("the browser's LAN is WebRTC (`bind`)"),
    };
    // the short code's ticket waits for the home relay (nothing else reaches a browser), not the bind (B224)
    let ep = Endpoint::builder(presets::Minimal).secret_key(SecretKey::generate()).alpns(vec![ALPN.to_vec()]).relay_mode(mode).bind().await.map_err(|e| e.to_string())?;
    Ok((ep, ()))
}

#[cfg(not(target_arch = "wasm32"))]
async fn bind(relay: &Relay) -> Result<(Option<Endpoint>, MdnsAddressLookup), String> {
    endpoint(relay).await.map(|(ep, m)| (Some(ep), m))
}

/// The browser with the relay off: no iroh at all, WebRTC on the LAN (M2k).
#[cfg(target_arch = "wasm32")]
async fn bind(relay: &Relay) -> Result<(Option<Endpoint>, ()), String> {
    match relay {
        Relay::Off => Ok((None, ())),
        _ => endpoint(relay).await.map(|(ep, m)| (Some(ep), m)),
    }
}

async fn run(cfg: Online, mut cmds: amp::UnboundedReceiver<Cmd>, ev: mpsc::Sender<Event>) {
    let _ = ev.send(Event::Status("Connecting...".into()));
    // timed (B151): binding with a home relay ~3 s; the short code ~1-2 s more (DHT), the pkarr relay's copy 5-20 s
    let t0 = n0_future::time::Instant::now();
    let (ep, mdns) = match bind(&cfg.relay).await {
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
    let mut cx = Ctx { ep, dht, lan: watch_lan(mdns), ev, name: cfg.name.clone(), relays: cfg.relay != Relay::Off };
    bevy::log::info!("online in {:?}", t0.elapsed());
    cx.say(Event::Status("Online.".into()));
    // B321: a Host, Join, Quickplay or Find that came in during a Find (dropped for it)
    let mut cut: Option<Cmd> = None;
    loop {
        let Some(c) = (match cut.take() {
            Some(c) => Some(c),
            None => cmds.recv().await,
        }) else {
            break;
        };
        match c {
            Cmd::Host(room) => host(&cx, room, &mut cmds).await,
            Cmd::Find => {
                cx.say(Event::Status("Looking for lobbies...".into()));
                // B267: the LAN's wait and the DHT's lookups at once, each list shown as it answers
                let mut f = Finding::start(&cx);
                let mut name = None;
                // B321: the Online screen's search (the browser's slot reads take 7-12 s) never holds up a Host
                // chosen meanwhile: the first host after a page load showed no code until it was over
                loop {
                    tokio::select! {
                        list = f.next(&cx) => match list {
                            Some(list) => cx.say(Event::Found(list)),
                            None => break,
                        },
                        c = cmds.recv() => match c {
                            Some(Cmd::Name(n)) => name = Some(n),
                            Some(c @ (Cmd::Host(_) | Cmd::Join(..) | Cmd::Quick(_) | Cmd::Find)) => {
                                cut = Some(c);
                                break;
                            }
                            Some(_) => {}
                            None => return,
                        },
                    }
                }
                drop(f);
                if let Some(n) = name {
                    cx.name = n;
                }
            }
            Cmd::Quick(room) => {
                cx.say(Event::Status("Looking for a lobby...".into()));
                let mine: Vec<Pick> = room.seats.iter().flatten().map(|s| s.pick.clone()).collect();
                let mut joined = false;
                // B267: the first fitting lobby from whichever list answers first (each tried once)
                let mut tried: Vec<String> = Vec::new();
                let mut f = Finding::start(&cx);
                'look: while let Some(list) = f.next(&cx).await {
                    for l in quick_pick(&list, room.doubles, mine.len()) {
                        if tried.contains(&l.id) {
                            continue;
                        }
                        tried.push(l.id.clone());
                        if join(&cx, l.dial(), mine.clone(), &mut cmds).await {
                            joined = true;
                            break 'look;
                        }
                    }
                }
                drop(f);
                if !joined {
                    host(&cx, room, &mut cmds).await;
                }
            }
            Cmd::Join(code, picks) => {
                join(&cx, &code, picks, &mut cmds).await;
            }
            Cmd::Name(n) => cx.name = n,
            _ => {}
        }
    }
    if let Some(ep) = &cx.ep {
        ep.close().await;
    }
}

/// How many lists Find sends (`Event::Found`): native the LAN's, the DHT board's and the short-code slots', each
/// merged with those in so far; the browser only the slots'.
pub const FINDS: u8 = if cfg!(target_arch = "wasm32") { 1 } else { 3 };

/// One of Find's lookups: which answer (0 the board, 1 the slots, 2 the LAN's wait) and its lobbies.
type Lookup<'a> = std::pin::Pin<Box<dyn Future<Output = (usize, Vec<Listing>)> + 'a>>;

/// B267: Find's lookups all running at once; `next` gives the open lobbies so far as each answers.
struct Finding<'a> {
    left: n0_future::FuturesUnordered<Lookup<'a>>,
    /// The DHT board's and the slots' answers.
    dht: [Vec<Listing>; 2],
}

impl<'a> Finding<'a> {
    fn start(cx: &'a Ctx) -> Finding<'a> {
        let mut left = n0_future::FuturesUnordered::<Lookup<'a>>::new();
        #[cfg(not(target_arch = "wasm32"))]
        {
            // the LAN's is mDNS's, heard meanwhile: this only says when it's had its time
            left.push(Box::pin(async {
                n0_future::time::sleep(LAN_WAIT).await;
                (2, vec![])
            }));
            left.push(Box::pin(async { (0, find(&cx.dht).await) }));
        }
        left.push(Box::pin(async { (1, find_slots(&cx.dht).await) }));
        Finding { left, dht: Default::default() }
    }

    /// The open lobbies on the LAN and the DHT once one more lookup answered (the LAN's copy of one listed on both);
    /// None once all have.
    async fn next(&mut self, cx: &Ctx) -> Option<Vec<Listing>> {
        let (i, l) = n0_future::StreamExt::next(&mut self.left).await?;
        if let Some(d) = self.dht.get_mut(i) {
            *d = l;
        }
        let mut list = lan_list(&cx.lan);
        // the first copy of each: the LAN's (its full lobbies too), then the DHT board's (dialled by ID: a native host
        // may have no code, relays off), then a slot's (a browser's, joined by its code)
        let mut seen: Vec<String> = cx.lan.lock().unwrap().keys().map(|id| id.to_string()).collect();
        for l in self.dht.iter().flatten() {
            if !seen.contains(&l.id) {
                seen.push(l.id.clone());
                list.push(l.clone());
            }
        }
        // B332: never this machine's own lobby (its listing lingers on the DHT and the LAN after it stops hosting)
        Some(not_mine(list, cx.ep.as_ref().map(|ep| ep.id().to_string())))
    }
}

/// `list` without the lobby listed by endpoint `me` (this machine's), so Find, the Online screen's card and Quickplay
/// never offer it.
fn not_mine(mut list: Vec<Listing>, me: Option<String>) -> Vec<Listing> {
    list.retain(|l| Some(&l.id) != me.as_ref());
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
    let Some(ep) = &cx.ep else { return };
    let mut l = Listing::of(ep.id(), room);
    if room.private {
        // B213: gone private: shown as full, which Find and Quickplay skip
        l.taken = l.seats;
    }
    let value = seal(ep.secret_key(), n, &l);
    // (µs: B213's Private / Public a second apart still overwrite each other)
    let seq = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_micros()) as i64;
    let item = MutableItem::new(&SigningKey::from_bytes(BOARD), &value, seq, Some(&salt(n)));
    if let Err(e) = cx.dht.put_mutable(item, None).await {
        bevy::log::warn!("lobby listing: {e}");
    }
    if let Err(e) = cx.dht.announce_peer(Id::from_bytes(TOPIC).unwrap(), Some(n)).await {
        bevy::log::warn!("lobby announce: {e}");
    }
}

/// A joiner's side, inside the host task.
enum Guest {
    In { peer: String, name: String, picks: Vec<Pick>, build: String, link: Link, conn: Conn },
    Said(String, Msg),
    Gone(String),
    /// A datagram from the first peer for the second.
    Datagram(String, String, Vec<u8>),
    /// A mod's looks from its owner: hash, pack.
    Pack(u64, Vec<u8>),
}

/// Mod packs (M2i) go on their own uni streams, one each: the hash (u64 LE), then the pack. Each received one goes to
/// `out` as it was sent.
async fn read_packs(c: Conn, out: impl Fn(u64, Vec<u8>)) {
    // ponytail: QUIC only; the browser's LAN (M2k) gets stand-ins, add a pack channel to rtc.rs if wanted
    #[allow(irrefutable_let_patterns)]
    let Conn::Quic(c) = c else { return };
    while let Ok(mut r) = c.accept_uni().await {
        if let Ok(b) = r.read_to_end(crate::mods_online::CAP + 8).await
            && b.len() >= 8
        {
            out(u64::from_le_bytes(b[..8].try_into().unwrap()), b[8..].to_vec());
        }
    }
}

fn send_pack(c: &Conn, h: u64, b: std::sync::Arc<Vec<u8>>) {
    #[allow(irrefutable_let_patterns)]
    let Conn::Quic(c) = c.clone() else { return };
    n0_future::task::spawn(async move {
        if let Ok(mut s) = c.open_uni().await {
            let _ = s.write_all(&h.to_le_bytes()).await;
            let _ = s.write_all(&b).await;
            let _ = s.finish();
        }
    });
}

type Packs = std::collections::HashMap<u64, std::sync::Arc<Vec<u8>>>;

/// Peer `to` wants mod `h`'s looks: the pack if the host has it (its own mod's, or one passed through already), else
/// the mod's owner is asked (once) and `to` waits for it.
fn want(to: &str, h: u64, room: &Room, joined: &[(String, Link, Conn)], packs: &mut Packs, wants: &mut std::collections::HashMap<u64, Vec<String>>) -> Option<std::sync::Arc<Vec<u8>>> {
    if let Some(b) = packs.get(&h) {
        return Some(b.clone());
    }
    let (seat, _) = room.mods().find(|(_, w)| w.looks == h)?;
    // B216: the host's computer players' mods are the host's own
    if seat.peer == "host" || seat.peer == CPU {
        let b = std::sync::Arc::new(own_pack(room.seats.iter().flatten().filter(|s| s.peer == "host" || s.peer == CPU).map(|s| s.pick.clone()), h)?);
        packs.insert(h, b.clone());
        return Some(b);
    }
    let waiting = wants.entry(h).or_default();
    if waiting.is_empty()
        && let Some((_, l, _)) = joined.iter().find(|(p, ..)| *p == seat.peer)
    {
        l.send(&Msg::Want(h));
    }
    waiting.push(to.into());
    None
}

/// Mod `h`'s pack, if this machine has the mod (its own pick).
fn own_pack(picks: impl IntoIterator<Item = Pick>, h: u64) -> Option<Vec<u8>> {
    let w = picks.into_iter().filter_map(|p| p.r#mod).find(|w| w.looks == h)?;
    let b = crate::mods_online::pack(w.dir.as_deref()?).ok()?;
    (crate::mods_online::hash(&b) == h).then_some(b)
}

/// The host's clock for the round-trip probes, in µs.
fn micros(t0: n0_future::time::Instant) -> u64 {
    t0.elapsed().as_micros() as u64
}

/// A guest's arrivals into the host task: its Hello first (else it's dropped), then what it says and sends, then gone.
async fn guest(gtx: amp::UnboundedSender<Guest>, peer: String, conn: Conn, link: Link, mut inbox: Inbox) {
    let Some(Ok(Msg::Hello { build, picks, name })) = inbox.msgs.recv().await else {
        conn.close(b"no hello");
        return;
    };
    let g = gtx.clone();
    n0_future::task::spawn(read_packs(conn.clone(), move |h, b| drop(g.send(Guest::Pack(h, b)))));
    let _ = gtx.send(Guest::In { peer: peer.clone(), name, picks, build, link, conn });
    loop {
        tokio::select! {
            m = inbox.msgs.recv() => match m {
                Some(Ok(m)) => drop(gtx.send(Guest::Said(peer.clone(), m))),
                _ => break,
            },
            Some(d) = inbox.dgrams.recv() => if let Some((to, data)) = undatagram(&d) {
                let _ = gtx.send(Guest::Datagram(peer.clone(), to, data.to_vec()));
            },
        }
    }
    let _ = gtx.send(Guest::Gone(peer));
}

/// The next iroh connection (never, on the browser's LAN).
async fn accept(ep: &Option<Endpoint>) -> Option<iroh::endpoint::Incoming> {
    match ep {
        Some(ep) => ep.accept().await,
        None => std::future::pending().await,
    }
}

/// The browser's LAN: a fresh WebRTC offer, its code shown.
async fn new_offer(cx: &Ctx) -> Option<rtc::Offer> {
    match rtc::offer().await {
        Ok((o, code)) => {
            bevy::log::info!("LAN offer: {code}");
            cx.say(Event::Code(code));
            Some(o)
        }
        Err(e) => {
            cx.say(Event::Failed(e));
            None
        }
    }
}

/// Refuse a guest (`why`), closing once the refusal has had time to arrive.
fn refuse(link: &Link, conn: Conn, why: String) {
    link.send(&Msg::Refused(why));
    n0_future::task::spawn(async move {
        n0_future::time::sleep(Duration::from_secs(2) + Duration::from_millis(LAG_MS.load(std::sync::atomic::Ordering::Relaxed))).await;
        conn.close(b"refused");
    });
}

async fn host(cx: &Ctx, mut room: Room, cmds: &mut amp::UnboundedReceiver<Cmd>) {
    room.names.insert("host".into(), cx.name.clone());
    cx.say(Event::Hosting);
    // the lobby first: the codes follow as they're ready (B151: the short code took ~30 s before the screen came up)
    cx.say(Event::Room(room.clone()));
    // iroh: the ticket, and the short code rolled in the background; the browser's LAN: a WebRTC offer per guest, a
    // fresh one each time one is answered
    let mut answers = rtc::answers();
    let mut offer = None;
    let mut code: Option<String> = None;
    let (ctx_, mut rolled) = amp::unbounded_channel::<Option<String>>();
    // B266: the host's home relay, last seen (its code's record keeps it while a net report has none)
    let (htx, mut homes) = amp::unbounded_channel::<RelayUrl>();
    let mut home: Option<RelayUrl> = None;
    match &cx.ep {
        Some(ep) => {
            cx.say(Event::Code(EndpointTicket::new(ep.addr()).to_string()));
            // B224: the short code shown at once, checked and published in the background (re-rolled if taken)
            let first = new_short();
            bevy::log::info!("short code {first} shown");
            cx.say(Event::Short(first.clone()));
            cx.say(Event::Status("Putting the code online...".into()));
            let mut addrs = iroh::Watcher::stream(ep.watch_addr());
            let (dht, ep, relays, ev) = (cx.dht.clone(), ep.clone(), cx.relays, cx.ev.clone());
            n0_future::task::spawn(async move {
                let t = n0_future::time::Instant::now();
                // B175: a browser dials only the ticket's relay, so a code without one (the relay slower than bind's
                // 10 s) would time out there; native joiners would still find the host by the DHT
                // B266: and never without one while the relays are on (the 30 s cap published a relay-less code,
                // which a browser read as Relay off): a code without a relay is a Relay off host's
                while relays && ep.addr().relay_urls().next().is_none() {
                    if n0_future::time::timeout(Duration::from_secs(30), ep.online()).await.is_err() {
                        if ctx_.is_closed() {
                            return;
                        }
                        let _ = ev.send(Event::Status("Waiting for a relay to put the code online (no internet?)...".into()));
                    }
                }
                // B114: relays off, right after bind the address can still be empty; a bare ID in the code fails
                // the join with "no addressing information" when the DHT has no record of the host yet
                for _ in 0..50 {
                    if ep.addr().ip_addrs().next().is_some() || ep.addr().relay_urls().next().is_some() {
                        break;
                    }
                    n0_future::time::sleep(Duration::from_millis(100)).await;
                }
                bevy::log::info!("ticket with its addresses in {:?}", t.elapsed());
                // B321: a put that fails (the pkarr relay answers 429 for a while after Find's slot reads, the first
                // host after a page load) is tried again; meanwhile the lobby shows its whole ticket as the link
                let shown = std::sync::Mutex::new(first);
                let mut pause = Duration::from_secs(2);
                let c = loop {
                    let ticket = code_ticket(&ep, None);
                    let code = shown.lock().unwrap().clone();
                    let c = roll(&dht, code, &ticket, |c| {
                        *shown.lock().unwrap() = c.into();
                        drop(ev.send(Event::Short(c.into())))
                    })
                    .await;
                    if c.is_some() || !relays || ctx_.is_closed() {
                        break c;
                    }
                    bevy::log::warn!("short code not out in {:?}: the ticket instead, again in {pause:?}", t.elapsed());
                    bevy::log::info!("lobby link {}", link(&ticket));
                    let _ = ev.send(Event::Short(String::new()));
                    let _ = ev.send(Event::Code(ticket));
                    let _ = ev.send(Event::Status("No short code yet (retrying): share the lobby's link meanwhile.".into()));
                    n0_future::time::sleep(pause).await;
                    pause = (pause * 2).min(Duration::from_secs(30));
                };
                bevy::log::info!("short code {c:?} in {:?}", t.elapsed());
                let _ = ctx_.send(c);
            });
            // B266: the home relay as it changes (a new one picked, or a net report without one), so the code's
            // record follows it at once instead of sending browsers to the old relay until the next REPUBLISH
            n0_future::task::spawn(async move {
                let mut last = None;
                while let Some(a) = addrs.next().await {
                    let r = a.relay_urls().next().cloned();
                    if r.is_some() && r != last {
                        bevy::log::info!("home relay {r:?}");
                        last = r.clone();
                        if htx.send(r.unwrap()).is_err() {
                            return;
                        }
                    }
                }
            });
        }
        None => {
            offer = new_offer(cx).await;
            cx.say(Event::Status("Players scan this code, then you scan the one their screen shows (or paste it here).".into()));
        }
    }
    let (gtx, mut guests) = amp::unbounded_channel::<Guest>();
    let mut joined: Vec<(String, Link, Conn)> = Vec::new();
    let mut tick = n0_future::time::interval(REPUBLISH);
    let mut dirty = true;
    // B275: the code's record goes out every REPUBLISH, not on every lobby change (the pkarr relay allows 10 puts)
    let mut republish = true;
    let mut listed = false;
    // the board slot this lobby is listed in (B116; SLOTS: none yet)
    let slot = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(SLOTS));
    // the round trips: each guest's last, and (full) the guests not yet answered since the start's probe, until when
    let t0 = n0_future::time::Instant::now();
    let mut rtt: std::collections::HashMap<String, Duration> = Default::default();
    // a host with every seat (`HST_NET_SYNCTEST`) starts at once
    let mut starting: Option<(Vec<String>, n0_future::time::Instant)> = room.seats.iter().all(Option::is_some).then(|| (Vec::new(), t0));
    // B215: the countdown's end (Start, or everyone ready), then the match starts once (with mods) everyone's settled;
    // `auto`: everyone being ready may start one (not again after a cancel until someone readies)
    let mut go = false;
    let mut count: Option<n0_future::time::Instant> = None;
    let mut auto = true;
    // B281: after Start, until when the players still choosing about a mod may (then they get its stand-in)
    let mut wait: Option<n0_future::time::Instant> = None;
    let mut ping = n0_future::time::interval(Duration::from_secs(1));
    // mods (M2i): the packs passed through, who waits for which, who has settled every mod's looks for this line-up
    let mut packs: Packs = Default::default();
    let mut wants: std::collections::HashMap<u64, Vec<String>> = Default::default();
    let mut ready: std::collections::HashSet<String> = Default::default();
    let mut rates = Rates::default();
    let deliver = |to: &str, h: u64, b: &std::sync::Arc<Vec<u8>>, joined: &[(String, Link, Conn)]| {
        if to == "host" {
            cx.say(Event::Pack(h, b.to_vec()));
        } else if let Some((.., c)) = joined.iter().find(|(p, ..)| p == to) {
            send_pack(c, h, b.clone());
        }
    };
    loop {
        // started, everyone ready, and with mods everyone has their looks or stand-ins: a last round trip first
        if starting.is_none() && count.is_none() && !go && auto && room.all_set() {
            auto = false;
            count = Some(n0_future::time::Instant::now() + Duration::from_millis(COUNTDOWN_MS.load(std::sync::atomic::Ordering::Relaxed)));
            room.count = left(count);
            broadcast(&joined, &Msg::Room(room.clone()));
            cx.say(Event::Room(room.clone()));
        }
        if starting.is_none()
            && go
            && (room.mods().next().is_none() || std::iter::once("host").chain(joined.iter().map(|(p, ..)| p.as_str())).all(|p| ready.contains(p)))
        {
            // a fresh round trip from everyone before the input delay is picked (3 s at most)
            broadcast(&joined, &Msg::Ping(micros(t0)));
            starting = Some((joined.iter().map(|(p, ..)| p.clone()).collect(), n0_future::time::Instant::now() + Duration::from_secs(3)));
        }
        // B281: Start waits DECIDE_MS for the players still choosing about a mod; none left or no start: no wait
        let was = (room.wait, room.stand_in);
        if !go || room.deciding.is_empty() {
            wait = None;
            room.wait = 0;
        } else if wait.is_none() && !room.stand_in && starting.is_none() {
            wait = Some(n0_future::time::Instant::now() + Duration::from_millis(DECIDE_MS.load(std::sync::atomic::Ordering::Relaxed)));
            room.wait = left(wait);
        }
        room.stand_in &= go;
        if was != (room.wait, room.stand_in) {
            broadcast(&joined, &Msg::Room(room.clone()));
            cx.say(Event::Room(room.clone()));
        }
        if dirty {
            dirty = false;
            // listing in the background: a slow DHT put never holds up a join
            if let Some(ep) = &cx.ep {
                let l = room.clone();
                // B213: a private lobby keeps only its short code; once, its public listing is overwritten as full
                let put = !l.private || std::mem::take(&mut listed);
                listed |= !l.private;
                ep.set_user_data_for_address_lookup(lan_data(&Listing::of(ep.id(), &l)).filter(|_| !l.private));
                let cx2 = Ctx { ep: cx.ep.clone(), dht: cx.dht.clone(), lan: cx.lan.clone(), ev: cx.ev.clone(), name: String::new(), relays: cx.relays };
                let n = lobby_number(ep.id());
                let (code, ticket) = (code.clone(), code_ticket(ep, home.as_ref()));
                let (id, slot) = (ep.id(), slot.clone());
                let again = std::mem::take(&mut republish);
                n0_future::task::spawn(async move {
                    if let Some(c) = code {
                        if again {
                            publish(&cx2.dht, &c, &ticket).await;
                        }
                        if put {
                            // B213: a private lobby's board slot is overwritten as full, like its DHT listing
                            let full = Listing::of(id, &l);
                            let taken = if l.private { full.seats } else { full.taken };
                            list_in_slot(&cx2.dht, &slot, &Listing { code: c, taken, ..full }).await;
                        }
                    }
                    if put {
                        announce(&cx2, n, &l).await
                    }
                });
            }
        }
        if let Some((waiting, until)) = &starting
            && (waiting.is_empty() || n0_future::time::Instant::now() >= *until)
        {
            // ponytail: no probe answer in time counts as 250 ms; a slower peer just stalls the match more
            let worst = joined.iter().map(|(p, ..)| rtt.get(p).copied().unwrap_or(Duration::from_millis(250))).max().unwrap_or_default();
            room.delay = delay_for(worst);
            room.lockstep = std::env::var_os("HST_NET_LOCKSTEP").is_some();
            room.seed = (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_nanos()) as u32) | 1;
            room.bots();
            broadcast(&joined, &Msg::Start(room.clone()));
            cx.say(Event::Status(format!("Round trip {} ms, input delay {} ticks.", worst.as_millis(), room.delay)));
            cx.say(Event::Start(room.clone(), "host".into()));
            if let Some(ep) = &cx.ep {
                ep.set_user_data_for_address_lookup(None);
            }
            // B196: back from the match to this lobby with the players still in
            let Some((r, j, g)) = relay(cx, room, joined, gtx.clone(), guests, cmds).await else { return };
            (room, joined, guests, starting, go, count, auto) = (r, j, g, None, false, None, true);
            ready.clear();
            dirty = true;
            continue;
        }
        tokio::select! {
            Some(inc) = accept(&cx.ep) => {
                let gtx = gtx.clone();
                n0_future::task::spawn(async move {
                    let Ok(conn) = async { inc.accept()?.await.map_err(Into::into) }.await.map_err(|e: Box<dyn std::error::Error + Send + Sync>| e) else { return };
                    let peer = conn.remote_id().to_string();
                    let Ok((tx, rx)) = conn.accept_bi().await else { return };
                    let (conn, link, inbox) = quic(conn, tx, rx);
                    guest(gtx, peer, conn, link, inbox).await;
                });
            }
            Some(a) = answers.recv() => match offer.take() {
                Some(o) if o.answers(&a) => {
                    let (gtx, ev) = (gtx.clone(), cx.ev.clone());
                    n0_future::task::spawn(async move {
                        match o.accept(&a).await {
                            Ok((peer, conn, link, inbox)) => guest(gtx, peer, conn, link, inbox).await,
                            Err(e) => drop(ev.send(Event::Status(e))),
                        }
                    });
                    cx.say(Event::Status("Connecting a player...".into()));
                    offer = new_offer(cx).await;
                }
                o => {
                    offer = o;
                    cx.say(Event::Status("That code answers an older lobby code: they need to scan the one shown now.".into()));
                }
            },
            Some(g) = guests.recv() => match g {
                Guest::In { peer, name, picks, build, link, conn } => {
                    let why = if build != BUILD {
                        Some(format!("different version (host {BUILD}, you {build})"))
                    } else if let Err(e) = picks_ok(&picks) {
                        Some(e)
                    } else if starting.is_some() || !room.take(&peer, &picks) {
                        Some(format!("not enough open seats ({} open, you bring {})", room.open(), picks.len()))
                    } else {
                        None
                    };
                    if let Some(why) = why {
                        refuse(&link, conn, why);
                        continue;
                    }
                    let name = if name.trim().is_empty() { short(&peer).to_string() } else { name.chars().take(16).collect() };
                    cx.say(Event::Status(format!("{name} joined.")));
                    room.names.insert(peer.clone(), name);
                    room.picking.insert(peer.clone());
                    link.send(&Msg::Ping(micros(t0)));
                    joined.push((peer, link, conn));
                    ready.clear();
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                }
                Guest::Said(peer, Msg::Ready(v)) if v == room.ver => drop(ready.insert(peer)),
                Guest::Said(peer, Msg::Deciding(d)) => deciding(cx, &joined, &mut room, peer, d),
                Guest::Said(peer, Msg::Prepared(p)) => {
                    let changed = if p { room.ready.insert(peer) } else { room.ready.remove(&peer) };
                    auto |= p;
                    if changed && !p && count.take().is_some() {
                        room.count = 0;
                    }
                    if changed {
                        broadcast(&joined, &Msg::Room(room.clone()));
                        cx.say(Event::Room(room.clone()));
                    }
                }
                Guest::Said(peer, Msg::Picks(p)) => {
                    let ok = room.allowed(&p);
                    // B275: a refused pick is a message to both, not a seat change (the guest stays)
                    if let Err(why) = &ok {
                        let name = room.names.get(&peer).cloned().unwrap_or_else(|| short(&peer).into());
                        cx.say(Event::Status(format!("{name}'s pick was refused: {why}")));
                        if let Some((_, l, _)) = joined.iter().find(|(p, ..)| *p == peer) {
                            l.send(&Msg::Chat { from: "host".into(), name: "Lobby".into(), text: format!("Your pick was refused: {why}") });
                        }
                    }
                    if starting.is_none() && ok.is_ok() && room.repick(&peer, &p) {
                        ready.clear();
                        broadcast(&joined, &Msg::Room(room.clone()));
                        cx.say(Event::Room(room.clone()));
                        dirty = true;
                    }
                }
                // B201: a guest's cursors, shown live to everyone (not a line-up change: no relisting, no ready reset)
                Guest::Said(peer, Msg::Live(l)) => {
                    if starting.is_none() && room.names.contains_key(&peer) && room.live.get(&peer) != Some(&l) {
                        room.live.insert(peer, l);
                        broadcast(&joined, &Msg::Room(room.clone()));
                        cx.say(Event::Room(room.clone()));
                    }
                }
                Guest::Said(_, Msg::Cancel) if count.take().is_some() => {
                    room.count = 0;
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                }
                Guest::Said(peer, Msg::Want(h)) => {
                    if let Some(b) = want(&peer, h, &room, &joined, &mut packs, &mut wants) {
                        deliver(&peer, h, &b, &joined);
                    }
                }
                Guest::Pack(h, b) if crate::mods_online::hash(&b) == h => {
                    let b = std::sync::Arc::new(b);
                    for to in wants.remove(&h).unwrap_or_default() {
                        deliver(&to, h, &b, &joined);
                    }
                    packs.insert(h, b);
                }
                Guest::Said(peer, Msg::Chat { text, .. }) => chat(cx, &peer, &text, &room.names, &joined, &mut rates),
                Guest::Said(peer, Msg::Pong(sent)) => {
                    rtt.insert(peer.clone(), Duration::from_micros(micros(t0).saturating_sub(sent)));
                    if let Some((waiting, _)) = &mut starting {
                        waiting.retain(|p| *p != peer);
                    }
                }
                Guest::Said(..) | Guest::Datagram(..) | Guest::Pack(..) => {}
                Guest::Gone(peer) => {
                    if !joined.iter().any(|(p, ..)| *p == peer) {
                        continue;
                    }
                    ready.clear();
                    let name = room.names.get(&peer).cloned().unwrap_or_else(|| short(&peer).into());
                    room.leave(&peer);
                    joined.retain(|(p, ..)| *p != peer);
                    (starting, go, count, room.count) = (None, false, None, 0);
                    cx.say(Event::Status(format!("{name} left.")));
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                }
            },
            _ = ping.tick() => {
                broadcast(&joined, &Msg::Ping(micros(t0)));
                // B370: the round trips the lobby's cards show, to everyone when one moves (5 ms steps)
                let ms: std::collections::BTreeMap<String, u32> = joined.iter().filter_map(|(p, ..)| Some((p.clone(), (rtt.get(p)?.as_millis() as u32).div_ceil(5) * 5))).collect();
                if starting.is_none() && ms != room.ping {
                    room.ping = ms;
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                }
            }
            // B215: the countdown's next second (shown on every peer), and at 0 the start
            _ = n0_future::time::sleep(next_second(count)), if count.is_some() => {
                room.count = left(count);
                if room.count == 0 {
                    (count, go) = (None, true);
                }
                broadcast(&joined, &Msg::Room(room.clone()));
                cx.say(Event::Room(room.clone()));
            }
            // B281: the wait's next second, and at 0 the stand-ins for every mod still not chosen
            _ = n0_future::time::sleep(next_second(wait)), if wait.is_some() => {
                room.wait = left(wait);
                if room.wait == 0 {
                    (wait, room.stand_in) = (None, true);
                }
                broadcast(&joined, &Msg::Room(room.clone()));
                cx.say(Event::Room(room.clone()));
            }
            _ = tick.tick() => (dirty, republish) = (true, true),
            Some(r) = homes.recv() => {
                if code.is_some() && home.as_ref() != Some(&r) {
                    (dirty, republish) = (true, true);
                }
                home = Some(r);
            }
            Some(c) = rolled.recv() => {
                code = c;
                cx.say(Event::Short(code.clone().unwrap_or_default()));
                // B322: the guests show it too
                room.short = code.clone().unwrap_or_default();
                broadcast(&joined, &Msg::Room(room.clone()));
                cx.say(Event::Room(room.clone()));
                cx.say(Event::Status(if code.is_some() { "Waiting for players..." } else { "No short code (no internet?): players on this network find the lobby with Find." }.into()));
                dirty = true;
            }
            c = cmds.recv() => match c {
                Some(Cmd::Ready(v)) if v == room.ver => drop(ready.insert("host".into())),
                Some(Cmd::Deciding(d)) => deciding(cx, &joined, &mut room, "host".into(), d),
                Some(Cmd::Start) if count.is_none() && !go && starting.is_none() => {
                    auto = false;
                    count = Some(n0_future::time::Instant::now() + Duration::from_millis(COUNTDOWN_MS.load(std::sync::atomic::Ordering::Relaxed)));
                    room.count = left(count);
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                }
                Some(Cmd::Cancel) if count.take().is_some() => {
                    room.count = 0;
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                }
                Some(Cmd::Picks(p)) if room.allowed(&p).is_err() => cx.say(Event::Status(format!("Pick refused: {}", room.allowed(&p).unwrap_err()))),
                Some(Cmd::Bot(_, Some(p))) if room.allowed(std::slice::from_ref(&p)).is_err() => {
                    cx.say(Event::Status(format!("Pick refused: {}", room.allowed(std::slice::from_ref(&p)).unwrap_err())));
                }
                Some(Cmd::Picks(p)) if starting.is_none() && room.repick("host", &p) => {
                    auto = true;
                    ready.clear();
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                }
                Some(Cmd::Live(l)) if starting.is_none() => {
                    room.live.insert("host".into(), l);
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                }
                Some(c @ (Cmd::Bot(..) | Cmd::Doubles(_))) if starting.is_none() => {
                    let changed = match c {
                        Cmd::Bot(i, p) => room.bot(i, p),
                        Cmd::Doubles(d) => room.set_doubles(d),
                        _ => false,
                    };
                    if changed {
                        ready.clear();
                        broadcast(&joined, &Msg::Room(room.clone()));
                        cx.say(Event::Room(room.clone()));
                        dirty = true;
                    }
                }
                Some(Cmd::Vanilla(v)) if starting.is_none() && room.set_vanilla(v) => {
                    ready.clear();
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                }
                Some(Cmd::Private(p)) if starting.is_none() && p != room.private => {
                    room.private = p;
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                }
                Some(Cmd::Venue(court, sets, games, umpire)) if starting.is_none() => {
                    (room.court, room.sets, room.games, room.umpire) = (court, sets, games, umpire);
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                }
                Some(Cmd::UmpireMod(u)) if starting.is_none() && u != room.umpire_mod => {
                    room.umpire_mod = u;
                    broadcast(&joined, &Msg::Room(room.clone()));
                    cx.say(Event::Room(room.clone()));
                    dirty = true;
                }
                Some(Cmd::Kick(peer)) => {
                    if let Some((_, l, c)) = joined.iter().find(|(p, ..)| *p == peer) {
                        refuse(l, c.clone(), "the host took you out of the lobby".into());
                        let name = room.names.get(&peer).cloned().unwrap_or_else(|| short(&peer).into());
                        ready.clear();
                        room.leave(&peer);
                        joined.retain(|(p, ..)| *p != peer);
                        (starting, go, count, room.count) = (None, false, None, 0);
                        cx.say(Event::Status(format!("{name} was taken out.")));
                        broadcast(&joined, &Msg::Room(room.clone()));
                        cx.say(Event::Room(room.clone()));
                        dirty = true;
                    }
                }
                Some(Cmd::Chat(t)) => chat(cx, "host", &t, &room.names, &joined, &mut rates),
                Some(Cmd::Want(h)) => {
                    if let Some(b) = want("host", h, &room, &joined, &mut packs, &mut wants) {
                        deliver("host", h, &b, &joined);
                    }
                }
                Some(Cmd::Leave) | None => {
                    for (_, _, conn) in &joined {
                        conn.close(b"host left");
                    }
                    if let Some(ep) = &cx.ep {
                        ep.set_user_data_for_address_lookup(None);
                    }
                    // an empty listing ages out in STALE; a full one isn't shown
                    return;
                }
                Some(_) => {}
            },
        }
    }
}

/// B281: `peer` still choosing about a mod (or not any more), shown to everyone.
fn deciding(cx: &Ctx, joined: &[(String, Link, Conn)], room: &mut Room, peer: String, d: bool) {
    if (d && room.names.contains_key(&peer) && room.deciding.insert(peer.clone())) || (!d && room.deciding.remove(&peer)) {
        broadcast(joined, &Msg::Room(room.clone()));
        cx.say(Event::Room(room.clone()));
    }
}

/// B215: the countdown ending at `end`: whole seconds left (rounded up; 0 when over or none).
fn left(end: Option<n0_future::time::Instant>) -> u32 {
    end.map_or(0, |e| e.saturating_duration_since(n0_future::time::Instant::now()).as_millis().div_ceil(1000) as u32)
}

/// B215: how long until the countdown's shown seconds drop (its end within the last second).
fn next_second(end: Option<n0_future::time::Instant>) -> Duration {
    let rest = end.map_or(Duration::ZERO, |e| e.saturating_duration_since(n0_future::time::Instant::now()));
    rest.saturating_sub(Duration::from_secs(left(end).saturating_sub(1) as u64))
}

/// The host in the match: every guest's match messages go to the app and the other guests, the app's to every guest;
/// a guest dropping ends the match for everyone.
/// The host in the match: passes the match's messages and datagrams on. B196: a guest's connection going away says
/// nothing (the match sees the silence, `play/recover.rs`); while the match says someone is away, a joiner (by the
/// same name first) takes their seats and gets the line-up to rejoin with.
/// The match; back to the lobby (B196): the lobby's room, the guests still in, their messages.
type Back = (Room, Vec<(String, Link, Conn)>, amp::UnboundedReceiver<Guest>);

async fn relay(cx: &Ctx, mut room: Room, mut joined: Vec<(String, Link, Conn)>, gtx: amp::UnboundedSender<Guest>, mut guests: amp::UnboundedReceiver<Guest>, cmds: &mut amp::UnboundedReceiver<Cmd>) -> Option<Back> {
    let mut away: Vec<String> = Vec::new();
    let mut rates = Rates::default();
    loop {
        let to = |peer: &str| joined.iter().find(|(p, ..)| p == peer).map(|(_, _, c)| c.clone());
        tokio::select! {
            Some(inc) = accept(&cx.ep) => {
                let gtx = gtx.clone();
                n0_future::task::spawn(async move {
                    let Ok(conn) = async { inc.accept()?.await.map_err(Into::into) }.await.map_err(|e: Box<dyn std::error::Error + Send + Sync>| e) else { return };
                    let peer = conn.remote_id().to_string();
                    let Ok((tx, rx)) = conn.accept_bi().await else { return };
                    let (conn, link, inbox) = quic(conn, tx, rx);
                    guest(gtx, peer, conn, link, inbox).await;
                });
            }
            Some(g) = guests.recv() => match g {
                Guest::Said(peer, Msg::Chat { text, .. }) => chat(cx, &peer, &text, &room.names, &joined, &mut rates),
                // only the host says who's away and restarts the match
                Guest::Said(peer, m) if m.in_match() && !matches!(m, Msg::Away { .. } | Msg::Restart { .. }) => {
                    joined.iter().filter(|(p, ..)| *p != peer).for_each(|(_, l, _)| l.send(&m));
                    cx.say(Event::Match(m));
                }
                Guest::Gone(peer) => joined.retain(|(p, ..)| *p != peer),
                Guest::In { peer, name, build, link, conn, .. } => {
                    // ponytail: the rejoiner plays the seat's old picks (a mod pick it lacks shows its stand-in)
                    let back = away.iter().find(|p| room.names.get(*p).is_some_and(|n| *n == name)).or(away.first()).cloned();
                    match back {
                        Some(old) if build == BUILD => {
                            bevy::log::info!("{name} ({}) rejoins for {}", short(&peer), short(&old));
                            away.retain(|p| *p != old);
                            if let Some(i) = joined.iter().position(|(p, ..)| *p == old) {
                                joined.remove(i).2.close(b"rejoined");
                            }
                            let mut r = room.clone();
                            r.rejoined(&old, &peer, &name);
                            r.rejoin = true;
                            link.send(&Msg::Start(r));
                            joined.push((peer.clone(), link, conn));
                            cx.say(Event::Match(Msg::Rejoined { old, new: peer, name }));
                        }
                        _ => refuse(&link, conn, "the match has started".into()),
                    }
                }
                Guest::Datagram(from, to_, data) if to_ == "host" => cx.say(Event::Datagram(from, data)),
                Guest::Datagram(from, to_, data) => {
                    if let Some(c) = to(&to_) {
                        send_datagram(&c, &from, &data);
                    }
                }
                Guest::Said(..) | Guest::Pack(..) => {}
            },
            c = cmds.recv() => match c {
                Some(Cmd::Send(Msg::Lobby(r))) => {
                    for (_, _, conn) in joined.iter().filter(|(p, ..)| away.contains(p)) {
                        conn.close(b"away");
                    }
                    joined.retain(|(p, ..)| !away.contains(p));
                    broadcast(&joined, &Msg::Lobby(r.clone()));
                    return Some((r, joined, guests));
                }
                Some(Cmd::Send(m)) => {
                    match &m {
                        Msg::Away { who, .. } => away.clone_from(who),
                        Msg::Restart { room: r, .. } => room.clone_from(r),
                        _ => {}
                    }
                    broadcast(&joined, &m)
                }
                Some(Cmd::Datagram(peer, data)) => {
                    if let Some(c) = to(&peer) {
                        send_datagram(&c, "host", &data);
                    }
                }
                Some(Cmd::Chat(t)) => chat(cx, "host", &t, &room.names, &joined, &mut rates),
                Some(Cmd::Leave) | None => {
                    // the goodbyes go out with the lag, then the connections close
                    n0_future::time::sleep(Duration::from_millis(LAG_MS.load(std::sync::atomic::Ordering::Relaxed) + 200)).await;
                    for (_, _, conn) in &joined {
                        conn.close(b"host left");
                    }
                    return None;
                }
                Some(_) => {}
            },
        }
    }
}

/// B198: the biggest datagram that surely gets through (QUIC's 1200-byte minimum path MTU less its headers; iroh
/// drops a bigger one, silently).
pub const DATAGRAM_MAX: usize = 1100;

/// Whether `data` fits in a datagram with its peer name (an endpoint id, 64 characters) in front.
pub fn fits(data: &[u8]) -> bool {
    data.len() + 65 <= DATAGRAM_MAX
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
fn send_datagram(c: &Conn, peer: &str, data: &[u8]) {
    let d = datagram(peer, data);
    let lag = LAG_MS.load(std::sync::atomic::Ordering::Relaxed);
    if lag == 0 {
        c.send_datagram(d);
    } else {
        let c = c.clone();
        n0_future::task::spawn(async move {
            n0_future::time::sleep(Duration::from_millis(lag)).await;
            c.send_datagram(d);
        });
    }
}

/// M20: each peer's recent chat lines, for the rate limit.
type Rates = std::collections::HashMap<String, Vec<n0_future::time::Instant>>;

/// M20: peer `from`'s chat line (`"host"`: this machine's), cleaned and rate-limited (`chat::clean`, `chat::allow`;
/// one over the limit is dropped), to every other peer with the sender's lobby name, and a guest's to this app too.
fn chat(cx: &Ctx, from: &str, text: &str, names: &std::collections::BTreeMap<String, String>, joined: &[(String, Link, Conn)], rates: &mut Rates) {
    let Some(text) = crate::chat::clean(text) else { return };
    if !crate::chat::allow(rates.entry(from.into()).or_default(), n0_future::time::Instant::now()) {
        return;
    }
    let name = names.get(from).and_then(|n| crate::chat::clean(n)).unwrap_or_else(|| short(from).into());
    let m = Msg::Chat { from: from.into(), name, text };
    joined.iter().filter(|(p, ..)| p != from).for_each(|(_, l, _)| l.send(&m));
    if from != "host"
        && let Msg::Chat { from, name, text } = m
    {
        cx.say(Event::Chat { from, name, text });
    }
}

fn broadcast(to: &[(String, Link, Conn)], m: &Msg) {
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

/// Connect to the host by `code`: the connection, its link and arrivals, and this machine's peer name; None (the
/// failure said) if it didn't.
async fn connect(cx: &Ctx, code: &str, cmds: &mut amp::UnboundedReceiver<Cmd>) -> Option<(Conn, Link, Inbox, String)> {
    let code = code.trim();
    if rtc::is_offer(code) {
        // the browser's LAN: our answer shown for the host to scan, then wait for them (or a Leave)
        let (pending, answer) = match rtc::answer(code).await {
            Ok(p) => p,
            Err(e) => {
                cx.say(Event::Failed(e));
                return None;
            }
        };
        let me = pending.me();
        bevy::log::info!("LAN answer: {answer}");
        cx.say(Event::Code(answer));
        cx.say(Event::Status("Now the host scans this code (or you send it to them to paste).".into()));
        let open = pending.open();
        n0_future::pin!(open);
        loop {
            tokio::select! {
                r = &mut open => return match r {
                    Ok((conn, link, inbox)) => {
                        cx.say(Event::Code(String::new()));
                        Some((conn, link, inbox, me))
                    }
                    Err(e) => {
                        cx.say(Event::Failed(e));
                        None
                    }
                },
                c = cmds.recv() => if matches!(c, Some(Cmd::Leave) | None) {
                    return None;
                },
            }
        }
    }
    let resolved;
    // B268: "Joining" names the short code that was dialled, not the first letters of the host's endpoint ID
    let shown = is_short(code).then(|| code.to_string());
    let code = if is_short(code) {
        cx.say(Event::Status(format!("Looking up {code}...")));
        let Some(t) = resolve(&cx.dht, code).await else {
            cx.say(Event::Failed(format!("No open lobby has the code {code}.")));
            return None;
        };
        resolved = t;
        resolved.as_str()
    } else {
        code
    };
    let Some(ep) = &cx.ep else {
        cx.say(Event::Failed("That's an internet lobby code: set Relay (Online) to n0 or your own relay to join it.".into()));
        return None;
    };
    let addr: EndpointAddr = match code.parse::<EndpointTicket>() {
        Ok(t) => t.into(),
        Err(_) => match code.parse::<EndpointId>() {
            Ok(id) => id.into(),
            Err(_) => {
                cx.say(Event::Failed("That code isn't a lobby code.".into()));
                return None;
            }
        },
    };
    bevy::log::info!("dialling {addr:?}");
    if let Some(why) = unreachable(&addr, cx.relays, code.parse::<EndpointTicket>().is_ok()) {
        cx.say(Event::Failed(why.into()));
        return None;
    }
    cx.say(Event::Status(format!("Joining {}...", shown.unwrap_or_else(|| short(&addr.id.to_string()).into()))));
    let conn = match n0_future::time::timeout(Duration::from_secs(30), ep.connect(addr, ALPN)).await {
        Ok(Ok(c)) => c,
        Ok(Err(iroh::endpoint::ConnectError::Connect { source: iroh::endpoint::ConnectWithOptsError::NoAddress { .. }, .. })) => {
            cx.say(Event::Failed("Couldn't find the host's address: the lobby may have closed, or the host has Relay off on another network (ask them to set Relay (Online) to n0).".into()));
            return None;
        }
        Ok(Err(e)) => {
            cx.say(Event::Failed(format!("Couldn't reach the host: {e}")));
            return None;
        }
        Err(_) => {
            cx.say(Event::Failed("Couldn't reach the host (timed out).".into()));
            return None;
        }
    };
    let Ok((tx, rx)) = conn.open_bi().await else {
        cx.say(Event::Failed("The host closed the connection.".into()));
        return None;
    };
    let (conn, link, inbox) = quic(conn, tx, rx);
    Some((conn, link, inbox, ep.id().to_string()))
}

/// Why a ticket's host can't be dialled from here, before a 30 s timeout says so (B175): a browser reaches peers
/// only through the ticket's relay; with the relays off only the ticket's direct addresses (or, for a bare ID, the
/// LAN's and the DHT's).
fn unreachable(addr: &EndpointAddr, relays: bool, ticket: bool) -> Option<&'static str> {
    if cfg!(target_arch = "wasm32") && addr.relay_urls().next().is_none() {
        return Some("That host has Relay off (LAN only), and a browser can only reach a host through a relay: ask them to set Relay (Online) to n0.");
    }
    (!relays && ticket && addr.ip_addrs().next().is_none())
        .then_some("That lobby can only be reached through a relay (a browser's?): set Relay (Online) to n0 to join it.")
}

/// B275: every mod in a seat's picks one this game can play.
fn picks_ok(p: &[Pick]) -> Result<(), String> {
    p.iter().filter_map(|p| p.r#mod.as_deref()).try_for_each(crate::mods_online::Wire::check)
}

/// Join by code; true once seated (the lobby then runs until it starts or someone leaves).
async fn join(cx: &Ctx, code: &str, mut picks: Vec<Pick>, cmds: &mut amp::UnboundedReceiver<Cmd>) -> bool {
    // a home relay first, as when bind waited for it (B224 moved that here; bounded: a dead relay still leaves LAN)
    if let Some(ep) = cx.ep.as_ref().filter(|_| cx.relays) {
        let _ = n0_future::time::timeout(Duration::from_secs(10), ep.online()).await;
    }
    let Some((conn, link, mut inbox, me)) = connect(cx, code, cmds).await else { return false };
    link.send(&Msg::Hello { build: BUILD.into(), picks: picks.clone(), name: cx.name.clone() });
    cx.say(Event::Me(me.clone()));
    let Inbox { msgs: mut inbox, mut dgrams } = inbox;
    let ev = cx.ev.clone();
    n0_future::task::spawn(read_packs(conn.clone(), move |h, b| drop(ev.send(Event::Pack(h, b)))));
    let mut seated = false;
    let mut playing = false;
    // B114: connected but no seat from the host by then: give up instead of "Joining..." for ever
    let handshake = n0_future::time::sleep(Duration::from_secs(20));
    n0_future::pin!(handshake);
    cx.say(Event::Status("Connected: waiting for the host's lobby...".into()));
    loop {
        tokio::select! {
            _ = &mut handshake, if !seated => {
                cx.say(Event::Failed("The host didn't answer (no lobby after 20 s).".into()));
                conn.close(b"left");
                return false;
            }
            m = inbox.recv() => match m.unwrap_or_else(|| Err("closed".into())) {
                // B275: a pick this game can't play only ends the link at the start; in the lobby it's a message
                Ok(Msg::Room(r)) if r.check().is_err() => {
                    cx.say(Event::Status(format!("A pick this game can't play: {}", r.check().unwrap_err())));
                    seated = true;
                    cx.say(Event::Room(r));
                }
                Ok(Msg::Start(r)) if r.check().is_err() => {
                    cx.say(Event::Failed(format!("Left: {}", r.check().unwrap_err())));
                    conn.close(b"left");
                    return seated;
                }
                Ok(Msg::Room(r)) => {
                    if !seated {
                        // B114: the last status was "Joining...", which read as still trying
                        cx.say(Event::Status("Joined: pick your character, then choose Ready.".into()));
                    }
                    seated = true;
                    cx.say(Event::Room(r));
                }
                Ok(Msg::Want(h)) => {
                    // the host passes on a peer's ask for this machine's mod
                    let (picks, conn) = (picks.clone(), conn.clone());
                    n0_future::task::spawn(async move {
                        if let Some(b) = own_pack(picks, h) {
                            send_pack(&conn, h, std::sync::Arc::new(b));
                        }
                    });
                }
                Ok(Msg::Lobby(r)) if playing => {
                    playing = false;
                    cx.say(Event::Match(Msg::Lobby(r)));
                }
                Ok(Msg::Start(r)) => {
                    // a rejoiner (B196) gets Start without a Room first
                    (playing, seated) = (true, true);
                    cx.say(Event::Start(r, me.clone()));
                }
                Ok(Msg::Ping(t)) => link.send(&Msg::Pong(t)),
                Ok(Msg::Chat { from, name, text }) if seated => cx.say(Event::Chat { from, name, text }),
                Ok(m) if playing && m.in_match() => cx.say(Event::Match(m)),
                Ok(Msg::Refused(why)) => {
                    cx.say(Event::Failed(format!("Refused: {why}")));
                    conn.close(b"refused");
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
            Some(d) = dgrams.recv() => if playing && let Some((from, data)) = undatagram(&d) {
                cx.say(Event::Datagram(from, data.to_vec()));
            },
            c = cmds.recv() => match c {
                Some(Cmd::Send(m)) => link.send(&m),
                Some(Cmd::Want(h)) => link.send(&Msg::Want(h)),
                Some(Cmd::Ready(v)) => link.send(&Msg::Ready(v)),
                Some(Cmd::Deciding(d)) => link.send(&Msg::Deciding(d)),
                Some(Cmd::Prepared(p)) => link.send(&Msg::Prepared(p)),
                Some(Cmd::Cancel) => link.send(&Msg::Cancel),
                Some(Cmd::Live(l)) => link.send(&Msg::Live(l)),
                Some(Cmd::Chat(text)) => link.send(&Msg::Chat { from: String::new(), name: String::new(), text }),
                Some(Cmd::Picks(p)) => {
                    link.send(&Msg::Picks(p.clone()));
                    picks = p;
                }
                Some(Cmd::Datagram(to, data)) => send_datagram(&conn, &to, &data),
                Some(Cmd::Leave) | None => {
                    n0_future::time::sleep(Duration::from_millis(if playing { LAG_MS.load(std::sync::atomic::Ordering::Relaxed) + 200 } else { 0 })).await;
                    conn.close(b"left");
                    return true;
                }
                Some(_) => {}
            },
        }
    }
}

// ---------------------------------------------------------------- terminal

#[cfg(not(target_arch = "wasm32"))]
/// `hst net host [doubles] [players] [--bots] | find | quick [doubles] [players] | join <code> [players]`, with `online.txt`'s
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
    // start once a guest is in and ready, the open seats to the computer (else once every seat is taken)
    let bots = a.iter().any(|x| x == "--bots");
    let players = a.iter().filter_map(|x| x.parse::<usize>().ok()).next().unwrap_or(1);
    let picks: Vec<Pick> = (0..players).map(|i| Pick::disc(i, 0)).collect();
    let mut room = Room { name: cfg.name.clone(), doubles, court: 0, sets: 0, games, umpire: 4, seats: vec![None; if doubles { 4 } else { 2 }], ..Default::default() };
    room.take("host", &picks);
    let net = Net::start(cfg);
    let cmd = match a.first().map(String::as_str) {
        Some("host") => Cmd::Host(room),
        Some("find") => Cmd::Find,
        Some("quick") => Cmd::Quick(room),
        Some("join") => Cmd::Join(a.get(1).cloned().unwrap_or_default(), picks),
        _ => {
            eprintln!("usage: hst net host|find|quick [doubles] [players] | join <code> [players]  [--relay n0|off|<url>] [--games N] [--bots] [--play [disc] [flags]]");
            return None;
        }
    };
    // Find answers once per lookup (B267)
    let mut finds = if matches!(cmd, Cmd::Find) { FINDS } else { 0 };
    let host = matches!(cmd, Cmd::Host(_) | Cmd::Quick(_));
    net.send(cmd);
    let t0 = std::time::Instant::now();
    let mut said_ready = false;
    loop {
        for e in net.poll() {
            println!("{:6.2}s {e:?}", t0.elapsed().as_secs_f32());
            match e {
                // no menus: a guest is ready once seated, the host starts once every seat is taken and everyone's ready
                Event::Room(_) if !host && !said_ready => {
                    said_ready = true;
                    net.send(Cmd::Prepared(true));
                }
                Event::Room(r) if host && (r.open() == 0 || bots && r.seats.iter().flatten().any(|s| s.peer != "host")) && r.all_ready() => net.send(Cmd::Start),
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

/// `code` is a join code: a host's ticket, a bare endpoint ID or a short code.
pub fn is_code(code: &str) -> bool {
    is_short(code) || rtc::is_offer(code) || code.parse::<EndpointTicket>().is_ok() || code.parse::<EndpointId>().is_ok()
}

/// `code` is a host's whole ticket.
pub fn is_ticket(code: &str) -> bool {
    code.parse::<EndpointTicket>().is_ok()
}

/// The join code in pasted or typed text: the code itself, or a lobby link's `?code=` (short codes in capitals).
pub fn code_in(text: &str) -> Option<String> {
    let t = text.trim();
    let t = t.split_once("code=").map_or(t, |(_, c)| c.split(['&', '#']).next().unwrap_or(""));
    let up = t.to_ascii_uppercase();
    if is_short(&up) {
        Some(up)
    } else {
        is_code(t).then(|| t.to_string())
    }
}

// ---------------------------------------------------------------- short codes (B104)
//
// A hosted lobby also gets a [`SHORT`]-symbol code that people can read out and type. The code names a well-known
// ed25519 key (its seed is the code, [`code_key`]); the host puts its ticket under that key as a BEP 44 mutable item
// on the mainline DHT (no salt; seq = unix µs), as a one-TXT-record DNS packet so the same item is also a pkarr record.
// The browser has no DHT and goes through a pkarr relay ([`PKARR`], an HTTP gateway onto the same DHT items; native hosts put
// there too, as the relay's own lookup can miss a fresh DHT put). A host
// re-rolls a code someone wrote less than [`STALE`] ago and republishes with its listing; once it stops (match
// started, left), the code goes stale and is free again. Anyone may overwrite a code's item (the key is public):
// the joiner still dials the ticket's endpoint, authenticated by QUIC.

/// The short codes' symbols: digits and capitals without the look-alikes 0/O and 1/I (32^4 ≈ 1M codes).
pub const SYMBOLS: &[u8; 32] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
pub const SHORT: usize = 4;
/// The browser build's page: a lobby link is this with `?code=<short code>`. Beta builds are built with
/// `HST_SITE=https://beta.opentennis.dev/` (pages.yml), so their links open the beta site.
pub const SITE: &str = match option_env!("HST_SITE") {
    Some(s) if !s.is_empty() => s,
    _ => "https://opentennis.dev/",
};
/// The pkarr relay the browser reads and writes short codes through (it allows the site's origin); native hosts put
/// theirs there too.
const PKARR: &str = "https://relay.pkarr.org";

pub fn is_short(code: &str) -> bool {
    code.len() == SHORT && code.bytes().all(|b| SYMBOLS.contains(&b))
}

/// The link that opens `short`'s lobby in the browser build.
pub fn link(short: &str) -> String {
    format!("{SITE}?code={short}")
}

/// A random short code.
fn new_short() -> String {
    // ponytail: a fresh key's bytes as the randomness (no rand dependency); 5 bits of each of the first 4
    SecretKey::generate().to_bytes()[..SHORT].iter().map(|b| SYMBOLS[(b & 31) as usize] as char).collect()
}

/// The (public) key a short code's record is written under.
fn code_key(code: &str) -> SecretKey {
    let mut seed = [0u8; 32];
    let s = format!("hst-lobby-code/1/{code}");
    seed[..s.len()].copy_from_slice(s.as_bytes());
    SecretKey::from_bytes(&seed)
}

/// `text` as a DNS response with one TXT record `_hst` (strings of ≤ 255 bytes): a pkarr relay only takes DNS packets.
fn dns_txt(text: &str) -> Vec<u8> {
    let mut rdata = Vec::new();
    for c in text.as_bytes().chunks(255) {
        rdata.push(c.len() as u8);
        rdata.extend(c);
    }
    let mut p = vec![0, 0, 0x84, 0, 0, 0, 0, 1, 0, 0, 0, 0, 4, b'_', b'h', b's', b't', 0, 0, 16, 0, 1, 0, 0, 1, 44];
    p.extend((rdata.len() as u16).to_be_bytes());
    p.extend(rdata);
    p
}

/// The first answer's TXT strings, joined (`dns_txt` read back).
fn txt_of(p: &[u8]) -> Option<String> {
    let mut i = 12;
    loop {
        match *p.get(i)? {
            0 => break i += 1,
            l if l >= 0xc0 => break i += 2,
            l => i += 1 + l as usize,
        }
    }
    let n = u16::from_be_bytes(p.get(i + 8..i + 10)?.try_into().ok()?) as usize;
    let mut r = p.get(i + 10..i + 10 + n)?;
    let mut out = Vec::new();
    while let Some((&l, rest)) = r.split_first() {
        out.extend(rest.get(..l as usize)?);
        r = &rest[l as usize..];
    }
    String::from_utf8(out).ok()
}

fn unix_micros() -> i64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_micros() as i64)
}

/// Written less than `STALE` ago.
fn fresh(seq: i64) -> bool {
    unix_micros().saturating_sub(seq) <= STALE as i64 * 1_000_000
}

/// The ticket under `code` and when it was written.
#[cfg(not(target_arch = "wasm32"))]
async fn lookup(dht: &Dht, code: &str) -> Option<(String, i64)> {
    let item = dht.get_mutable_most_recent(code_key(code).public().as_bytes(), None).await.ok().flatten()?;
    Some((txt_of(item.value())?, item.seq()))
}

#[cfg(target_arch = "wasm32")]
async fn lookup(_: &Dht, code: &str) -> Option<(String, i64)> {
    relay_get(code).await
}

/// The record under `code` on the pkarr relay (the browser's [`lookup`]).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
async fn relay_get(code: &str) -> Option<(String, i64)> {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = rustls::crypto::ring::default_provider().install_default();
    // B275: the relay answers with max-age=300, so a browser's second lookup (a rejoin) got its cached copy, by then
    // older than STALE: "No open lobby has the code"; the query makes each lookup a fresh request
    let r = reqwest::get(format!("{PKARR}/{}?t={}", code_key(code).public().to_z32(), unix_micros())).await.ok()?;
    let b = r.error_for_status().ok()?.bytes().await.ok()?;
    // the relay's payload: signature, seq (big-endian), the packet
    Some((txt_of(b.get(72..)?)?, i64::from_be_bytes(b.get(64..72)?.try_into().ok()?)))
}

/// Put `ticket` under `code`; true if it went out (on the DHT, or the relay when the DHT put fails).
#[cfg(not(target_arch = "wasm32"))]
async fn publish(dht: &Dht, code: &str, ticket: &str) -> bool {
    let item = MutableItem::new(&SigningKey::from_bytes(&code_key(code).to_bytes()), &dns_txt(ticket), unix_micros(), None);
    let put = dht.put_mutable(item, None).await.map_err(|e| bevy::log::warn!("lobby code {code}: {e}")).is_ok();
    // ponytail: the relay's own DHT lookup misses our put now and then, so browsers read it from the relay directly;
    // its put takes 5-20 s (B151), so after a DHT put it goes on in the background (native joiners read the DHT)
    // (not from the tests: their DHT is a local testnet)
    if put && !cfg!(test) {
        let (code, ticket) = (code.to_string(), ticket.to_string());
        n0_future::task::spawn(async move { drop(relay_put(&code, &ticket).await) });
        return true;
    }
    relay_put(code, ticket).await
}

#[cfg(target_arch = "wasm32")]
async fn publish(_: &Dht, code: &str, ticket: &str) -> bool {
    relay_put(code, ticket).await
}

/// Puts the code's record on the pkarr relay (which also puts it on the DHT).
async fn relay_put(code: &str, ticket: &str) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = rustls::crypto::ring::default_provider().install_default();
    let key = code_key(code);
    let (v, seq) = (dns_txt(ticket), unix_micros());
    // BEP 44's signed form: bencoded seq and v
    let sig = key.sign(&[format!("3:seqi{seq}e1:v{}:", v.len()).into_bytes(), v.clone()].concat());
    let body = [sig.to_bytes().to_vec(), seq.to_be_bytes().to_vec(), v].concat();
    let r = reqwest::Client::new().put(format!("{PKARR}/{}", key.public().to_z32())).body(body).send().await;
    r.and_then(|r| r.error_for_status()).map_err(|e| bevy::log::warn!("lobby code {code}: {e}")).is_ok()
}

/// A short code for the lobby with `ticket`, `code` first: a free one, published (None: the DHT or relay can't be
/// reached). Each code tried after a taken one goes to `show`.
async fn roll(dht: &Dht, mut code: String, ticket: &str, show: impl Fn(&str)) -> Option<String> {
    for _ in 0..5 {
        match n0_future::time::timeout(Duration::from_secs(15), lookup(dht, &code)).await {
            Ok(Some((_, seq))) if fresh(seq) => {
                code = new_short();
                show(&code);
            }
            _ => return publish(dht, &code, ticket).await.then_some(code),
        }
    }
    None
}

/// How many board slots there are: the browser reads every one through the pkarr relay, which answers about 10
/// requests at once before it rate-limits (HTTP 429), and a slot nobody wrote takes it ~7 s.
// ponytail: 8 lobbies listed in the browser (native Find also has the DHT board, unbounded); a bigger board needs
// a server of ours or paced reads
const SLOTS: usize = 8;

/// Board slot `i`'s record name (its key's seed, as a short code's); the tests' own, as a testnet's items can
/// reach the real DHT.
fn slot_name(i: usize) -> String {
    format!("{}{i}", if cfg!(test) { "tslot" } else { "slot" })
}

/// The listing in board slot `i` and when it was written.
async fn read_slot(dht: &Dht, i: usize) -> Option<(Listing, i64)> {
    let (t, seq) = n0_future::time::timeout(Duration::from_secs(12), lookup(dht, &slot_name(i))).await.ok()??;
    Some((serde_json::from_str(&t).ok()?, seq))
}

async fn read_slots(dht: &Dht) -> Vec<Option<(Listing, i64)>> {
    n0_future::join_all((0..SLOTS).map(|i| read_slot(dht, i))).await
}

/// The open lobbies in the board slots, each with its short code.
async fn find_slots(dht: &Dht) -> Vec<Listing> {
    let mut list: Vec<Listing> =
        read_slots(dht).await.into_iter().flatten().filter(|(l, seq)| fresh(*seq) && l.open() > 0 && is_short(&l.code)).map(|(l, _)| l).collect();
    // one host in two slots (it moved to a freed one): its newest
    list.sort_by(|a, b| (&a.id, b.time).cmp(&(&b.id, a.time)));
    list.dedup_by(|a, b| a.id == b.id);
    list
}

/// Put `l` (with its short code) in a board slot: `slot`'s if it's still ours, else the first free, stale or ours
/// from the one its ID picks; `slot` remembers it (`SLOTS`: the board is full, listed again next time).
async fn list_in_slot(dht: &Dht, slot: &std::sync::atomic::AtomicUsize, l: &Listing) {
    use std::sync::atomic::Ordering::Relaxed;
    // ponytail: a slot that didn't answer (timeout, 429) counts as free, and two hosts claiming one at once both
    // write it; the next republish's check moves the loser on
    let ours = |r: &Option<(Listing, i64)>| r.as_ref().is_none_or(|(o, seq)| o.id == l.id || !fresh(*seq));
    let mut i = slot.load(Relaxed);
    if i >= SLOTS || !ours(&read_slot(dht, i).await) {
        let all = read_slots(dht).await;
        let start = l.id.parse().map_or(0, |id| lobby_number(id) as usize);
        i = (0..SLOTS).map(|k| (start + k) % SLOTS).find(|&k| ours(&all[k])).unwrap_or(SLOTS);
        slot.store(i, Relaxed);
    }
    if i < SLOTS {
        // B321: the browser's put right after the slot reads is often refused (the pkarr relay's rate limit), which
        // left the lobby off the board until the next REPUBLISH: tried again a few seconds on
        for pause in [0, 3, 6] {
            n0_future::time::sleep(Duration::from_secs(pause)).await;
            if publish(dht, &slot_name(i), &serde_json::to_string(l).unwrap()).await {
                break;
            }
        }
    } else {
        bevy::log::warn!("lobby board full: listed on the DHT only");
    }
}

/// The ticket the short code `code` stands for, if its lobby is open.
async fn resolve(dht: &Dht, code: &str) -> Option<String> {
    // B114: asked again until 20 s are up, as a code just rolled can take a few seconds to reach the relay or DHT
    // nodes this machine asks
    let until = n0_future::time::Instant::now() + Duration::from_secs(20);
    loop {
        let left = until.saturating_duration_since(n0_future::time::Instant::now());
        if let Ok(Some((t, seq))) = n0_future::time::timeout(left, lookup(dht, code)).await
            && fresh(seq)
        {
            return Some(t);
        }
        if n0_future::time::Instant::now() + Duration::from_secs(3) >= until {
            return None;
        }
        n0_future::time::sleep(Duration::from_secs(3)).await;
    }
}

/// The ticket put under a short code: the endpoint's whole address if it fits a DHT item (1000 bytes), else its ID
/// and relay; `home` if the address has no relay just now (B266).
fn code_ticket(ep: &Endpoint, home: Option<&RelayUrl>) -> String {
    let mut addr = ep.addr();
    if let Some(h) = home.filter(|_| addr.relay_urls().next().is_none()) {
        addr = addr.with_relay_url(h.clone());
    }
    let t = EndpointTicket::new(addr.clone()).to_string();
    if t.len() <= 900 {
        return t;
    }
    let mut a = EndpointAddr::new(addr.id);
    for u in addr.relay_urls() {
        a = a.with_relay_url(u.clone());
    }
    EndpointTicket::new(a).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(doubles: bool) -> Room {
        // B215: a shorter start countdown for the tests (each starts one)
        COUNTDOWN_MS.store(1000, std::sync::atomic::Ordering::Relaxed);
        Room { name: "t".into(), doubles, court: 0, sets: 0, games: 4, umpire: 4, seats: vec![None; if doubles { 4 } else { 2 }], ..Default::default() }
    }

    /// B292: every person in ready doesn't count down while a seat is open; a full lobby does.
    #[test]
    fn no_autostart_with_open_seats() {
        let mut r = room(true);
        r.take("host", &[Pick::disc(0, 0)]);
        r.take("g", &[Pick::disc(1, 0)]);
        r.picking.clear();
        r.ready.insert("g".into());
        assert!(r.all_ready() && !r.all_set(), "two seats open");
        r.take("h", &[Pick::disc(2, 0), Pick::disc(3, 0)]);
        r.picking.clear();
        r.ready.insert("h".into());
        assert!(r.all_set(), "full and ready");
    }

    #[test]
    fn relay_off_fails_fast_on_relay_only_tickets() {
        let id = SecretKey::from_bytes(&[7; 32]).public();
        let relay = EndpointAddr::new(id).with_relay_url("https://use1-1.relay.n0.iroh.link./".parse().unwrap());
        let direct = EndpointAddr::new(id).with_ip_addr("192.168.1.2:4000".parse().unwrap());
        // a browser host's ticket: only its relay
        assert!(unreachable(&relay, false, true).is_some());
        assert!(unreachable(&relay, true, true).is_none());
        // a relay-off host on the LAN, and bare IDs (found by mDNS/DHT)
        assert!(unreachable(&direct, false, true).is_none());
        assert!(unreachable(&EndpointAddr::new(id), false, false).is_none());
    }

    /// B266: a code's record keeps the last home relay while the address has none (a net report without one), so a
    /// browser isn't told the host has Relay off.
    #[test]
    fn code_ticket_keeps_the_last_relay() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let ep = Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled).bind().await.unwrap();
            let home: RelayUrl = "https://use1-1.relay.n0.iroh.link./".parse().unwrap();
            let relay = |t: String| EndpointAddr::from(t.parse::<EndpointTicket>().unwrap()).relay_urls().next().cloned();
            assert_eq!(relay(code_ticket(&ep, None)), None);
            assert_eq!(relay(code_ticket(&ep, Some(&home))), Some(home));
        });
    }

    #[test]
    fn seats_fill_and_free() {
        let mut r = room(true);
        assert!(r.take("a", &[Pick::disc(1, 0), Pick::disc(2, 0)]));
        assert!(!r.take("b", &vec![Pick::disc(3, 0); 3]));
        assert!(r.take("b", &[Pick::disc(3, 0)]));
        assert_eq!(r.open(), 1);
        r.leave("a");
        assert_eq!(r.open(), 3);
        assert_eq!(r.seats[2].as_ref().unwrap().peer, "b");
    }

    /// B195: a joiner's placeholder seats take its chosen picks (same count only), no longer picking or ready.
    /// A vanilla lobby refuses mod picks, and going vanilla puts the seated mods' donors in.
    #[test]
    fn vanilla_lobby_has_no_mods() {
        let w = crate::mods_online::Wire {
            id: "x".into(), name: "X".into(), donor: 5, hand: 1.0, base: 5, overrides: vec![], ai_row: 5,
            names: vec!["a".into()], parent: vec![None], rest: vec![[0; 16]], looks: 1, dir: None,
        };
        let modded = Pick { r#mod: Some(Box::new(w)), ..Pick::disc(5, 2) };
        let mut r = room(false);
        assert!(r.take("host", &[modded.clone()]));
        assert!(r.allowed(std::slice::from_ref(&modded)).is_ok());
        let v = r.ver;
        assert!(r.set_vanilla(true) && !r.set_vanilla(true));
        assert_eq!((r.seats[0].as_ref().unwrap().pick.clone(), r.ver), (Pick::disc(5, 0), v + 1));
        assert!(r.allowed(&[modded]).is_err() && r.allowed(&[Pick::disc(1, 0)]).is_ok());
    }

    #[test]
    fn repick_fills_held_seats() {
        let mut r = room(true);
        assert!(r.take("host", &[Pick::disc(0, 0)]));
        assert!(r.take("g", &[Pick::disc(0, 0), Pick::disc(0, 0)]));
        (r.picking, r.ready) = (["g".into()].into(), ["g".into()].into());
        let v = r.ver;
        assert!(!r.repick("g", &[Pick::disc(3, 1)]), "two seats held");
        assert!(r.repick("g", &[Pick::disc(3, 1), Pick::disc(5, 0)]));
        let chars: Vec<_> = r.seats.iter().flatten().map(|s| s.pick.char).collect();
        assert_eq!((chars, r.picking.is_empty(), r.ready.is_empty(), r.ver), (vec![0, 3, 5], true, true, v + 1));
    }

    /// B201: the host's computer players (open and people's seats only), singles/doubles (the taken seats first, if
    /// they fit), and each seat's live place by its peer's held-seat order.
    #[test]
    fn host_bots_mode_and_live_places() {
        let mut r = room(true);
        assert!(r.take("host", &[Pick::disc(0, 0)]) && r.take("g", &[Pick::disc(1, 0), Pick::disc(2, 0)]));
        assert!(!r.bot(1, Some(Pick::disc(4, 0))), "a person's seat");
        assert!(!r.bot(9, None));
        let v = r.ver;
        assert!(r.bot(3, Some(Pick::disc(4, 2))) && r.bot(3, Some(Pick::disc(5, 1))));
        assert_eq!((r.seats[3].as_ref().map(|s| (s.peer.as_str(), s.pick.char)), r.ver), (Some((CPU, 5)), v + 2));
        assert!(!r.set_doubles(false), "four taken");
        assert!(r.bot(3, None) && r.seats[3].is_none());
        r.live.insert("g".into(), vec![Live { char: 7, ..Default::default() }, Live { char: 8, ready: true, ..Default::default() }]);
        assert_eq!((r.live_of(1).map(|l| l.char), r.live_of(2).map(|l| l.char), r.live_of(0)), (Some(7), Some(8), None));
        r.leave("g");
        assert!(r.live.is_empty() && r.set_doubles(false) && !r.set_doubles(false));
        assert_eq!((r.doubles, r.seats.len(), r.seats[0].as_ref().map(|s| s.peer.as_str())), (false, 2, Some("host")));
        assert!(r.set_doubles(true) && r.seats.len() == 4);
    }

    /// B321: Host chosen while the Online screen's search still runs (its LAN wait alone is 3 s) opens the lobby at
    /// once, the search dropped (~1 s).
    #[test]
    fn host_cuts_a_running_find() {
        let net = Net::start(Online { relay: Relay::Off, ..Default::default() });
        net.send(Cmd::Find);
        std::thread::sleep(Duration::from_millis(200));
        let t = std::time::Instant::now();
        net.send(Cmd::Name("h".into()));
        net.send(Cmd::Host(room(false)));
        let mut found = false;
        while t.elapsed() < LAN_WAIT - Duration::from_millis(500) {
            for e in net.poll() {
                found |= matches!(e, Event::Found(_));
                if let Event::Room(r) = e {
                    assert!(!found, "the search ran on");
                    assert_eq!(r.names.get("host").map(String::as_str), Some("h"), "the name sent during the search");
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("no lobby in {:?}: Host waited for the search", t.elapsed());
    }

    /// B201 over a local connection: a guest's live place reaches the host, the host's computer player and court reach
    /// the guest, and Start carries them (~2 s).
    #[test]
    fn live_places_and_a_hosts_bot_reach_everyone() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let testnet = n0_mainline::Testnet::new(3).await.unwrap();
            let cx = || async {
                let (ep, _) = endpoint(&Relay::Off).await.unwrap();
                let (tx, rx) = mpsc::channel();
                (Ctx { ep: Some(ep), dht: Dht::builder().bootstrap(&testnet.bootstrap).build().unwrap(), lan: Lan::default(), ev: tx, name: "g".into(), relays: false }, rx)
            };
            async fn next(ev: &mpsc::Receiver<Event>, want: impl Fn(&Event) -> bool) -> Event {
                for _ in 0..1500 {
                    while let Ok(e) = ev.try_recv() {
                        if want(&e) {
                            return e;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                panic!("timed out")
            }
            let ((hcx, hev), (gcx, gev)) = (cx().await, cx().await);
            let (hcmd, mut hrx) = amp::unbounded_channel();
            let (gcmd, mut grx) = amp::unbounded_channel();
            let mut r = room(true);
            r.take("host", &[Pick::disc(0, 0)]);
            tokio::spawn(async move { host(&hcx, r, &mut hrx).await });
            let Event::Code(code) = next(&hev, |e| matches!(e, Event::Code(_))).await else { unreachable!() };
            tokio::spawn(async move { join(&gcx, &code, vec![Pick::disc(1, 0)], &mut grx).await });
            let Event::Me(me) = next(&gev, |e| matches!(e, Event::Me(_))).await else { unreachable!() };
            next(&gev, |e| matches!(e, Event::Room(r) if r.seats[1].as_ref().is_some_and(|s| s.peer == me))).await;
            let l = Live { char: 6, costume: 3, ..Default::default() };
            gcmd.send(Cmd::Live(vec![l.clone()])).unwrap();
            hcmd.send(Cmd::Live(vec![Live { char: 2, ready: true, ..Default::default() }])).unwrap();
            let Event::Room(seen) = next(&hev, |e| matches!(e, Event::Room(r) if r.live.len() == 2)).await else { unreachable!() };
            assert_eq!(seen.live_of(1), Some(&l));
            hcmd.send(Cmd::Bot(2, Some(Pick::disc(9, 4)))).unwrap();
            hcmd.send(Cmd::Bot(1, None)).unwrap();
            hcmd.send(Cmd::Venue(5, 1, 4, 2)).unwrap();
            let Event::Room(seen) = next(&gev, |e| matches!(e, Event::Room(r) if r.court == 5)).await else { unreachable!() };
            assert_eq!(seen.seats[2].as_ref().map(|s| (s.human(), s.pick.char, s.pick.costume)), Some((false, 9, 4)));
            assert_eq!(seen.seats[1].as_ref().map(|s| s.peer.as_str()), Some(me.as_str()), "the guest's seat isn't the host's to clear");
            assert_eq!(seen.live_of(0).map(|l| l.char), Some(2));
            gcmd.send(Cmd::Picks(vec![Pick::disc(6, 3)])).unwrap();
            gcmd.send(Cmd::Prepared(true)).unwrap();
            next(&hev, |e| matches!(e, Event::Room(r) if r.ready.len() == 1)).await;
            hcmd.send(Cmd::Start).unwrap();
            let Event::Start(r, _) = next(&gev, |e| matches!(e, Event::Start(..))).await else { unreachable!() };
            let seats: Vec<_> = r.seats.iter().flatten().map(|s| (s.pick.char, s.pick.costume)).collect();
            assert_eq!(seats[..3], [(0, 0), (6, 3), (9, 4)], "the match has the lobby's picks and the host's computer player");
        });
    }

    /// B215 over a local connection: the host's Start counts down with the guest not ready and the guest's Cancel
    /// stops it for both; everyone ready counts down by itself to the start; un-ready during it cancels (~4 s).
    #[test]
    fn ready_up_countdown() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let testnet = n0_mainline::Testnet::new(3).await.unwrap();
            let cx = || async {
                let (ep, _) = endpoint(&Relay::Off).await.unwrap();
                let (tx, rx) = mpsc::channel();
                (Ctx { ep: Some(ep), dht: Dht::builder().bootstrap(&testnet.bootstrap).build().unwrap(), lan: Lan::default(), ev: tx, name: "g".into(), relays: false }, rx)
            };
            async fn next(ev: &mpsc::Receiver<Event>, want: impl Fn(&Event) -> bool) -> Event {
                for _ in 0..1500 {
                    while let Ok(e) = ev.try_recv() {
                        if want(&e) {
                            return e;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                panic!("timed out")
            }
            let started = |ev: &mpsc::Receiver<Event>| ev.try_iter().any(|e| matches!(e, Event::Start(..)));
            let ((hcx, hev), (gcx, gev)) = (cx().await, cx().await);
            let (hcmd, mut hrx) = amp::unbounded_channel();
            let (gcmd, mut grx) = amp::unbounded_channel();
            let mut r = room(false);
            r.take("host", &[Pick::disc(0, 0)]);
            r.picking.clear();
            tokio::spawn(async move { host(&hcx, r, &mut hrx).await });
            let Event::Code(code) = next(&hev, |e| matches!(e, Event::Code(_))).await else { unreachable!() };
            tokio::spawn(async move { join(&gcx, &code, vec![Pick::disc(1, 0)], &mut grx).await });
            next(&gev, |e| matches!(e, Event::Room(r) if r.open() == 0)).await;
            gcmd.send(Cmd::Picks(vec![Pick::disc(1, 0)])).unwrap();
            next(&hev, |e| matches!(e, Event::Room(r) if r.picking.is_empty())).await;
            // the host's Start with the guest not ready: a countdown on both; the guest's Cancel stops it for both
            hcmd.send(Cmd::Start).unwrap();
            next(&gev, |e| matches!(e, Event::Room(r) if r.count > 0)).await;
            gcmd.send(Cmd::Cancel).unwrap();
            next(&gev, |e| matches!(e, Event::Room(r) if r.count == 0)).await;
            next(&hev, |e| matches!(e, Event::Room(r) if r.count == 0)).await;
            tokio::time::sleep(Duration::from_millis(1200)).await;
            assert!(!started(&gev) && !started(&hev), "started after the guest's cancel");
            // the guest ready: everyone is, the countdown starts by itself; un-ready during it cancels
            gcmd.send(Cmd::Prepared(true)).unwrap();
            next(&gev, |e| matches!(e, Event::Room(r) if r.count > 0)).await;
            gcmd.send(Cmd::Prepared(false)).unwrap();
            next(&hev, |e| matches!(e, Event::Room(r) if r.count == 0 && r.ready.is_empty())).await;
            tokio::time::sleep(Duration::from_millis(1200)).await;
            assert!(!started(&gev) && !started(&hev), "started after the guest un-readied");
            // the host's Start with the guest still not ready runs out and starts on both, no countdown in the line-up
            hcmd.send(Cmd::Start).unwrap();
            let Event::Start(r, _) = next(&gev, |e| matches!(e, Event::Start(..))).await else { unreachable!() };
            assert!(matches!(next(&hev, |e| matches!(e, Event::Start(..))).await, Event::Start(h, _) if h == r));
            assert_eq!((r.count, r.ready.len(), r.seats[1].as_ref().map(|s| s.pick.char)), (0, 0, Some(1)));
            // back in the lobby (B196): the guest ready makes everyone ready, and that counts down to the start by itself
            let mut back = r.clone();
            back.back_to_lobby(&[]);
            hcmd.send(Cmd::Send(Msg::Lobby(back))).unwrap();
            next(&gev, |e| matches!(e, Event::Match(Msg::Lobby(_)))).await;
            gcmd.send(Cmd::Prepared(true)).unwrap();
            next(&gev, |e| matches!(e, Event::Room(r) if r.count > 0)).await;
            let Event::Start(r, _) = next(&gev, |e| matches!(e, Event::Start(..))).await else { unreachable!() };
            assert!(matches!(next(&hev, |e| matches!(e, Event::Start(..))).await, Event::Start(h, _) if h == r));
            assert_eq!((r.count, r.ready.len()), (0, 1));
        });
    }

    #[test]
    fn quickplay_takes_the_busiest_that_fits() {
        let l = |id: &str, taken, seats, time| Listing { id: id.into(), name: String::new(), doubles: seats == 4, court: 0, taken, seats, time, code: String::new() };
        let list = [l("a", 1, 4, 5), l("b", 2, 4, 1), l("c", 3, 4, 9), l("d", 1, 2, 9)];
        assert_eq!(quick_pick(&list, true, 2).iter().map(|l| l.id.as_str()).collect::<Vec<_>>(), ["b", "a"]);
        assert_eq!(quick_pick(&list, true, 1)[0].id, "c");
        assert_eq!(quick_pick(&list, false, 1)[0].id, "d");
    }

    #[test]
    fn find_never_lists_our_own_lobby() {
        let l = |id: &str| Listing { id: id.into(), name: String::new(), doubles: false, court: 0, taken: 1, seats: 2, time: 0, code: String::new() };
        let ids = |v: Vec<Listing>| v.into_iter().map(|l| l.id).collect::<Vec<_>>();
        assert_eq!(ids(not_mine(vec![l("me"), l("a"), l("b")], Some("me".into()))), ["a", "b"]);
        // B332: Quickplay would otherwise dial itself
        assert!(quick_pick(&not_mine(vec![l("me")], Some("me".into())), false, 1).is_empty());
        assert_eq!(ids(not_mine(vec![l("a")], None)), ["a"]);
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
        let o = Online::parse("private = yes\n");
        assert!(o.private && Online::parse(&o.text()) == o);
    }

    /// B323: the menu's relay cycle (n0, Off, own once set) and what a typed relay URL may be.
    #[test]
    fn relay_cycle_and_urls() {
        let mut o = Online::default();
        let mut seen = vec![];
        for _ in 0..3 {
            o.next_relay();
            seen.push(o.relay.clone());
        }
        assert_eq!(seen, [Relay::Off, Relay::N0, Relay::Off]);
        o.set_own("https://relay.example.org/".into());
        assert_eq!(Online::parse(&o.text()), o);
        o.relay = Relay::Off;
        o.next_relay();
        assert_eq!(o.relay, Relay::Own("https://relay.example.org/".into()));
        o.next_relay();
        assert_eq!(o.relay, Relay::N0);
        assert_eq!(relay_url(" HTTPS://Relay.Example.org "), Ok("https://relay.example.org/".into()));
        assert_eq!(relay_url("http://10.0.0.2:3340"), Ok("http://10.0.0.2:3340/".into()));
        for bad in ["", "https://", "relay.example.org", "ftp://relay.example.org", "https://exa mple.org"] {
            assert!(relay_url(bad).is_err(), "{bad}");
        }
    }

    /// B116: the board slots on a local DHT testnet: two hosts take different slots, keep theirs on republish,
    /// and a third node finds both with their codes; a full board lists no more.
    #[test]
    fn slots_on_a_testnet() {
        use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let net = n0_mainline::Testnet::new(10).await.unwrap();
            let dht = |b: &[String]| Dht::builder().bootstrap(b).build().unwrap();
            let (a, b, c) = (dht(&net.bootstrap), dht(&net.bootstrap), dht(&net.bootstrap));
            let mut r = room(false);
            r.take("host", &[Pick::disc(0, 0)]);
            let listing = |code: &str| Listing { code: code.into(), ..Listing::of(SecretKey::generate().public(), &r) };
            let (la, lb) = (listing("AAAA"), listing("BBBB"));
            let (sa, sb) = (AtomicUsize::new(SLOTS), AtomicUsize::new(SLOTS));
            list_in_slot(&a, &sa, &la).await;
            list_in_slot(&b, &sb, &lb).await;
            let first = sa.load(Relaxed);
            assert!(first < SLOTS && sb.load(Relaxed) < SLOTS && first != sb.load(Relaxed));
            list_in_slot(&a, &sa, &la).await;
            assert_eq!(sa.load(Relaxed), first);
            let mut found: Vec<String> = find_slots(&c).await.into_iter().map(|l| l.code).collect();
            found.sort();
            assert_eq!(found, ["AAAA", "BBBB"]);
            // the rest of the board taken: one more host isn't listed
            let json = serde_json::to_string(&listing("CCCC")).unwrap();
            let free = (0..SLOTS).filter(|&i| i != first && i != sb.load(Relaxed)).map(|i| slot_name(i)).collect::<Vec<_>>();
            n0_future::join_all(free.iter().map(|n| publish(&c, n, &json))).await;
            let full = AtomicUsize::new(SLOTS);
            list_in_slot(&c, &full, &listing("DDDD")).await;
            assert_eq!(full.load(Relaxed), SLOTS);
        });
    }

    /// B116: a listing through the pkarr relay, as the browser reads and writes the board (a test name, not a real
    /// slot: no fake lobby on the live board). Needs the internet: `tools/check.sh -p hst slot_through_the_pkarr_relay -- --ignored`.
    #[test]
    #[ignore]
    fn slot_through_the_pkarr_relay() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut r = room(true);
            r.name = "🎾".repeat(24);
            r.take("host", &[Pick::disc(0, 0)]);
            let l = Listing { code: "TEST".into(), ..Listing::of(SecretKey::generate().public(), &r) };
            let name = format!("t{}", now() % 1_000_000_000);
            assert!(relay_put(&name, &serde_json::to_string(&l).unwrap()).await);
            let (t, seq) = relay_get(&name).await.expect("read back");
            assert!(fresh(seq));
            assert_eq!(serde_json::from_str::<Listing>(&t).unwrap(), l);
        });
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
            let cx = Ctx { ep: Some(ep), dht: a, lan: Lan::default(), ev: mpsc::channel().0, name: String::new(), relays: false };
            let mut r = room(true);
            r.take("host", &[Pick::disc(0, 0)]);
            announce(&cx, 4242, &r).await;
            let list = find(&b).await;
            assert_eq!(list.len(), 1);
            assert_eq!((list[0].id.clone(), list[0].taken, list[0].seats), (cx.ep.as_ref().unwrap().id().to_string(), 1, 4));
        });
    }

    /// B213 on a local DHT testnet, through the host task: a private lobby isn't in Find but joins by code; Public
    /// lists it on the next refresh, Private again takes it off (shown full) (~3 s).
    #[test]
    fn private_lobby_joins_by_code_only() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let net = n0_mainline::Testnet::new(10).await.unwrap();
            let dht = |b: &[String]| Dht::builder().bootstrap(b).build().unwrap();
            let finder = dht(&net.bootstrap);
            let cx = || async {
                let (ep, _) = endpoint(&Relay::Off).await.unwrap();
                let (tx, rx) = mpsc::channel();
                (Ctx { ep: Some(ep), dht: dht(&net.bootstrap), lan: Lan::default(), ev: tx, name: "g".into(), relays: false }, rx)
            };
            let ((hcx, hev), (gcx, gev)) = (cx().await, cx().await);
            let id = hcx.ep.as_ref().unwrap().id().to_string();
            let (hcmd, mut hrx) = amp::unbounded_channel();
            let (_gcmd, mut grx) = amp::unbounded_channel();
            let mut r = room(true);
            r.take("host", &[Pick::disc(0, 0)]);
            r.private = true;
            tokio::spawn(async move { host(&hcx, r, &mut hrx).await });
            let code = loop {
                if let Ok(Event::Code(c)) = hev.recv_timeout(Duration::from_secs(5)) {
                    break c;
                }
            };
            // listed or not, waiting up to 5 s for it to settle
            let listed = |want: bool| {
                let (finder, id) = (finder.clone(), id.clone());
                async move {
                    for _ in 0..10 {
                        if find(&finder).await.iter().any(|l| l.id == id) == want {
                            return true;
                        }
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                    false
                }
            };
            tokio::time::sleep(Duration::from_secs(1)).await;
            assert!(find(&finder).await.iter().all(|l| l.id != id), "a private lobby is in Find");
            tokio::spawn(async move { join(&gcx, &code, vec![Pick::disc(1, 0)], &mut grx).await });
            let mut joined = false;
            for _ in 0..250 {
                tokio::time::sleep(Duration::from_millis(20)).await;
                if gev.try_iter().any(|e| matches!(e, Event::Room(r) if r.private && r.open() == 2)) {
                    joined = true;
                    break;
                }
            }
            assert!(joined, "the code didn't join the private lobby");
            hcmd.send(Cmd::Private(false)).unwrap();
            assert!(listed(true).await, "Public didn't list it");
            hcmd.send(Cmd::Private(true)).unwrap();
            assert!(listed(false).await, "Private again didn't take it off");
        });
    }

    #[test]
    fn lan_listing_fits_mdns() {
        let mut r = room(true);
        // the longest name: 24 four-byte characters
        r.name = "🎾".repeat(24);
        r.take("host", &[Pick::disc(0, 0)]);
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
            let (host2, _hm2) = endpoint(&Relay::Off).await.unwrap();
            let (guest, gm) = endpoint(&Relay::Off).await.unwrap();
            let lan = watch_lan(gm);
            let mut r = room(false);
            r.take("host", &[Pick::disc(0, 0)]);
            host.set_user_data_for_address_lookup(lan_data(&Listing::of(host.id(), &r)));
            // B195: a doubles lobby beside it; Find lists both
            let mut r2 = room(true);
            r2.take("host", &[Pick::disc(0, 0)]);
            host2.set_user_data_for_address_lookup(lan_data(&Listing::of(host2.id(), &r2)));
            let t = tokio::time::Instant::now();
            // (only these hosts': other tests' hosts are on the LAN too)
            let ids = [host.id().to_string(), host2.id().to_string()];
            let mine = || lan_list(&lan).into_iter().filter(|l| ids.contains(&l.id)).collect::<Vec<_>>();
            while mine().len() < 2 && t.elapsed() < Duration::from_secs(15) {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            let mut list = mine();
            list.sort_by_key(|l| l.doubles);
            assert_eq!(list.len(), 2, "no LAN listings in 15 s (is multicast blocked?)");
            assert_eq!((list[0].id.clone(), list[0].doubles, list[0].open()), (host.id().to_string(), false, 1));
            assert_eq!((list[1].id.clone(), list[1].doubles, list[1].open()), (host2.id().to_string(), true, 3));
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

    /// B275 on loopback: a mod pick this game can't play is a message, not a dropped guest. A host's (an older
    /// build's) bad COM pick: the guest stays seated with a status; a guest's bad pick: the host says so to both.
    #[test]
    fn a_refused_mod_pick_keeps_the_guest() {
        let bad = crate::mods_online::Wire {
            id: "x".into(), name: "X".into(), donor: 3, hand: 1.0, base: 3, overrides: vec![("nope".into(), "1".into())], ai_row: 3,
            names: vec!["a".into()], parent: vec![None], rest: vec![[0; 16]], looks: 1, dir: None,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let testnet = n0_mainline::Testnet::new(3).await.unwrap();
            let cx = || async {
                let (ep, _) = endpoint(&Relay::Off).await.unwrap();
                let (tx, rx) = mpsc::channel();
                (Ctx { ep: Some(ep), dht: Dht::builder().bootstrap(&testnet.bootstrap).build().unwrap(), lan: Lan::default(), ev: tx, name: "g".into(), relays: false }, rx)
            };
            let ((hcx, hev), (gcx, gev)) = (cx().await, cx().await);
            let (_hcmd, mut hrx) = amp::unbounded_channel();
            let (gcmd, mut grx) = amp::unbounded_channel();
            let mut r = room(true);
            r.take("host", &[Pick::disc(3, 0), Pick { r#mod: Some(Box::new(bad.clone())), ..Pick::disc(3, 0) }]);
            tokio::spawn(async move { host(&hcx, r, &mut hrx).await });
            async fn next(ev: &mpsc::Receiver<Event>, want: impl Fn(&Event) -> bool) -> Event {
                for _ in 0..1500 {
                    while let Ok(e) = ev.try_recv() {
                        assert!(!matches!(e, Event::Failed(_)), "{e:?}");
                        if want(&e) {
                            return e;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                panic!("timed out")
            }
            let Event::Code(code) = next(&hev, |e| matches!(e, Event::Code(_))).await else { unreachable!() };
            tokio::spawn(async move { join(&gcx, &code, vec![Pick::disc(1, 0)], &mut grx).await });
            let Event::Status(s) = next(&gev, |e| matches!(e, Event::Status(s) if s.contains("can't play"))).await else { unreachable!() };
            assert!(s.contains("nope"), "{s}");
            next(&gev, |e| matches!(e, Event::Room(_))).await;
            gcmd.send(Cmd::Picks(vec![Pick { r#mod: Some(Box::new(bad)), ..Pick::disc(1, 0) }])).unwrap();
            next(&hev, |e| matches!(e, Event::Status(s) if s.contains("pick was refused"))).await;
            next(&gev, |e| matches!(e, Event::Chat { text, .. } if text.starts_with("Your pick was refused"))).await;
        });
    }

    /// M2i on loopback: the host brings a mod; the guest sees its wire, asks for its looks and gets the folder's pack;
    /// the match waits until both have said they're settled for this line-up.
    #[test]
    fn mod_looks_peer_to_peer_and_the_ready_gate() {
        let dir = std::env::temp_dir().join(format!("hst-m2i-net-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.json"), "{}").unwrap();
        let pack = crate::mods_online::pack(&dir).unwrap();
        let h = crate::mods_online::hash(&pack);
        let wire = crate::mods_online::Wire {
            id: "x".into(), name: "X".into(), donor: 3, hand: 1.0, base: 3, overrides: vec![], ai_row: 3,
            names: vec!["a".into()], parent: vec![None], rest: vec![[0; 16]], looks: h, dir: Some(dir.clone()),
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let testnet = n0_mainline::Testnet::new(3).await.unwrap();
            let cx = || async {
                let (ep, _) = endpoint(&Relay::Off).await.unwrap();
                let (tx, rx) = mpsc::channel();
                (Ctx { ep: Some(ep), dht: Dht::builder().bootstrap(&testnet.bootstrap).build().unwrap(), lan: Lan::default(), ev: tx, name: "g".into(), relays: false }, rx)
            };
            let ((hcx, hev), (gcx, gev)) = (cx().await, cx().await);
            let (hcmd, mut hrx) = amp::unbounded_channel();
            let (gcmd, mut grx) = amp::unbounded_channel();
            let mut r = room(false);
            r.take("host", &[Pick { r#mod: Some(Box::new(wire.clone())), ..Pick::disc(3, 0) }]);
            tokio::spawn(async move { host(&hcx, r, &mut hrx).await });
            // the next event `want` matches (30 s at most: the DHT testnet is slow under parallel load)
            async fn next(ev: &mpsc::Receiver<Event>, want: impl Fn(&Event) -> bool) -> Event {
                for _ in 0..1500 {
                    while let Ok(e) = ev.try_recv() {
                        if want(&e) {
                            return e;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                panic!("timed out")
            }
            let Event::Code(code) = next(&hev, |e| matches!(e, Event::Code(_))).await else { unreachable!() };
            tokio::spawn(async move { join(&gcx, &code, vec![Pick::disc(1, 0)], &mut grx).await });
            let Event::Room(seen) = next(&gev, |e| matches!(e, Event::Room(r) if r.open() == 0)).await else { unreachable!() };
            let (_, w) = seen.mods().next().expect("the mod's wire");
            assert_eq!((w.looks, w.dir.is_none(), &w.names), (h, true, &wire.names));
            gcmd.send(Cmd::Want(h)).unwrap();
            assert_eq!(next(&gev, |e| matches!(e, Event::Pack(..))).await, Event::Pack(h, pack));
            // ready and started, but the mods aren't settled yet
            gcmd.send(Cmd::Prepared(true)).unwrap();
            hcmd.send(Cmd::Start).unwrap();
            // the guest settled, the host not yet: no start
            gcmd.send(Cmd::Ready(seen.ver)).unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
            assert!(gev.try_iter().all(|e| !matches!(e, Event::Start(..))), "started before everyone was ready");
            hcmd.send(Cmd::Ready(seen.ver)).unwrap();
            let Event::Start(r, _) = next(&gev, |e| matches!(e, Event::Start(..))).await else { unreachable!() };
            assert_eq!(r.seats[0].as_ref().unwrap().pick.r#mod.as_ref().unwrap().looks, h);
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// B281 on loopback: the guest hasn't chosen about the host's mod; Start shows them deciding with the wait
    /// counting down, and at its end the stand-ins: the guest settles and the match starts.
    #[test]
    fn start_waits_for_a_mod_choice_then_gives_stand_ins() {
        DECIDE_MS.store(2000, std::sync::atomic::Ordering::Relaxed);
        let wire = crate::mods_online::Wire {
            id: "x".into(), name: "X".into(), donor: 3, hand: 1.0, base: 3, overrides: vec![], ai_row: 3,
            names: vec!["a".into()], parent: vec![None], rest: vec![[0; 16]], looks: 9, dir: None,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let testnet = n0_mainline::Testnet::new(3).await.unwrap();
            let cx = || async {
                let (ep, _) = endpoint(&Relay::Off).await.unwrap();
                let (tx, rx) = mpsc::channel();
                (Ctx { ep: Some(ep), dht: Dht::builder().bootstrap(&testnet.bootstrap).build().unwrap(), lan: Lan::default(), ev: tx, name: "g".into(), relays: false }, rx)
            };
            async fn next(ev: &mpsc::Receiver<Event>, want: impl Fn(&Event) -> bool) -> Event {
                for _ in 0..1500 {
                    while let Ok(e) = ev.try_recv() {
                        if want(&e) {
                            return e;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                panic!("timed out")
            }
            let ((hcx, hev), (gcx, gev)) = (cx().await, cx().await);
            let (hcmd, mut hrx) = amp::unbounded_channel();
            let (gcmd, mut grx) = amp::unbounded_channel();
            let mut r = room(false);
            r.take("host", &[Pick { r#mod: Some(Box::new(wire)), ..Pick::disc(3, 0) }]);
            tokio::spawn(async move { host(&hcx, r, &mut hrx).await });
            let Event::Code(code) = next(&hev, |e| matches!(e, Event::Code(_))).await else { unreachable!() };
            tokio::spawn(async move { join(&gcx, &code, vec![Pick::disc(1, 0)], &mut grx).await });
            let Event::Room(seen) = next(&gev, |e| matches!(e, Event::Room(r) if r.open() == 0)).await else { unreachable!() };
            let guest = seen.seats[1].as_ref().unwrap().peer.clone();
            gcmd.send(Cmd::Deciding(true)).unwrap();
            next(&hev, |e| matches!(e, Event::Room(r) if r.deciding.contains(&guest))).await;
            hcmd.send(Cmd::Ready(seen.ver)).unwrap();
            hcmd.send(Cmd::Start).unwrap();
            // the countdown, then the wait shown to the guest; no start meanwhile
            next(&gev, |e| matches!(e, Event::Room(r) if r.wait > 0 && !r.stand_in)).await;
            let Event::Room(over) = next(&gev, |e| matches!(e, Event::Room(r) if r.stand_in) || matches!(e, Event::Start(..))).await else {
                panic!("started before the wait was over")
            };
            assert_eq!(over.wait, 0);
            // the guest takes the stand-in (the menu's `settle`) and says so
            gcmd.send(Cmd::Deciding(false)).unwrap();
            gcmd.send(Cmd::Ready(over.ver)).unwrap();
            next(&gev, |e| matches!(e, Event::Start(..))).await;
        });
    }

    /// B216 on loopback: the host's computer player on a mod (in costume 2): the guest sees its wire, gets its looks
    /// from the host (the host's own mod, though the seat is the computer's) and the match starts with it.
    #[test]
    fn a_hosts_computer_player_on_a_mod_reaches_the_guest() {
        let dir = std::env::temp_dir().join(format!("hst-b216-net-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mod.json"), "{}").unwrap();
        let pack = crate::mods_online::pack(&dir).unwrap();
        let h = crate::mods_online::hash(&pack);
        let wire = crate::mods_online::Wire {
            id: "x".into(), name: "X".into(), donor: 3, hand: 1.0, base: 3, overrides: vec![], ai_row: 3,
            names: vec!["a".into()], parent: vec![None], rest: vec![[0; 16]], looks: h, dir: Some(dir.clone()),
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let testnet = n0_mainline::Testnet::new(3).await.unwrap();
            let cx = || async {
                let (ep, _) = endpoint(&Relay::Off).await.unwrap();
                let (tx, rx) = mpsc::channel();
                (Ctx { ep: Some(ep), dht: Dht::builder().bootstrap(&testnet.bootstrap).build().unwrap(), lan: Lan::default(), ev: tx, name: "g".into(), relays: false }, rx)
            };
            async fn next(ev: &mpsc::Receiver<Event>, want: impl Fn(&Event) -> bool) -> Event {
                for _ in 0..1500 {
                    while let Ok(e) = ev.try_recv() {
                        if want(&e) {
                            return e;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                panic!("timed out")
            }
            let ((hcx, hev), (gcx, gev)) = (cx().await, cx().await);
            let (hcmd, mut hrx) = amp::unbounded_channel();
            let (gcmd, mut grx) = amp::unbounded_channel();
            let mut r = room(true);
            r.take("host", &[Pick::disc(0, 0)]);
            tokio::spawn(async move { host(&hcx, r, &mut hrx).await });
            let Event::Code(code) = next(&hev, |e| matches!(e, Event::Code(_))).await else { unreachable!() };
            tokio::spawn(async move { join(&gcx, &code, vec![Pick::disc(1, 0)], &mut grx).await });
            next(&hev, |e| matches!(e, Event::Room(r) if r.open() == 2)).await;
            hcmd.send(Cmd::Bot(3, Some(Pick { r#mod: Some(Box::new(wire.clone())), ..Pick::disc(3, 2) }))).unwrap();
            let Event::Room(seen) = next(&gev, |e| matches!(e, Event::Room(r) if r.seats[3].is_some())).await else { unreachable!() };
            let s = seen.seats[3].as_ref().unwrap();
            assert_eq!((s.human(), s.pick.costume, s.pick.r#mod.as_ref().map(|w| w.looks)), (false, 2, Some(h)));
            gcmd.send(Cmd::Want(h)).unwrap();
            assert_eq!(next(&gev, |e| matches!(e, Event::Pack(..))).await, Event::Pack(h, pack));
            gcmd.send(Cmd::Prepared(true)).unwrap();
            gcmd.send(Cmd::Ready(seen.ver)).unwrap();
            hcmd.send(Cmd::Ready(seen.ver)).unwrap();
            next(&hev, |e| matches!(e, Event::Room(r) if r.ready.len() == 1)).await;
            hcmd.send(Cmd::Start).unwrap();
            let Event::Start(r, _) = next(&gev, |e| matches!(e, Event::Start(..))).await else { unreachable!() };
            let s = r.seats[3].as_ref().unwrap();
            assert_eq!((s.peer.as_str(), s.pick.char, s.pick.costume, s.pick.r#mod.as_ref().map(|w| w.id.as_str())), (CPU, 3, 2, Some("x")));
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn three_peers_see_the_hosts_court_and_start_on_it() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let testnet = n0_mainline::Testnet::new(3).await.unwrap();
            let cx = || async {
                let (ep, _) = endpoint(&Relay::Off).await.unwrap();
                let (tx, rx) = mpsc::channel();
                (Ctx { ep: Some(ep), dht: Dht::builder().bootstrap(&testnet.bootstrap).build().unwrap(), lan: Lan::default(), ev: tx, name: "g".into(), relays: false }, rx)
            };
            async fn next(ev: &mpsc::Receiver<Event>, want: impl Fn(&Event) -> bool) -> Event {
                for _ in 0..1500 {
                    while let Ok(e) = ev.try_recv() {
                        if want(&e) {
                            return e;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                panic!("timed out")
            }
            let ((hcx, hev), (acx, aev), (bcx, bev)) = (cx().await, cx().await, cx().await);
            let (hcmd, mut hrx) = amp::unbounded_channel();
            let (acmd, mut arx) = amp::unbounded_channel();
            let (bcmd, mut brx) = amp::unbounded_channel();
            let mut r = room(true);
            r.take("host", &[Pick::disc(0, 0)]);
            tokio::spawn(async move { host(&hcx, r, &mut hrx).await });
            let Event::Code(code) = next(&hev, |e| matches!(e, Event::Code(_))).await else { unreachable!() };
            let c2 = code.clone();
            tokio::spawn(async move { join(&acx, &code, vec![Pick::disc(1, 0)], &mut arx).await });
            next(&aev, |e| matches!(e, Event::Room(_))).await;
            tokio::spawn(async move { join(&bcx, &c2, vec![Pick::disc(2, 0)], &mut brx).await });
            next(&hev, |e| matches!(e, Event::Room(r) if r.open() == 1)).await;
            // the host alone picks the court: every peer sees it
            hcmd.send(Cmd::Venue(7, 0, 4, 4)).unwrap();
            next(&aev, |e| matches!(e, Event::Room(r) if r.court == 7)).await;
            next(&bev, |e| matches!(e, Event::Room(r) if r.court == 7)).await;
            acmd.send(Cmd::Prepared(true)).unwrap();
            bcmd.send(Cmd::Prepared(true)).unwrap();
            next(&hev, |e| matches!(e, Event::Room(r) if r.ready.len() == 2)).await;
            hcmd.send(Cmd::Start).unwrap();
            for ev in [&hev, &aev, &bev] {
                let Event::Start(r, _) = next(ev, |e| matches!(e, Event::Start(..))).await else { unreachable!() };
                assert_eq!(r.court, 7);
            }
        });
    }

    /// M20: chat through the host in the lobby and in the match: everyone else hears a line with the sender's lobby
    /// name (a guest can't say it's someone else), cleaned, and a burst past the limit is dropped.
    #[test]
    fn chat_in_the_lobby_and_the_match() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let testnet = n0_mainline::Testnet::new(3).await.unwrap();
            let cx = |name: &str| {
                let name = name.to_string();
                let bootstrap = testnet.bootstrap.clone();
                async move {
                    let (ep, _) = endpoint(&Relay::Off).await.unwrap();
                    let (tx, rx) = mpsc::channel();
                    (Ctx { ep: Some(ep), dht: Dht::builder().bootstrap(&bootstrap).build().unwrap(), lan: Lan::default(), ev: tx, name, relays: false }, rx)
                }
            };
            async fn next(ev: &mpsc::Receiver<Event>, want: impl Fn(&Event) -> bool) -> Event {
                for _ in 0..1500 {
                    while let Ok(e) = ev.try_recv() {
                        if want(&e) {
                            return e;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                panic!("timed out")
            }
            /// The chat lines heard within `ms` (name, text).
            async fn heard(ev: &mpsc::Receiver<Event>, ms: u64) -> Vec<(String, String)> {
                tokio::time::sleep(Duration::from_millis(ms)).await;
                // two senders' lines can cross on the way: order doesn't matter here
                let mut v: Vec<_> = ev.try_iter().filter_map(|e| if let Event::Chat { name, text, .. } = e { Some((name, text)) } else { None }).collect();
                v.sort();
                v
            }
            let ((hcx, hev), (acx, aev), (bcx, bev)) = (cx("Host").await, cx("Ann").await, cx("Bob").await);
            let (hcmd, mut hrx) = amp::unbounded_channel();
            let (acmd, mut arx) = amp::unbounded_channel();
            let (bcmd, mut brx) = amp::unbounded_channel();
            let mut r = room(true);
            r.take("host", &[Pick::disc(0, 0)]);
            tokio::spawn(async move { host(&hcx, r, &mut hrx).await });
            let Event::Code(code) = next(&hev, |e| matches!(e, Event::Code(_))).await else { unreachable!() };
            let c2 = code.clone();
            tokio::spawn(async move { join(&acx, &code, vec![Pick::disc(1, 0)], &mut arx).await });
            next(&aev, |e| matches!(e, Event::Room(_))).await;
            tokio::spawn(async move { join(&bcx, &c2, vec![Pick::disc(2, 0)], &mut brx).await });
            next(&hev, |e| matches!(e, Event::Room(r) if r.open() == 1)).await;
            next(&bev, |e| matches!(e, Event::Room(_))).await;
            // the lobby: a guest's line (claiming to be the host, with a bell and a newline) and the host's
            acmd.send(Cmd::Send(Msg::Chat { from: "host".into(), name: "Host".into(), text: "hi\u{7} all\n".into() })).unwrap();
            hcmd.send(Cmd::Chat("welcome".into())).unwrap();
            let ann = ("Ann".to_string(), "hi all".to_string());
            let host = ("Host".to_string(), "welcome".to_string());
            assert_eq!(heard(&bev, 1500).await, [ann.clone(), host.clone()]);
            assert_eq!(heard(&hev, 0).await, [ann.clone()]);
            assert_eq!(heard(&aev, 0).await, [host.clone()]);
            // a burst: only the first few get through
            for i in 0..8 {
                bcmd.send(Cmd::Chat(format!("spam {i}"))).unwrap();
            }
            assert_eq!(heard(&aev, 1500).await.len(), crate::chat::BURST);
            // the match
            acmd.send(Cmd::Prepared(true)).unwrap();
            bcmd.send(Cmd::Prepared(true)).unwrap();
            next(&hev, |e| matches!(e, Event::Room(r) if r.ready.len() == 2)).await;
            hcmd.send(Cmd::Start).unwrap();
            for ev in [&hev, &aev, &bev] {
                next(ev, |e| matches!(e, Event::Start(..))).await;
            }
            acmd.send(Cmd::Chat("good luck".into())).unwrap();
            hcmd.send(Cmd::Chat("you too".into())).unwrap();
            let (ann, host) = (("Ann".to_string(), "good luck".to_string()), ("Host".to_string(), "you too".to_string()));
            assert_eq!(heard(&bev, 1500).await, [ann.clone(), host.clone()]);
            assert_eq!(heard(&hev, 0).await, [ann]);
            assert_eq!(heard(&aev, 0).await, [host]);
        });
    }

    #[test]
    fn short_code_parts() {
        let t = "endpointabc".repeat(60);
        assert_eq!(txt_of(&dns_txt(&t)).as_deref(), Some(t.as_str()));
        assert_eq!(code_in(" k7qx "), Some("K7QX".into()));
        assert_eq!(code_in(&link("K7QX")), Some("K7QX".into()));
        assert_eq!(code_in("https://fizl0.github.io/hot-shots-tennis-rust/?code=k7qx&x=1"), Some("K7QX".into()));
        assert_eq!(code_in("https://opentennis.dev/?code=k7qx&x=1"), Some("K7QX".into()));
        assert_eq!(code_in("K0QX"), None, "0 isn't a symbol");
        assert_eq!(code_in("K7QXY"), None);
        let c = new_short();
        assert!(is_short(&c), "{c}");
        assert_ne!(code_key("AAAA").public(), code_key("AAAB").public());
        assert!(fresh(unix_micros() - 80_000_000) && !fresh(unix_micros() - 100_000_000));
    }

    /// A local DHT testnet, two lobbies through a short code (~20 s): `tools/check.sh -p hst short_code_on_a_testnet
    /// -- --ignored` after touching the DHT or short codes.
    #[test]
    #[ignore]
    fn short_code_on_a_testnet() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let testnet = n0_mainline::Testnet::new(10).await.unwrap();
            let cx = || async {
                let (ep, _) = endpoint(&Relay::Off).await.unwrap();
                let (tx, rx) = mpsc::channel();
                (Ctx { ep: Some(ep), dht: Dht::builder().bootstrap(&testnet.bootstrap).build().unwrap(), lan: Lan::default(), ev: tx, name: "g".into(), relays: false }, rx)
            };
            async fn next(ev: &mpsc::Receiver<Event>, want: impl Fn(&Event) -> bool) -> Event {
                for _ in 0..1500 {
                    while let Ok(e) = ev.try_recv() {
                        if want(&e) {
                            return e;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                panic!("timed out")
            }
            let ((hcx, hev), (gcx, gev)) = (cx().await, cx().await);
            let (hcmd, mut hrx) = amp::unbounded_channel();
            let (gcmd, mut grx) = amp::unbounded_channel();
            let mut r = room(true);
            r.take("host", &[Pick::disc(3, 0)]);
            tokio::spawn(async move { host(&hcx, r, &mut hrx).await });
            // the host's short code, read off its lobby, typed into the second instance
            let Event::Short(code) = next(&hev, |e| matches!(e, Event::Short(_))).await else { unreachable!() };
            assert!(is_short(&code), "no short code: {code:?}");
            // a code no lobby has
            let (_, mut nrx) = amp::unbounded_channel();
            let other = if code == "2222" { "3333" } else { "2222" };
            assert!(connect(&gcx, other, &mut nrx).await.is_none());
            assert!(matches!(next(&gev, |e| matches!(e, Event::Failed(_))).await, Event::Failed(f) if f.contains(other)));
            let joining = format!("Joining {code}...");
            let shown = code.clone();
            tokio::spawn(async move { join(&gcx, &code, vec![Pick::disc(1, 0)], &mut grx).await });
            // B268: the status names the dialled code, not the host's endpoint ID
            next(&gev, |e| matches!(e, Event::Status(s) if *s == joining)).await;
            // doubles with two seats open: the guest's ready starts it, the computer takes the open seats
            let Event::Room(seen) = next(&hev, |e| matches!(e, Event::Room(r) if r.open() == 2)).await else { unreachable!() };
            let guest = seen.seats[1].as_ref().unwrap().peer.clone();
            // B114: seated, the guest's status moves on from "Joining..."
            next(&gev, |e| matches!(e, Event::Status(s) if s.starts_with("Joined"))).await;
            assert_eq!(seen.names.get(&guest).map(String::as_str), Some("g"), "the guest's name from its Hello");
            // B195: the guest's seat held while it picks; its picks and the host's court change reach everyone
            assert!(seen.picking.contains(&guest));
            gcmd.send(Cmd::Picks(vec![Pick::disc(5, 1)])).unwrap();
            hcmd.send(Cmd::Venue(2, 1, 4, 3)).unwrap();
            let Event::Room(seen) = next(&gev, |e| matches!(e, Event::Room(r) if r.court == 2 && r.picking.is_empty())).await else { unreachable!() };
            assert_eq!((seen.seats[1].as_ref().unwrap().pick.char, seen.seats[1].as_ref().unwrap().pick.costume, seen.umpire), (5, 1, 3));
            // B322: the guest's lobby has the host's short code to pass on
            assert_eq!(seen.short, shown);
            // B215: everyone ready; B292: two seats still open, so no countdown until the host's Start
            gcmd.send(Cmd::Prepared(true)).unwrap();
            next(&hev, |e| matches!(e, Event::Room(r) if r.ready.contains(&guest))).await;
            tokio::time::sleep(Duration::from_millis(1200)).await;
            assert!(gev.try_iter().all(|e| !matches!(e, Event::Start(..)) && !matches!(e, Event::Room(r) if r.count > 0)), "counted down with open seats");
            hcmd.send(Cmd::Start).unwrap();
            let Event::Start(r, me) = next(&gev, |e| matches!(e, Event::Start(..))).await else { unreachable!() };
            assert_eq!(me, guest);
            assert_eq!(r.open(), 0);
            let cpus: Vec<usize> = r.seats.iter().flatten().filter(|s| !s.human()).map(|s| s.pick.char).collect();
            assert_eq!(cpus, [0, 1], "the open seats to the computer, characters nobody picked");
            assert_eq!((r.court, r.seats[1].as_ref().unwrap().pick.char), (2, 5), "the match has the lobby's last picks");
            assert!(matches!(next(&hev, |e| matches!(e, Event::Start(..))).await, Event::Start(_, h) if h == "host"));
            // B196: back to the lobby together (the computer's seats open again), ready again, and a new start
            let mut back = r.clone();
            back.back_to_lobby(&[]);
            hcmd.send(Cmd::Send(Msg::Lobby(back))).unwrap();
            let Event::Match(Msg::Lobby(l)) = next(&gev, |e| matches!(e, Event::Match(Msg::Lobby(_)))).await else { unreachable!() };
            assert_eq!(l.open(), 2);
            gcmd.send(Cmd::Prepared(true)).unwrap();
            hcmd.send(Cmd::Start).unwrap();
            let Event::Start(r, _) = next(&gev, |e| matches!(e, Event::Start(..))).await else { unreachable!() };
            assert_eq!(r.open(), 0);
        });
    }
}
