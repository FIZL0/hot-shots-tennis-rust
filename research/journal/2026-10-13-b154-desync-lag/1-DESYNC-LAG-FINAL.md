# B154 — online desync and lag

## What changed
- **Change Sides desync** (ede68b5): `change_ends::Swing`, `replay_pace::Pace` and `bodyhit::Posed` were outside the
  rollback snapshot, so a re-run across the Change Sides screen diverged. They are snapshotted now;
  `synctest_through_a_change_of_ends` (7-tick re-runs) fails without them and passes with them.
- **Native vs browser**: the hashed `Player::facing` came from std `atan2`, and `replay_pace` used std `powf`. glibc
  and wasm's Rust libm can differ by an ulp, which is enough to split a native↔browser match. Both now use
  `hst_sim::musl`. An audit of the remaining std trig in `play.rs` found it is drawing-only (not hashed).
- **(1) Desync report on every platform**: `netplay::save_desync` writes the file natively. In the browser it calls
  `web::save_file` → `window.hstSaveFile` (web/index.html), which writes OPFS `reports/<name>`, downloads it, and
  also prints the JSON to the console.
- **(4) Play on after a desync**: `netplay::desync` logs a warning on every mismatch. On the first one it writes the
  `desync-*.json` report and puts a 6 s on-screen note ("Out of sync at tick N …: playing on. Inputs saved to …"),
  and the match continues (lockstep and rollback). Tests: `a_desync_is_saved_and_play_goes_on`,
  `rollback_desync_is_saved_and_play_goes_on`. Real resync by state transfer → **B154b** (plan/TODO.md).
- **(3) Lag stats**: `rollback::stats` logs every 10 s of real time:
  `rollback at tick N: 2P 114 ms round trip, this machine k ticks behind it; last T ticks: R re-run, S sat out, H held (delay 2, window 8)`.

## Two-instance run (`two.sh context/b154/run1 50 <secs> --games 2`)
Two native release processes, host and guest both mashing, `HST_NET_LAG=50` (~110–120 ms round trip), relay off.
- The full match finished (1–3 games, `match_over: true`) on both sides at tick 8215, with two changes of ends
  (`swapped` true after game 1 and false after game 3), no "Out of sync" and no desync file.
- Per 600 ticks: host 136–340 ticks re-run, guest ~55–70. **0 sat out, 0 held** throughout. Frame advantage was −1…4.
- Logs: `context/b154/run1/{host,guest}.log`.

## Tuning decision
Kept `DELAY = 2`, `WINDOW = 8`. At ~115 ms RTT (~3.5 ticks each way) prediction never hit the window (0 held) and
ggrs never asked a side to wait (0 sat out), so the input feel is the 2-tick delay (33 ms) plus re-runs, which are
cheap. Raising the delay would add constant lag for everyone. Shrinking it to 1 would double the re-runs for a 16 ms
gain. Window 8 covers about 260 ms RTT before the game holds. If players report lag on worse links, the stats line
now shows whether it comes from holds (raise the window) or sat-out ticks (clock drift).

## Not verified / not 1:1
- **Browser two-instance run**: not done. Building and serving the wasm build plus driving a browser peer
  unattended is heavy. The native↔wasm divergence found by audit (std atan2/powf in hashed state) is fixed, but no
  native↔browser match has been played to the end. Same for `hstSaveFile` (OPFS + download): it compiles for
  wasm32 but has not been exercised in a browser.
- **Real desync recovery** (host snapshot → peers resume): not implemented. The game state is not serializable
  (`snapshot::save` holds closures; hundreds of types), so it is split to B154b. For now the match plays on
  diverged, with the report and note.
- **Lag on real networks**: only measured with simulated 50 ms lag on one machine; nothing tested over the internet or
  a relay.
- No 1:1 question: online play is a remaster addition.
- Harmless: the stats line keeps logging after the match ends, and bevy_render prints slab_allocator
  "Use-after-free" spam when the process is killed by `timeout`.
