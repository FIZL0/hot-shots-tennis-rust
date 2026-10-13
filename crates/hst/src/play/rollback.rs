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
/// How many ticks a machine may run on guessed pads before it waits (ggrs' prediction window). A re-run of the whole
/// window is `WINDOW + 1` ticks and as many saves in one frame: measured in `tick_cost` (journal: 2026-10-12-m2h).
pub const WINDOW: usize = 8;
/// Time sync: while this machine runs `n` ticks ahead of the others' middle, it sits out one tick in every
/// `SPREAD / n` (at least every other), never several in a row (B163).
pub const SPREAD: u32 = 20;

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
    /// `HST_NET_SYNCTEST=n` with every seat on this machine: ggrs' sync test runs the ticks instead, each tick going
    /// back `n` ticks and running them again, and stops on a state hash that comes out differently.
    sync: Option<ggrs::SyncTestSession<Cfg>>,
    /// The sync test's first run of each tick's state (`state_text`), to write out beside a re-run that differs.
    texts: std::collections::HashMap<i32, String>,
    /// Runs one match tick (the mode's fixed schedule; tests run theirs).
    pub step: fn(&mut World),
    /// Ticks run again after a restore, in all.
    pub reruns: u64,
    /// The frame-time and network trace (B163).
    trace: Trace,
}

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
            .with_desync_detection_mode(DesyncDetection::On { interval: super::netplay::HASH_EVERY });
        for (h, &s) in seats.iter().enumerate() {
            let peer = &room.seats[s].as_ref().unwrap().peer;
            b = b.add_player(if peer == me { PlayerType::Local } else { PlayerType::Remote(peer.clone()) }, h)?;
        }
        let sync = std::env::var("HST_NET_SYNCTEST").ok().and_then(|v| v.parse().ok()).filter(|_| seats.iter().all(|&s| room.seats[s].as_ref().unwrap().peer == me));
        let sync = match sync {
            Some(n) => Some(SessionBuilder::<Cfg>::new().with_num_players(seats.len())?.with_check_distance(n).with_input_delay(DELAY).start_synctest_session()?),
            None => None,
        };
        let wires = Wires::default();
        Ok(Rollback { sess: b.start_p2p_session(wires.clone())?, wires, seats, sync, texts: Default::default(), frame: 0, since: 0, step, reruns: 0, trace: Trace::new() })
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
    if let Ok(m) = bincode::deserialize::<Message>(data) {
        rb.wires.inbox.lock().unwrap().push((from, m));
    }
}

/// ggrs' datagrams for the network: (peer, bytes).
pub fn outgoing(w: &World) -> Vec<(String, Vec<u8>)> {
    let Some(rb) = w.get_resource::<Rollback>() else { return Vec::new() };
    let out = std::mem::take(&mut *rb.wires.out.lock().unwrap());
    out.into_iter().map(|(to, m)| (to, bincode::serialize(&m).unwrap())).collect()
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
                super::netplay::desync(w, (frame as u32, local_checksum as u64, remote_checksum as u64))
            }
            GgrsEvent::Disconnected { addr } => {
                let who: String = addr.chars().take(8).collect();
                w.resource_mut::<Netplay>().finish(format!("Player {who} stopped answering. The match is over."), true);
            }
            _ => {}
        }
    }
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
            info!("rollback tick {frame}: hash {h:016x}");
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
        if let Some(mut np) = w.get_resource_mut::<Netplay>() {
            np.state = (frame as u32, text);
        }
        h as u128
    });
    cell.save(frame, Some(Arc::new((s, queue))), hash);
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
                rb.reruns += (rb.frame - frame as usize) as u64;
                rb.frame = frame as usize;
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
    if w.resource::<Netplay>().end.is_some() || w.resource::<Rollback>().sess.current_state() != SessionState::Running {
        return false;
    }
    {
        let mut rb = w.resource_mut::<Rollback>();
        rb.since += 1;
        let ahead = rb.sess.frames_ahead().max(0) as u32;
        if ahead > 0 && rb.since >= (SPREAD / ahead).max(2) {
            rb.since = 0;
            note(w, (0, 0), 0.0, true);
            return false;
        }
    }
    // this machine's seats' pads (each seat's local device in the slot of the same number, `--pads`)
    let (handles, seats, frame) = {
        let rb = w.resource::<Rollback>();
        (rb.sess.local_player_handles(), rb.seats.clone(), rb.sync.as_ref().map_or(rb.sess.current_frame(), |s| s.current_frame()))
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
        return false;
    }
    for (&h, &p) in handles.iter().zip(&local) {
        rb.sess.add_local_input(h, Input(wire(p))).expect("a local seat");
    }
    let (t0, f0, r0) = (web_time::Instant::now(), rb.frame, rb.reruns);
    match rb.sess.advance_frame() {
        Ok(reqs) => {
            handle(w, reqs, super::netplay::HASH_EVERY as i32);
            w.resource_mut::<Netplay>().stalled = 0.0;
        }
        // too far ahead of a peer: wait for its pads ("Waiting for …" after a while)
        Err(GgrsError::PredictionThreshold) => {}
        Err(e) => warn!("rollback: {e}"),
    }
    let rb = w.resource::<Rollback>();
    let ran = (rb.frame + (rb.reruns - r0) as usize - f0, (rb.reruns - r0) as usize);
    note(w, ran, t0.elapsed().as_secs_f32() * 1000.0, false);
    super::netplay::pump(w);
    false
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
