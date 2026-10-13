//! Rollback netplay (M2h) with `ggrs`: each machine runs the next tick at once on its own seats' pads (sent
//! [`DELAY`] ticks ahead) and a guess for the others' (their last pads held); when a peer's real pads for a guessed
//! tick arrive and differ, the match goes back to that tick (`snapshot::restore`) and runs the ticks since again on
//! the real ones, all before the next frame is drawn. M2d keeps the re-run ticks' sounds from playing twice.
//!
//! ggrs' messages go as bincode in iroh datagrams (`net.rs` `Cmd::Datagram`, through the host like the streams), its
//! state is `snapshot::save`, and every [`super::netplay::HASH_EVERY`]th tick's `state_hash` is its desync checksum,
//! compared on confirmed ticks (a mismatch stops the match and saves the inputs, as lockstep does). A peer more than
//! [`WINDOW`] ticks behind stalls the others ("Waiting for …"). The streams still carry pause and the end of the
//! match (`netplay.rs`); the host's `HST_NET_LOCKSTEP` picks M2g's lockstep instead.

use super::netplay::{Netplay, unwire, wire};
use super::tick_input::{self, IDLE, Replay};
use super::{Pads, snapshot};
use crate::net::Room;
use bevy::prelude::*;
use ggrs::{Config, DesyncDetection, GgrsError, GgrsEvent, GgrsRequest, Message, NonBlockingSocket, P2PSession, PlayerType, SessionBuilder, SessionState};
use std::sync::{Arc, Mutex};

/// The local input delay, ticks.
pub const DELAY: usize = 2;
/// How many ticks a machine may run on guessed pads before it waits (ggrs' prediction window). B197: 80 (1.3 s), so
/// the others play on through a peer's 1 s hitch instead of stalling (ggrs' input queue holds 128). A re-run of the
/// whole window is `WINDOW + 1` ticks and as many saves in one frame (~45 ms here, `tick_cost`, journal
/// 2026-10-12-m2h), but a hitching peer's pads mostly come back as guessed (held), so it's rarely re-run at all.
pub const WINDOW: usize = 80;
/// Time sync: while this machine runs ahead of (behind) the others' middle, its clock goes back (forward) a tick in
/// every `SPREAD` (B163: never several in a row; B197: gently, 3 ticks a second, so a peer's hitch doesn't show as
/// the others slowing down and speeding up again: every machine keeps real time, both sides of a gap close it).
pub const SPREAD: u32 = 20;
/// B197: how far time sync may set this machine's clock off real time, ticks. Each machine's match runs at 60 ticks a
/// real second; time sync only evens out their start and time a frozen peer lost, so a peer that can't keep up
/// slows the others by 5% at most, and not past this (then they wait on it at `WINDOW`).
pub const SHIFT: i32 = WINDOW as i32 / 2;
/// B197: the most ticks a machine catches up after a stall (waiting on a peer's pads); a longer wait is dropped.
pub const OWE: i32 = WINDOW as i32;
/// B197: a frame this long still runs all its ticks (Bevy's fixed loop drops time past `Time<Virtual>`'s max delta,
/// 250 ms by default): a peer's 1 s hitch is caught up at once.
pub const CATCH_UP: std::time::Duration = std::time::Duration::from_secs(2);

pub struct Cfg;

/// A seat's pad on the wire (`netplay::wire`); idle by default (what ggrs assumes before the first real one).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Input(pub [u8; 6]);

impl Default for Input {
    fn default() -> Self {
        Input(wire(IDLE))
    }
}

impl Config for Cfg {
    type Input = Input;
    // a held button stays held: no press is guessed
    type InputPredictor = ggrs::PredictRepeatLast;
    /// The match, and an instant replay's pads still to play (only while one plays: else each tick brings its own).
    type State = Arc<(snapshot::Snapshot, Option<Replay>)>;
    /// Peer names (`Room` seats' `peer`).
    type Address = String;
}

/// ggrs' socket: what it sends waits in `out` for the network (or a test's wire), what came in waits in `inbox`.
#[derive(Clone, Default)]
pub struct Wires {
    pub out: Arc<Mutex<Vec<(String, Message)>>>,
    pub inbox: Arc<Mutex<Vec<(String, Message)>>>,
}

