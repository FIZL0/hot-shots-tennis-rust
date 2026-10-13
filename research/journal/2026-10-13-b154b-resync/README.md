# B154b — resync after an online desync

After a desync the host sends its match state; the peers take it on and run their logged pads again from it, so the
match plays on in sync. B154's log line, `desync-*.json` report and on-screen note still happen for every desync
(the `desynced` flag clears once a resync is taken, so the next one is reported again).

## How

- **State codec** (`play/snapshot.rs` `encode`/`decode`): bincode of `Game`, `TickPads`, the `Figure` rigs' `Motion`s
  (by figure number, not Entity), the instant-replay queue while one plays, and the 21 module resources (same list as
  `snapshot::save`, now one `modules()` walk shared by both). serde derives went onto the hst-sim / hst-data / hst
  types behind them; arrays past 32 use `hst_sim::big`.
  - `Const<T>` (per-match constants: serve tables, court world, disc data) is now `Option<Arc<T>>` and isn't sent:
    `decode` puts this machine's own back.
  - Handles and Entities in module resources are `#[serde(skip)]`; each resource's `Wire::local` copies this
    machine's own back in after decoding.
  - The instant-replay recorder's serve state (`Recorder.serve`, a `snapshot::save`) goes along too, encoded by putting
    it into the world for a moment. Leaving it local was the first version's bug: the next instant replay put back the
    guest's pre-resync state, and the run desynced again at ticks 570 and 750 (`rng.ai` still the test's reseed).
- **Messages** (`net.rs`): `Msg::State { tick, data, last }` in 48 KB pieces (a browser data channel takes ~256 KB a
  message; JSON is ~4 bytes a byte), `Msg::Resynced(n)` back. Both are match messages (relayed).
- **Lockstep** (`play/resync.rs`): the host sends its state at its gate's tick F; a peer at tick N ≥ F decodes it and
  runs F..N again on its log, then says `Resynced(N)`.
- **Rollback** (`play/rollback.rs`): `Rollback.cells` remembers ggrs' state cells by frame. The host sends the state at
  `confirmed_frame()` (loaded from its cell, the live state put back after encoding). A peer waits until its own confirmed frame reaches F too (otherwise
  a late pad from before F rolls it back into a pre-resync cell; the harness delivers match messages at once but
  datagrams 5 ticks late), then decodes it, runs F..N again
  through `handle` with hand-made `SaveGameState`/`AdvanceFrame` requests (its logged pads as confirmed), re-saving
  into the cells that are still ggrs' so a later rollback loads the mended state and the checksums sent from them are
  the right ones.
- **Old desyncs**: hashes of ticks before the peer's N, and any while a state is out and not taken, are ignored
  (`Resync::stale`); the host only asks once per sent state.

## Results

- `a_desync_is_saved_and_play_goes_on` (lockstep, guest's tick 119 on flipped pads): reported at 120, resynced in sync
  from tick 132, no desync to tick 390.
- `rollback_desync_is_saved_and_play_goes_on` (`play_rollback`, guest reseeds the AI generator at frame 300):
  reported at 330, resynced from 340, ggrs' checksums agree to 840.
- `a_sent_state_hashes_the_same`: the host's encoded state at tick 700 decoded into the guest and into a fresh match
  gives the host's `state_text` on both. 194 KB at tick 700, 232 KB at tick 3000 (the instant-replay recorder ring is
  the largest part, ~66 KB), about twice that with the serve state; sent in 48 KB pieces.
- `a_resynced_match_finishes` (ignored; `cargo test --release -p hst a_resynced_match_finishes -- --ignored`): the
  rollback match with that forced desync, resynced at 340, plays to the end (tick 6150; 9330 on another run: the harness's mashing differs run to run) with no desync
  after; ~25 s.

## Not verified / not 1:1

- Not 1:1 by nature: online play is ours, the original has none.
- No real two-machine run: everything is the in-process two-app harness (lagged wire). The browser path's piece size
  is from the spec'd 256 KB SCTP limit, not tried in a browser.
- Not transferred: the device pad latches (`Pads`) and the non-`Figure` rigs' motions (umpire, walkers, creatures;
  the hash doesn't cover them). If one ever drives hashed state a resync won't mend it: the next hash desyncs again
  and is reported and resynced again.
- More than two machines (doubles with three or four peers): not run. Only the host asks for a resync; any guest
  that went its own way differs from the host too, so the host sees it, and the state goes to every peer (the ones
  still in sync just run their ticks again). Every guest's `Resynced` counts as a resync on the host; a slower
  guest's hashes from before its resync can land after the first guest's `Resynced` and ask for one more resync
  (harmless: it mends nothing that wasn't, and it's reported again).
