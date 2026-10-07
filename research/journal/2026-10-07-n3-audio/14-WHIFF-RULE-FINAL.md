# N3f: which whiffs shout (FINAL)

## Bug
The port voiced every missed swing; the original mutes some.

## Rule
- The miss handler (`0x350840`, from the stroke countdown `0x349410` and the serve countdown `0x351dd0`) plays the
  miss motion and shouts program 4 unless the replay flag +0x4088 or the re-press flag +0x3f04 is set. No chance roll.
- +0x3f04 = 1 is set by the stroke handler when a press comes while the re-press window +0x3e54 is open
  (+0x3e54 ≥ 0 and ≤ frames since the pose +0x3f00: 2 after a stroke's miss motion, 30 after a smash's). Every other
  press path writes +0x3f04 = 0 (the serve press too).
- +0x3e54 is set (2, or 30 for a smash) whenever the miss handler plays a miss motion, and reset to −1 only by the
  press-swing start (`0x34d8a0`, from `0x34afc0` mode 2) and the serve swing setups (`0x352880`, `0x352c40`). Not by
  the point reset (`0x3449f0`), not by a dive (mode 3), not by the serve's miss path.
- So: of a run of whiffs by one player with no swing start between them, only the first shouts; a hit (stroke press
  that connects) or a serve swing restarts the run; the mute carries over the end of a point for a receiver. A missed
  serve swing always shouts.

## Proof
`shouts_match_the_game` on `hits_s04.bin` (doubles, P1 human pressing ✕): 9 missed swings, 6 shout program 4 on the
player's bank 8–10 frames after the miss; player 0's whiffs at 152, 200 and 249 (no launch between) shout only at 152.
Every whiff after a launch by the same player shouts (736, 1268 after hits at 649, 1179). `hits_s05.bin`: the one
whiff (p3, 3044) shouts.

## App
`play.rs`: player flag `missed`, set with the miss motion in `whiff_frame`, cleared by a taken press and by the serve
swing press; `press` makes the next whiff quiet while it is set. The serve whiff (`serve_turn`, search finds nothing)
now shouts program 4 with its miss motion.
