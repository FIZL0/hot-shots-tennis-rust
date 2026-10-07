# N1f

- [ ] **N1f — Full follow-through after contact (bug, user report 2026-10-06).** In the app the stroke's
  follow-through after hitting the ball doesn't play out in full: the player leaves the swing early. Find what
  cuts it (`play.rs` `motions` / `hst_sim::motion` switching back to stand/run or ready before the clip ends,
  hold flag, motion time) and match the original: compare motion number + time per frame after each contact
  against `match_s05.bin` (player +0x54 motion object) until the stroke→ready hand-off frame is exact.
