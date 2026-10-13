# B67 — modded characters' faces in the match panel

**State: FINAL.** Builds on B133/B160/B176 (`../2026-10-12-b133-mod-icons/`, `../2026-10-13-b160-face-icons/`,
`../2026-10-13-b176-face-framing/`).

## What was already there
B133/B160/B176 already render every player's face from its own 3D model and costume, mods included: the select
preview bake (`play/main_menu/previews.rs`) also draws the head square as a 320² close-up, and `play/panel.rs` puts
it in the `Art` face slot. No PNGs ship with a mod. What was missing: the panel faces used the select grid's framing
(head and shoulders, hands), not INPANE's, which B176 had only tuned by eye.

## Measured against the disc
- `face_CC_OO.tm2`: a 40×40 head in the top left of 64×64, transparent background, alpha = the head's silhouette;
  it frames hair top to chin.
- `fit.py` (in this folder; run from the repo root with the disc faces in `context/b67/tm/`): for each of the 14
  disc characters' costume 0, searches every square (side, x, y) of our baked `dNN-0-face.png` close-up for the
  best grey-level correlation with the disc face over its opaque pixels. Results (fractions of the close-up):

  | c | ncc | side | x | y |
  |---|---|---|---|---|
  | 00 | .52 | .59 | .26 | .01 |
  | 01 | .59 | .70 | .08 | .03 |
  | 02 | .86 | .70 | .18 | .09 |
  | 03 | .68 | .71 | .13 | .13 |
  | 05 | .65 | .61 | .28 | .11 |
  | 06 | .53 | .70 | .15 | .08 |
  | 07 | .68 | .66 | .21 | .09 |
  | 08 | .54 | .84 | .06 | .00 |
  | 09 | .77 | .68 | .20 | .08 |
  | 10 | .62 | .75 | .13 | .09 |
  | 11 | .69 | .64 | .10 | .09 |
  | 04 / 12 / 13 | — | locked onto the shoulder / weak (.44, .42) | | |

  Median: side 0.68, top 0.085, x ≈ centred (0.16). That fixed crop is `inspect::PANEL_CROP`, applied to the panel's
  icons only (`inner` < 64); the select grid keeps B176's framing.
- Contact sheet, disc (top) vs ours cut round (bottom), all 14: `context/b67/fit.png`. Heads line up in size and
  position on all of them, pc00 included.

## Checked
- `tools/shot.sh context/b67/match.png --play --mod mods/hst-mods/fore_pc03_sam`: the panel shows the mod's own
  (Fore Sam) face and the disc players' rendered faces at the new framing.
- Test `inspect::tests::face_icon_finds_the_head` (extended for the crop).

## Not verified / not 1:1
- Angle: the disc faces look a little to the viewer's left; ours use the select preview's pose and camera (one
  render shared with the select cards). A separate head-on-pose render per player would be needed; menu/presentation
  art only has to look right (PLAN.md), so not done.
- Alpha: ours is a round cut of the render; the disc's is the head's own silhouette (hair tips stick out of the
  circle, nothing of the body shows). A hand or collar raised near the face in the preview pose (pc05, pc13) can
  still show inside the circle.
- Background: both transparent; no difference.
- pc04/12/13 didn't fit well (expression/pose differences between the preview and the disc art); the median crop
  still lines them up by eye in `fit.png`.
