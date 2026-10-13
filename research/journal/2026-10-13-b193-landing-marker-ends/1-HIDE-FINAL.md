# B193: the ball's landing marker during a change of ends (FINAL)

## Which indicator
- The court bounce marks (`BallBounce` marks) are not the bug. B51 showed that the original keeps them through point
  ends and changes of ends. Only their aging stops at message 0x19.
- The bug is the red landing marker `chakudan_p`, and the yellow `smash_p` the same way. The port kept the last
  shot's aim (`g.marks.red`) on screen until `next_point`, so it was still up through the whole change of ends.

## What the original does
- The marker manager's message handler (decompile; vtable entry 8 of the object whose constructor loads
  `taguchi/Other/chakudan_p`) clears the red marker's show byte (+0x1a0) and the yellow one's (+0x5c) on several
  messages:
  - 0x19: the players react to the point, about 33 ticks into the point-over;
  - 0x14: the next strike;
  - 0x1a;
  - 0xe: the serve set-up;
  - 0xc and 6.
- Its draw only draws the red marker while +0x1a0 is set, and never during a replay (gm+0x344).
- So both markers are gone from the reaction on: through the rest of the point-over and the change of ends.
- Screenshot of the original, slot 5, change-ends tick 75 (`research/b73_hold_shot.py 5 75`,
  `context/b193/run1/t75.png`): the four COM tags are up and there is no red marker on the court.
- `research/b193_post_shot.py` takes a shot N frames before the change of ends starts. At 40 frames
  (`pre40.png`) the screen shows the "Game" cut-away, so the court isn't in view.

## Port
- `play.rs`: a new `landing_shown()` hides both markers' drawing once `post.reacted` is set and in
  `Phase::ChangeEnds`. `g.marks` itself is still cleared at `next_point`, because the singles AI reads
  `g.marks.red`, so the sim is untouched.
- Test: the `change_ends.rs` glide (both variants) asserts `!landing_shown` on every change-ends tick. It also asserts
  that the point before the change left a red marker (`g.marks.red`), so there was one to hide.

## Not verified / not 1:1
- No app screenshot at the change of ends. The `--shot-at 47` run (the same args as B73's) landed on a serve with no
  court loaded (`context/b193/app_t47.png`), and I didn't hunt for the right args. The drawing path is direct:
  `red_at`/`smash_at` = None → `Visibility::Hidden`.
- The tick the marker goes was taken from the decompile (0x19 = the port's `reacted`). It wasn't compared against a
  live RAM trace of +0x1a0.
- The original's other clears (0x1a, 0xc, 6) and the replay hide (gm+0x344) weren't checked here: the port's
  phases don't send those messages during a change of ends.
