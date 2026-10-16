# B535 — receivers idle through the change of ends

## What the port did
I logged every figure's motion over the phase (bot singles and doubles, `change_of_ends_glides*`). The change-ends
placement (`reset_positions`) gives everyone a default motion command: motion 0 (`ad00`), not looping, serial 0.
The figures cut to its frame 0 and hold it. A few ticks in (n = 78/77), the sim's own `locomote` commands the
looping stand and the figure crossfades into it over 8 frames. In doubles, the server's partner gets the turned
stance 0x1 instead. A command that isn't looping can also run out partway through the move. That's the
"ready, then idle" the user saw.

## The original
I took no new capture. The B272 captures of the original (`context/b272/orig3/`, slot 5 bot doubles, counters
about 21 → 0) show the far pair standing and the server bouncing the ball. At 674×379 behind the "Change Sides"
pop-up I can't tell the stand from a ready crouch, or whether either changes motion. The task makes holding idle
the user's call either way, so I didn't record again.

## Port (presentation only)
- `change_ends::idle(g, i)`: during `Phase::ChangeEnds`, every player except the next server (`server`, B262) and a
  player whose emote is still playing out (B278).
- `change_ends::hold_idle`: the figure plays the looping stand. It cuts to it on the phase's first tick (the players
  jump to their new ends) and crossfades if an emote hands back mid-move. It never restarts. Its serial becomes
  `u32::MAX`, a value no command carries. Commands count up from 0 after each placement, so the next point's first
  command takes over just as it did before.
- `motions` (play.rs): a 4-line branch calls it before the generic command path. The sim's commands (`p.cmd`) are
  untouched.
- Test: `change_of_ends_glides`, `…_on_the_players_side` and the new `…_in_doubles` assert on every change-ends
  tick that each non-server figure plays motion 0 looping, with an unchanged `sets` count (no restart). Without the
  fix, all three fail at tick 80 ("plays 0x0 (looping false)"). In doubles the red-marker precondition is skipped,
  because that match's last point needn't have one.
- `HST_NETPLAY=1 … synctest_through_a_change_of_ends --ignored` passes (the figures' motions are in the snapshot).

## Not verified / not 1:1
- What the original's receivers play during the phase: the existing captures are too small to tell (see above). Per
  the task, holding them idle is the user's decision.
- The server's partner in doubles is held idle too, instead of the sim's turned stance 0x1. The task asks for
  "receivers", but the partner's ready→turned crossfade is the same kind of mid-move motion change. Not checked
  against the original, for the same reason.
- `p.played` (the end-of-motion flag, read off the figure) stays false for held players during the phase, where a
  one-shot command could have run out before. Its readers (emote start with a root motion, the follow-through,
  serve and whiff ends) don't apply to a just-placed player, and the synctest/replays pass.
- No `--shot` of the app: the tests check the motions directly.
