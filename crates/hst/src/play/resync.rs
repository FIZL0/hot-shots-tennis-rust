//! B154b: after a desync the host sends the peers its state and they take it on, so the match plays on in sync. The
//! state (`snapshot::encode`) is the host's before tick F; a peer that gets it (`Msg::State`) at tick N restores it
//! and runs ticks F..N again on the pads it ran them on (its log; in lockstep those are final, with rollback any
//! guessed ones are put right by ggrs as usual), then tells the host (`Msg::Resynced(N)`). Hashes of ticks before N
//! are from before the resync: neither side reports them, nor anything while a sent state isn't taken yet. Every
//! desync after that is logged and reported again (`netplay::desync`).
//!
//! Lockstep's side is here; rollback's (the host's state at a confirmed tick from ggrs' cells, the peer's re-run
//! through `rollback::handle` so ggrs' cells hold the mended states) is in `rollback.rs`.

use super::netplay::{Netplay, run_on};
use super::snapshot;
use super::tick_input::IDLE;
use crate::net::Msg;
use bevy::prelude::*;

/// Bytes of the state a message.
pub const PIECE: usize = 48 << 10;

pub struct Resync {
    /// A peer: the host's state before tick `.0`, waiting for this machine to get there.
    pub(super) pending: Option<(u32, Vec<u8>)>,
    /// A peer: the pieces of the host's state heard so far.
    pub(super) pieces: Vec<u8>,
    /// The host: a desync was seen, the state goes out at the next tick gate.
    pub(super) due: bool,
    /// The host: a state went out and no peer has taken it yet.
    pub(super) sent: bool,
    /// Hashes of ticks before this are from before the last resync.
    pub fresh: u32,
    /// Resyncs this machine has done (sent or taken).
    pub done: u32,
    /// Runs one match tick in lockstep (the mode's fixed schedule; tests run theirs).
    pub step: fn(&mut World),
}

impl Default for Resync {
    fn default() -> Self {
        Resync { pending: None, pieces: Vec::new(), due: false, sent: false, fresh: 0, done: 0, step: crate::mode::run_fixed }
    }
}

impl Resync {
    /// A desync at tick `t` is one a resync is already mending.
    pub(super) fn stale(&self, t: u32) -> bool {
        t < self.fresh || self.sent
    }
}

/// The host saw a desync: its state goes out at the next gate.
pub(super) fn ask(np: &mut Netplay) {
    if np.me == "host" && !np.resync.sent {
        np.resync.due = true;
    }
}

/// A peer's message about a resync.
pub(super) fn heard(np: &mut Netplay, m: Msg) {
    match m {
        Msg::State { tick, data, last } if np.me != "host" => {
            np.resync.pieces.extend(data);
            if last {
                np.resync.pending = Some((tick, std::mem::take(&mut np.resync.pieces)));
            }
        }
        Msg::Resynced(n) if np.me == "host" => {
            info!("online: a peer took the state, in sync from tick {n}");
            let r = &mut np.resync;
            (r.fresh, r.sent, r.done) = (r.fresh.max(n), false, r.done + 1);
            np.desynced = false;
        }
        _ => {}
    }
}

/// The host's state before tick `tick` goes out.
pub(super) fn send(np: &mut Netplay, tick: u32, data: Vec<u8>) {
    info!("online: sending the state before tick {tick} ({} KB) to resync", data.len() / 1024);
    np.resync.due = false;
    np.resync.sent = true;
    // ~4 bytes a byte as JSON
    let n = data.len().div_ceil(PIECE);
    np.out.extend(data.chunks(PIECE).enumerate().map(|(i, d)| Msg::State { tick, data: d.to_vec(), last: i + 1 == n }));
}

/// The peer took the host's state and is at tick `n` again.
pub(super) fn taken(np: &mut Netplay, from: u32, n: u32) {
    info!("online: took the host's state before tick {from}, ran {} ticks again: in sync from tick {n}", n - from);
    let r = &mut np.resync;
    (r.pending, r.fresh, r.done) = (None, n, r.done + 1);
    np.desynced = false;
    np.out.push(Msg::Resynced(n));
}

/// The lockstep gate's part, before the tick's pads are checked: the host's state out, or the host's state in.
pub(super) fn lockstep(w: &mut World) {
    let Some(np) = w.get_resource::<Netplay>() else { return };
    let t = np.lock.tick;
    if np.resync.due {
        let data = snapshot::encode(w);
        send(&mut w.resource_mut::<Netplay>(), t, data);
        return;
    }
    let Some((from, _)) = np.resync.pending.as_ref().filter(|p| p.0 <= t) else { return };
    let from = *from;
    let data = w.resource_mut::<Netplay>().resync.pending.take().unwrap().1;
    if let Err(e) = snapshot::decode(w, &data) {
        return warn!("online: the host's state didn't load: {e}");
    }
    let (log, step) = {
        let np = w.resource::<Netplay>();
        (np.lock.log.clone(), np.resync.step)
    };
    for f in from as usize..t as usize {
        run_on(w, log[f], f.checked_sub(1).map_or([IDLE; 4], |l| log[l]));
        step(w);
    }
    taken(&mut w.resource_mut::<Netplay>(), from, t);
}
