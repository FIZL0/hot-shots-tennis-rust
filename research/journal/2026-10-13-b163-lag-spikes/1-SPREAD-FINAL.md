# B163 — trace, causes, fixes (FINAL)

## Trace tool
`HST_NET_TRACE=file.csv hst net …` writes one row per rollback gate call:
`t_ms,dt_ms,frame,ran,rolled,cost_ms,skip,stall,ahead,ping_ms,behind,gap_ms`
(ran = ticks simulated this call, rolled = ticks re-run, skip = time-sync sit-out, stall = ggrs gave nothing to
run, gap_ms = longest gap between received datagrams). Every 30 s the log also gets one summary line:
`rollback, last 30 s: N frames over 33 ms (longest X ms), longest re-run R ticks, S ticks sat out, T stalls, ping P ms, A ticks ahead`.
Runs: `context/b163/run.sh DIR SECONDS LAG_MS` (host + guest, HST_NET_LAG/MASH/TRACE); results in `context/b163/`.

## What the spikes lined up with
- **ggrs time sync (the cause).** After any hiccup on one peer (a slow frame, a stopped/background process),
  ggrs' `WaitRecommendation` arrives every 60 frames once the peer is ≥3 frames ahead, and we skipped that whole
  lead at once: 3+ ticks in a row with no sim = a 50 ms+ visible freeze, and the lead (6–8 frames) lingered with
  rollbacks up to 7 ticks (run4: guest SIGSTOPped 0.4 s ×3).
- **Flare mesh spam.** In netplay the flare's rays/sprites meshes were rewritten every frame even when empty;
  Bevy's slab allocator logs "Use-after-free … unallocated key" for a 0-vertex mesh, twice a frame (~10k error
  lines per 40 s, synchronous stderr writes).
- Not causes: umpire voice timing (`centre_frames`, 0.3–0.6 ms), rollback depth at 80 ms RTT (≤4 ticks, cheap).
  Most remaining slow frames in run1 (≈180/137 s) clustered at the same instants on both processes with tick cost
  doubled machine-wide: external load from other agents (load avg ~9). Offline single instance: ~3/150 s.

## Fixes
- `rollback.rs`: ignore `WaitRecommendation`; while `frames_ahead() = n > 0`, sit out one tick in every
  `SPREAD / n` gate calls (`SPREAD = 20`, at least every other), never two in a row. run5 (same freezes as run4):
  0 consecutive skips, host 1 frame over 33 ms (35.6 ms) in 100 s; guest only its injected STOP frames + one 38 ms.
- `flare.rs`: an empty batch leaves the (hidden) mesh alone. Errors: ~10k/40 s → 2 at spawn.
- Test `rollback_time_sync_spreads_the_waits` (netplay.rs): guest drops out for 20 rounds, host must never go
  2 rounds without advancing and must settle to ≤1 frame ahead. Mutation (skip on every call while ahead): fails
  with 559 ticks sat out in a row.

## Not verified / not 1:1
- Browser (wasm) two-instance run: not done — unattended, no browser automation here. The trace uses `web_time`
  so it works there; background-tab throttling stalls the peer, recovery is now spread but untested in a browser.
- A full online match to the result: runs were 100–200 s with mashed pads, not a complete match.
- One machine with simulated lag (HST_NET_LAG); no real network, loss, jitter or relay hops (star topology) measured.
- The machine was loaded by other parallel agents, so slow-frame counts are noisy; "no frame over 33 ms" could not be
  shown cleanly — remaining slow frames line up with machine-wide load, not with net events.
- One unexplained 204 ms host freeze in run4 (old code; no net event at that moment, likely load).
- 2 slab-allocator errors still logged once at flare spawn.
- Online time sync is our own (not part of the original game); nothing to match 1:1.
