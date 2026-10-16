//! Lockstep netplay (M2g): a tick runs only once every seat's pads for it are in. Each machine sends its own seats'
//! pads `delay` ticks ahead (the host picks the delay from the slowest guest's round trip, `net::delay_for`), so at
//! steady speed nobody waits; a peer that falls behind stalls everyone, and after [`STALL`] the screen says whose pads
//! it waits for. The first `delay` ticks are idle for every seat.
//!
//! The gate (`mode::Gate`) runs before each of the match's ticks: it hands the tick's `[Pad; 4]` to the match as a
//! one-tick `Replay` (`tick_input`), so the match never reads local devices itself (while an instant replay plays its
//! own queue, only the tick's presses go in, for its ✕/○ skip: `run_on`). Before every [`HASH_EVERY`]th tick
//! each machine sends its `snapshot::state_hash`; on a mismatch each machine logs it and saves the inputs (every
//! seat's, every tick, the same on every machine) with the line-up as `desync-*.json` (beside the disc; in the browser a
//! download and the console; the ignored test `replays_a_desync` runs one again), says so on screen and plays on
//! (B154), and the host sends the peers its state to take on (B154b, `resync.rs`). The pause screen opens and closes
//! on every machine; the post-point press is a pad press, so it travels with the pads. A peer going silent pauses
//! everyone until it rejoins, a bot takes its seat or the match ends (B196, `recover.rs`). The court intro before the
//! match runs its ticks on a lockstep of its own (`Msg::Intro`, `intro_tick`), so a ✕ on any machine skips it on the
//! same tick everywhere.

use super::tick_input::{self, IDLE, Replay};
use super::{Game, Pads, front_end, snapshot};
use crate::net::{Cmd, Event, Msg, Net, Room};
use bevy::prelude::*;
use hst_sim::replay::Pad;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Hashes are compared before every this many ticks.
pub const HASH_EVERY: u32 = 30;
/// A stall this long (real seconds) puts up "Waiting for ...".
const STALL: f32 = 0.25;
/// How long the end message stays up before the match closes.
const END: f32 = 5.0;
/// The desync message's start (a peer that hears it saves its inputs too).
const DESYNC: &str = "Out of sync";
/// How long a desync's note stays up (real seconds).
const NOTE: f32 = 6.0;

/// `--pads`' gamepad for a peer's seat.
const REMOTE: usize = 99;

/// The online match's seed for the match setup (`main.rs`, the weather generator's), `u64::MAX` offline.
pub static SEED: AtomicU64 = AtomicU64::new(u64::MAX);

/// The online match's seed, once (the next match is offline unless it sets one again).
pub fn seed() -> Option<u32> {
    u32::try_from(SEED.swap(u64::MAX, Ordering::Relaxed)).ok()
}

pub fn wire(p: Pad) -> [u8; 6] {
    let b = p.buttons.to_le_bytes();
    [b[0], b[1], p.rx, p.ry, p.lx, p.ly]
}

pub fn unwire(w: [u8; 6]) -> Pad {
    Pad { buttons: u16::from_le_bytes([w[0], w[1]]), rx: w[2], ry: w[3], lx: w[4], ly: w[5] }
}

/// The lockstep bookkeeping: every seat's pads by tick as they come in, this machine's hashes and the peers'.
pub struct Lockstep {
    /// The next tick to run.
    pub tick: u32,
    pub delay: u32,
    /// Seats 1P..4P taken (an empty seat is always idle).
    seats: [bool; 4],
    /// This machine's seats.
    pub mine: Vec<usize>,
    pending: BTreeMap<u32, [Option<Pad>; 4]>,
    /// Every tick's pads as run.
    pub log: Vec<[Pad; 4]>,
    own: BTreeMap<u32, u64>,
    theirs: Vec<(u32, u64)>,
}

impl Lockstep {
    pub fn new(seats: [bool; 4], mine: Vec<usize>, delay: u32) -> Self {
        let mut l = Lockstep { tick: 0, delay, seats, mine, pending: BTreeMap::new(), log: Vec::new(), own: BTreeMap::new(), theirs: Vec::new() };
        for t in 0..delay {
            *l.slot(t) = [Some(IDLE); 4];
        }
        l
    }

    /// B196: the lockstep from tick `tick` on (a restart), on the ticks before it that `log` ran (`last` the one
    /// before `tick`; a rejoiner's are idle).
    pub fn from(seats: [bool; 4], mine: Vec<usize>, delay: u32, tick: u32, mut log: Vec<[Pad; 4]>, last: [Pad; 4]) -> Self {
        let mut l = Lockstep::new(seats, mine, delay);
        l.pending = (tick..tick + delay).map(|t| (t, [Some(IDLE); 4])).collect();
        log.resize(tick as usize, [IDLE; 4]);
        if let Some(p) = log.last_mut() {
            *p = last;
        }
        (l.tick, l.log) = (tick, log);
        l
    }

    fn slot(&mut self, t: u32) -> &mut [Option<Pad>; 4] {
        let seats = self.seats;
        self.pending.entry(t).or_insert(seats.map(|s| if s { None } else { Some(IDLE) }))
    }

    /// A peer's pads for `tick`.
    pub fn heard(&mut self, tick: u32, seats: &[(u8, [u8; 6])]) {
        if tick < self.tick {
            return;
        }
        let slot = self.slot(tick);
        for &(s, p) in seats {
            if let Some(x) = slot.get_mut(s as usize) {
                *x = Some(unwire(p));
            }
        }
    }

    /// The seats whose pads for the next tick aren't in.
    pub fn missing(&self) -> Vec<usize> {
        match self.pending.get(&self.tick) {
            Some(s) => (0..4).filter(|&i| s[i].is_none()).collect(),
            None => (0..4).filter(|&i| self.seats[i]).collect(),
        }
    }

    /// Runs the next tick (`missing` empty): its pads, and the message carrying this machine's `local` pads (its seats
    /// in order) for the tick `delay` on.
    pub fn advance(&mut self, local: &[Pad]) -> ([Pad; 4], Msg) {
        let now = self.pending.remove(&self.tick).expect("the tick's pads").map(|p| p.expect("every seat's pad"));
        let at = self.tick + self.delay;
        // P52: lent out while the slot is borrowed, not copied
        let mine = std::mem::take(&mut self.mine);
        let slot = self.slot(at);
        for (&s, &p) in mine.iter().zip(local) {
            slot[s] = Some(p);
        }
        self.log.push(now);
        self.tick += 1;
        let msg = Msg::Pads { tick: at, seats: mine.iter().zip(local).map(|(&s, &p)| (s as u8, wire(p))).collect() };
        self.mine = mine;
        (now, msg)
    }

    /// This machine's hash before tick `t`; a mismatch with a peer's: (tick, ours, theirs).
    pub fn own_hash(&mut self, t: u32, h: u64) -> Option<(u32, u64, u64)> {
        self.own.insert(t, h);
        self.check()
    }

    pub fn their_hash(&mut self, t: u32, h: u64) -> Option<(u32, u64, u64)> {
        self.theirs.push((t, h));
        self.check()
    }

    fn check(&mut self) -> Option<(u32, u64, u64)> {
        let mut bad = None;
        // ponytail: our hashes are all kept (one per 30 ticks, ~16 bytes each)
        self.theirs.retain(|&(t, h)| match self.own.get(&t) {
            Some(&o) => {
                if o != h && bad.is_none() {
                    bad = Some((t, o, h));
                }
                false
            }
            None => true,
        });
        bad
    }
}

/// The online match: the lockstep, the line-up, what goes out to the peers.
#[derive(Resource)]
pub struct Netplay {
    pub lock: Lockstep,
    pub room: Room,
    /// This machine's peer name in `room`.
    pub me: String,
    /// For the peers (the pump hands them to `Net`; without one, e.g. in tests, whoever runs it carries them).
    pub out: Vec<Msg>,
    /// Why the match ended; it closes `END` seconds later.
    pub end: Option<String>,
    /// The states went apart (a report is saved for the first mismatch; until a resync, B154b), and the note on
    /// screen with its seconds left.
    pub desynced: bool,
    /// Where the match's one desync report went (B276: a match that keeps going out of sync and back saved its whole
    /// input log again each time, until the browser ran out of memory).
    report: Option<String>,
    note: Option<(String, f32)>,
    paused: bool,
    /// The last hashed tick and its state's text (`snapshot::state_text`), for a desync's report.
    pub(super) state: (u32, String),
    pub(super) stalled: f32,
    ended: f32,
    /// `HST_NET_MASH=seed`: this machine's seats press random buttons (unattended tests).
    pub(super) mash: Option<u64>,
    /// Bringing the states back together after a desync (B154b).
    pub resync: super::resync::Resync,
    /// The court intro's own lockstep while it runs (`intro.rs` sets it; B42e): each tick's pad says whether that
    /// seat pressed ✕.
    pub(super) intro: Option<Lockstep>,
    /// A peer gone silent: waiting, a bot taking over, a rejoin (B196).
    pub recover: super::recover::Recover,
}

impl Netplay {
    pub fn new(room: Room, me: String) -> Self {
        let taken = |i: usize| room.seats.get(i).is_some_and(|s| s.as_ref().is_some_and(crate::net::Seat::human));
        let mine = (0..room.seats.len()).filter(|&i| room.seats[i].as_ref().is_some_and(|s| s.peer == me)).collect();
        let rejoin = room.rejoin;
        let others: Vec<String> = room.seats.iter().flatten().filter(|s| s.human() && s.peer != me).map(|s| s.peer.clone()).collect();
        let mash = std::env::var("HST_NET_MASH").ok().and_then(|v| v.parse::<u64>().ok()).map(|s| s | 1);
        Netplay {
            lock: Lockstep::new(std::array::from_fn(taken), mine, room.delay.max(1)),
            room,
            me,
            out: Vec::new(),
            end: None,
            desynced: false,
            report: None,
            note: None,
            paused: false,
            state: (0, String::new()),
            stalled: 0.0,
            ended: 0.0,
            mash,
            intro: None,
            resync: Default::default(),
            recover: super::recover::Recover::new(rejoin, others.into_iter()),
        }
    }

    /// The court intro's lockstep starts: the match's seats and delay.
    pub(super) fn start_intro(&mut self) {
        self.intro = Some(Lockstep::new(self.lock.seats, self.lock.mine.clone(), self.lock.delay));
    }

    /// Ends the match (the first reason stays), telling the peers if `tell`.
    pub(super) fn finish(&mut self, why: String, tell: bool) {
        if self.end.is_none() {
            if tell {
                self.out.push(Msg::Bye(why.clone()));
            }
            info!("online match over: {why}");
            self.end = Some(why);
        }
    }

    /// The next random pad for seat `s` (`HST_NET_MASH`): the d-pad changes every half second, a face button pressed
    /// about every 20 ticks.
    pub(super) fn mashed(&mut self, s: usize) -> Pad {
        let r = self.mash.as_mut().unwrap();
        *r ^= *r << 13;
        *r ^= *r >> 7;
        *r ^= *r << 17;
        let t = self.lock.tick as u64 + s as u64 * 7;
        let dir = [0, 0x10, 0x20, 0x40, 0x80, 0x30, 0x60, 0xc0, 0x90][((t / 30).wrapping_mul(0x9e37_79b9) >> 7) as usize % 9];
        let press = if (*r).is_multiple_of(20) { [0x1000, 0x2000, 0x4000, 0x8000][(*r >> 8) as usize % 4] } else { 0 };
        Pad { buttons: dir | press, ..IDLE }
    }
}

