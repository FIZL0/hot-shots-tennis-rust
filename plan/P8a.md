# P8a

- [ ] **P8a — Missed swings (whiffs).** Pressing a shot button when the contact search finds no ball (out of
  reach, too early/late, ball not in play) must swing at nothing exactly as the original: the miss branch of the
  hit routine / contact search (`3467b0`, `0x34d8a0`), which swing animation it plays (forehand/backhand/volley/
  smash variant, by ball side and height), its frame timing, and the movement lockout — the frames the player
  can't move or swing again after whiffing, plus any slowed recovery. Also whether a ball arriving during the
  whiff can still be hit. Verify with recorded whiffs (pad + player position/state over PINE): lockout start/end
  and positions frame-exact through P0's harness.
