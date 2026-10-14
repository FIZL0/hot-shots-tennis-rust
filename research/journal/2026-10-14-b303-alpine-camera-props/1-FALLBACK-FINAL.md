# 1 — near-lens fallback (FINAL)

## Search
- Logged the drawn camera and first-hit props (ray grid over the view) through 15-min bot singles and doubles
  matches on court 6: only far birches/cedars, no blocked view. Bots never end a point off court, so this matches
  the user's diagnosis (a human runs off court, the cut-away follows the player into the props behind it).
- Alpine's knock-over props (`k-pblu-table`, `k-miniguard`: categories 17–20, B240-tested, with collision meshes;
  `k-horse`/`k-horse2`: no collision mesh, so B240's `buried` can never hide them) sit behind and beside the court.
- The original's cut-away has no obstacle check: its shot frames (`hst_sim::cutaway`) only look at the players'
  bones and the court corner, and the court-object pass has nothing beyond the B240 eye-buried sphere test and
  VU1's guard-band reject (already ported: `gs.wgsl` `GS_GUARD`). So the original shows these props in front of
  the lens too, apart from triangles reaching outside the guard band.
- PCSX2 copy 4: saved slot 9 at Alpine Heights doubles (P1 human serving). Driving P1 off court to end a point
  there didn't work out in the time box (points ended on aces, the cut-away showed the COM partner by the net).

## Fix (B240's fallback, `play/net_hide.rs` `in_front`, `main.rs` `NearBox`)
Every tree/prop plant (not structures, billboards or the net) carries its game-space box. While a view's lens line
out to 3r (r as `buried`: tan 20° / tan half-fov, 1..15) passes within r/2 of the box, the model is hidden. Marked
`ponytail:` as not the original's rule. Presentation only: no sim state touched.

## Shots (`context/b303/fix/`, before | after, HST_EYE + `--play`, court 6)
- `grid2.jpg` G / `grid.jpg` B: a cut-away-like eye by the net post side, a grass clump and birch trunk 1–2 m
  ahead fill a third of the view → hidden.
- A/C/E/F: lattice fences and tables 4+ m ahead stay drawn (they don't fill the view).
- Match camera, `--play` stages 6, 8, 1, 4 with and without the fallback: ≤0.11 % of pixels differ (animation).

## Not verified / not 1:1
- Not compared with the original at the same moment: getting the human off court at the end of a point in PCSX2
  wasn't reached (see above); the decompile shows no obstacle handling, so the original likely shows the props.
- The fallback is our rule, not the original's: a prop the original would draw just in front of the cut-away lens
  is hidden here (by design, the task's fallback).
- Box test sampled at 13 points along the lens line; a box thinner than ~0.25·r between samples and > r/2 off the
  samples could be missed (r/2 margin makes that practically impossible for real props).
- The views were set with HST_EYE at spots next to the props, not from a recorded human off-court point.
