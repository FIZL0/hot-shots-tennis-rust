# B200 — presses and silence while an online match loads

## Why the load disconnected

Two causes, both in our code:

1. **Presses under the loading screen reached the match's pause menu.** The match mode's `Update` runs while its
   loading screen is up (game time paused). Start/Esc opened the pause menu there, which is sent to every peer
   (`Msg::Pause`), and ✕ latched `confirm`. Once the loading screen lifted, a mashing player was on an open pause
   menu with ✕/↓ pending: with no point scored, "Return to the Menu" quits at once (no B68 confirm), the match
   mode exits, its `Net` drops, and the other peer gets "The host left" / match over. The same presses latched shot
   and serve presses in `Pads` (taken at the first tick) and could skip the court intro; the old menu also kept
   reading presses in the frames between `launch` and the switch.
2. **The silence limit dropped to 3 s too early.** `recover.rs` used `LOADING` (30 s) only until a peer's *first*
   word. A peer's first keep-alive goes out as soon as its match mode is built, but its loading screen then draws
   frames that can each take seconds (the browser compiles pipelines on the main thread, B110; a CPU-throttled tab
   more so). One frame over 3 s → the host marked it away → "Waiting for …" → after 20 + 15 s back to the lobby.

Keep-alives themselves: natively the network runs on its own thread (QUIC stays up through a blocking build); the
match's own `Msg::Alive` goes out every frame of the match mode, loading screen included.

## Fix

- `mode.rs` `hush` (PreUpdate, after Bevy's input systems, before the mode's PreUpdate): while a switch is pending,
  the loading screen is up, or the mode inserts `mode::Hush`, every just-pressed/just-released of keys, mouse,
  touches and gamepads is cleared. Held buttons stay held, so a button held through the load is not a press until
  it's released and pressed again (key repeats don't re-press a held key). Offline switches get it too.
- `Msg::Loaded(peer)` (net.rs, relayed like every match message): each machine says it once its loading screen is
  down (`netplay::loaded`, from `pump`). `Recover::loaded` holds who has; `netplay::started` = every human seat's
  peer has. Until then the lockstep gate, rollback's gate and the court intro's lockstep run no tick, `mode::Hush`
  stays in (inserted at netplay's setup, removed once started), and a loaded peer shows "Waiting for <name> to
  load...". The match's first tick (the intro's) then runs on every peer within a link's latency.
- Silence limit: `SILENT` (3 s) only for a peer that has loaded; before that `LOADING`, now 60 s since it was last
  heard. A rejoiner (B196) starts with every other peer counted as loaded (they're playing); the others wait for
  the rejoiner's `Loaded` after the restart.
- `mash.rs`'s fake presses run before `hush`, so `HST_MASH` exercises it.

## Verified

- `mode::tests::presses_wait_out_a_switch`: a press with a switch pending or `Hush` in reaches nothing; a key held
  through counts as no press afterwards.
- `netplay::tests::the_match_waits_for_every_peer_to_load`: intro and gate hold, `Loaded` goes out once, `Hush`
  stays until the peer's `Loaded` comes in, then the intro ticks.
- `tools/check.sh -p hst`: 264 passed. Ignored runs: `a_silent_peer_rejoins_and_a_silent_host_ends_it`,
  `full_doubles_match_rollback_at_150ms_stays_in_sync`, `full_doubles_match_at_100ms_stays_in_sync` pass.
  `cargo check --target wasm32-unknown-unknown -p hst` builds.
- Two native processes (`context/b200/run.sh`: host `HST_NET_THROTTLE`, guest `HST_MASH` = Enter/Esc/arrows every
  2 frames from launch on, rollback, relay off; logs `context/b200/run1`, `run2`): the guest loaded 1.75 s before
  the slow host and waited; both skipped the intro on the same tick (4: the masher's first press after the start),
  75 s of match with no disconnect, no "went silent", no pause opened during the load.

## Not verified / not 1:1

- Browser peer, CPU-throttled: not run (unattended, no second browser here). The browser shares the code paths
  (hush, `Loaded`, the 60 s limit), but a blocking build there also blocks the page's network task; if a load
  blocks the main thread longer than the relay/WebRTC keep-alive timeout, the connection itself could still drop.
  Moving the browser's network off the main thread (a worker) is B162's open part.
- The native slow-load case was only throttled frames (`HST_NET_THROTTLE` acts once the match mode exists), not a
  long blocking build; the load here takes ~1-2 s.
- A held d-pad direction (a level, not a press) still counts on the first tick once the match starts.
- The original has no online play: all of this is ours.
