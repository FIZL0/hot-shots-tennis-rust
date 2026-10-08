# B37 server's spot after a change of ends — FINAL

## The original keeps the server's stance across a change of ends (live)
`research/b37_stance_probe.py` (slot 5 bot doubles; pokes stances 1.5/2.0/2.5 on players 1..3 at the first rally
and 40-0 for team 0 at every rally of game 1; polls the phase at ~30 Hz). Output (kept in
`context/notes/b37_probe.txt`):

    enter 1 v 10902 phase 1 srv 1 side 0 pts 0 0 games 1 0 x [-3.0, 1.5, 2.7425, -2.7425] stance [4.7666, 1.5, 2.0, 2.5]
    enter 2 v 10983 phase 2 srv 1 side 0 pts 0 0 games 1 0 x [-3.0, 1.5, 2.7425, -2.7425] stance [4.7666, 1.5, 2.0, 2.5]

- The new server (1) stands at its poked stance 1.5, so +0x140c survives the change: only 0x423040 (match start /
  rematch) forces 3.0, as the code read in part 1 said. The port's `stance` (|x| at the toss, 3.0 at match start)
  already does this; nothing to change there.
- Everyone is already at the next serve's spots on the change-ends phase's *entry* (message 0xc, phase 1) and
  stays there through it (phase-2 entry x identical). The port left the players where the point ended for the
  whole 80-tick phase and only placed them at `next_point`, so the server appeared somewhere else after the ends
  changed — the user's report.

## Port
`play.rs change_ends_placement`: on entering `Phase::ChangeEnds`, advance the score and rally on copies (as
`next_point` will) and run `reset_positions` (serve placement, ball in the server's hand, camera cut). `next_point`
places them again at the phase's end: same spots (as the original's singles serve entry; doubles skips it, same
result). Checked in the port with a temporary log, HST_AUTOPLAY singles: entry / last tick / serve set-up
positions all `[[-3.0, 0, 12.25], [3.0, 0, -12.25]]`, server 1. RNG draws unchanged (`placement_draws` still runs
in `next_point` only). `tools/check.sh`: 279 passed.

## Not verified / not 1:1
- The original's placement draw (one shared RNG value per player) at the change-ends entry *and* the serve entry:
  the port keeps its existing draws in `next_point` (P3d3 owns that order); not re-checked here.
- The camera: `reset_positions` cuts (and for a solo human turns) the camera at the phase's entry instead of its
  end; the original's change-ends camera is B39(b)'s, not compared here.
- Not checked against a screenshot of the original's change-ends phase (positions only, via RAM).
- Human servers: the probe used bots; the stance code path (0x3522b0 state 2 → +0x140c) is the same for humans.