impl NonBlockingSocket<String> for Wires {
    fn send_to(&mut self, msg: &Message, addr: &String) {
        self.out.lock().unwrap().push((addr.clone(), msg.clone()));
    }

    fn receive_all_messages(&mut self) -> Vec<(String, Message)> {
        std::mem::take(&mut *self.inbox.lock().unwrap())
    }
}

/// The rollback session: ggrs', its wires, which seat each ggrs player is.
#[derive(Resource)]
pub struct Rollback {
    pub sess: P2PSession<Cfg>,
    pub wires: Wires,
    /// ggrs player `h` is seat `seats[h]`.
    seats: Vec<usize>,
    /// The next tick a run gives its pads to (where the last restore put the match, then counting).
    frame: usize,
    /// Gate calls since this machine last sat out a tick (time sync).
    since: u32,
    /// B197: real-time ticks (gate calls) not run yet, and how many of those time sync keeps this machine behind
    /// (negative: ahead of real time).
    owed: i32,
    shift: i32,
    /// `HST_NET_SYNCTEST=n` with every seat on this machine: ggrs' sync test runs the ticks instead, each tick going
    /// back `n` ticks and running them again, and stops on a state hash that comes out differently.
    sync: Option<ggrs::SyncTestSession<Cfg>>,
    /// The sync test's first run of each tick's state (`state_text`), to write out beside a re-run that differs.
    texts: std::collections::HashMap<i32, String>,
    /// Runs one match tick (the mode's fixed schedule; tests run theirs).
    pub step: fn(&mut World),
    /// Ticks run again after a restore, in all.
    pub reruns: u64,
    /// Ticks sat out (time sync) and frames held on the prediction window, in all (B154's lag numbers).
    pub skipped: u64,
    pub held: u64,
    /// The last stats line: (real seconds, reruns, ticks run) then.
    logged: (f32, u64, usize),
    /// The frame-time and network trace (B163).
    trace: Trace,
    /// ggrs' state cells by the frame last saved in each (its ring's slots): a resync's state comes from one and its
    /// re-run saves into them (B154b).
    cells: Vec<Option<(i32, Cell)>>,
    /// B196: the match tick ggrs' frame 0 is (a session restarted mid-match, `recover.rs`), and the restart's number
    /// (every datagram carries it, so an older session's are dropped).
    pub base: usize,
    pub round: u8,
}

type Cell = ggrs::GameStateCell<Arc<(snapshot::Snapshot, Option<Replay>)>>;

/// What each tick gate saw (B163): every 30 s a summary line in the log, and with `HST_NET_TRACE=file.csv` (native)
/// a row per gate call.
#[derive(Default)]
struct Trace {
    file: Option<std::io::BufWriter<std::fs::File>>,
    t0: Option<web_time::Instant>,
    /// When the last datagram came in, and the longest wait between two since the last row.
    heard: Option<web_time::Instant>,
    gap: f32,
    /// Since the last summary: frames over 33 ms, the longest frame, the longest re-run, ticks sat out, stalls.
    slow: u32,
    worst: f32,
    rolled: usize,
    skipped: u32,
    stalls: u32,
    next: f32,
}

impl Trace {
    fn new() -> Self {
        let file = std::env::var("HST_NET_TRACE").ok().and_then(|p| std::fs::File::create(p).ok()).map(std::io::BufWriter::new);
        let mut t = Trace { file, t0: Some(web_time::Instant::now()), next: 30.0, ..default() };
        t.row(format_args!("t_ms,dt_ms,frame,ran,rolled,cost_ms,skip,stall,ahead,ping_ms,behind,gap_ms"));
        t
    }

    fn row(&mut self, r: std::fmt::Arguments) {
        use std::io::Write;
        if let Some(f) = &mut self.file {
            let _ = writeln!(f, "{r}");
        }
    }
}

