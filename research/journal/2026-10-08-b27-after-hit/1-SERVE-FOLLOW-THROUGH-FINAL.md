# B27 — movement right after the hit (FINAL)

- **Strokes** were already right: N1f's `follow_over` hand-off is checked by `anim_s05_follow_through`. In match_s05 and
  1p3goodcpus no player moves after a stroke's contact (x/z frozen through the follow-through) except in dives
  (0x1e, N6's `dive_frame`).
- **Serve gap**: after the serve's contact the app moved the server at once (phase Rally → locomote). It kept the
  serve clip on screen with a `time < 30` hack, so the server slid. The swing also kept its pre-contact speed (8/frames).
- **Original** (serve state 1, sub-state 3): a counter +0x3f00 counts frames since contact. Once it is ≥ the recovery
  +0x3e50 and the stick is held (pad direction past the dead square; shot presses are ignored), or the motion's time
  has reached its length, play state goes to 0 (stand/run from the next frame). The position is frozen until then.
  Recovery is set at the hit in the serve branch: 15 for a weak (+0x3ea0 2) or underhand (4) toss or a slice (kind
  bits 2), else 30. After contact the swing clip plays at speed 1.
- **Bots**: the AI's stick stays centred through the follow-through (p7f_s05), so slot-5 serves play out (54/55
  frames, 59 for the underhand 0x26, 61 for character 5). In round1 some bots break off at 47–53, always after the
  receiver has returned the serve (hit count 2): the AI then moves.
- **Port**: `hst_sim::motion::{serve_recovery, serve_over}`; play.rs `Player::served`, `serve_follow` (called from
  `human`/`bot`; holds the server and ignores presses), set at the serve strike. `motions` plays the serve clip on at
  speed 1 (the hack is removed), and the reaction clears it.
  ponytail: the stand-in bot breaks off once the serve is returned and the recovery has passed; the original's
  stick there comes from its rally routine (B27b).
- **Test**: `serve_follow_through` (tests/player.rs). 13 computer serves in anim_s05 play out on the exact frame, and
  23 human break-offs (1p3goodcpus, round1, p7b_*, p7_*, p5) all land on the frame `serve_over` predicts (15 on the
  first frame allowed). The recovery matches every serve, and x/z stay frozen throughout.
- **App** (`HST_AUTOPLAY=1 --play`): hand-offs at 54, 55, 54 and 59 frames after contact, the same counts as the
  original.
