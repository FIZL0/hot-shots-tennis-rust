# 1 — tab-out: the own camera flipped to the other end; held ticks counted as run

## Repro (Chromium, `tools/web_lan.mjs`, two browser windows on one machine, singles over WebRTC LAN)

`WEB_HIDE=ms[:host[:frozen]]` (new in `web_lan.mjs`): minimises the guest's (or host's) window for `ms` mid-match
(`Browser.setWindowBounds` minimized: `visibilityState` hidden, rAF stops, network and timers go on, as a real
tab-out), or with `frozen` uses `Page.setWebLifecycleState` (timers and network stop too). The script's menu path
was also brought up to the lobby-first flow (B195).

A temporary per-frame probe (removed again) logged real delta, ticks run and the overstep fraction:

- minimised 1.5 s (run7) and 3 s (run10): the first frame back is 1.5 s / 3.1 s long and runs ~97 / ~180 ticks in
  one go (Bevy's fixed loop with `CATCH_UP` 2 s, 1–2 rollback ticks per fixed call); after it, 60 fps, one tick a
  frame, the overstep fraction continuous. The host stalls (waits on the guest's pads at the prediction window) and
  both hashes agree at tick 1800.
- frozen 1.5 s (run5): the page's network is frozen too, so the guest came back with a 10 s ping and both ended on
  "Waiting for … to come back": that's the emulation stopping the data channel, not a tab-out (browsers only
  freeze a background tab after minutes; B162 covers peers that go silent).
- control without a hide (run6): clean.

The camera reads only ticks (the match camera, `own_camera`, interpolation by `overstep_fraction`), never the frame
delta, so the long frame doesn't re-ease it: a catch-up is a jump forward, as it has to be to stay in step with
the peer.

## The glitch: the picture flipped to the shared camera

Online each machine shows its own copy of the match camera (B218, `own_camera.rs`), stepped only on new ticks, and
`shown()` used it while the shared camera's view equalled what the own camera expected it to be *at render time*.
A rollback that re-runs ticks without running a new one leaves the shared camera on corrected scenes the own camera
never stepped, so for that frame the picture showed the shared camera, which for a guest looks from the other end
of the court: a one-frame flip. ggrs 0.13 does exactly that at the prediction window (the case after a tab-out:
`advance_frame` returns the rollback's load and re-runs and no `AdvanceFrame`, never `Err(PredictionThreshold)`).

Fixes:

- `own_camera.rs`: whether the shared camera shows the plain view is decided once, after the newest tick
  (`plain`, set by a system after `simulate` when `step` stepped), not compared per frame.
- `rollback.rs` `advance()`: an `Ok` with no new tick is counted as held and returns false, so the gate keeps that
  tick owed (before, the owed tick was dropped, as if it had run).

## Check

`rollback_survives_a_tab_out` (netplay.rs, the B162 harness, guest away 5 s) now has the own camera plugin and
records per round whether the guest's picture is its own camera's; it asserts no run of the shared camera of 10
rounds or fewer (cut-aways and replays last seconds). With the old comparison it fails with flips of `[1, 66, 125]`
rounds (the 1-round one is the glitch); with the fix it passes. Browser re-check (run10, 3 s minimise): in sync at
tick 1800, the guest's picture stays on its own end after coming back.

## Not verified / not 1:1

- Zen (Gecko) not tried: no way to drive it unattended here (no CDP for Zen/Firefox in `web_lan.mjs`), and B298
  has Gecko's WebGPU on Windows rendering the match wrong anyway; the Chromium repro is the same rAF stop.
- Online doubles not run in the browser: the two-window script plays singles; the camera fix is per machine and
  doesn't depend on the player count.
- The visual flip wasn't caught on screen in the browser: the scripted match has idle humans (no rally, so no
  rollback corrections change the scene); it's shown by the headless test.
- A 0-tick fixed call (time sync sat out) still restarts the overstep fraction between the same two ticks (a
  sub-tick back-step of the interpolation, ~1 in 100 ticks); left, it's the B163 time sync and not tab-out specific.
- Presentation only: nothing here touches the sim, RNG or match timing (the held-tick change only delays a tick
  that would have been dropped; the replay hashes agree).