impl Rollback {
    /// `room`'s session for peer `me`: a ggrs player per taken seat, in seat order.
    pub fn new(room: &Room, me: &str, step: fn(&mut World)) -> Result<Self, GgrsError> {
        let seats: Vec<usize> = (0..room.seats.len()).filter(|&s| room.seats[s].as_ref().is_some_and(crate::net::Seat::human)).collect();
        let mut b = SessionBuilder::<Cfg>::new()
            .with_num_players(seats.len())?
            .with_input_delay(DELAY)
            .with_max_prediction_window(WINDOW)
            .with_desync_detection_mode(DesyncDetection::On { interval: super::netplay::HASH_EVERY })
            // B196: a silent peer is `recover.rs`'s to deal with (ggrs' own drops it after 2 s)
            .with_disconnect_timeout(std::time::Duration::from_secs(1 << 20))
            .with_disconnect_notify_delay(std::time::Duration::from_secs(1 << 20));
        for (h, &s) in seats.iter().enumerate() {
            let peer = &room.seats[s].as_ref().unwrap().peer;
            b = b.add_player(if peer == me { PlayerType::Local } else { PlayerType::Remote(peer.clone()) }, h)?;
        }
        let sync = std::env::var("HST_NET_SYNCTEST").ok().and_then(|v| v.parse().ok()).filter(|_| seats.iter().all(|&s| room.seats[s].as_ref().unwrap().peer == me));
        let sync = match sync {
            Some(n) => Some(SessionBuilder::<Cfg>::new().with_num_players(seats.len())?.with_max_prediction_window(WINDOW).with_check_distance(n).with_input_delay(DELAY).start_synctest_session()?),
            None => None,
        };
        let wires = Wires::default();
        Ok(Rollback { sess: b.start_p2p_session(wires.clone())?, wires, seats, sync, texts: Default::default(), frame: 0, since: 0, owed: 0, shift: 0, step, reruns: 0, skipped: 0, held: 0, logged: (0.0, 0, 0), trace: Trace::new(), cells: vec![None; WINDOW + 1], base: 0, round: 0 })
    }

    /// The tick the next run is.
    #[cfg(test)]
    pub fn frame(&self) -> usize {
        self.frame
    }

    /// The seats on other machines.
    pub fn remote_seats(&self) -> Vec<usize> {
        self.sess.remote_player_handles().into_iter().map(|h| self.seats[h]).collect()
    }
}

/// A peer's datagram into ggrs.
pub fn heard(w: &mut World, from: String, data: &[u8]) {
    let Some(mut rb) = w.get_resource_mut::<Rollback>() else { return };
    let now = web_time::Instant::now();
    if let Some(t) = rb.trace.heard.replace(now) {
        rb.trace.gap = rb.trace.gap.max((now - t).as_secs_f32() * 1000.0);
    }
    if data.first() != Some(&rb.round) {
        return;
    }
    if let Ok(m) = bincode::deserialize::<Message>(&data[1..]) {
        rb.wires.inbox.lock().unwrap().push((from, m));
    }
}

/// ggrs' datagrams for the network: (peer, bytes).
pub fn outgoing(w: &World) -> Vec<(String, Vec<u8>)> {
    let Some(rb) = w.get_resource::<Rollback>() else { return Vec::new() };
    let out = std::mem::take(&mut *rb.wires.out.lock().unwrap());
    out.into_iter().map(|(to, m)| (to, [vec![rb.round], bincode::serialize(&m).unwrap()].concat())).collect()
}

/// ggrs' upkeep (acks, keep-alives, its events) each frame, also while the match is paused.
pub fn poll(w: &mut World) {
    let Some(mut rb) = w.get_resource_mut::<Rollback>() else { return };
    rb.sess.poll_remote_clients();
    let events: Vec<_> = rb.sess.events().collect();
    for e in events {
        match e {
            // ggrs' advice sits out the whole lead at once, and only from 3 ticks: a freeze of 50 ms or more, every
            // second while it lasts (B163); `gate` spreads the waits out instead
            GgrsEvent::WaitRecommendation { .. } => {}
            GgrsEvent::DesyncDetected { frame, local_checksum, remote_checksum, .. } => {
                let t = w.resource::<Rollback>().base as u32 + frame as u32;
                super::netplay::desync(w, (t, local_checksum as u64, remote_checksum as u64))
            }
            _ => {}
        }
    }
    stats(w);
}

