# B486 — online change of ends: no snap back

**Cause.** Online, every machine counts all the match's humans, so the shared match camera (`Game::cam`) keeps its
end through a change of ends. Its change-ends glide (`change_ends::step`) runs round the player who is at *that* end
after the swap. Through the glide the own camera (B218) stepped aside: `own_camera::plain` saw the shared camera off
its plain view and `shown` returned the shared glide. On the machine whose player had moved to the other end, the
picture glided behind the opponent, with its own player at the far end. When the phase ended it cut back to its own
camera, behind its own player, so the court turned 180° and the players seemed to snap back to the sides they had
just left. The machine whose player ended up at the shared camera's end was fine. That's why it showed on one
client.

**Fix.** Each machine now glides its own camera:
- `change_ends.rs`: the glide's start (scripted shot 2 round the player at the target view's end, with that pivot)
  and its per-tick view are now `start` / `at`. The shared camera uses them as before: same views, same state hash.
- `own_camera.rs`: `plain` also works out `OwnCam::glide`. Its end view is a freshly cut match camera turned for
  this machine's player on the new set-up, and it starts from shot 2 round the player at that end. `shown` shows
  the glide through the change of ends. It's a function of the phase count and the set-up only, so a rollback's
  re-run ticks give the same picture. The own camera is outside the sim, so the sim RNG and match timing are not
  touched.

**Tests.**
- New `own_camera::tests::the_change_of_ends_glides_behind_the_own_player` follows the player the shared glide
  isn't behind. Every tick's eye stays behind that player's new baseline, each tick continues from the last, and
  the serve's first tick continues from the glide's end with no cut. Without the fix it fails on tick 79 with the eye
  at the far end.
- Also passing: `change_ends` (2), `own_camera` (4), `HST_NETPLAY=1 … netplay` (27) and the ignored
  `synctest_through_a_change_of_ends`.

## Not verified / not 1:1
- Not tried in a real two-machine online match: no second peer in an unattended run. The headless test covers the
  machine that was wrong.
- In doubles, the glide pivots on the first player at the machine's end, as the shared glide already does. It's
  not recorded which of the two partners the original's spot 0x76 follows (noted in `change_ends.rs`).
- Presentation only (PLAN.md: cameras match by eye), so it wasn't compared to a PCSX2 capture. A solo game's glide,
  which is already checked against the original (B49/B174), is the same move, now run from each machine's own end.
