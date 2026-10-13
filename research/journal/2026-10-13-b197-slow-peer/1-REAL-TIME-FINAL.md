# B197: the online match clock runs in real time on every peer

## Why the others slowed down
- The rollback gate runs once per Bevy FixedUpdate step (`mode.rs` `Gate`). If a call was held (at the prediction
  window) or sat out (time sync), that tick was lost for good.
- B163's time sync could only make the peer that was ahead sit out ticks. It was a one-way ratchet: every peer
  drifted down to the slowest peer's clock.
- A prediction window of 8 ticks stalled the others whenever a peer hitched for more than ~130 ms.
- `Time<Virtual>` max_delta of 250 ms threw away a hitching peer's own time, so it never caught up.

## Fix (`play/rollback.rs`)
- `owed`: the ticks real time has asked for. At real speed the gate runs up to 2 ticks per call to pay them back,
  so a slow peer (20 fps → 3 calls of FixedUpdate per frame anyway, and a hitch → a burst) catches up instead of
  dropping time. `max_delta` is set to 2 s (`CATCH_UP`) while the speed is 1, so a 1 s hitch is fully owed. Replay
  skip and slow motion (relative speed ≠ 1) keep 1 tick per call, as before.
- `shift`: time sync now moves this peer's clock off real time by at most ±`SHIFT` (40) ticks. It moves one tick
  every `SPREAD` (20) calls toward the others' middle, in both directions. Both sides of a gap close it, and a
  peer that can't keep up slows the others by 5% at most and never by more than `SHIFT`.
- `WINDOW` 8 → 80: rollback predicts through a hitch of up to ~1.3 s instead of stalling. ggrs 0.13 queues 128
  inputs, so 80 is safely inside its limits. Synctest now sets the prediction window above its check distance.
- `advance()` is the old tail of the gate, split out so one call can run 0–2 ticks.
- `HST_NET_THROTTLE="FPS HITCH_MS"` (native, test knob): caps this process's frame rate and adds a hitch every
  3 s.
- `hst-sim/src/effect.rs`: a swing trail's rise/fall is now at least 1. A 60-tick synctest rolled a trail past its
  swing's length, and the game's code divides by zero there. The original never rolls back, so this changes
  nothing for stock play.

## Tests (`play/netplay.rs`)
- `a_slow_peer_leaves_the_other_at_full_speed` and `a_slow_peer_leaves_three_others_at_full_speed`: 2 and 4 peers
  over a lagged in-memory link. One peer gets 3 gate calls every third round, 1 s with no calls every 3 s, then a
  burst of 60.
  - Every peer stays within `SHIFT + 3` ticks of its own real time; the slow peer also gets +60 for the hitch.
  - Fast peers never go two rounds without a tick.
  - Pad logs agree, and there is no desync or end.
- `rollback_time_sync_spreads_the_waits`: the final check is now `frames_ahead() <= 2`. Its frozen guest loses
  its 20 ticks for good (that harness doesn't owe time), and time sync now evens the gap out rather than the
  host waiting all of it out.
- A temporary 6000-tick run of the 2-peer case went through 31 instant replays with no desync (since removed).
- A 60-tick-distance synctest ran 5280 ticks with no mismatch.
- `tools/check.sh -p hst --bin hst -- netplay rollback recover resync`: 19 passed. The ignored
  `a_silent_peer_rejoins_and_a_silent_host_ends_it` passed.

## Live (two release `hst net` processes, relay off, HST_NET_LAG=30, mashed pads, guest throttled)
| run | guest | host ticks/s | guest ticks/s | stalls ≥250 ms | desync |
|---|---|---|---|---|---|
| run2 | – | 60 | 60 | 0 | – |
| run3 | 20 fps | 60 | 60 | 0 | – |
| run1 | 20 fps + 1 s hitch / 3 s | 59.8 | 60 | 0 | 1 (tick 2520, after an instant replay, resynced by B154b) |
| run4, run5 | 20 fps + 1 s hitch / 3 s | 60 | 60 | 0 | – |
| run6 (final SPREAD) | 20 fps + 1 s hitch / 3 s | 60.3 | 60.3 | 0 | – |

Scripts and traces are in `context/b197/` (`run.sh`, `rate.py`).

## Not verified / not 1:1
- run1's single desync: the cause wasn't found. The host's dump is `context/b197/run1/desync-*-host.json`. It
  came right after an instant replay and the B154b resync recovered it. It didn't recur in runs 4–6 or in the
  6000-tick harness run. `replays_a_desync` differs from the live run from tick 0, so it couldn't be compared.
- When a hitching peer comes back, the fast peer re-simulates ~50 ticks in one frame. Its frame takes 50–80 ms,
  up to 120–200 ms during an instant replay. The run is still at full speed with no stall, but there's a visible
  stutter. That's the worst case with mashed pads; real held pads mostly match the prediction.
- A peer that truly can't run 60 ticks a second can't be left behind. Input-sync rollback needs its pads, so once
  `SHIFT` is spent, the others wait on it at `WINDOW`.
- No "connection poor" notice was added. Lockstep mode is untouched.
- 4 peers were tested only in the in-memory harness, not live. Everything ran on one machine with simulated lag.
- There's no original to compare against: online play is ours.