/// Every 10 s in the log: each peer's round trip and how far ahead it runs, and the ticks re-run, sat out and held
/// since (B154: what online lag is made of).
fn stats(w: &mut World) {
    let now = w.resource::<Time<Real>>().elapsed_secs();
    let mut rb = w.resource_mut::<Rollback>();
    if now - rb.logged.0 < 10.0 {
        return;
    }
    let peers: Vec<String> = rb
        .sess
        .remote_player_handles()
        .into_iter()
        .filter_map(|h| rb.sess.network_stats(h).ok().map(|n| format!("{}P {} ms round trip, this machine {} ticks behind it", rb.seats[h] + 1, n.ping, n.local_frames_behind)))
        .collect();
    let ticks = rb.frame - rb.logged.2.min(rb.frame);
    info!(
        "rollback at tick {}: {}; last {ticks} ticks: {} re-run, {} sat out, {} held (delay {DELAY}, window {WINDOW})",
        rb.frame,
        peers.join(", "),
        rb.reruns - rb.logged.1,
        rb.skipped,
        rb.held
    );
    rb.logged = (now, rb.reruns, rb.frame);
    (rb.skipped, rb.held) = (0, 0);
}

/// Saves the state into ggrs' cell, with its hash every `every` ticks (ggrs compares those).
fn save(w: &mut World, cell: ggrs::GameStateCell<Arc<(snapshot::Snapshot, Option<Replay>)>>, frame: i32, every: i32) {
    let s = snapshot::save(w);
    let queue = super::instant_replay::playing(w).then(|| w.get_resource::<Replay>().cloned()).flatten();
    let hash = (frame % every == 0).then(|| {
        let text = snapshot::state_text(w);
        let h = snapshot::fnv(&text);
        // progress in the log, as lockstep's (a rolled-back frame logs again, corrected)
        if frame % (every * 60) == 0 {
            info!("rollback tick {}: hash {h:016x}", frame as usize + w.get_resource::<Rollback>().map_or(0, |rb| rb.base));
        }
        if let Some(mut rb) = w.get_resource_mut::<Rollback>().filter(|rb| rb.sync.is_some()) {
            match rb.texts.get(&frame) {
                Some(first) if *first != text => {
                    for (n, t) in [("first", first), ("rerun", &text)] {
                        let _ = std::fs::write(format!("synctest-{frame}-{n}.txt"), t);
                    }
                }
                Some(_) => {}
                None => {
                    rb.texts.insert(frame, text.clone());
                    rb.texts.remove(&(frame - 64));
                }
            }
        }
        let base = w.get_resource::<Rollback>().map_or(0, |rb| rb.base) as u32;
        if let Some(mut np) = w.get_resource_mut::<Netplay>() {
            np.state = (base + frame as u32, text);
        }
        h as u128
    });
    if let Some(mut rb) = w.get_resource_mut::<Rollback>() {
        let n = rb.cells.len();
        rb.cells[frame as usize % n] = Some((frame, cell.clone()));
    }
    cell.save(frame, Some(Arc::new((s, queue))), hash);
}

/// The host after a desync (B154b): the state at the last tick every seat's pads are in for goes out.
fn send_state(w: &mut World) {
    if !w.resource::<Netplay>().resync.due {
        return;
    }
    if let Some((t, data)) = confirmed(w) {
        super::resync::send(&mut w.resource_mut::<Netplay>(), t, data);
    }
}

/// The match before the last tick every seat's pads are in for, from ggrs' cell (the match is put back as it is now),
/// and that tick; before the first tick, the match as it is.
pub(super) fn confirmed(w: &mut World) -> Option<(u32, Vec<u8>)> {
    let rb = w.resource::<Rollback>();
    let f = rb.sess.confirmed_frame().max(0);
    let t = rb.base as u32 + f as u32;
    if rb.frame == rb.base {
        return Some((t, snapshot::encode(w)));
    }
    let cell = rb.cells.iter().flatten().find(|c| c.0 == f).map(|c| c.1.clone())?;
    let then = cell.load()?;
    let (now, queue) = (snapshot::save(w), w.get_resource::<Replay>().cloned());
    snapshot::restore(w, &then.0);
    if let Some(q) = &then.1 {
        w.insert_resource(q.clone());
    }
    let data = snapshot::encode(w);
    snapshot::restore(w, &now);
    if let Some(q) = queue {
        w.insert_resource(q);
    }
    Some((t, data))
}

