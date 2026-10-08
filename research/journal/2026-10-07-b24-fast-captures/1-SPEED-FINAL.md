# B24: faster captures from the original

Result: PCSX2 copies (`HST_PCSX2=N`) now launch at 1x (NominalScalar 1, was 0.25) and the recorders poll as
before. A frame that ticks mid-read is skipped and logged (`missed frame …`). `HST_LOCKSTEP=1` steps the game one
frame at a time while paused, for a capture that must have every frame. `HST_REALTIME=1` launches a copy at 0.25 for
the wall-clock tools (`play_p2m2.py`, `trace_*.py`, which refuse a copy at 1x or faster).

## The sweep

Same scene (slot 5, bot game), each speed against the 0.5 capture (`research/b24/sweep.sh`, `cmp.py`; captures in
`context/b24/`). "gaps" = frames missing from the faster capture; "differ" = common frames with other values.

| recorder | speed | wall | gaps | differ |
|---|---|---|---|---|
| record_live 5 600 (light) | 0.25 / 0.5 | 40 / 20 s | — | — |
| | 1 | 10.3 s | 0 | 0 |
| | 2 / 4 / unlimited | 6.9 / 6.2 / 6.9 s | 0 | 0 / 0 / 1 (7566) |
| record_p2m2 5 1800 (heavy, 8324 B/frame) | 0.5 | 60 s | — | — |
| | 1 (final run) | 30 s | 0 | 1 (7566) |
| | 2 | 17.8 s | 1 | 2 |
| | 4 / unlimited | 18.6 / 18.5 s | 110 / 103 | 10 / 18 |

- The host tops out near 95–100 polled frames/s for p2m2: past that, 2x and up drop frames in bursts.
- Frame 7566 differs because the 0.5 reference itself read it stale. `settle()` saw two identical reads one frame early.
  Only lock-step rules that out; the faster captures have the right value.
- Menus at unlimited ran ~800 fps (user), so wall-clock menu drives (`tools/pick.py`) failed there.

## Lock-step (kept as HST_LOCKSTEP=1)

Pause (pad Guide), FrameAdvance (pad R3, bound by `pcsx2-hst.sh`, taken off the PS2 pad), wait until vsync moves and
status is paused, read. Bit-identical to the reference except 7566 (the reference's stale read).
- 50–85 frames/s at unlimited, but ~10 frames/s at 2x: FrameAdvance follows the speed limiter, and the
  pause/step handshake makes the window useless to watch. The user rejected it as the default.
- Loading a state while paused leaves the vsync counter 2 frames behind the state, and the first FrameAdvance
  does nothing. `load_state` loads while running and pauses after.
- Pad presses in lock-step are counted in frames (`record_aim.py`, `record_human_smash.py`: down, then up 3 samples
  later), so they land on the same frames at any speed.

## Also fixed

- `pcsx2-hst.sh stop` exited 1 under `set -e` when no pad server ran (`nopad`).
- `overnight.sh` seeded copies' save states from the wrong folder (now the user's `SaveStates` setting), so copy 6
  had no slot 5.