/// Every seat's pads, every tick, and the line-up: written when hashes differ, once a match (later calls say where).
fn save_desync(w: &mut World, why: &str) -> String {
    if let Some(at) = &w.resource::<Netplay>().report {
        return at.clone();
    }
    let np = w.resource::<Netplay>();
    let iso = w.get_resource::<crate::Args>().map_or(String::new(), |a| a.iso.clone());
    let unix = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let me: String = np.me.chars().take(8).collect();
    // tests running at once (threads, processes) share one folder and the names host/g: two desyncs in the same
    // second took each other's report (one test deleted the file the other then expected)
    let path = if cfg!(test) {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!("hst-desync-{}-{n}-{me}.json", std::process::id()))
    } else {
        crate::net::Online::path(&iso).with_file_name(format!("desync-{unix}-{me}.json"))
    };
    let pads: Vec<[[u8; 6]; 4]> = np.lock.log.iter().map(|t| t.map(wire)).collect();
    let json = serde_json::json!({ "why": why, "room": np.room, "me": np.me, "pads": pads, "state_tick": np.state.0, "state": np.state.1 });
    #[cfg(target_arch = "wasm32")]
    let saved = crate::web::save_file(&path.file_name().unwrap().to_string_lossy(), &json.to_string());
    #[cfg(not(target_arch = "wasm32"))]
    let saved = std::fs::write(&path, json.to_string()).map(|()| path.display().to_string());
    let at = match saved {
        Ok(at) => at,
        Err(e) => format!("(not saved: {e})"),
    };
    w.resource_mut::<Netplay>().report = Some(at.clone());
    at
}

/// The states differ at tick `t`: logged every time, the first time also the report and a note on screen; the match
/// plays on.
pub(super) fn desync(w: &mut World, (t, ours, theirs): (u32, u64, u64)) {
    // B249: a guest's checksums can tell it only after it took the host's state: still noted and saved, once
    let np = w.resource::<Netplay>();
    let mended = np.resync.stale(t);
    if mended && np.note.is_some() {
        return;
    }
    let why = format!("{DESYNC} at tick {t} (hash {ours:016x}, a peer's {theirs:016x})");
    warn!("online: {why}");
    if !mended {
        super::resync::ask(&mut w.resource_mut::<Netplay>());
        if w.resource::<Netplay>().desynced {
            return;
        }
    }
    let path = save_desync(w, &why);
    info!("online: inputs saved to {path}");
    let mut np = w.resource_mut::<Netplay>();
    np.desynced = !mended;
    np.note = Some((format!("{why}: playing on.\nInputs saved to {path}"), NOTE));
}

/// A network event into the match.
pub fn take(w: &mut World, e: Event) {
    let over = w.get_resource::<Game>().is_some_and(|g| g.match_stats.result.is_some());
    let Some(mut np) = w.get_resource_mut::<Netplay>() else { return };
    np.recover.heard_from(&e);
    match e {
        Event::Match(Msg::Pads { tick, seats }) => np.lock.heard(tick, &seats),
        Event::Match(Msg::Intro { tick, seats }) => {
            if let Some(l) = &mut np.intro {
                l.heard(tick, &seats);
            }
        }
        Event::Match(Msg::Hash { tick, hash }) => {
            if let Some(bad) = np.lock.their_hash(tick, hash) {
                desync(w, bad);
            }
        }
        Event::Match(Msg::Pause(open)) => {
            np.paused = open;
            front_end::set_paused(w, open);
        }
        Event::Datagram(from, data) => super::rollback::heard(w, from, &data),
        Event::Match(m @ (Msg::State { .. } | Msg::Resynced(_))) => super::resync::heard(&mut np, m),
        // B279: the host left the result screen: its lobby once this machine leaves it too (`again`)
        Event::Match(Msg::Lobby(room)) if over => np.recover.lobby = Some(room),
        Event::Match(m @ (Msg::Away { .. } | Msg::Choose(_) | Msg::Restart { .. } | Msg::Rejoined { .. } | Msg::Lobby(_))) => super::recover::heard(&mut np, m),
        Event::Match(Msg::Bye(why)) | Event::Dropped(why) => {
            if why.starts_with(DESYNC) && np.end.is_none() {
                let path = save_desync(w, &why);
                w.resource_mut::<Netplay>().finish(format!("{why}\nInputs saved to {path}"), false);
            } else if why.contains("The match is over") {
                np.finish(why, false);
            } else {
                np.finish(format!("{why} The match is over."), false);
            }
        }
        _ => {}
    }
}

/// The network's events in, the match's messages (and rollback's datagrams) out.
pub(super) fn pump(w: &mut World) {
    loaded(w);
    let Some(net) = w.get_resource::<Net>() else { return };
    // B196: going back to the lobby: its events are the menu's from now on
    let lobby = w.get_resource::<Netplay>().is_some_and(|n| n.recover.lobby.is_some());
    for e in if lobby { Vec::new() } else { net.poll() } {
        take(w, e);
    }
    let out = w.get_resource_mut::<Netplay>().map(|mut n| std::mem::take(&mut n.out)).unwrap_or_default();
    let datagrams = super::rollback::outgoing(w);
    let net = w.resource::<Net>();
    for m in out {
        net.send(Cmd::Send(m));
    }
    for (to, d) in datagrams {
        net.send(Cmd::Datagram(to, d));
    }
}

/// B200: this machine's loading screen down: the peers hear it. Once everyone's is, presses reach the match again.
pub(super) fn loaded(w: &mut World) {
    let covered = crate::mode::covered(w);
    let Some(mut np) = w.get_resource_mut::<Netplay>() else { return };
    // P52: the name is copied only the once it goes in
    if !covered && !np.recover.loaded.contains(&np.me) {
        let me = np.me.clone();
        np.recover.loaded.insert(me.clone());
        info!("online: loaded the match");
        np.out.push(Msg::Loaded(me));
    }
    if np.recover.all_loaded(&np.room) {
        w.remove_resource::<crate::mode::Hush>();
    }
}

/// B200: every peer has loaded the match (its ticks, the court intro's too, wait until then).
pub(super) fn started(np: &Netplay) -> bool {
    np.recover.all_loaded(&np.room)
}

/// The match's tick gate: true runs the next tick on every seat's pads.
pub fn gate(w: &mut World) -> bool {
    pump(w);
    super::resync::lockstep(w);
    super::recover::restarted(w);
    let Some(np) = w.get_resource::<Netplay>() else { return true };
    if np.end.is_some() || np.recover.hold || !started(np) || !np.lock.missing().is_empty() {
        return false;
    }
    let t = np.lock.tick;
    if t % HASH_EVERY == 0 {
        let text = snapshot::state_text(w);
        let h = snapshot::fnv(&text);
        if t % (HASH_EVERY * 60) == 0 {
            info!("online tick {t}: hash {h:016x}");
        }
        let mut np = w.resource_mut::<Netplay>();
        np.state = (t, text);
        np.out.push(Msg::Hash { tick: t, hash: h });
        if let Some(bad) = np.lock.own_hash(t, h) {
            desync(w, bad);
        }
    }
    // this machine's seats' pads (each seat's local device sits in the slot of the same number, `--pads`)
    // P52: lent out (not copied) while the pads are read
    let mine = std::mem::take(&mut w.resource_mut::<Netplay>().lock.mine);
    let mut local: Vec<Pad> = {
        let mut pads = w.resource_mut::<Pads>();
        mine.iter().map(|&s| tick_input::take(&mut pads.slots[s])).collect()
    };
    let mut np = w.resource_mut::<Netplay>();
    if np.mash.is_some() {
        local = mine.iter().map(|&s| np.mashed(s)).collect();
    }
    np.lock.mine = mine;
    let (now, msg) = np.lock.advance(&local);
    np.out.push(msg);
    np.stalled = 0.0;
    let last = np.lock.log.len().checked_sub(2).map_or([IDLE; 4], |p| np.lock.log[p]);
    run_on(w, now, last);
    pump(w);
    true
}

/// The court intro's next tick in lockstep (B42e), this machine's seats pressing ✕ if `skip`: `None` while a seat's
/// pad for it isn't in, else whether any seat pressed ✕ on it (the same tick on every machine).
pub(super) fn intro_tick(w: &mut World, skip: bool) -> Option<bool> {
    pump(w);
    let mut np = w.resource_mut::<Netplay>();
    let np = &mut *np;
    // B196: a restart ended the intro (it skips to its end)
    if np.intro.is_none() {
        return Some(true);
    }
    if !started(np) {
        return None;
    }
    let l = np.intro.as_mut().filter(|l| l.missing().is_empty())?;
    let local = vec![Pad { buttons: skip as u16, ..IDLE }; l.mine.len()];
    let (now, msg) = l.advance(&local);
    let Msg::Pads { tick, seats } = msg else { unreachable!() };
    np.out.push(Msg::Intro { tick, seats });
    np.stalled = 0.0;
    let skip = now.iter().any(|p| p.buttons != 0);
    pump(w);
    Some(skip)
}

/// Hands a tick's synced pads (`last`: the tick before's) to the match: as a one-tick `Replay`, or while an instant
/// replay plays its own queue, as the presses its ✕/○ skip reads (every seat's, the same on every machine; this
/// machine's unsent latches are dropped), as rollback's `handle` does.
pub(super) fn run_on(w: &mut World, now: [Pad; 4], last: [Pad; 4]) {
    if !super::instant_replay::playing(w) {
        w.get_resource_or_insert_with(|| Replay(Default::default())).0.push_back(now);
        return;
    }
    let Some(mut pads) = w.get_resource_mut::<Pads>() else { return };
    for s in 0..4 {
        let d = tick_input::decode(now[s], last[s]);
        (pads.slots[s].shot, pads.slots[s].serve) = (d.shot, d.serve);
    }
}

fn setup(mut commands: Commands, np: Option<Res<Netplay>>) {
    let Some(np) = np else { return };
    if np.room.lockstep {
        commands.insert_resource(crate::mode::Gate(gate));
    } else {
        let rb = super::rollback::Rollback::new(&np.room, &np.me, crate::mode::run_fixed).expect("the room's seats");
        commands.insert_resource(rb);
        commands.insert_resource(crate::mode::Gate(super::rollback::gate));
    }
    // the match reads only the pads the gate hands it
    commands.insert_resource(Replay(Default::default()));
    // B200: and no press at all until every peer has loaded it
    commands.insert_resource(crate::mode::Hush);
}

/// Each frame: the network, the pause going out, the stall/end text, and closing the match after the end.
fn watch(w: &mut World) {
    pump(w);
    super::recover::keep(w);
    super::rollback::poll(w);
    // a long frame (loading, a hidden tab) isn't everyone else going silent
    let dt = w.resource::<Time<Real>>().delta_secs().min(0.25);
    let notice = super::recover::tick(w, dt);
    let remote = w.get_resource::<super::rollback::Rollback>().map(|r| r.remote_seats());
    let dt = w.resource::<Time<Real>>().delta_secs();
    let open = front_end::paused(w);
    let result = w.get_resource::<Game>().is_some_and(|g| g.match_stats.result.is_some());
    let mut np = w.resource_mut::<Netplay>();
    if open != np.paused {
        np.paused = open;
        np.out.push(Msg::Pause(open));
    }
    if np.end.is_some() {
        np.ended += dt;
    } else if !open {
        np.stalled += dt;
    }
    let text = match &np.end {
        Some(why) => why.clone(),
        None if !notice.is_empty() => notice,
        None if np.stalled > STALL && !started(&np) => {
            let who: Vec<String> = np.recover.loading(&np.room).iter().map(|p| np.room.names.get(p).cloned().unwrap_or_else(|| p.chars().take(8).collect())).collect();
            format!("Waiting for {} to load...", who.join(", "))
        }
        None if np.stalled > STALL && !result => {
            let who: Vec<String> = np.intro.as_ref().map(Lockstep::missing).or(remote).unwrap_or_else(|| np.lock.missing()).iter().map(|s| format!("{}P", s + 1)).collect();
            format!("Waiting for {}...", who.join(", "))
        }
        None => match &mut np.note {
            Some((say, left)) if *left > 0.0 => {
                *left -= dt;
                say.clone()
            }
            _ => String::new(),
        },
    };
    let close = np.ended > END;
    pump(w);
    // B196a: in the pause dialog's window (`main_menu/pause.rs`), the bot-or-lobby question on its pills
    let ask = w.resource::<Netplay>().recover.ask_me && w.resource::<Netplay>().end.is_none();
    if let Some(mut n) = w.get_resource_mut::<OnlineNotice>() {
        if ask && !n.ask {
            n.sel = 0;
        }
        if n.text != text || n.ask != ask {
            (n.text, n.ask) = (text, ask);
        }
    }
    if close {
        // B196: the connection goes on into the lobby (`main_menu/online.rs` `launch`'s way back)
        if let Some(room) = w.resource_mut::<Netplay>().recover.lobby.take()
            && let Some(net) = w.remove_resource::<Net>()
        {
            *LOBBY.lock().unwrap() = Some((net, room, false));
        }
        w.write_message(AppExit::Success);
    }
}