/// B196: a new session for `room` from tick `tick` on (the match already put there), restart number `round`.
pub(super) fn restart(w: &mut World, room: &Room, me: &str, tick: u32, round: u8) {
    let step = w.get_resource::<Rollback>().map_or(crate::mode::run_fixed as fn(&mut World), |rb| rb.step);
    let mut rb = Rollback::new(room, me, step).expect("the room's seats");
    (rb.base, rb.frame, rb.round) = (tick as usize, tick as usize, round);
    w.insert_resource(rb);
}

/// A peer with the host's state before tick F, once every seat's pads before F are in here too (a rollback never goes
/// back past it to an unmended cell) (B154b): the state in, ticks F.. run again on the pads it ran them on, saved into
/// ggrs' cells where they're still its (a later rollback loads the mended state).
fn take_state(w: &mut World) {
    let rb = w.resource::<Rollback>();
    let (frame, base) = (rb.frame, rb.base);
    let confirmed = base as i32 + rb.sess.confirmed_frame();
    let mut np = w.resource_mut::<Netplay>();
    let Some((from, data)) = np.resync.pending.take_if(|p| p.0 as i32 <= confirmed) else { return };
    let log = np.lock.log.clone();
    if let Err(e) = snapshot::decode(w, &data) {
        return warn!("online: the host's state didn't load: {e}");
    }
    let mut rb = w.resource_mut::<Rollback>();
    let mut reqs = Vec::new();
    for f in from as usize..frame {
        let g = (f - base) as i32;
        let cell = rb.cells.iter().flatten().find(|c| c.0 == g).map_or_else(Cell::default, |c| c.1.clone());
        reqs.push(GgrsRequest::SaveGameState { cell, frame: g });
        let inputs = rb.seats.iter().map(|&s| (Input(wire(log[f][s])), ggrs::InputStatus::Confirmed)).collect();
        reqs.push(GgrsRequest::AdvanceFrame { inputs });
    }
    rb.frame = from as usize;
    rb.reruns += (frame - from as usize) as u64;
    handle(w, reqs, super::netplay::HASH_EVERY as i32);
    super::resync::taken(&mut w.resource_mut::<Netplay>(), from, frame as u32);
}

/// Carries out ggrs' requests: save, restore, run a tick on these pads. The local devices' latches (`Pads`) stay as
/// they are now (a restore would bring back old ones).
pub fn handle(w: &mut World, reqs: Vec<GgrsRequest<Cfg>>, every: i32) {
    let pads = w.get_resource::<Pads>().cloned();
    let (seats, step) = {
        let rb = w.resource::<Rollback>();
        (rb.seats.clone(), rb.step)
    };
    for r in reqs {
        match r {
            GgrsRequest::SaveGameState { cell, frame } => save(w, cell, frame, every),
            GgrsRequest::LoadGameState { cell, frame } => {
                let s = cell.load().expect("a saved state");
                snapshot::restore(w, &s.0);
                if let Some(q) = &s.1 {
                    w.insert_resource(q.clone());
                }
                let mut rb = w.resource_mut::<Rollback>();
                let f = rb.base + frame as usize;
                rb.reruns += (rb.frame - f) as u64;
                rb.frame = f;
            }
            GgrsRequest::AdvanceFrame { inputs } => {
                let mut now = [IDLE; 4];
                for (h, (i, _)) in inputs.iter().enumerate() {
                    now[seats[h]] = unwire(i.0);
                }
                let f = w.resource::<Rollback>().frame;
                let mut last = [IDLE; 4];
                if let Some(mut np) = w.get_resource_mut::<Netplay>() {
                    np.lock.log.truncate(f);
                    last = f.checked_sub(1).and_then(|p| np.lock.log.get(p).copied()).unwrap_or(last);
                    np.lock.log.push(now);
                }
                if super::instant_replay::playing(w) {
                    // an instant replay plays its own pads: the tick's presses (✕/○ skip) come as the latches it
                    // reads then, the same on every machine (`Pads` goes back after)
                    if let Some(mut p) = w.get_resource_mut::<Pads>() {
                        for s in 0..4 {
                            p.slots[s] = tick_input::decode(now[s], last[s]);
                        }
                    }
                } else {
                    w.insert_resource(Replay([now].into()));
                }
                step(w);
                w.resource_mut::<Rollback>().frame += 1;
            }
        }
    }
    if let Some(p) = pads {
        w.insert_resource(p);
    }
}

