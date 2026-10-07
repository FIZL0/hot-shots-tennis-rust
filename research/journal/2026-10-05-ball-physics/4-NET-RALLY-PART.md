# Net + sandbox rally (PART)

- Net (commit 1331419): material-generic bounce. Net = special material row 2 (restitution 0.18, spin loss 0.95):
  first touch slide ×0.1, kick 0.1, spin relax = spin_loss, push 3 mm/frame away after. Geometry is the game
  predictor's flat plane (z=0, 0.91 m, |x| ≤ 6.4); live play uses the court collision mesh (FUN 32f690 on
  *(gm+0x84)) — not ported. IMPORTANT: stored paths (path recorder, param_2=1) test the court plane ONLY, so
  replay fixtures can never show net hits; Flight::net=false for them.
- Sandbox (commit after 1331419): `hst <iso> --stage 1 --ball` rallies with tr_pc00_strk0 + hst-sim, logs hits.
- Shot bearing: the stroke launches toward target+offset while the table is looked up with the plain target;
  offset = caller's param (likely AI aim error, AIParam "ずれ" columns) — belongs to the AI port.

## Next candidates
1. Per-character shot param record (0x4287c0 + char*0x451 + kind*0xdd + class*0xd): spin/first-bounce values,
   find its source (TParam.csv?) so spin comes from data, not the 2.9671 placeholder.
2. Court collision mesh (net cord/posts/walls): FUN_0032f690 + court data (HMP?).
3. AI: target choice + shot type + aim error, using AIParam.csv and slot 5 captures.
4. Players: movement/animation (ANI2) — verify numerically, render from disc at runtime.
