# N3c4: bounce sounds (FINAL)

## Paths
- Ball object update (`0x38f510`, end): when the live ball's bounce count n (+0x224) differs from obj+0xe60 and a
  contact list exists (+0x8f8), with 1 <= n <= 3 and obj+0xd09 == 0: key = 0 (0xd under menu option 0x2ef7e2 == 1).
  With obj+0xe64 == 0 it plays at the ball (+0xe0); after message 0x12 (rally phase enter) e64 = 1 and it plays at
  contact record n-1 (+0x10), only if that material's court byte (row +0) is set. A first bounce on a court
  material after a smash (hitter branch fx+0xdd == 4, more than one player, ball speed `0x375cb0` >= 85 km/h)
  sets the play's speed to 0.5 (`0x1a04d0(0.5)`). km/h = |vel (+0x140)| * 60 * 3600 / 1000.
- Ball effects (`0x392400`), when n changes (tracker +0x720, set even if blocked), n < 6, gate +0x73c (message
  0x12), cooldown +0x840 < 1, effect count +0x844 < 10: row = table[material]. +0x849 cleared on a court material.
  Unless (+0x849 and kind == 2) or kind == 5: kind != 0 spawns an effect (cooldown 4, count + 1); with +0x848 set,
  plays key (row +0xd) if nonzero, and key 0 for materials 0x30/0x17/0xd; then +0x849 = (kind == 2). The cooldown
  drops by 1 every frame. Rolling: ball +0x264 != 0 and |vel| >= 0.08 plays key 3 at the ball (never in the
  recordings). The count drops when a type-2 effect expires, so the cap is not reached in play.
- Messages 0x19 (players react, phase 4) and 0x1a (phase 5 enter) clear +0x848; 0xe/0xc set it. Material table
  rows: 1/0xc/0x22 court, key 0; 2 net kind 2 key 2; 0xd kind 4; 0x30 kind 7 (the ground beyond the court).

## Proof
`bounce_sounds_match_the_game` (hits_s05.bin): `sound::Bounces` from the recorded bounce count, contact material
and point gives 23 plays (court bounces n 1..2/3, two 0x30 ground bounces), all keyed bit-exact (L/R from bearing,
falloff volume) this frame or the next. The double bounce that decides the point still sounds; every bounce after
it (7: n 3..5 on 0x30, 0xd, court) is silent until the next serve. That is earlier than message 0x19 (30 ticks of
scoreboard pause); the silencing source was not found, the test models it as "after the n >= 2 bounce". 25 program 2
key 0 tones keyed in all: the 23 plus two not at the ball (frame 998 at 28°/22 m, 3127 with a 128 m triple), both
shortly after a point (N3c6). No net touch and no smash bounce at 85 km/h happen in the recording.

## App
`Flight::landing` keeps the counted bounce's surface point and material (`Material` now carries id, effect kind and
sound key from `exe::Surface`). `simulate` feeds `Game::bounces` each frame with new bounces while `Phase::Rally`
(the deciding bounce plays; the point-over phase is silent), and the ball speed when the last shot was a smash.
