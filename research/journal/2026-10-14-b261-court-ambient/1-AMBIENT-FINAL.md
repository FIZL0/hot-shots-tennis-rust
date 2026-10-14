# B261 iteration 1 — court ambience (FINAL)

## Beach plane (court 2, trigger type 6, model trg_M-02, sound 30)
- Path, trigger and model were already ported (hst-sim `npc.rs`, messages 7/2/4). Checked against the running game's
  RAM (`research/b261_plane.py`, capture in `context/b261/run2/plane.txt`): y = −60, z steps −3.9857666 per tick from
  ≈340; the game's handler has no distance cut-off, the flight only ends at message 4 (post-point reset, ≈3.1 s later,
  z ≈ −406). Same as the port; now pinned by `beach_plane_flies_over` (hst-sim `tests/npc.rs`).
- The bug: the port played the engine with the 3-D falloff (silent beyond 128 m, i.e. most of the flight). The game
  sets the volume straight from the plane's z, (1 − (z/550)²)·128, and re-pans it by bearing every tick. `npcs.rs` now
  does that (`play_toward` + `turn`).
- The remaster renders it when it flies (forced: `context/b261/port/up_*.png`); the game's serve camera doesn't show
  it and my cut-away screenshots of the game missed it.

## Every court's sound-making events
Roster (type(sound)): court 1: 3(18), 2, 4×2, 1(29)×2, 0×2 · court 2: 5×6, 7(5)×2, 8(25)×4, 6(30) · court 3: 11(9),
10(21), 13(15), 12(14) · court 4: 14×4, 15(33), 16(0), 17(10) · court 5: 18, 19(36) · court 6: 22, 23(2), 24(4),
25(7)×2, 26(15) · court 7: 27, 28, 29×2, 30(16) · court 8: 31(34)×2, 32×2, 33(23), 35(13)×2, 36(24)×4, 34(3) ·
court 9: 37(20), 38×5, 39, 40(1), 42(19) · court 10: 43(35)×2, 44(37)×6, 45(5)×2, 47(26), 46(27)×2.

Already ported by P14 (emitters, startle sounds, walkers): 4, 7, 11–17, 19, 21, 23–26, 33–37, 40–46, 50, 51.
New here:
- **3 / 30 / 49 (courts 1, 7 chimes)**: every 14400 match ticks (not at 0), if not already playing, a non-positional
  play at volume 0x40; 30/49 replay as each ends until rung `rings` times, then `rings` goes 1→4 and wraps; type 3
  rings once. `Ambience::chime`, test `chimes_ring_one_to_four_times_in_turn`.
- **8 (court 2 surf, key 25, ×4)**: started once at spawn, never restarted. Volume 43 for instances 0 and 3, 32 for 1
  and 2; bearings from the game's table (`exe::surf_bearings`, [90, 135, 225, 270]); a court-RNG coin at spawn sets
  each sweep's direction; ±1° every 5 ticks, flipping every 90 steps (first after 45). Instance 0 runs a 1200-tick
  cycle: a coin starts a dip on one channel (another draw picks it, never the same one twice running), fading it down
  0.064/tick for 300 ticks, holding 300, back up 300. Draws use the court RNG like the game, not the sim RNG. Test
  `surf_dips_one_loop_at_a_time`. In the app the surf plays from the first match tick for ≈9–10 s (the sequence's
  length) and stays silent after, as in the game's code.
- **10 (court 3, sound 21)**: at the serve set-up's reset of a normal point, when the games played so far are even,
  non-zero and changed since the last one → a positional play at the creature (0x80, falloff); not in replays.

None of these touch the sim RNG or match timing (court RNG / presentation state only).

## Not verified / not 1:1
- Plane never seen on screen in the original's cameras: its path was matched via RAM instead.
- No audio was compared by ear against the game (the plane, surf, chimes, type 10): ported from the code.
- The plane's trigger misses one named-point kind (kind 4: a per-player flag the swing code sets from the two
  characters' stats and the shot/contact) — too deep in the sim's swing code for a presentation task; the plane
  flies a bit less often than in the game.
- Surf/chime restart and counter save/restore around instant replays not ported (the game restarts the surf after a
  replay and saves the chime counters).
- The surf's two debug words (sweep on, no solo channel) are hardcoded to the retail values.
- The surf starts at the first match tick after the intro rather than at court load.
- Chime period and ring counts not checked against a recording (a 14400-tick wait was never recorded).
- Type 10 not seen in the game or the app: an all-bot shot run only reached 4 points in 8 min; ported from the code.
- Pause: the chimes/surf are held by the app's audio pause rather than the game's volume-0.