/// B279: the result screen left (its Continue): back to the same lobby, seats, picks and court as they were. The host
/// takes everyone there (`Msg::Lobby`; its relay goes back to its lobby loop); a guest takes the lobby the host sent,
/// or, ahead of the host, waits in this one for it (`main_menu/online.rs`).
fn again(w: &mut World) {
    let over = w.get_resource::<Game>().is_some_and(|g| g.match_stats.result.is_some());
    if !over || w.resource::<Messages<AppExit>>().is_empty() || w.resource::<Netplay>().end.is_some() || !w.contains_resource::<Net>() {
        return;
    }
    let mut np = w.resource_mut::<Netplay>();
    let wait = np.me != "host" && np.recover.lobby.is_none();
    let room = np.recover.lobby.take().unwrap_or_else(|| {
        let mut r = np.room.clone();
        r.again();
        r
    });
    let net = w.remove_resource::<Net>().expect("checked");
    if w.resource::<Netplay>().me == "host" {
        info!("online: back to the lobby after the match");
        net.send(Cmd::Send(Msg::Lobby(room.clone())));
    }
    *LOBBY.lock().unwrap() = Some((net, room, wait));
}

/// B196a: an online match's notice (a peer away, the bot-or-lobby question, why it ended), drawn in the pause
/// dialog's window: dimmed court, our box, the words in the game's font, and while asking the two choices on pills
/// with the 3D hand. `netplay.rs` fills it each frame; `recover.rs` reads the choice (`sel`, a tap in `tapped`).
#[derive(Resource, Default)]
pub struct OnlineNotice {
    pub text: String,
    /// The question is up: pill 0 "A bot plays for them", 1 "Everyone to the lobby", 2 "Leave".
    pub ask: bool,
    pub sel: usize,
    pub tapped: bool,
}

/// The choices' pills.
pub const CHOICES: [&str; 3] = ["A bot plays for them", "Everyone to the lobby", "Leave"];

pub fn plugin(app: &mut App) {
    app.init_resource::<OnlineNotice>();
    app.add_systems(Startup, setup).add_systems(Update, watch.run_if(resource_exists::<Netplay>)).add_systems(Last, again.run_if(resource_exists::<Netplay>));
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("HST_NET_THROTTLE").is_some() {
        app.add_systems(Last, super::rollback::throttle.run_if(resource_exists::<Netplay>));
    }
}

/// The match's flags (as `main_menu`'s) for `room`, this machine being `me`: its seats driven by `devices` in seat
/// order, the other taken seats by their peers: a human seat on a gamepad no machine has (`REMOTE`; it may pick up a
/// spare local pad, which the gate never reads), so every machine sets the match up with the same humans.
pub fn flags(room: &Room, me: &str, devices: &[Option<super::controls::Seat>], extra: &[String]) -> Vec<String> {
    let order: &[usize] = if room.doubles { &[0, 2, 1, 3] } else { &[0, 1] };
    let pick = |s: usize| room.seats.get(s).cloned().flatten().map(|x| x.pick);
    let join = |v: Vec<usize>| v.iter().map(usize::to_string).collect::<Vec<_>>().join(",");
    let mut mine = devices.iter();
    let seats: Vec<_> = (0..room.seats.len())
        .map(|s| match &room.seats[s] {
            Some(x) if x.peer == me => mine.next().copied().flatten().or(Some(super::controls::Seat::Pad(REMOTE))),
            Some(x) if x.human() => Some(super::controls::Seat::Pad(REMOTE)),
            // open, or the computer's (`net::CPU`, started with open seats)
            _ => None,
        })
        .collect();
    let court = (room.court + 1).to_string();
    let mut a = vec!["--play".to_string(), "--stage".into(), court.clone(), "--court".into(), court];
    if !room.doubles {
        a.push("--singles".into());
    }
    a.extend([
        "--chars".into(),
        join(order.iter().map(|&s| pick(s).map_or(0, |p| p.char)).collect()),
        "--outfits".into(),
        join(order.iter().map(|&s| pick(s).map_or(0, |p| p.costume)).collect()),
        "--umpire".into(),
        room.umpire.to_string(),
        "--sets".into(),
        (([1, 3, 5][room.sets.min(2)] + 1) / 2).to_string(),
        "--games".into(),
        room.games.to_string(),
        "--pads".into(),
        super::controls::Seat::flag(&seats),
    ]);
    // M8f3: the host's umpire mod, heard by a peer that has it (the donor, `--umpire`, times it either way)
    if let Some((id, _)) = room.umpire_mod.as_ref().filter(|_| !room.vanilla) {
        a.extend(["--umpire-mod-id".into(), id.clone()]);
    }
    // M8f8: the host's court mod over the layout court (`--stage`/`--court` above), drawn by a peer that has it
    if let Some(c) = room.court_mod.as_ref().filter(|_| !room.vanilla) {
        a.extend(["--court-mod-id".into(), c.id.clone()]);
    }
    // B370: the select's hand toggles, by match slot
    if order.iter().any(|&s| pick(s).is_some_and(|p| p.swap)) {
        a.extend(["--swap-hands".into(), order.iter().map(|&s| if pick(s).is_some_and(|p| p.swap) { "1" } else { "0" }).collect::<Vec<_>>().join(",")]);
    }
    a.extend(extra.iter().cloned());
    a
}

/// The online match as a mode: `game`, `flags(…)`'s match (`main.rs` `match_mode`), with the lockstep on `net`.
pub fn mode(game: crate::mode::Mode, net: Net, room: Room, me: String) -> crate::mode::Mode {
    SEED.store(room.seed as u64, Ordering::Relaxed);
    game.and(move |app| {
        app.insert_resource(Netplay::new(room, me)).insert_resource(net);
    })
}

/// B196: the lobby an ended match goes back to, with its connection; B279: true for a guest whose host hasn't left
/// the result screen yet (its `Msg::Lobby` to come).
pub static LOBBY: Mutex<Option<(Net, Room, bool)>> = Mutex::new(None);

