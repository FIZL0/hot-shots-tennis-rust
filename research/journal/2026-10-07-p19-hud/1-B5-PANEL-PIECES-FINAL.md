# B5 — serve panel background pieces (FINAL)
- Bug: the Sets/Games strip, the team-banner pill and the name plates are drawn from sub-rects of INPANE sheets
  stretched to another size (strip middle 16×16 → 200×20, banner middle 8×24 → 96×24). Bevy's `ImageNode` kept the
  sub-rect's aspect, so the strip middle showed as a 16-wide grey block behind the digits and the pill pieces were
  misplaced. Fixed by `NodeImageMode::Stretch` on the panel's quads (commit aaf7a30).
- Checked against the original's draw: the panel's draw helper takes (u, v, w, h, x, y, dst w, dst h, sprite) plus a
  10th scale argument on the stack, centred (dst −1 = native size). Every strip/banner call stores 1.0 there, so the
  sizes are the ones passed. The sprite objects normalise UVs by 128×16 (strip, `inpane_sen1`) and 128×64 (banner,
  `inpane_team1`), the sheets' own sizes, so texel rects carry over unchanged. Pill/face/slot/rank draws (the
  per-player helper) also match the port.
- Screenshot: slot 5 serve panel (`research/hud_shots.py 5 240 20`, `context/shots/b5/orig_100.png`) vs `--shot`:
  strip (y 410–430, 0–240 / 400–640), banners, plates, faces, digits line up.
- Test: `play::panel::tests::doubles_pieces` pins each strip and banner piece's sub-rect and screen rect.
- Left over (not geometry): Bevy UI blends in linear light, the GS blends the gamma-encoded values, so translucent
  pieces look stronger in ours (strip at 35 % measures ≈0.45 over the court vs ≈0.3 in the original; plates likewise).
  Needs gamma-space blending for the HUD pass (e.g. render the HUD into its own non-sRGB target and composite).
