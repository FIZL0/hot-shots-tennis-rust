# B42e — court intro in online matches

Task: the court intro (B42) was off online because a ✕ skip has to reach every peer. Play it in lockstep, skip
when either peer presses ✕.

## What changed

- `net.rs`: `Msg::Intro { tick, seats }`, the `Pads` payload for the intro's ticks (a match message, relayed).
- `netplay.rs`: `Netplay.intro`, a second `Lockstep` (the match's seats and input delay) that `intro.rs` starts;
  `intro_tick(w, skip)` runs the intro's next tick once every seat's pad for it is in and says whether any seat
  pressed ✕ on it. While it runs, "Waiting for NP..." names the seats the intro waits for.
- `intro.rs`: online, each 60 Hz intro tick goes through `intro_tick` (a local ✕ is latched until a tick takes it),
  so the flyover, the skip and the end land on the same intro tick on every machine; the match's lockstep / rollback
  starts after it as before (the intro never touches the sim). Online the intro ignores `HST_INTRO`/`--shot` (a
  peer without it would leave the other waiting); it still needs a window.
- `mode.rs`: forwards `RunFixedMainLoop` to the running mode. The intro's `hold` runs there, and wasn't run in any
  match started through a mode (menu or online), so there the intro never ticked: the flyover's first frame and the
  hidden HUD stayed up while the match ran under it. Fixes menu-started offline matches too.

## Verified

- `netplay::tests::intro_skip_reaches_both`: two machines, a 3-tick link, delay 4: the guest's ✕ on its tick 10
  skips on both at tick 14; neither runs more than the delay ahead of the other.
- Two real processes (`hst net host 1 --games 1 --relay off --play …` + `hst net join <code> 1 …`,
  `HST_NET_LAG=100`, `HST_NET_MASH`), logs in `context/b42e/`:
  - run4 (lockstep, guest presses Enter at 4 s): both logged "court intro skipped at its tick 209" (25 ms apart),
    both started match tick 0 1.5 s later (the 92 skipped ticks), hashes agree at ticks 0, 1800, 3600.
  - run5 (rollback, no press): the full intro played on both ("Start the Match!" banner over the match camera in
    the guest's shot), then the match, no desync or "stopped answering".
- `tools/check.sh -p hst`: 137 passed.

## Not verified / not 1:1

- The original has no online play: the lockstep intro is an addition. The intro itself is B42's.
- A peer whose intro is off (no window) never sends intro ticks, so a windowed peer would wait in its intro
  forever. Every real peer has a window; headless peers only exist in the tests (both off there).
- The menu-started offline match's intro (now ticking through `mode.rs`) wasn't run from the menu; the online
  runs go through the same mode path.
- Pausing during the intro online stalls the other machine's intro after the input delay (the pause screen itself
  opens on both, as in the match).
