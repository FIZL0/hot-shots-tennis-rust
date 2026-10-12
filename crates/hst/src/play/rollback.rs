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
    type State = Arc<snapshot::Snapshot>;
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
    /// Ticks to sit out (ggrs' advice when this machine runs ahead).
    skip: u32,
    /// Runs one match tick (the mode's fixed schedule; tests run theirs).
    pub step: fn(&mut World),
    /// Ticks run again after a restore, in all.
    pub reruns: u64,
}

impl Rollback {
    /// `room`'s session for peer `me`: a ggrs player per taken seat, in seat order.
    pub fn new(room: &Room, me: &str, step: fn(&mut World)) -> Result<Self, GgrsError> {
        let seats: Vec<usize> = (0..room.seats.len()).filter(|&s| room.seats[s].is_some()).collect();
        let mut b = SessionBuilder::<Cfg>::new()
            .with_num_players(seats.len())?
            .with_input_delay(DELAY)
            .with_max_prediction_window(WINDOW)
            .with_desync_detection_mode(DesyncDetection::On { interval: super::netplay::HASH_EVERY });
        for (h, &s) in seats.iter().enumerate() {
            let peer = &room.seats[s].as_ref().unwrap().peer;
            b = b.add_player(if peer == me { PlayerType::Local } else { PlayerType::Remote(peer.clone()) }, h)?;
        }
        let wires = Wires::default();
        Ok(Rollback { sess: b.start_p2p_session(wires.clone())?, wires, seats, frame: 0, skip: 0, step, reruns: 0 })
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
    let Some(rb) = w.get_resource::<Rollback>() else { return };
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
            GgrsEvent::WaitRecommendation { skip_frames } => w.resource_mut::<Rollback>().skip += skip_frames,
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
fn save(w: &mut World, cell: ggrs::GameStateCell<Arc<snapshot::Snapshot>>, frame: i32, every: i32) {
    let s = snapshot::save(w);
    let hash = (frame % every == 0).then(|| {
        let text = snapshot::state_text(w);
        let h = snapshot::fnv(&text);
        if let Some(mut np) = w.get_resource_mut::<Netplay>() {
            np.state = (frame as u32, text);
        }
        h as u128
    });
    cell.save(frame, Some(Arc::new(s)), hash);
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
                snapshot::restore(w, &cell.load().expect("a saved state"));
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
                if let Some(mut np) = w.get_resource_mut::<Netplay>() {
                    np.lock.log.truncate(f);
                    np.lock.log.push(now);
                }
                w.insert_resource(Replay([now].into()));
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
        if rb.skip > 0 {
            rb.skip -= 1;
            return false;
        }
    }
    // this machine's seats' pads (each seat's local device in the slot of the same number, `--pads`)
    let (handles, seats, frame) = {
        let rb = w.resource::<Rollback>();
        (rb.sess.local_player_handles(), rb.seats.clone(), rb.sess.current_frame())
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
    for (&h, &p) in handles.iter().zip(&local) {
        rb.sess.add_local_input(h, Input(wire(p))).expect("a local seat");
    }
    match rb.sess.advance_frame() {
        Ok(reqs) => {
            handle(w, reqs, super::netplay::HASH_EVERY as i32);
            w.resource_mut::<Netplay>().stalled = 0.0;
        }
        // too far ahead of a peer: wait for its pads ("Waiting for …" after a while)
        Err(GgrsError::PredictionThreshold) => {}
        Err(e) => warn!("rollback: {e}"),
    }
    super::netplay::pump(w);
    false
}
