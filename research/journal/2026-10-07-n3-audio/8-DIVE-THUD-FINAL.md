# N3c6: dive thud, footsteps, the rest (FINAL)

## Paths
- Player effects update (`0x3993f0`, once per frame, loops over the players). Per player p it reads the hit event
  block the effects tick copied from fx (`0x3a40c0`: 0x43b1d0+0x758+p = fx+0xd4+p, +0x75c+8p = offset, +0x761+8p =
  branch). Hit flag set and branch 3 (dive): +0xa09c = 1, timers +0xa094 = 1, +0xa098 = 5 (and dust particles if
  player +0x3f80 > 0.5). Then, still in p's iteration, each live timer drops by 1; one reaching 0 plays court bank
  program 3 at the *current* loop player's model origin, volume 0x80, key 3 when court +0x135 (weather) is 2 or 3, else
  key 0. Both timers below 0 clears +0xa09c. So the first thud plays at once and the second after 4 more player
  iterations: 4/P frames later at the same player for P = 1, 2, 4 (`sound::dive_echo`).
- The event is player +0x3f97 (`0x350980` reads it with +0x3ee8 grade, +0x3fa0 offset). `0x34afc0` sets it right
  after the contact search when the branch is not a ground stroke/volley still more than 8 frames out, so a dive
  raises it at its start, missed (offset 999) or not. fx's branch byte is only refreshed for offsets other than
  999/9999; `0x34d8a0` sets +0x3ec1 = 3 for both dive outcomes, so it reads 3 anyway.
- The 0x69 play in the same function is a walking footstep: toe bones (Bip01RToe0/LToe0, +0x34 height) lift below
  −0.08 and land past a per-motion threshold (0x411524 + 12·motion, cooldown 0x411520), but it only sounds with
  gm+0x344 set, player +0x3fa5 == 1, under 3 players and camera mode byte 0x51 in 0x56..0x59, with the player in
  front of the camera (cos ≥ 0.643, then ≥ 0.996 or a projected size > 125). Never in the recordings; left out.
- `0x3553d0(player, 3, 0, 2|1, -1)` at the dive start (2 with fewer than 3 players) picks a random key 0..2|1 other
  than the last (player +0x3b64 slot) and plays it via `0x3554c0`: bank slot = player +0x12b8 (side) + 1 (as is with
  3 players), at +0x3d70, volume 0x80. That is the character voice bank: the dive shout, for N3d. `0x3552c0` (2
  players, +0x3b98 == 0): serve/smash key +0x3b9c % 3 on program 0, other strokes program 1 key % 6, none for dives.

## Proof
`dive_thuds_match_the_game` (hits_s05.bin, doubles): the 2 dives (both misses, p2, samples 364 and 2129) key
program 3 key 0 at 365/366 and 2130/2131 and nowhere else; the triple at 365 is full volume (distance 7).

## Not found
- Server bouncing the ball before the toss: no program 2 (or other court) key-on near any serve in either recording
  beyond those explained; nothing to port.
- Rolling scrape: still never in the recordings (see 6-BOUNCE).
- The two stray program 2 key 0 tones (hits_s05 998, 3127) both come on the dead ball's third bounce (n 2 → 3) after
  the point, from the code path that plays at contact record n−1 (the ball object, or the effects' 0x30 key 0). Their
  L/R give 28° or 152° at 22 m and about 0° at 59–60 m, which is not the ball, the predictor ball or any recorded
  contact record (record 2 is at 133°/14 m and 4°/27 m). Likely the record held another point when it played (the
  recording reads the list after the frame). Not ported: the port keeps bounces after the deciding one silent.

## App
`play.rs` queues both thuds with the swing whooshes (now `(ticks, player, Play)`) when a dive starts; clear weather,
so key 0.
