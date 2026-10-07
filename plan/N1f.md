# N1f

- [x] **N1f — Full follow-through after contact (bug, user report 2026-10-06).** In the app the stroke's
  follow-through after hitting the ball doesn't play out in full: the player leaves the swing early. Find what
  cuts it (`play.rs` `motions` / `hst_sim::motion` switching back to stand/run or ready before the clip ends,
  hold flag, motion time) and match the original: compare motion number + time per frame after each contact
  against `match_s05.bin` (player +0x54 motion object) until the stroke→ready hand-off frame is exact.

Done: **Cause:** after contact the app's `human()` still called `locomote` with a zero stick, and locomote sets the stand/run motion, so the stroke was cut right after the hit. The swing state also ended 16 frames after contact. **Original's rule:** a frame counter runs from contact, and the recovery is 30 frames (15 after a slice off a ground stroke or volley). The earliest a stick or press can break off the follow-through is recovery + 2 frames after contact. Otherwise the stroke plays until the frame after its sampled time reaches the clip length, then the player stands. **Port:** `hst_sim::motion::recovery` and `follow_over`; play.rs `follow_through` and the `played_out` system; no locomotion during the swing. **Verification:** `anim_s05_follow_through` against anim_s05. 16 strokes play out on the exact frame, 22 are broken off (18 on the first frame allowed), and every recovery matches the shot. In an all-bot app match the hand-offs come at 32 and 17 frames after contact, the same counters as the fixture. **Ponytail:** the stand-in AI always breaks off at the recovery.