/// The match's tick gate (`mode::Gate`) with rollback: runs the next tick (and any re-runs) itself, so it always
/// says no to the mode's own run.
pub fn gate(w: &mut World) -> bool {
    super::netplay::pump(w);
    poll(w);
    take_state(w);
    super::recover::restarted(w);
    if w.resource::<Netplay>().end.is_some() || w.resource::<Netplay>().recover.hold || w.resource::<Rollback>().sess.current_state() != SessionState::Running {
        return false;
    }
    // B197: every call is a tick of real time (Bevy's fixed step); one that didn't run (a stall, time sync) is run
    // later, a second tick in a call, so the match keeps 60 ticks a second whatever the others' frame rate. An instant
    // replay's skip (the clock sped up) runs a tick a call and owes nothing.
    let real = w.get_resource::<Time<Virtual>>().is_none_or(|v| v.relative_speed() <= 1.0);
    if let Some(mut v) = w.get_resource_mut::<Time<Virtual>>().filter(|v| v.relative_speed() == 1.0 && v.max_delta() != CATCH_UP) {
        v.set_max_delta(CATCH_UP);
    }
    let want = {
        let mut rb = w.resource_mut::<Rollback>();
        rb.since += 1;
        let ahead = rb.sess.frames_ahead();
        if ahead != 0 && rb.since >= SPREAD {
            let s = (rb.shift + ahead.signum()).clamp(-SHIFT, SHIFT);
            if s != rb.shift {
                (rb.shift, rb.since) = (s, 0);
            }
        }
        if real {
            rb.owed = (rb.owed + 1).min(OWE.max(rb.shift + 1));
            (rb.owed - rb.shift).clamp(0, 2)
        } else {
            1
        }
    };
    if want == 0 {
        w.resource_mut::<Rollback>().skipped += 1;
        note(w, (0, 0), 0.0, true);
    }
    for _ in 0..want {
        if !advance(w) {
            break;
        }
        if real {
            w.resource_mut::<Rollback>().owed -= 1;
        }
    }
    false
}

/// Runs the next tick (and any re-runs ggrs asks for): false if it couldn't (waiting on a peer's pads).
fn advance(w: &mut World) -> bool {
    // this machine's seats' pads (each seat's local device in the slot of the same number, `--pads`)
    let (handles, seats, frame) = {
        let rb = w.resource::<Rollback>();
        (rb.sess.local_player_handles(), rb.seats.clone(), rb.base as i32 + rb.sync.as_ref().map_or(rb.sess.current_frame(), |s| s.current_frame()))
    };
    let mut local: Vec<_> = {
        let mut pads = w.resource_mut::<Pads>();
        handles.iter().map(|&h| tick_input::take(&mut pads.slots[seats[h]])).collect()
    };
    let mut np = w.resource_mut::<Netplay>();
    if np.mash.is_some() {
        np.lock.tick = frame.max(0) as u32;
        local = handles.iter().map(|&h| np.mashed(seats[h])).collect();
    }
    let mut rb = w.resource_mut::<Rollback>();
    if let Some(sync) = rb.sync.as_mut() {
        for (&h, &p) in handles.iter().zip(&local) {
            sync.add_local_input(h, Input(wire(p))).expect("a seat");
        }
        match sync.advance_frame() {
            Ok(reqs) => handle(w, reqs, 1),
            Err(e) => w.resource_mut::<Netplay>().finish(format!("Sync test: {e}"), true),
        }
        return true;
    }
    for (&h, &p) in handles.iter().zip(&local) {
        rb.sess.add_local_input(h, Input(wire(p))).expect("a local seat");
    }
    let (t0, f0, r0) = (web_time::Instant::now(), rb.frame, rb.reruns);
    let ok = match rb.sess.advance_frame() {
        Ok(reqs) => {
            handle(w, reqs, super::netplay::HASH_EVERY as i32);
            w.resource_mut::<Netplay>().stalled = 0.0;
            send_state(w);
            true
        }
        // too far ahead of a peer: wait for its pads ("Waiting for …" after a while)
        Err(GgrsError::PredictionThreshold) => {
            w.resource_mut::<Rollback>().held += 1;
            false
        }
        Err(e) => {
            warn!("rollback: {e}");
            false
        }
    };
    let rb = w.resource::<Rollback>();
    let ran = (rb.frame + (rb.reruns - r0) as usize - f0, (rb.reruns - r0) as usize);
    note(w, ran, t0.elapsed().as_secs_f32() * 1000.0, false);
    super::netplay::pump(w);
    ok
}

