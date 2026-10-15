# B402 — preview stand-ins' proportions (2026-10-15)

**Cause.** Menu rects are in 640×448 units shown on a 4:3 screen, so a unit is 14/15 as wide as it is tall on screen.
- The live 3D card drew into a target of `rect × unit_px` on both axes: 15/14 too wide for its node, then squeezed
  into it, so on the select (whose rect is a true on-screen 4:5, `0.8 * PX / PY`) the live model came out 7% thinner
  than the baked 4:5 PNG. That was the swap you could see.
- The confirm stage, the lobby seat cards and the mods screen's preview pictures used `h * 0.8` in menu units (0.747 on
  screen). The PNG was stretched into that shape (and the live model squeezed by the same amount), so both were 7% too
  thin there.

**Fix.** `previews::UNIT_W` (14/15) and `CARD_W` (0.8 / UNIT_W). The live target is now the node's on-screen
pixels (`w * UNIT_W`). Confirm, lobby, mods grid detail and mods screen detail use `CARD_W`, so they're a true 4:5 on
screen, and the PNG is drawn unstretched.

**Check.** `--shot` with `HST_PREVIEW=png` and then `live` (`HST_MENU=chars`, `confirm`, `lobby-bot`; shots in
`context/B402/run1` (before) and `run2` (after)). A silhouette fit gives the horizontal scale the live model needs
to match the PNG. On the select it was 1.07 before and is 1.00 now. On confirm it was 1.00 before (both squashed)
and is 1.00 now, at the true 4:5. The lobby matches by eye.

## Not verified / not 1:1
- The mods screens' `Tex::Shelf` pictures were changed with the same arithmetic but weren't screenshotted. They need
  a store mod with a preview, which a `--shot` setup can't produce here.
- I didn't compare against the original's inspect screen. This is front-end presentation, and the proportions follow
  from the camera's own `FOCAL` ratio (15/14, the same 640×448→4:3 pixel aspect).
