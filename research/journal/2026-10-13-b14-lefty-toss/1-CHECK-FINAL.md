# B14 — left-handed serve toss (FINAL)

Filed 2026-10-07 19:01 (commit 1a32e51). Already fixed 55 minutes later by d629148 (1p3goodcpus): `serve::toss_points`
takes the server's matrix (`play.rs player_matrix`, row 0 scaled by `hand`), so the release point and the apex are
mirrored for a lefty, and `held_ball` poses the held ball with the same matrix. The task was never ticked.

Verified:
- `hst-sim/tests/serve.rs match_s05_serves`: hand and apex at every toss bit-exact vs the original, including the
  5 tosses of lefty Carol in `1p3goodcpus` (passes).
- New app test `play/serve_check.rs lefty_toss_mirrored`: Carol serves from slot 3's set-up with hand −1 and +1;
  in the server's own frame the ball at release (across −0.12805 / +0.12805, up −1.5083, ahead 0.1164) and at the
  apex are mirrored across the facing line, same height and depth (|diff| < 2e-6).
- Rendering mirrors the figure with the same convention (`draw`: local x scaled by `hand`).

## Not verified / not 1:1
- No new PCSX2 side-by-side screenshot of a lefty toss: the sim's toss points are already bit-exact against a
  recording of the original lefty, and the app test checks the app uses them; nothing new to capture.