/// The trace's row for this gate call: `ran` ticks run of which `rolled` were re-runs, `cost` ms spent on them, or
/// a tick sat out (`skip`: time sync) or none run (a stall: waiting for a peer's pads).
fn note(w: &mut World, (ran, rolled): (usize, usize), cost: f32, skip: bool) {
    let stall = !skip && ran == 0;
    let dt = w.resource::<Time<Real>>().delta_secs() * 1000.0;
    let mut rb = w.resource_mut::<Rollback>();
    let frame = rb.sess.current_frame();
    let ahead = rb.sess.frames_ahead();
    let stats: Vec<_> = rb.sess.remote_player_handles().into_iter().filter_map(|h| rb.sess.network_stats(h).ok()).collect();
    let ping = stats.iter().map(|s| s.ping).max().unwrap_or(0);
    let behind = stats.iter().map(|s| s.local_frames_behind).max().unwrap_or(0);
    let tr = &mut rb.trace;
    let t = tr.t0.get_or_insert_with(web_time::Instant::now).elapsed().as_secs_f32();
    let gap = std::mem::take(&mut tr.gap);
    tr.row(format_args!("{:.1},{dt:.2},{frame},{ran},{rolled},{cost:.2},{},{},{ahead},{ping},{behind},{gap:.1}", t * 1000.0, skip as u8, stall as u8));
    tr.slow += (dt > 33.0) as u32;
    tr.worst = tr.worst.max(dt);
    tr.rolled = tr.rolled.max(rolled);
    tr.skipped += skip as u32;
    tr.stalls += stall as u32;
    if t >= tr.next {
        tr.next = t + 30.0;
        info!(
            "rollback, last 30 s: {} frames over 33 ms (longest {:.0} ms), longest re-run {} ticks, {} ticks sat out, {} stalls, ping {ping} ms, {ahead} ticks ahead",
            tr.slow, tr.worst, tr.rolled, tr.skipped, tr.stalls
        );
        let (file, t0) = (tr.file.take(), tr.t0);
        *tr = Trace { file, t0, next: tr.next, ..default() };
        if let Some(f) = &mut tr.file {
            let _ = std::io::Write::flush(f);
        }
    }
}

/// B197's test knob, `HST_NET_THROTTLE="FPS HITCH_MS"` (native): this machine's frames capped to FPS, and every 3 s
/// one frame HITCH_MS longer (a slow peer, a hitch).
#[cfg(not(target_arch = "wasm32"))]
pub fn throttle(mut last: Local<Option<(std::time::Instant, std::time::Instant)>>) {
    use std::time::{Duration, Instant};
    let v = std::env::var("HST_NET_THROTTLE").unwrap_or_default();
    let mut it = v.split_whitespace().map(|n| n.parse::<f32>().unwrap_or(0.0));
    let (fps, hitch) = (it.next().unwrap_or(0.0), it.next().unwrap_or(0.0));
    let now = Instant::now();
    let (frame, hitched) = last.get_or_insert((now, now));
    let mut until = *frame + Duration::from_secs_f32(if fps > 0.0 { 1.0 / fps } else { 0.0 });
    if hitch > 0.0 && now - *hitched >= Duration::from_secs(3) {
        *hitched = now;
        until += Duration::from_secs_f32(hitch / 1000.0);
    }
    std::thread::sleep(until.saturating_duration_since(now));
    *frame = Instant::now();
}