/// `hst net … --play`: the match `main` starts instead of the menu (it was waiting for the lobby).
pub static PENDING: Mutex<Option<(Net, Room, String)>> = Mutex::new(None);

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(b: u16) -> Pad {
        Pad { buttons: b, ..IDLE }
    }

    /// Two machines, 2 seats each, a link `lag` ticks long: both run the same pads, every tick, in order.
    #[test]
    fn lockstep_over_a_slow_link() {
        let (lag, delay) = (6u32, 7u32);
        let mut m = [Lockstep::new([true; 4], vec![0, 2], delay), Lockstep::new([true; 4], vec![1, 3], delay)];
        let mut wire: Vec<(usize, u32, Msg)> = Vec::new();
        let mut stalls = [0; 2];
        for round in 0..2000u32 {
            for (to, at, msg) in std::mem::take(&mut wire) {
                if at <= round {
                    if let Msg::Pads { tick, seats } = &msg {
                        m[to].heard(*tick, seats);
                    }
                } else {
                    wire.push((to, at, msg));
                }
            }
            for i in 0..2 {
                if !m[i].missing().is_empty() {
                    stalls[i] += 1;
                    continue;
                }
                let t = m[i].tick;
                let local: Vec<Pad> = m[i].mine.iter().map(|&s| pad((t * 4 + s as u32) as u16)).collect();
                let (_, msg) = m[i].advance(&local);
                wire.push((1 - i, round + lag, msg));
            }
        }
        let n = m[0].tick.min(m[1].tick) as usize;
        assert!(n > 1900, "ran {n} of 2000 ticks; stalls {stalls:?}");
        assert_eq!(m[0].log[..n], m[1].log[..n]);
        // seat s's pad for tick t was sent at tick t - delay
        assert_eq!(m[0].log[100][3], pad(((100 - delay) * 4 + 3) as u16));
        assert_eq!(m[0].log[3], [IDLE; 4]);
        // a delay shorter than the link stalls
        let mut a = Lockstep::new([true, true, false, false], vec![0], 2);
        a.advance(&[IDLE]);
        a.advance(&[IDLE]);
        assert_eq!(a.missing(), vec![1]);
    }

    #[test]
    fn hashes_compare_whichever_comes_first() {
        let mut l = Lockstep::new([true; 4], vec![0], 3);
        assert_eq!(l.their_hash(30, 5), None);
        assert_eq!(l.own_hash(30, 5), None);
        assert_eq!(l.own_hash(60, 6), None);
        assert_eq!(l.their_hash(60, 7), Some((60, 6, 7)));
    }

    #[test]
    fn pads_wire_round_trip() {
        let p = Pad { buttons: 0xa0f1, rx: 1, ry: 2, lx: 0x80, ly: 0xff };
        assert_eq!(unwire(wire(p)), p);
    }

    #[test]
    fn flags_put_each_seat_in_match_order() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str, char| Some(Seat { peer: peer.into(), pick: Pick::disc(char, char % 3) });
        let room = Room { doubles: true, court: 2, sets: 1, games: 3, umpire: 1, seats: vec![s("host", 5), s("g", 6), s("g", 7), s("host", 8)], ..Default::default() };
        let a = flags(&room, "g", &[Some(super::super::controls::Seat::Keys), Some(super::super::controls::Seat::Pad(0))], &[]);
        let flag = |f: &str| a[a.iter().position(|x| x == f).unwrap() + 1].clone();
        assert_eq!((flag("--chars"), flag("--outfits")), ("5,7,6,8".into(), "2,1,0,2".into()));
        assert_eq!((flag("--pads"), flag("--court"), flag("--sets"), flag("--games")), ("99,k,0,99".into(), "3".into(), "2".into(), "3".into()));
        assert!(!a.contains(&"--swap-hands".to_string()), "no toggle on: no flag");
        // B370: 2P's (the guest's first) hand toggle, at its match slot
        let mut room = room;
        room.seats[1].as_mut().unwrap().pick.swap = true;
        let a = flags(&room, "host", &[None, None], &[]);
        assert_eq!(a[a.iter().position(|x| x == "--swap-hands").unwrap() + 1], "0,0,1,0");
    }

    /// M8f3: the host's umpire mod goes in the flags by id (each peer looks it up under its own `mods/`, the donor
    /// `--umpire` either way); none in a vanilla lobby.
    #[test]
    fn flags_carry_the_umpire_mod_id() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let mut room = Room { umpire: 2, umpire_mod: Some(("gag_ump00".into(), "Umpire 00".into())), seats: vec![s("host"), s("g")], ..Default::default() };
        let a = flags(&room, "g", &[None], &[]);
        let flag = |a: &[String], f: &str| a.iter().position(|x| x == f).map(|i| a[i + 1].clone());
        assert_eq!((flag(&a, "--umpire"), flag(&a, "--umpire-mod-id")), (Some("2".into()), Some("gag_ump00".into())));
        room.vanilla = true;
        assert_eq!(flag(&flags(&room, "g", &[None], &[]), "--umpire-mod-id"), None);
    }

    /// M8f8: the host's court mod goes in the flags by id over the room's court (its layout, `--stage`/`--court`);
    /// none in a vanilla lobby, where the layout plays as a disc court.
    #[test]
    fn flags_carry_the_court_mod_id() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let mut room = Room { court: 4, court_mod: Some(crate::mods_online::CourtWire { id: "gag_c01_yamate".into(), name: "Yamate".into(), looks: 1, dir: None }), seats: vec![s("host"), s("g")], ..Default::default() };
        let a = flags(&room, "g", &[None], &[]);
        let flag = |a: &[String], f: &str| a.iter().position(|x| x == f).map(|i| a[i + 1].clone());
        assert_eq!((flag(&a, "--stage"), flag(&a, "--court"), flag(&a, "--court-mod-id")), (Some("5".into()), Some("5".into()), Some("gag_c01_yamate".into())));
        room.vanilla = true;
        let a = flags(&room, "g", &[None], &[]);
        assert_eq!((flag(&a, "--stage"), flag(&a, "--court-mod-id")), (Some("5".into()), None));
    }

    /// The court intro online (B42e): the guest's ✕ on its intro tick 10 skips on both machines at tick 10 + delay,
    /// over a link 3 ticks long; a machine waits for the other's ticks.
    #[test]
    fn intro_skip_reaches_both() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let room = Room { seats: vec![s("host"), s("g")], delay: 4, ..Default::default() };
        let mut w = ["host", "g"].map(|me| {
            let mut w = World::new();
            let mut np = Netplay::new(room.clone(), me.into());
            np.start_intro();
            w.insert_resource(np);
            w
        });
        let mut wire: Vec<(usize, u32, Msg)> = Vec::new();
        let mut skipped = [None; 2];
        let mut ticks = [0u32; 2];
        for round in 0..40u32 {
            for (to, at, m) in std::mem::take(&mut wire) {
                if at <= round {
                    take(&mut w[to], Event::Match(m));
                } else {
                    wire.push((to, at, m));
                }
            }
            for i in 0..2 {
                if let Some(skip) = intro_tick(&mut w[i], i == 1 && ticks[i] == 10) {
                    if skip && skipped[i].is_none() {
                        skipped[i] = Some(ticks[i]);
                    }
                    ticks[i] += 1;
                }
                let out = std::mem::take(&mut w[i].resource_mut::<Netplay>().out);
                wire.extend(out.into_iter().map(|m| (1 - i, round + 3, m)));
            }
        }
        assert_eq!(skipped, [Some(14), Some(14)]);
        assert!(ticks[0] > 30 && ticks[0].abs_diff(ticks[1]) <= 4, "{ticks:?}");
    }

    /// B200: the match's ticks (and its court intro's) wait for every peer's loading screen to be down, and no press
    /// reaches anything until then.
    #[test]
    fn the_match_waits_for_every_peer_to_load() {
        use crate::net::{Pick, Seat};
        let s = |peer: &str| Some(Seat { peer: peer.into(), pick: Pick::disc(0, 0) });
        let room = Room { seats: vec![s("host"), s("g")], delay: 2, ..Default::default() };
        let mut w = World::new();
        let mut np = Netplay::new(room, "host".into());
        np.start_intro();
        w.insert_resource(np);
        w.insert_resource(crate::mode::Hush);
        assert_eq!(intro_tick(&mut w, false), None);
        assert!(!gate(&mut w));
        let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
        assert!(matches!(&out[..], [Msg::Loaded(p)] if p == "host"), "{out:?}");
        assert!(w.contains_resource::<crate::mode::Hush>());
        take(&mut w, Event::Match(Msg::Loaded("g".into())));
        assert_eq!(intro_tick(&mut w, false), Some(false));
        assert!(!w.contains_resource::<crate::mode::Hush>());
        assert!(w.resource::<Netplay>().out.iter().all(|m| !matches!(m, Msg::Loaded(_))), "said once");
    }

    /// The match on two headless apps (`snapshot.rs`'s harness, doubles): what each gate sends reaches the other
    /// `lag` rounds (ticks) later.
    mod matches {
        use super::super::super::*;
        use super::super::{Lockstep, Netplay, gate, run_on, take, unwire, wire};
        use crate::net::{Event, Msg, Pick, Room, Seat};
        use crate::{Args, GameSpace};
        use std::path::Path;

        const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

        fn room(delay: u32) -> Room {
            let s = |peer: &str, char| Some(Seat { peer: peer.into(), pick: Pick::disc(char, 0) });
            // 1P and 2P (a team) on the host, 3P and 4P on the guest
            Room { doubles: true, games: 1, sets: 0, umpire: 4, seats: vec![s("host", 6), s("host", 1), s("g", 2), s("g", 3)], delay, seed: 0x28c7c4a1, ..Default::default() }
        }

        thread_local! {
            /// Instant replays in the harness's matches. A full match's mashed pads keep asking for them (△ during one
            /// asks again), so it never ends with them on.
            static REPLAYS: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
        }

        fn app(room: &Room, me: &str) -> App {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default()));
            app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
            app.init_asset::<crate::gs::GsMaterial>();
            let chars = [0, 2, 1, 3].map(|s| room.seats[s].as_ref().unwrap().pick.char).to_vec();
            app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: false, chars, outfits: vec![], viewer: None, sound: None, music: false, umpire: room.umpire, sets: 1, games: room.games, pads: None });
            // the computer plays the seats no person has (B151: started with open seats)
            let human = std::array::from_fn(|i| room.seats[i].as_ref().is_some_and(Seat::human));
            app.insert_resource(Pads { seats: Some(human), autoplay: false, ..default() });
            app.init_resource::<tick_input::TickPads>();
            app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
            app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, foot_fx::plugin, weather::plugin, heard::plugin, change_ends::plugin));
            app.add_systems(PostStartup, setup);
            app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain());
            if REPLAYS.get() {
                app.add_systems(FixedUpdate, instant_replay::run.after(held_ball));
            }
            app.finish();
            app.cleanup();
            app.world_mut().run_schedule(Startup);
            app.world_mut().run_schedule(PostStartup);
            let mut np = Netplay::new(room.clone(), me.into());
            np.mash = Some(if me == "host" { 0x1234_5679 } else { 0x9abc_def1 });
            np.resync.step = step;
            app.insert_resource(np).insert_resource(tick_input::Replay(Default::default()));
            app
        }

        /// Rounds until the match is over or `limit`, `bad` changes the guest's pads at that tick; the two apps.
        fn play(lag: u32, limit: u32, bad: Option<u32>) -> [App; 2] {
            play_room(room(crate::net::delay_for(std::time::Duration::from_millis(2 * lag as u64 * 1000 / 60))), lag, limit, bad)
        }

        fn play_room(room: Room, lag: u32, limit: u32, bad: Option<u32>) -> [App; 2] {
            let mut apps = [app(&room, "host"), app(&room, "g")];
            let mut wire: Vec<(usize, u32, Msg)> = Vec::new();
            // an instant replay's queue as each tick of it runs: it only plays its own pads (M2h1)
            let mut queue = [None; 2];
            for round in 0..limit {
                for (to, at, msg) in std::mem::take(&mut wire) {
                    if at <= round {
                        take(apps[to].world_mut(), Event::Match(msg));
                    } else {
                        wire.push((to, at, msg));
                    }
                }
                for i in 0..2 {
                    let w = apps[i].world_mut();
                    if gate(w) {
                        if i == 1 && bad == Some(w.resource::<Netplay>().lock.tick) {
                            w.resource_mut::<tick_input::Replay>().0.back_mut().unwrap()[2].buttons ^= !0;
                        }
                        let q = instant_replay::playing(w).then(|| (w.resource::<Game>().instant.replays, w.resource::<tick_input::Replay>().0.len()));
                        if let (Some((r, a)), Some((s, b))) = (queue[i], q) {
                            assert!(r != s || b < a, "a network tick's pads went into instant replay {r}'s queue");
                        }
                        queue[i] = q;
                        w.run_schedule(FixedUpdate);
                    }
                    let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                    wire.extend(out.into_iter().map(|m| (1 - i, round + lag, m)));
                }
                let over = |a: &App| a.world().resource::<Game>().match_stats.result.is_some() || a.world().resource::<Netplay>().end.is_some();
                if apps.iter().all(over) && wire.is_empty() {
                    break;
                }
            }
            apps
        }

        /// M2g's acceptance: a doubles match, 2 seats on each machine, 100 ms each way, random pads, to the end
        /// with the same state on both (every 30 ticks' hashes agree: no desync). ~20 min unoptimised:
        /// `cargo test -p hst full_doubles -- --ignored`; `tools/check.sh` runs its first 2400 ticks.
        #[test]
        #[ignore]
        fn full_doubles_match_at_100ms_stays_in_sync() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let _ = in_sync(200_000, true);
        }

        /// One 100 ms doubles match (2400 ticks) for three checks: both machines stay in sync, a state sent from
        /// it hashes the same elsewhere (B154b), and its saved inputs replay to the same state.
        #[test]
        fn doubles_at_100ms_stays_in_sync_sends_and_replays() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let [mut host, mut g] = in_sync(2400, false);
            sent_state_hashes_the_same(&mut host, &mut g);
            saved_inputs_replay_the_same(&mut host);
        }

        /// B151: the host started with two seats open, the computer plays them on both machines alike.
        #[test]
        fn doubles_with_computer_seats_stays_in_sync() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let mut room = room(crate::net::delay_for(std::time::Duration::from_millis(200)));
            room.seats[1] = Some(Seat { peer: crate::net::CPU.into(), pick: Pick::disc(1, 0) });
            room.seats[3] = Some(Seat { peer: crate::net::CPU.into(), pick: Pick::disc(3, 0) });
            let apps = play_room(room, 6, 1500, None);
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            assert_eq!((a.end.as_deref(), b.end.as_deref()), (None, None));
            assert_eq!((a.lock.mine.clone(), b.lock.mine.clone()), (vec![0], vec![2]));
            let n = a.lock.log.len().min(b.lock.log.len());
            assert!(n > 1400, "{n} ticks");
            eprintln!("computer seats in sync over {n} ticks");
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
        }

        fn in_sync(limit: u32, to_the_end: bool) -> [App; 2] {
            REPLAYS.set(!to_the_end);
            let apps = play(6, limit, None);
            for a in &apps {
                let np = a.world().resource::<Netplay>();
                assert_eq!(np.end, None);
                assert!(!to_the_end || a.world().resource::<Game>().match_stats.result.is_some(), "the match didn't finish in {} ticks", np.lock.tick);
            }
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            let n = a.lock.log.len().min(b.lock.log.len());
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
            // a mashed match asks for instant replays all the time: they start and end in lockstep (M2h1)
            let replays = apps[0].world().resource::<Game>().instant.replays;
            assert!(to_the_end || replays > 1, "{replays} instant replays");
            eprintln!("in sync over {n} ticks, {replays} instant replays");
            apps
        }

        /// A tick run on different pads: both see it at the next hash, save the inputs, say so, the guest takes the
        /// host's state and they play on in sync (B154, B154b).
        #[test]
        fn a_desync_is_saved_and_play_goes_on() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            // tick 119's pads are in the state hashed before tick 120
            for mut a in play(6, 400, Some(119)) {
                let path = reported(&a, 120);
                let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("saved")).unwrap();
                assert!(saved["pads"].as_array().unwrap().len() >= 120);
                std::fs::remove_file(&path).unwrap();
                // B276: going out of sync again after the note is down notes it, but the match's report isn't saved again
                let t = a.world().resource::<Netplay>().state.0 + 30;
                a.world_mut().resource_mut::<Netplay>().note = None;
                super::super::desync(a.world_mut(), (t, 1, 2));
                let (say, _) = a.world().resource::<Netplay>().note.clone().expect("the second desync's note");
                assert!(say.ends_with(&format!("Inputs saved to {path}")) && !Path::new(&path).exists(), "{say}");
            }
        }

        /// A desync at tick `t` was noted and saved, resynced once, and the match went on past it with every hash
        /// since agreeing: the report's path.
        fn reported(a: &App, t: u32) -> String {
            let np = a.world().resource::<Netplay>();
            assert_eq!(np.end, None);
            assert!(np.lock.log.len() > t as usize + 60, "played on to {}", np.lock.log.len());
            assert_eq!(np.resync.done, 1, "resyncs");
            assert!(!np.desynced && np.state.0 >= np.resync.fresh + 2 * super::super::HASH_EVERY, "out of sync again by tick {}", np.state.0);
            eprintln!("{}: resynced from tick {}, hashed to {}", np.me, np.resync.fresh, np.state.0);
            let (say, _) = np.note.clone().expect("the desync's note");
            assert!(say.starts_with(&format!("Out of sync at tick {t} ")), "{say}");
            say.rsplit("Inputs saved to ").next().unwrap().to_string()
        }

        /// B154b: a state from `snapshot::encode` taken on by a match that's elsewhere hashes the same, and its
        /// pieces fit a message.
        fn sent_state_hashes_the_same(host: &mut App, g: &mut App) {
            let data = super::super::snapshot::encode(host.world_mut());
            let json = serde_json::to_vec(&Msg::State { tick: 0, data: vec![255; crate::play::resync::PIECE], last: false }).unwrap().len();
            eprintln!("state {} KB, a piece {} KB as JSON", data.len() / 1024, json / 1024);
            // a browser's data channel takes 256 KB
            assert!(json < 256 << 10, "{json} bytes");
            let mut fresh = app(&room(0), "g");
            for b in [g.world_mut(), fresh.world_mut()] {
                super::super::snapshot::decode(b, &data).unwrap();
                assert_eq!(super::super::snapshot::state_text(b), super::super::snapshot::state_text(host.world_mut()));
            }
        }

        /// A lockstep match's inputs run again as `replays_a_desync` does come out the same, instant replays and all.
        fn saved_inputs_replay_the_same(host: &mut App) {
            let room = room(crate::net::delay_for(std::time::Duration::from_millis(200)));
            let log = host.world().resource::<Netplay>().lock.log.clone();
            assert!(host.world().resource::<Game>().instant.replays > 1);
            let mut a = app(&room, "replay");
            a.world_mut().remove_resource::<Netplay>();
            for (t, &p) in log.iter().enumerate() {
                run_on(a.world_mut(), p, t.checked_sub(1).map_or([tick_input::IDLE; 4], |l| log[l]));
                a.world_mut().run_schedule(FixedUpdate);
            }
            assert_eq!(super::super::snapshot::state_hash(a.world_mut()), super::super::snapshot::state_hash(host.world_mut()));
        }

        /// `HST_DESYNC=desync-….json cargo test -p hst replays_a_desync -- --ignored`: runs the saved inputs again,
        /// printing the hash every 30 ticks (the machines' `Hash` ticks).
        #[test]
        #[ignore]
        fn replays_a_desync() {
            let path = std::env::var("HST_DESYNC").expect("HST_DESYNC=file");
            let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            let room: Room = serde_json::from_value(saved["room"].clone()).unwrap();
            let pads: Vec<[[u8; 6]; 4]> = serde_json::from_value(saved["pads"].clone()).unwrap();
            let mut a = app(&room, "replay");
            a.world_mut().remove_resource::<Netplay>();
            for (t, p) in pads.iter().enumerate() {
                if t % 30 == 0 {
                    println!("tick {t}: {:016x}", super::super::snapshot::state_hash(a.world_mut()));
                }
                let last = t.checked_sub(1).map_or([tick_input::IDLE; 4], |l| pads[l].map(unwire));
                run_on(a.world_mut(), p.map(unwire), last);
                a.world_mut().run_schedule(FixedUpdate);
            }
            let _ = Lockstep::new;
        }

        /// C2: one machine's mashed doubles match, every tick's state hash written to `HST_HASH_TRACE` (one hex line
        /// a tick, `HST_HASH_TICKS` ticks, default 30000, or until the match of `HST_HASH_GAMES` games, default 1, ends). Run before and after a change
        /// that must not change the sim and diff the files:
        /// `HST_HASH_TRACE=context/c2/a.txt tools/check.sh -p hst hash_trace -- --ignored` (~1 min optimised).
        #[test]
        #[ignore]
        fn hash_trace() {
            let path = std::env::var("HST_HASH_TRACE").expect("HST_HASH_TRACE=file");
            let ticks: u32 = std::env::var("HST_HASH_TICKS").map_or(30_000, |t| t.parse().unwrap());
            REPLAYS.set(false);
            let mut room = room(0);
            room.games = std::env::var("HST_HASH_GAMES").map_or(1, |g| g.parse().unwrap());
            room.seats.iter_mut().for_each(|s| s.as_mut().unwrap().peer = "host".into());
            let mut a = app(&room, "replay");
            let mut np = a.world_mut().remove_resource::<Netplay>().unwrap();
            let mut out = String::new();
            let mut last = [tick_input::IDLE; 4];
            for t in 0..ticks {
                out += &format!("{:016x}\n", super::super::snapshot::state_hash(a.world_mut()));
                if a.world().resource::<Game>().match_stats.result.is_some() {
                    break;
                }
                np.lock.tick = t;
                let now = [0, 1, 2, 3].map(|s| np.mashed(s));
                run_on(a.world_mut(), now, last);
                a.world_mut().run_schedule(FixedUpdate);
                last = now;
            }
            std::fs::write(path, out).unwrap();
        }

        // ---------------------------------------------------------------- rollback (M2h)

        use super::super::super::rollback::{self, Rollback};

        fn step(w: &mut bevy::prelude::World) {
            w.run_schedule(FixedUpdate);
        }

        /// A tick that comes out differently on one machine: tick 300 reseeds the AI's generator.
        fn nudged(w: &mut bevy::prelude::World) {
            if w.resource::<Rollback>().frame() == 300 {
                w.resource_mut::<Game>().rng.ai = hst_sim::rng::Mt::new(7);
            }
            step(w);
        }

        /// The two apps with rollback sessions; their datagrams reach the other `lag` rounds (ticks) later. `bad`:
        /// the guest's tick 300 comes out differently.
        fn play_rollback(lag: u32, limit: u32, bad: bool) -> [App; 2] {
            play_rollback_with(lag, limit, bad, 0..0, |_, _| {})
        }

        /// `play_rollback`, the guest frozen for the rounds in `hitch` (a hiccup: nothing runs, sent or heard), and
        /// `each(round, apps)` after every round. A round is a tick of time by ggrs' clock (B494), so the outcome
        /// doesn't depend on how long it really takes (a loaded machine).
        fn play_rollback_with(lag: u32, limit: u32, bad: bool, hitch: std::ops::Range<u32>, mut each: impl FnMut(u32, &mut [App; 2])) -> [App; 2] {
            ggrs::clock::drive(true);
            let room = room(0);
            let mut apps = ["host", "g"].map(|me| {
                let mut a = app(&room, me);
                let f = if bad && me == "g" { nudged } else { step };
                a.insert_resource(Rollback::new(&room, me, f as fn(&mut _)).unwrap());
                a
            });
            let mut wire: Vec<(usize, u32, String, Vec<u8>)> = Vec::new();
            for round in 0..limit {
                ggrs::clock::advance(std::time::Duration::from_micros(16_667));
                for (to, at, from, d) in std::mem::take(&mut wire) {
                    if at <= round && !(to == 1 && hitch.contains(&round)) {
                        take(apps[to].world_mut(), Event::Datagram(from, d));
                    } else {
                        wire.push((to, at, from, d));
                    }
                }
                for i in 0..2 {
                    if i == 1 && hitch.contains(&round) {
                        continue;
                    }
                    let w = apps[i].world_mut();
                    rollback::gate(w);
                    let me = w.resource::<Netplay>().me.clone();
                    wire.extend(rollback::outgoing(w).into_iter().map(|(_, d)| (1 - i, round + lag, me.clone(), d)));
                    let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                    for m in out {
                        take(apps[1 - i].world_mut(), Event::Match(m));
                    }
                }
                each(round, &mut apps);
                let over = |a: &App| a.world().resource::<Game>().match_stats.result.is_some() || a.world().resource::<Netplay>().end.is_some();
                if apps.iter().all(over) {
                    break;
                }
            }
            ggrs::clock::drive(false);
            apps
        }

        /// M2h's acceptance: a doubles match (2 seats on each machine, random pads) at 150 ms round trip (5 ticks each
        /// way: 167 ms) with 2 ticks of input delay, to the end: ggrs' checksums (every 30th tick's state hash) agree
        /// on every confirmed tick and both ran the same pads. ~1 h unoptimised:
        /// `cargo test --release -p hst full_doubles_match_rollback -- --ignored`; `tools/check.sh` runs 1200 ticks.
        #[test]
        #[ignore]
        fn full_doubles_match_rollback_at_150ms_stays_in_sync() {
            rollback_in_sync(400_000, true);
        }

        #[test]
        fn rollback_at_150ms_stays_in_sync() {
            rollback_in_sync(1200, false);
        }

        fn rollback_in_sync(limit: u32, to_the_end: bool) {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            const { assert!(rollback::DELAY <= 2) };
            let t0 = std::time::Instant::now();
            REPLAYS.set(!to_the_end);
            let apps = play_rollback(5, limit, false);
            for a in &apps {
                let np = a.world().resource::<Netplay>();
                assert_eq!(np.end, None);
                assert!(!to_the_end || a.world().resource::<Game>().match_stats.result.is_some(), "the match didn't finish in {} ticks", np.lock.log.len());
            }
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            // the last few may still be guesses
            let n = a.lock.log.len().min(b.lock.log.len()) - rollback::WINDOW;
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
            let reruns = apps.each_ref().map(|a| a.world().resource::<Rollback>().reruns);
            // a guess is wrong whenever a peer's pad changes: that's what rollback is for
            assert!(reruns.iter().all(|&r| r > 100), "re-run ticks {reruns:?}");
            let checked = a.state.0.min(b.state.0);
            assert!(checked >= (n as u32).saturating_sub(120), "the checksums stopped at tick {checked} of {n}");
            let replays = apps[0].world().resource::<Game>().instant.replays;
            assert!(to_the_end || replays > 1, "{replays} instant replays");
            eprintln!("in sync over {n} ticks, {reruns:?} ticks re-run, {replays} instant replays, {:.0?}", t0.elapsed());
        }

        /// B162: the guest's frame loop stops (a hidden browser tab, a minimised window) at tick 300 for `away` of real
        /// time while its network still queues the host's datagrams; the host stalls, nobody is dropped, and after
        /// it comes back both run on in sync.
        fn tabbed_out(away: std::time::Duration) {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let room = room(0);
            let mut apps = ["host", "g"].map(|me| {
                let mut a = app(&room, me);
                own_camera::plugin(&mut a);
                a.insert_resource(Rollback::new(&room, me, step as fn(&mut _)).unwrap());
                a
            });
            let mut wire: Vec<(usize, String, Vec<u8>)> = Vec::new();
            let mut gone: Option<std::time::Instant> = None;
            // B299: per round after the guest is back, whether its picture is its own camera's (it looks from the
            // other end, so the shared camera's is a different view)
            let mut own = Vec::new();
            let mut round = 0;
            while round < 900 {
                for (to, from, d) in std::mem::take(&mut wire) {
                    take(apps[to].world_mut(), Event::Datagram(from, d));
                }
                let g = apps[1].world().resource::<Rollback>().frame();
                if g == 300 && gone.is_none() {
                    gone = Some(std::time::Instant::now());
                }
                let hidden = gone.is_some_and(|t| t.elapsed() < away);
                for i in 0..2 {
                    if i == 1 && hidden {
                        continue;
                    }
                    let w = apps[i].world_mut();
                    rollback::gate(w);
                    let me = w.resource::<Netplay>().me.clone();
                    wire.extend(rollback::outgoing(w).into_iter().map(|(_, d)| (1 - i, me.clone(), d)));
                    for m in std::mem::take(&mut w.resource_mut::<Netplay>().out) {
                        take(apps[1 - i].world_mut(), Event::Match(m));
                    }
                }
                // B376: a cut-away or an instant replay has the shared camera (the harness's replays can start on the
                // tick after a cut-away's first, and a replay's first tick cuts): those aren't flips
                let w = apps[1].world();
                if gone.is_some() && !hidden && !w.resource::<Game>().instant.replaying && !w.resource::<cutaway::Director>().showing() {
                    let g = w.resource::<Game>();
                    own.push(own_camera::shown(g, w.get_resource()) != own_camera::shown(g, None));
                }
                if hidden {
                    std::thread::sleep(std::time::Duration::from_millis(16));
                    assert!(apps[0].world().resource::<Rollback>().frame() <= 300 + rollback::WINDOW + rollback::DELAY + 1, "the host ran on without the guest");
                } else {
                    round += 1;
                }
            }
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            assert_eq!((&a.end, &b.end), (&None, &None));
            let n = a.lock.log.len().min(b.lock.log.len()) - rollback::WINDOW;
            assert!(n > 800, "only {n} ticks");
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
            // a cut-away or replay has the shared camera for seconds; a round or two of it is the catch-up's re-runs
            // flipping the picture to the other end
            let runs: Vec<_> = own.chunk_by(|a, b| a == b).filter(|r| !r[0]).map(<[bool]>::len).collect();
            assert!(runs.iter().all(|&r| r > 10), "the guest's picture flipped to the shared camera for {runs:?} rounds");
        }

        /// Longer than ggrs' 2 s default disconnect timeout.
        #[test]
        fn rollback_survives_a_tab_out() {
            tabbed_out(std::time::Duration::from_secs(5));
        }

        /// The task's 2 minutes: `cargo test --release -p hst tab_out_for_2_min -- --ignored`.
        #[test]
        #[ignore]
        fn rollback_survives_a_tab_out_for_2_min() {
            tabbed_out(std::time::Duration::from_secs(120));
        }


        /// B163: after the guest freezes for 20 ticks the host runs ahead; it catches the guest up by sitting out one
        /// tick at a time (never two in a row once it's running again: ggrs' own advice froze 3+ at once), and the
        /// two end up within a tick of each other.
        #[test]
        fn rollback_time_sync_spreads_the_waits() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let frame = |a: &App| a.world().resource::<Rollback>().frame();
            let (mut last, mut idle, mut longest) = (0, 0, 0);
            let apps = play_rollback_with(3, 900, false, 300..320, |round, apps| {
                let f = frame(&apps[0]);
                // the host waits on the guest's pads until a little after it's back
                if round > 340 {
                    idle = if f == last { idle + 1 } else { 0 };
                    longest = longest.max(idle);
                }
                last = f;
            });
            assert_eq!(longest, 1, "the host sat out {longest} ticks in a row");
            let [a, b] = apps.each_ref().map(frame);
            assert!(a.abs_diff(b) <= 2, "host at tick {a}, guest at {b}");
            // B197: time sync moves each clock at most `SHIFT` off real time; this guest's 20 lost ticks (the harness
            // doesn't hand them back as Bevy's fixed loop now does) take both up to it, a tick or two short
            assert!(apps[0].world().resource::<Rollback>().sess.frames_ahead() <= 2);
        }

        /// B228: the host's own emote (1P's flick, a local seat on the host) plays on both machines, in sync. The
        /// host's mashed pads stop for the tick it flicks, so its latch goes out as a real device's would.
        #[test]
        fn the_hosts_emote_plays_on_both() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let (mut flicked, mut seen, mut mash) = (None, [false; 2], None);
            let apps = play_rollback_with(5, 900, false, 0..0, |round, apps| {
                if let Some(m) = mash.take() {
                    apps[0].world_mut().resource_mut::<Netplay>().mash = Some(m);
                }
                for (i, a) in apps.iter().enumerate() {
                    seen[i] |= a.world().resource::<Game>().players[0].emote > 0;
                }
                let w = apps[0].world_mut();
                let g = w.resource::<Game>();
                // from 1P's first serve (the host serves first: before the toss) or a rally after it
                let open = g.phase == Phase::Rally || g.phase == Phase::Serve && g.serving.toss.is_none();
                if !seen[0] && round % 10 == 0 && open {
                    flicked.get_or_insert(round);
                    mash = w.resource_mut::<Netplay>().mash.take();
                    w.resource_mut::<Pads>().slots[0].emote = Some(2);
                }
            });
            assert!(flicked.is_some(), "no serve or rally to flick in");
            assert_eq!(seen, [true, true], "the host's emote played (host, guest), flicked from round {flicked:?}");
            let [a, b] = apps.each_ref().map(|a| a.world().resource::<Netplay>());
            assert_eq!((a.end.clone(), b.end.clone()), (None, None));
            let n = a.lock.log.len().min(b.lock.log.len()) - rollback::WINDOW;
            assert_eq!(a.lock.log[..n], b.lock.log[..n]);
            assert!(!a.desynced && !b.desynced);
        }

        /// A tick that runs differently on one machine: both see it at the next checksum, save their inputs, the
        /// guest takes the host's state, and the hashes from then on agree (B154b).
        #[test]
        fn rollback_desync_is_saved_and_play_goes_on() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            // ggrs compares the confirmed ticks' checksums: none differ after the resync (`reported`)
            let apps = play_rollback(5, 900, true);
            for a in &apps {
                std::fs::remove_file(reported(a, 330)).expect("saved");
            }
        }

        /// B494: the harness's outcome doesn't depend on how long its rounds really take (ggrs' pings, resends and
        /// time sync and `rollback`'s resend go by ggrs' clock, a tick a round): rounds slowed now and then (a loaded
        /// machine) play the forced desync's match the same, tick for tick.
        #[test]
        fn rollback_rounds_play_the_same_however_long_they_take() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let outcome = |slow: bool| {
                let apps = play_rollback_with(5, 420, true, 0..0, |r, _| {
                    if slow && r % 9 == 0 {
                        std::thread::sleep(std::time::Duration::from_millis(10 + (r as u64 * 7) % 40));
                    }
                });
                apps.each_ref().map(|a| {
                    let (np, rb) = (a.world().resource::<Netplay>(), a.world().resource::<Rollback>());
                    if let Some(at) = &np.report {
                        let _ = std::fs::remove_file(at);
                    }
                    (np.lock.log.clone(), (np.state.0, np.resync.fresh, np.resync.done, rb.frame(), rb.reruns, rb.skipped, rb.held, rb.resent))
                })
            };
            let (quick, slow) = (outcome(false), outcome(true));
            for (q, s) in quick.iter().zip(&slow) {
                assert_eq!(q.1, s.1, "state, fresh, resyncs, frame, re-runs, sat out, held, resent");
                let at = q.0.iter().zip(&s.0).position(|(a, b)| a != b);
                assert!(at.is_none() && q.0.len() == s.0.len(), "pads differ from tick {at:?} (ticks {} and {})", q.0.len(), s.0.len());
                assert_eq!(q.1.2, 1, "resyncs");
            }
        }

        /// B154b's acceptance: the forced desync's match, resynced, plays to the end in sync (~25 s optimised):
        /// `cargo test --release -p hst a_resynced_match_finishes -- --ignored`.
        #[test]
        #[ignore]
        fn a_resynced_match_finishes() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            REPLAYS.set(false);
            for a in &play_rollback(5, 400_000, true) {
                assert!(a.world().resource::<Game>().match_stats.result.is_some(), "the match didn't finish");
                std::fs::remove_file(reported(a, 330)).expect("saved");
            }
        }

        /// ggrs' `SyncTestSession`: every tick, back 7 ticks and those again, each tick's state hash compared with its
        /// first run's (it panics on a difference). The determinism check for rollback, in CI.
        #[test]
        fn synctest_reruns_every_tick_the_same() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            // into the first point's rally (the serve is around tick 200), mashing
            synctest(|t, _| t >= 420);
        }

        /// B154: re-runs through the first change of ends stay the same (its camera glide kept state outside the
        /// snapshot, so a rollback mid-glide came out differently and the match stopped on the Change Sides screen).
        /// ~40 s: `tools/check.sh -p hst synctest_through -- --ignored` after touching the snapshot or change of ends.
        #[test]
        #[ignore]
        fn synctest_through_a_change_of_ends() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            REPLAYS.set(false);
            let mut seen = false;
            synctest(|t, g| {
                assert!(t < 30_000, "no change of ends in {t} ticks");
                let now = matches!(g.phase, Phase::ChangeEnds(_));
                let done = seen && !now;
                seen |= now;
                done
            });
        }

        /// ggrs' synctest on the doubles room, all seats on one machine and mashed, until `stop(tick, game)`.
        fn synctest(mut stop: impl FnMut(u32, &Game) -> bool) {
            let mut room = room(0);
            room.seats.iter_mut().for_each(|s| s.as_mut().unwrap().peer = "host".into());
            let mut a = app(&room, "host");
            a.insert_resource(Rollback::new(&room, "host", step as fn(&mut _)).unwrap());
            let mut sess = ggrs::SessionBuilder::<rollback::Cfg>::new().with_num_players(4).unwrap().with_check_distance(7).with_input_delay(rollback::DELAY).start_synctest_session().unwrap();
            for t in 0.. {
                let w = a.world_mut();
                if stop(t, w.resource::<Game>()) {
                    break;
                }
                let mut np = w.resource_mut::<Netplay>();
                np.lock.tick = t;
                for s in 0..4 {
                    let p = np.mashed(s);
                    sess.add_local_input(s, rollback::Input(wire(p))).unwrap();
                }
                let reqs = sess.advance_frame().expect("the re-run ticks' hashes match the first run's");
                rollback::handle(w, reqs, 1);
            }
            assert!(a.world().resource::<Rollback>().reruns > 2000);
        }

        // ---------------------------------------------------------------- a peer gone silent (B196)

        use super::super::super::recover;

        /// Three peers (host 1P, `h` 2P and 4P, `g` 3P; `two`: host 1P 2P, `g` 3P 4P), rollback, every message 3 rounds (ticks) on its way; a peer
        /// not in `live` hears and does nothing (killed). `each(round, apps, live)` after every round may kill one or
        /// add one (a rejoiner).
        fn peers(two: bool, limit: u32, mut each: impl FnMut(u32, &mut Vec<App>, &mut Vec<bool>)) -> Vec<App> {
            let mut room = room(0);
            let names: &[&str] = if two { &["host", "g"] } else { &["host", "h", "g"] };
            if !two {
                room.seats[1].as_mut().unwrap().peer = "h".into();
                room.seats[3].as_mut().unwrap().peer = "h".into();
            }
            let mut apps: Vec<App> = names
                .iter()
                .map(|&me| {
                    let mut a = app(&room, me);
                    a.insert_resource(Rollback::new(&room, me, step as fn(&mut _)).unwrap());
                    a
                })
                .collect();
            let mut live = vec![true; names.len()];
            let mut wire: Vec<(String, u32, Event)> = Vec::new();
            let me = |a: &App| a.world().resource::<Netplay>().me.clone();
            for round in 0..limit {
                for (to, at, e) in std::mem::take(&mut wire) {
                    if at > round {
                        wire.push((to, at, e));
                    } else if let Some(i) = (0..apps.len()).find(|&i| live[i] && me(&apps[i]) == to) {
                        take(apps[i].world_mut(), e);
                    }
                }
                for i in (0..apps.len()).filter(|&i| live[i]) {
                    let w = apps[i].world_mut();
                    rollback::gate(w);
                    recover::tick(w, 1.0 / 60.0);
                    let from = w.resource::<Netplay>().me.clone();
                    wire.extend(rollback::outgoing(w).into_iter().map(|(to, d)| (to, round + 3, Event::Datagram(from.clone(), d))));
                    let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                    let others: Vec<String> = apps.iter().map(me).filter(|p| *p != from).collect();
                    for m in out {
                        wire.extend(others.iter().map(|p| (p.clone(), round + 3, Event::Match(m.clone()))));
                    }
                }
                each(round, &mut apps, &mut live);
            }
            apps
        }

        /// The notice the next frame would show (a frame of no time).
        fn notice(a: &mut App) -> String {
            recover::tick(a.world_mut(), 0.0)
        }

        fn press(a: &mut App, k: KeyCode) {
            let mut keys = ButtonInput::<KeyCode>::default();
            keys.press(k);
            a.insert_resource(keys);
        }

        /// The peers in `apps` ran the same pads from the restart on, ggrs' checksums agreeing, `ran` ticks or more.
        fn in_step(apps: &[&App], ran: usize) -> usize {
            let base = apps[0].world().resource::<Rollback>().base;
            let logs: Vec<_> = apps.iter().map(|a| a.world().resource::<Netplay>()).collect();
            let n = logs.iter().map(|np| np.lock.log.len()).min().unwrap() - rollback::WINDOW;
            assert!(n >= base + ran, "ran {} ticks after the restart at {base}", n - base);
            for (a, np) in apps.iter().zip(&logs) {
                assert_eq!(a.world().resource::<Rollback>().base, base);
                assert_eq!((np.end.as_deref(), np.desynced), (None, false), "{}", np.me);
                assert!(np.state.0 as usize >= n - 120, "{}: checksums stopped at {}", np.me, np.state.0);
                assert_eq!(np.lock.log[base..n], logs[0].lock.log[base..n], "{} ran other pads", np.me);
            }
            base
        }

        /// B197: rollback peers (`two`: host 1P 2P, `g` 3P 4P; else one seat each: host, `h`, `g`, `k`), every datagram 3
        /// rounds on its way, a round being a 60th of a second of real time; peer `i` runs the tick gate `calls(i, round)`
        /// times in it (Bevy's fixed loop: a 20 fps peer 3 times every third round, one hitching none for a while and
        /// then all it missed at once). Each peer's ticks run (`Rollback::frame`) after every round.
        fn paced(two: bool, limit: u32, calls: impl Fn(usize, u32) -> u32) -> (Vec<App>, Vec<Vec<usize>>) {
            let mut room = room(0);
            let names: &[&str] = if two { &["host", "g"] } else { &["host", "h", "g", "k"] };
            if !two {
                for (s, p) in [(1, "h"), (2, "g"), (3, "k")] {
                    room.seats[s].as_mut().unwrap().peer = p.into();
                }
            }
            let mut apps: Vec<App> = names.iter().map(|&me| {
                let mut a = app(&room, me);
                a.insert_resource(Rollback::new(&room, me, step as fn(&mut _)).unwrap());
                a
            }).collect();
            let mut ran = vec![Vec::new(); apps.len()];
            let mut wire: Vec<(usize, u32, String, Vec<u8>)> = Vec::new();
            for round in 0..limit {
                for (to, at, from, d) in std::mem::take(&mut wire) {
                    if at <= round {
                        take(apps[to].world_mut(), Event::Datagram(from, d));
                    } else {
                        wire.push((to, at, from, d));
                    }
                }
                for i in 0..apps.len() {
                    for _ in 0..calls(i, round) {
                        let w = apps[i].world_mut();
                        // a frozen peer's datagrams wait for it in the network's queue (`net.rs`), as they would
                        rollback::gate(w);
                        let me = w.resource::<Netplay>().me.clone();
                        for (to, d) in rollback::outgoing(w) {
                            let j = names.iter().position(|n| **n == to).unwrap();
                            wire.push((j, round + 3, me.clone(), d));
                        }
                        let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                        for j in (0..names.len()).filter(|&j| j != i) {
                            for m in &out {
                                take(apps[j].world_mut(), Event::Match(m.clone()));
                            }
                        }
                    }
                    ran[i].push(apps[i].world().resource::<Rollback>().frame());
                }
            }
            (apps, ran)
        }

        /// B197: the slow peer's schedule: 20 fps (3 ticks every third 60th of a second) and, every 3 s, a 1 s hitch
        /// (nothing for 60 rounds, then 60 gate calls in one frame).
        fn slow(round: u32) -> u32 {
            match round % 180 {
                120..179 => 0,
                179 => 60,
                r => (r % 3 == 2) as u32 * 3,
            }
        }

        /// B197's acceptance: one slow peer (20 fps with 1 s hitches) leaves the others at full speed (60 ticks a
        /// second of real time, never two rounds without a tick, at most `SHIFT` behind real time) and every peer in
        /// sync; the slow one keeps up in real time too.
        fn a_slow_peer(two: bool) {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let limit = 900;
            let (apps, ran) = paced(two, limit, |i, r| if i == 1 { slow(r) } else { 1 });
            for (i, f) in ran.iter().enumerate() {
                // its session's start (ggrs' handshake) and first tick
                let start = f.iter().position(|&f| f > 0).unwrap();
                let behind = (limit as usize - start) as i32 - f[limit as usize - 1] as i32;
                let rb = apps[i].world().resource::<Rollback>();
                eprintln!("peer {i}: {behind} ticks behind real time, {} held, {} sat out, {} re-run, ahead {}", rb.held, rb.skipped, rb.reruns, rb.sess.frames_ahead());
                assert!(behind.abs() <= rollback::SHIFT + 3 + 60 * (i == 1) as i32, "peer {i} {behind} ticks behind real time");
                if i == 1 {
                    continue;
                }
                let idle = f[start + 30..].windows(3).filter(|w| w[0] == w[2]).count();
                assert_eq!(idle, 0, "peer {i} went two rounds without a tick");
            }
            let logs: Vec<_> = apps.iter().map(|a| a.world().resource::<Netplay>()).collect();
            let n = logs.iter().map(|np| np.lock.log.len()).min().unwrap() - rollback::WINDOW;
            assert!(n > 600, "only {n} ticks in step");
            for np in &logs {
                assert_eq!((np.end.as_deref(), np.desynced), (None, false), "{}", np.me);
                assert_eq!(np.lock.log[..n], logs[0].lock.log[..n], "{} ran other pads", np.me);
            }
        }

        #[test]
        fn a_slow_peer_leaves_the_other_at_full_speed() {
            a_slow_peer(true);
        }

        #[test]
        fn a_slow_peer_leaves_three_others_at_full_speed() {
            a_slow_peer(false);
        }

        /// B196: `g` killed mid-rally: everyone waits, nobody answers, the others are asked, the host chooses a bot
        /// (✕), which plays 3P from the same tick on both machines left, in lockstep.
        #[test]
        fn a_bot_takes_a_silent_peers_seat() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let (kill, s) = (600, |x: f32| (x * 60.0) as u32);
            let (mut said, mut asked) = (Vec::new(), Vec::new());
            let apps = peers(false, kill + s(recover::SILENT + recover::WAIT) + 1200, |round, apps, live| {
                if round == kill {
                    live[2] = false;
                }
                if round == kill + s(recover::SILENT) + 30 || round == kill + s(recover::SILENT + recover::WAIT) + 30 {
                    said.push([0, 1].map(|i| notice(&mut apps[i])));
                    asked.push([0, 1].map(|i| apps[i].world().resource::<Netplay>().recover.ask_me));
                }
                if round == kill + s(recover::SILENT + recover::WAIT) + 60 {
                    press(&mut apps[0], KeyCode::Enter);
                }
            });
            eprintln!("{said:#?}");
            assert!(said[0].iter().all(|t| t.starts_with("Waiting for g to come back... 2")), "{:?}", said[0]);
            assert!(said[1].iter().all(|t| t.starts_with("g didn't come back.")), "{:?}", said[1]);
            // B196a: the waiting has no choice; then both machines left get the pills
            assert_eq!(asked, [[false; 2], [true; 2]]);
            let base = in_step(&[&apps[0], &apps[1]], 600);
            assert!(base > kill as usize - 30 && base <= kill as usize + rollback::WINDOW, "restarted at {base}");
            for a in &apps[..2] {
                assert_eq!(a.world().resource::<Pads>().seats, Some([true, true, false, true]));
                // B196: no desync after the restart (the instant replay's old copy brought the old seats back)
                assert_eq!(a.world().resource::<Netplay>().resync.done, 0, "resynced after the restart");
                assert_eq!(a.world().resource::<Netplay>().room.seats[2].as_ref().unwrap().peer, crate::net::CPU);
            }
        }

        /// B196, two peers: `g` killed, the host alone chooses a bot, which plays 3P and 4P on.
        #[test]
        fn a_bot_takes_the_only_guests_seats() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let (kill, s) = (600, |x: f32| (x * 60.0) as u32);
            let apps = peers(true, kill + s(recover::SILENT + recover::WAIT) + 800, |round, apps, live| {
                if round == kill {
                    live[1] = false;
                }
                if round == kill + s(recover::SILENT + recover::WAIT) + 60 {
                    press(&mut apps[0], KeyCode::Enter);
                }
            });
            in_step(&[&apps[0]], 600);
            assert_eq!(apps[0].world().resource::<Pads>().seats, Some([true, true, false, false]));
        }

        /// B162: peer `who` (of `host`, `h`, `g`) switches away at tick 600 for `away` seconds: its frames stop and its
        /// keeper says `Hidden` once a second; everyone else holds the match with "… switched away" (no bot question,
        /// nobody ends it), and back, it plays on in step with them. Returns the apps and the notices 20 s in.
        fn switched_away(who: usize, away: f32) -> (Vec<App>, Vec<String>) {
            let (gone, back, s) = (600, 600 + (away * 60.0) as u32, |x: f32| (x * 60.0) as u32);
            let peer = ["host", "h", "g"][who];
            let mut said = Vec::new();
            let apps = peers(false, back + 900, |round, apps, live| {
                if round == gone {
                    live[who] = false;
                }
                if round == back {
                    live[who] = true;
                }
                if (gone + 30..back).contains(&round) && (round - gone) % 60 == 30 {
                    for i in (0..3).filter(|&i| i != who) {
                        take(apps[i].world_mut(), Event::Match(Msg::Hidden(peer.into())));
                    }
                }
                if round == gone + s(20.0) {
                    said = (0..3).filter(|&i| i != who).map(|i| notice(&mut apps[i])).collect();
                    for a in apps.iter() {
                        assert!(!a.world().resource::<Netplay>().recover.ask_me);
                    }
                }
            });
            for a in &apps {
                let np = a.world().resource::<Netplay>();
                assert_eq!(np.end, None, "{}", np.me);
                assert!(np.recover.hidden.is_empty() && np.recover.away.is_empty(), "{}", np.me);
            }
            in_step(&apps.iter().collect::<Vec<_>>(), 600);
            (apps, said)
        }

        /// B162: a guest tabbed out for 30 s (2 min: `a_player_switched_away_for_2_min_comes_back`).
        #[test]
        fn a_player_switched_away_comes_back() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let (mut apps, said) = switched_away(2, 30.0);
            assert!(said.iter().all(|t| t.starts_with("g switched away from the game.\nWaiting for them to come back... 4:4")), "{said:?}");
            assert_eq!(notice(&mut apps[0]), "");
        }

        /// B162: the host tabbed out for 30 s: the guests hold too (a silent host ends it after 13 s).
        #[test]
        fn the_host_switched_away_comes_back() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let (_, said) = switched_away(0, 30.0);
            assert!(said.iter().all(|t| t.starts_with("host switched away from the game.")), "{said:?}");
        }

        /// B162, the task's 2 minutes (~1 min; `tools/check.sh -p hst-play a_player_switched_away_for_2_min -- --ignored`
        /// after touching `recover.rs`' waits).
        #[test]
        #[ignore]
        fn a_player_switched_away_for_2_min_comes_back() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            switched_away(2, 120.0);
        }

        /// B162: a player switched away for [`recover::LONG`] (forced: its wait starts 2 s short of it) is away: the
        /// others wait [`recover::WAIT`] for it as for a silent one.
        #[test]
        fn a_player_switched_away_too_long_is_away() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let s = |x: f32| (x * 60.0) as u32;
            let mut said = Vec::new();
            peers(false, 610 + s(5.0), |round, apps, live| {
                if round == 600 {
                    live[2] = false;
                }
                // past its last keep-alive's way here
                if round == 610 {
                    for a in &mut apps[..2] {
                        take(a.world_mut(), Event::Match(Msg::Hidden("g".into())));
                        *a.world_mut().resource_mut::<Netplay>().recover.hidden.get_mut("g").unwrap() = recover::LONG - 2.0;
                    }
                }
                if round > 600 && round % 60 == 0 {
                    for a in &mut apps[..2] {
                        take(a.world_mut(), Event::Match(Msg::Hidden("g".into())));
                    }
                }
                if round == 610 + s(1.0) || round == 610 + s(4.0) {
                    said.push(notice(&mut apps[1]));
                }
            });
            assert!(said[0].starts_with("g switched away from the game.\nWaiting for them to come back... 0:0"), "{said:?}");
            assert!(said[1].starts_with("Waiting for g to come back... "), "{said:?}");
        }

        /// B196: nobody wants a bot (○ on `h`): both machines left go back to the lobby together.
        #[test]
        fn a_silent_peer_can_end_the_match() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let (kill, s) = (600, |x: f32| (x * 60.0) as u32);
            let apps = peers(false, kill + s(recover::SILENT + recover::WAIT) + 200, |round, apps, live| {
                if round == kill {
                    live[2] = false;
                }
                if round == kill + s(recover::SILENT + recover::WAIT) + 60 {
                    press(&mut apps[1], KeyCode::Backspace);
                }
            });
            // everyone still in goes back to the lobby together, g's seat open again
            for a in &apps[..2] {
                let np = a.world().resource::<Netplay>();
                let end = np.end.clone().unwrap_or_default();
                assert!(end.starts_with("g didn't come back. Back to the lobby..."), "{end}");
                let lobby = np.recover.lobby.as_ref().expect("back to the lobby");
                let peers: Vec<_> = lobby.seats.iter().map(|s| s.as_ref().map(|s| s.peer.as_str())).collect();
                assert_eq!(peers, [Some("host"), Some("h"), None, Some("h")]);
                assert!(lobby.ready.is_empty() && !lobby.names.contains_key("g"));
            }
        }

        /// B279: the match over, nobody waits for anybody on the result screen (the host gone quiet there is it in the
        /// lobby); leaving it, the host takes everyone to the same lobby, nobody ready, and a guest goes to the lobby
        /// the host sent, or ahead of it to this one, waiting.
        #[test]
        fn the_result_screen_leads_back_to_the_lobby() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let over = 120;
            let mut apps = peers(true, over + ((recover::SILENT + 12.0) * 60.0) as u32, |round, apps, live| {
                if round == over {
                    for a in apps.iter_mut() {
                        let mut g = a.world_mut().resource_mut::<Game>();
                        g.match_stats.result = Some(g.match_stats.finish(0));
                    }
                    live[0] = false;
                }
            });
            let g = &mut apps[1];
            assert_eq!((notice(g), g.world().resource::<Netplay>().end.clone()), (String::new(), None));
            let mut room = g.world().resource::<Netplay>().room.clone();
            room.ready.insert("g".into());
            g.world_mut().resource_mut::<Netplay>().room = room.clone();
            let leave = |a: &mut App| {
                let (net, rx) = crate::net::Net::wired();
                a.insert_resource(net);
                a.world_mut().write_message(AppExit::Success);
                super::super::again(a.world_mut());
                assert!(!a.world().contains_resource::<crate::net::Net>());
                (super::super::LOBBY.lock().unwrap().take().map(|(_, r, wait)| (r, wait)).expect("a lobby"), rx)
            };
            // the guest ahead of the host: this lobby, nobody ready, waiting
            let ((r, wait), _) = leave(g);
            assert!(wait && r.ready.is_empty() && r.seats == room.seats && r.court == room.court);
            // the host: everyone told
            let ((r, wait), mut rx) = leave(&mut apps[0]);
            assert!(!wait && r.ready.is_empty() && r.seats == room.seats);
            let Ok(crate::net::Cmd::Send(Msg::Lobby(sent))) = rx.try_recv() else { panic!("no Lobby sent") };
            assert_eq!(sent.ver, r.ver);
            // the guest still on the results when the host's lobby comes: that one once it leaves too
            let g = &mut apps[1];
            take(g.world_mut(), Event::Match(Msg::Lobby(sent.clone())));
            assert_eq!(g.world().resource::<Netplay>().end, None);
            let ((r, wait), _) = leave(g);
            assert!(!wait && r.ver == sent.ver);
        }

        /// B196: `g` killed, then a new process for it joins (the relay gives it `g`'s seat as `g2`): the host restarts
        /// the match with it in, and all three play on from the same tick in lockstep. And the host killed: the
        /// guests end the match. ~25 s: `tools/check.sh -p hst a_silent_peer_rejoins -- --ignored` after touching
        /// `recover.rs` or the rejoin flow.
        #[test]
        #[ignore]
        fn a_silent_peer_rejoins_and_a_silent_host_ends_it() {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let (kill, back) = (600, 900);
            let mut room = None;
            let mut base = 0;
            let apps = peers(false, back + 1300 + 1200, |round, apps, live| {
                if round == kill {
                    live[2] = false;
                    room = Some(apps[0].world().resource::<Netplay>().room.clone());
                }
                if round == back {
                    let mut r = room.clone().unwrap();
                    r.rejoined("g", "g2", "g");
                    r.rejoin = true;
                    let mut a = app(&r, "g2");
                    a.insert_resource(Rollback::new(&r, "g2", step as fn(&mut _)).unwrap());
                    apps.push(a);
                    live.push(true);
                    take(apps[0].world_mut(), Event::Match(Msg::Rejoined { old: "g".into(), new: "g2".into(), name: "g".into() }));
                }
                if round == back + 1300 {
                    base = in_step(&[&apps[0], &apps[1], &apps[3]], 600);
                    assert!(base > kill as usize - 30 && base <= kill as usize + rollback::WINDOW, "restarted at {base}");
                    live[0] = false;
                }
            });
            for a in [&apps[1], &apps[3]] {
                let end = a.world().resource::<Netplay>().end.clone().unwrap_or_default();
                assert!(end.starts_with("The host stopped answering. The match is over.\nScore"), "{end}");
            }
        }

        /// B198: rollback peers (as `paced`, one gate call a round) over a bad network: each datagram lost with
        /// probability `loss`, the rest `lag` rounds on their way (the match's messages too, never lost: a stream),
        /// one bigger than [`crate::net::DATAGRAM_MAX`] lost always (as iroh's), and nothing at all through the
        /// rounds in `outage`. A round in which no peer runs a tick takes a 60th of a second of real time (ggrs
        /// resends on its clock). Each peer's ticks run after every round, and the biggest datagram sent.
        fn lossy(two: bool, loss: f64, lag: u32, down: u32, oneway: bool) -> (Vec<App>, Vec<Vec<usize>>, usize, std::ops::Range<u32>) {
            let mut room = room(0);
            let names: &[&str] = if two { &["host", "g"] } else { &["host", "h", "g", "k"] };
            if !two {
                for (s, p) in [(1, "h"), (2, "g"), (3, "k")] {
                    room.seats[s].as_mut().unwrap().peer = p.into();
                }
            }
            let mut apps: Vec<App> = names.iter().map(|&me| {
                let mut a = app(&room, me);
                a.insert_resource(Rollback::new(&room, me, step as fn(&mut _)).unwrap());
                a
            }).collect();
            let mut ran = vec![Vec::new(); apps.len()];
            let (mut wire, mut big) = (Vec::<(usize, u32, Event)>::new(), 0);
            let mut rng = 0x2545_f491_4f6c_dd1du64;
            let mut lost = move || {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                (rng >> 11) as f64 / (1u64 << 53) as f64 <= loss
            };
            let mut outage = u32::MAX..u32::MAX;
            for round in 0.. {
                // the outage 2 s after every session is running (ggrs' handshake, over the lossy network, first)
                if outage.start == u32::MAX && apps.iter().all(|a| a.world().resource::<Rollback>().sess.current_state() == ggrs::SessionState::Running) {
                    outage = round + 120..round + 120 + down;
                }
                if round == outage.end.saturating_add(300) {
                    break;
                }
                let t0 = std::time::Instant::now();
                for (to, at, e) in std::mem::take(&mut wire) {
                    if at <= round {
                        take(apps[to].world_mut(), e);
                    } else {
                        wire.push((to, at, e));
                    }
                }
                let before: Vec<usize> = apps.iter().map(|a| a.world().resource::<Rollback>().frame()).collect();
                for i in 0..apps.len() {
                    let w = apps[i].world_mut();
                    rollback::gate(w);
                    let me = w.resource::<Netplay>().me.clone();
                    for (to, d) in rollback::outgoing(w) {
                        big = big.max(d.len());
                        if (outage.contains(&round) && (!oneway || i == 0)) || lost() || !crate::net::fits(&d) {
                            continue;
                        }
                        let j = names.iter().position(|n| **n == to).unwrap();
                        wire.push((j, round + lag, Event::Datagram(me.clone(), d)));
                    }
                    let out = std::mem::take(&mut w.resource_mut::<Netplay>().out);
                    for j in (0..names.len()).filter(|&j| j != i) {
                        wire.extend(out.iter().map(|m| (j, round + lag, Event::Match(m.clone()))));
                    }
                    ran[i].push(apps[i].world().resource::<Rollback>().frame());
                }
                if apps.iter().zip(&before).all(|(a, &b)| a.world().resource::<Rollback>().frame() == b) {
                    std::thread::sleep(std::time::Duration::from_micros(16_667).saturating_sub(t0.elapsed()));
                }
            }
            (apps, ran, big, outage)
        }

        /// B198's acceptance: 2 or 4 peers, 20 % of datagrams lost and 150 ms round trip, then for 3 s nothing gets
        /// through (`oneway`: nothing from the host): every peer's ticks go on within a second of the network
        /// coming back (no mutual wait), and between, nobody waits a second; all in sync, no datagram too big.
        fn on_a_bad_network(two: bool, oneway: bool) {
            if !Path::new(ISO).is_file() {
                return eprintln!("no ISO, skipped");
            }
            let (apps, ran, big, outage) = lossy(two, 0.2, 5, 180, oneway);
            for (i, f) in ran.iter().enumerate() {
                // the longest run of rounds without a tick once every session is running, outside the outage and
                // its first second after
                let (mut still, mut worst) = (0, 0);
                for r in outage.start as usize - 120..f.len() {
                    still = if f[r] == f[r - 1] { still + 1 } else { 0 };
                    if !(outage.start..outage.end + 60).contains(&(r as u32)) {
                        worst = worst.max(still);
                    }
                }
                let rb = apps[i].world().resource::<Rollback>();
                eprintln!("peer {i}: {} ticks, longest wait {worst} rounds, {} held, {} re-run, biggest datagram {big} bytes", f[f.len() - 1], rb.held, rb.reruns);
                assert!(worst < 60, "peer {i} waited {worst} rounds");
            }
            let logs: Vec<_> = apps.iter().map(|a| a.world().resource::<Netplay>()).collect();
            let n = logs.iter().map(|np| np.lock.log.len()).min().unwrap() - rollback::WINDOW;
            assert!(n > 400, "only {n} ticks in step");
            for np in &logs {
                assert_eq!((np.end.as_deref(), np.desynced), (None, false), "{}", np.me);
                assert_eq!(np.lock.log[..n], logs[0].lock.log[..n], "{} ran other pads", np.me);
            }
        }

        #[test]
        fn two_peers_on_a_bad_network() {
            on_a_bad_network(true, false);
        }

        #[test]
        fn four_peers_on_a_bad_network() {
            on_a_bad_network(false, false);
        }

        /// Only the host's datagrams lost for 3 s: its unacked inputs pile up past ggrs' 128.
        #[test]
        fn a_one_way_outage() {
            on_a_bad_network(true, true);
        }

        /// What a tick costs (the match tick, the snapshot ggrs keeps, a state hash): sets `rollback::WINDOW`.
        /// `cargo test --release -p hst rollback_tick_cost -- --ignored --nocapture`
        #[test]
        #[ignore]
        fn rollback_tick_cost() {
            let room = room(0);
            let mut a = app(&room, "host");
            let (mut tick, mut save, mut hash) = (Vec::new(), Vec::new(), Vec::new());
            for t in 0..3000u32 {
                let w = a.world_mut();
                let mut np = w.resource_mut::<Netplay>();
                np.lock.tick = t;
                let p = [0, 1, 2, 3].map(|s| np.mashed(s));
                w.insert_resource(tick_input::Replay([p].into()));
                let c = std::time::Instant::now();
                w.run_schedule(FixedUpdate);
                tick.push(c.elapsed());
                let c = std::time::Instant::now();
                let s = super::super::super::snapshot::save(w);
                save.push(c.elapsed());
                let c = std::time::Instant::now();
                super::super::super::snapshot::state_hash(w);
                hash.push(c.elapsed());
                drop(s);
            }
            for (what, mut v) in [("tick", tick), ("save", save), ("hash", hash)] {
                v.sort();
                let mean = v.iter().sum::<std::time::Duration>() / v.len() as u32;
                println!("{what}: mean {mean:?}, median {:?}, 99% {:?}, max {:?}", v[v.len() / 2], v[v.len() * 99 / 100], v[v.len() - 1]);
            }
        }
    }
}
