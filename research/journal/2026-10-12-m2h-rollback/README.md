# M2h — rollback netplay with ggrs

## What it does

- `crates/hst/src/play/rollback.rs` (new): a `ggrs` 0.13 `P2PSession` (core crate, no bevy_ggrs).
  - Its state is M2c's `snapshot::save`/`restore`, held in an `Arc`.
  - Every 30th tick's `state_hash` is the checksum ggrs compares on confirmed ticks (`DesyncDetection::On`). A
    mismatch goes through M2g's `netplay::desync`: "Out of sync at tick N …", and every machine saves its inputs.
  - One ggrs player per taken seat, in seat order. Seats on this machine are `Local`; the rest are `Remote(peer name)`.
  - `rollback::gate` is the match's `mode::Gate`. It reads this machine's seats' pads (or `HST_NET_MASH`), adds them
    to ggrs and carries out ggrs' Save/Load/Advance requests. Advance hands the pads to the match as a one-tick
    `tick_input::Replay` and runs the fixed tick itself (`mode::run_fixed`), so the gate always says no to the mode's
    own run. The local devices' `Pads` latches are kept across a restore.
  - `np.lock.log` is truncated and rewritten on re-runs, so a desync file holds the pads actually run.
  - `WaitRecommendation` makes the machine sit out ticks. `PredictionThreshold` waits, and after 0.25 s M2g's
    "Waiting for 3P …" shows the remote seats. `Disconnected` ends the match ("Player xxxx stopped answering.").
  - `DELAY = 2` ticks of local input delay. `WINDOW = 8` is ggrs' prediction window.
- Transport (`net.rs`): ggrs' messages go out as bincode in iroh **datagrams** (`Cmd::Datagram`/`Event::Datagram`).
  - Each datagram is framed `[len][peer name][payload]`. It is star-shaped like the streams: a guest's datagram goes
    to the host, which keeps it or forwards it to the named guest.
  - `HST_NET_LAG` delays datagrams too.
  - The streams still carry pause, the end of the match and drops (M2g).
- Lockstep is still available: when the host sets `HST_NET_LOCKSTEP`, the `Room` field `lockstep` picks M2g's gate.
  Rollback is the default.
- ggrs builds on wasm32 too (`wasm-bindgen` feature), like the rest of netplay.

## Verified (crates/hst/src/play/netplay.rs tests, `matches`)

- `synctest_reruns_every_tick_the_same`: ggrs' `SyncTestSession` with check distance 7.
  - Each tick it goes back 7 ticks, re-runs them and compares every tick's state hash (it panics on a difference).
  - Four mashing seats, 420 ticks into the first rally, 2000+ re-run ticks.
  - This is the determinism check that runs in CI (`tools/check.sh`).
- `rollback_at_150ms_stays_in_sync`: two apps, host and guest, over a simulated wire with a 5-frame one-way lag
  (167 ms RTT), both mashing, `DELAY = 2`.
  - 1135 ticks: no stall end, no desync, and the confirmed pad logs are equal on both machines.
  - 618 and 921 ticks were re-run.
  - Checksums were compared up to the last 30-tick mark.
- `full_doubles_match_rollback_at_150ms_stays_in_sync` (ignored, release): the same, over a whole doubles match (below).
- `rollback_desync_stops_both_and_saves`: the guest reseeds its AI generator at tick 300. Both machines then stop at
  tick 330, the next checksum, with "Out of sync at tick 330" and the desync files saved.
- Not on the wire: the datagram framing (`datagram`/`undatagram`) and the host relay are exercised only by the real
  run below.

## Tick cost (`rollback_tick_cost`, ignored; release, Ryzen 7 9800X3D)

3000 ticks of a mashed doubles match:

| | mean | median | 99 % | max |
|---|---|---|---|---|
| tick | 0.50 ms | 0.47 ms | 1.24 ms | 2.03 ms |
| snapshot save | 0.033 ms | 0.033 ms | 0.05 ms | 0.17 ms |
| state hash (every 30th) | 0.20 ms | 0.18 ms | 0.40 ms | 0.80 ms |

- A rollback of the whole window is 9 ticks and 9 saves: about 4.8 ms here.
- At 150 ms RTT with delay 2, a typical rollback is about 3 ticks (75 ms one way ≈ 4.5 ticks, minus the 2-tick
  delay): about 1.6 ms here.
- A low-end phone is assumed to be 6–8× slower per thread. That puts a typical rollback at ~10–13 ms, under a 16.7 ms
  frame. A full 8-tick one, ~30–40 ms, would drop one or two frames, which is acceptable for a rare spike.
- So `WINDOW` stays at ggrs' default of 8. If a phone measurement says otherwise, lower it (the cost is more
  "Waiting for …" stalls at high RTT).

## Full match

`cargo test --release -p hst full_doubles_match_rollback -- --ignored --nocapture`: in sync over 10396 ticks, until the
match was over. 5168 and 9633 ticks were re-run. It took 23 s.

## Not verified / not 1:1

- **Tick cost on a low-end phone**: not measured. Native mobile was dropped (user 2026-10-12) and no phone or
  browser-on-phone run was available unattended. The prediction window is set from the desktop measurement and an
  assumed 6–8× slowdown. Measure in the browser build (M3/M9) on a phone and adjust `WINDOW`.
- **Two real machines / real network loss**: the two-machine 150 ms test is in-process with a simulated wire (no
  datagram loss or reordering). ggrs resends unacked inputs, so loss only delays confirmation, but loss was not
  exercised. Cross-network runs are already a human item (M2f).
- **Sounds of mispredicted ticks**: a tick run on a guessed pad may play a sound that the corrected re-run doesn't.
  It can't be taken back, because it has already played. M2d (branch task/M2d, not yet merged when this was written)
  dedupes sounds by tick, so a re-run doesn't play them twice. M2h relies on that merge.
- **Datagrams before the match has loaded** are dropped by the menu. ggrs resends until acked, so it only delays the
  sync handshake.
- **Star topology through the host**: a guest-to-guest datagram goes through the host (the same as M2g), so guest
  pairs see up to two hops of latency.
- **Original game**: it has no online play, so there is nothing to match 1:1. Offline matches don't go through this
  gate, and the replay tests stay bit-exact.
