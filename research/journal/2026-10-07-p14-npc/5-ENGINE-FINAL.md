# P14c1 trigger creature engine — FINAL

## Done
- `hst_data::exe::Game::trigger(ty) -> TriggerRow`: the 0x74-byte type parameter row plus the callback row's sound. Fields used:
  - start/mode (+0, +1), stage (+8), exact (+0xa), reverse roll (+0xc)
  - reset/end waits (+0xe/+0x10, +0x14/+0x18), retarget (+0x1c)
  - speed (+0x20), steer (+0x24), every (+0x28/+0x2c), pause (+0x30/+0x34)
  - snap (+0x38), wake (+0x3a), repeat (+0x3c/+0x40), orient (+0x45)
  - near (+0x48, +0x4c), stages (+0x50), idle (+0x54/+0x58)
  - `Game::deciding_voices()` gives type 44's (pan, gap) table.
- `hst_sim::npc::Trigger` has three entry points: `reset` (motion reset, msg 4), `step` (the generic tick, msg 2) and `callback` (idle animators 34, 37 and 44).
  - Jitter is `mul(n, msub(1, j, 2⁻³²·r))`; with j == 0 it draws nothing.
  - Path modes: 0 random node; 1, 2, 3 and 5 node counters (mode 1 at node 0 does a reset plus the end wait); 6 circles the anchor.
  - The tick also covers: velocity = (to − from)/ticks, the snap countdown, pause/retarget, animation speed scaled by distance (`near`), the idle repeat with a controller advance, and the +200 sound countdown.
- The anchor is the record's own position (the +0x50 of the trigger record), not a path node. It is the start point for start mode 1 and the circle centre for mode 6.
- Callbacks:
  - 34 msg 2: the counter counts down while idle, then restarts the animation; when the animation ends, it re-arms to `300 + 300·u`. Msg 4 does home, on, one counter draw.
  - 37: no draw at speed 0. Otherwise it makes one draw per tick, plays sound 0x28 when counter == 0 and the roll%100+1 < 41, and plays the type sound every 25 ticks.
  - 44: a gap countdown with the voice table plus an idle counter; it is ported from code only.
- Test `trigger_engine_matches_the_game` is bit-exact per tick on 5976 ticks:
  - court 5, type 18: 1991 ticks
  - court 8, type 34: 1981 ticks, 2 draws
  - court 9, type 37: 1998 ticks, 722 draws, 2 loop resets
  - The fixtures are `context/` recordings `trig_c05`, `trig_c08` and `trig_c09` (`tools/record_npc.py … trig`, which now honours HST_PCSX2).

## Read late
Six court-9 samples show two ticks followed by none: wait 60 → 58 → 58 → 57. The game ran one tick, and PINE read the sample after the next tick. The test accepts 0 or 2 steps when 1 does not match, and asserts that this happens in fewer than 1/50 of the ticks. This is probably the same thing as P14c2's open item 2 (court 2 emitter 528 → 526).

## Not ported (`ponytail:` in npc.rs)
- Steering (+0x24 ≠ 0 outside mode 6), the pause turn easing, spline paths (mode 4), area points, non-exact nodes (only `exact` nodes are used), ground snap height, start modes 2–3, row byte +0x6c, the model's own animating flag, the game flag in 37's first call.
- 44 is unrecorded: it only appears in the deciding set (court 10).
