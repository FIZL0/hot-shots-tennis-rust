# 11 — Match-event creatures (P14c5) — FINAL

Ported from the game's code without recordings: they are in-match presentation (PLAN, user 2026-10-12: feel
right, don't touch the sim RNG or match timing). The only draw among them is the game-end roll of types 19/33
(one each, whether it arms or not), which is kept. `hst_sim::npc::Trigger::message` takes the match's messages;
`crates/hst-sim/tests/npc.rs::match_event_creatures` checks each type's behaviour with the disc's rows.

## The messages

The trigger manager turns the match's messages into callback messages: point decided (match 0x17) → 6; the
scoreboard's pause over (0x19) → 7; a game's end (0xc, entering the game-over state) → 8 then a motion reset
(4). In the app (`play/npcs.rs`): 6 at the tick the umpire turns, 7 when `PostPoint::reacted` rises with an
event, 8 just before the new point's resets when the decided point ended a game or set.

## Per type (callback table in RAM at the creature type × 0x18; sounds from its second word)

- **2** (court 1): each tick it keeps the first bounce's spot (if it landed in the court with no body hit; stored
  when the bounce count turns 1). msg 6 with a game/set won (short of the match), a named point (point kind ≠ 0)
  and a plain call: shown there from frame 1, turned about Y to face the camera's eye (re-faced each tick while
  on). Reset: off, home.
- **6** (court 2): msg 7, same conditions: on, sound 30 at it (handle kept), volume
  `(1 − powf(1 − (1 − |z|/550), 2))·128` (0 past |z| 550). Tick while on: pitch and yaw turned fully to the
  reset's leg velocity's heading, matrix rot_x(pitch)·rot_y(yaw), position + velocity with y forced to −60; the
  sound moves with it and its volume is recomputed. Reset: off, home, sound stopped (not in a replay).
- **19, 33** (courts 5, 8; one callback): msg 8 draws once; bit 16 clear and not done → armed. Reset: if armed, 33
  plays sound 23 at itself (volume 0x80) and is done for the match; 19 starts a sweep: 360 ticks, every 10th a
  call of sound 36 (bearing, volume), the volume easing in to 128 at tick 180 (`1 − powf(1 − c/180, 4)`) and back
  out, the bearing from 135° down by 0.5 a tick (45° at 180, wrapping to 359); then done (row not rearmed: gone
  for the match).
- **22** (court 6): reset: on only in the deciding set (tiebreak flag and set index + 1 = 2·sets − 1), the
  gallery's manager paused while on; msg 8 re-sends the pause. Its tick turns its head to the ball.
- **44** (court 10): msg 8 pauses the manager in the deciding set, resumes it otherwise (its reset and tick were
  ported in P14c3).
- **40** (court 9): reset: on when one team has a human and the other not, remembering `team` = 0 if team 0 has
  the human, else 1. msg 7: cheers (sound 1 at itself) when a game/set was won, by team `team != 1` (the team
  **without** the human) and in under three shots. Ghidra's compare was checked in the disassembly (the stored
  team is `!human[0]`, the test `winner == (team ^ 1 != 0)`), so it is taken as written, although the survey
  called it the human team's cheer.
- **9**: on no court's roster (exe `npc_roster`): not ported.

The app now spawns 19 and 40 although they have no model (placeholder entity), so 19's roll happens.

## Not verified / not 1:1

- No recording of any of these: no fixture wins a game on courts 1, 2, 5, 6, 8, 9 or 10 with a named point,
  nor reaches a deciding set. Checked by code reading and the unit test only (presentation, per PLAN).
- Point kind (named point): the port names an ace or return winner (shots 1–2 won by the last hitter's team) and
  a smash winner; the game's fourth kind (passing / diving winners) is not told apart: those points aren't named.
- Message 8's timing: sent at the new point's resets instead of when the game-over state starts (B24c's state 4
  reset timing). 33's sound and 19's sweep therefore start at the next serve rather than at the game's end.
- 19's call: the sound call's arguments are read as (bearing, volume) from their ranges; not confirmed.
- 22's head following the ball is not drawn (render only); 22 has no other visible effect.
- 6's flyover sound: volume follows the game's formula, but `play_at` adds the mixer's own distance falloff on
  top (the game's positional call may not); the replay-mode exception for stopping it isn't modelled (no replays
  of it).
- Type 2/6 animations: their clips (2 has one, 6 none) are driven by the generic controller; not compared.
- Type 9: on no court, not ported.
