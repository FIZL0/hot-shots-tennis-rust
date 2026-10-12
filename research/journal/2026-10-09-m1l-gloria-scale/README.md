# M1l — mod sizes calibrated on Gloria

Mods repo commit c6c9d3f (`tools/rerig/sizing.py`, standard §2, notes). HST: `hst_skeleton.json` gains `top_height`
(head to foot, rest), select previews `RENDER` 1 → 2 so cached pictures redraw.

## What changed
- One factor per game = HST Gloria's Bip01 (0.8554 m) / the game's Gloria's raw height (cm × HST m/cm, or model
  Bip01 × the game's scale). Replaces M1j's "tallest at 0.95 m" fit.

  | game | Gloria | factor (was) | head share |
  |---|---|---|---|
  | Get a Grip | pc13 | 0.9706 (0.9394) | 0.909 |
  | Out of Bounds | pc16 | 1.0123 (0.8264) | 0.5 (her head is not shrunk) |
  | Open Tee 2 | pc20 | 1.5240 (1.4925) | 0.909 |
  | Open Tee | — (same ten as OT2 pc00–09) | 1.5240, OT2's sizes | 0.909 |
  | Fore! | — | 1.0 (0.9685) | 0.5 |

- Chibi games: the head share is solved so Gloria's head-to-foot also equals HST Gloria's 1.6547 m (bind).
  The share 0.909 is near the full head match the user found "a little small-headed" on 2026-10-08; the task asked
  for equal overall height, so that wins.
- Bjorn (OOB, 4'6") Bip01 0.603 → 0.738 m; Montcalm (OT2) Bip01 0.881 → 0.900 but head to foot 1.834 → 1.739 m
  (smaller heads). OOB LJ (7'1") lands at Bip01 1.164 m, above HST's 0.95: relative heights kept as asked.
- Every mod repackaged (getagrip, oob, opentee2, opentee, fore; all rerigs PASS check.py).

## Measured
Bind (packaged mod .glb, all costumes): every Gloria Bip01 0.8554 m; top GaG 1.6547, OT2 1.6547, OOB 1.6166.
In the remaster (`--character 12` / `--mod DIR`, `--motion 64 --radius 5`, same camera; context/m1l/v_*.png,
side_by_side.png), silhouette height in px: disc 312, GaG 314, OT2 315, OOB 296.

## Not verified / not 1:1
- Joel: no Joel in any packaged mod (Fore!, Get a Grip, Open Tee 1/2, Out of Bounds rosters, 92 mods) and no Joel model
  found on HST's disc (14 PCs, 5 umpires, gallery `g-NN`, ball boys; no "Joel" string in the extracted data). So the
  Joel calibration was not done; whoever knows which mod/NPC is meant should say.
- Fore! has no character shared with HST or the other games (its Brad is a different character from Get a Grip's),
  so it keeps its own model scale (factor 1), not a measured reference.
- OOB Gloria is 2.3 % shorter head to foot than HST's (bind; 5 % in the shot pose) with equal Bip01: her own
  proportions, and heads are never grown.
- Shot heights include pose differences (mods play their own motions.glb where they have one).
