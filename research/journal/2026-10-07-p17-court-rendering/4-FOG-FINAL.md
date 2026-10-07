# P17b — fog (PRIM FGE)

- Every court batch has FGE (0x20/0x30/0x70; the sky's batches are 0x20 too). The port masked PRIM to 0x50 when
  grouping packets (main.rs gs_models_anim), so fog never reached the material key; now 0x70.
- FOGCOL: one setter (0x132530) called from the per-frame environment update 0x32e8b0, which also sets the fog of
  three views via 0x1383d0(F near, F far, z near, z far, view) → 0x1389c0 stores view +0x520..+0x52c =
  (F far, F near, slope, intercept). The court view is the global at 0x1e7d10 (its +0x10 = eye, game space).
- Source: `envir_cNN.dat` (CMN.XB) row k (time of day) at 0x290 + k·0x60 = RAM table +0x7e0 (stride 0x60):
  +0 flag (≠0 → fog off: 255, 255, 0, 0.001), +0x18 FOGCOL RGB, +0x20 another colour (unused here), +0x28 F near,
  +0x2c F far, +0x30/+0x34 F near for the two other views, +0x38/+0x3c depth near/far. Court 10 row 0:
  FOGCOL b5 d1 dc, (255, 224.4, 40, 190) — identical to RAM 0x1e7d10+0x10c in save slot 5.
- Eye-height fade: t = clamp((−30 − eye_y)·0.02, 0, 0.7) (eye_y game y, −11.89 in a match → 0); both F ends
  become F + t·(255 − F). Runtime overrides at env +0x50/+0x5c/+0x60 (used when ≥ 0) not seen set.
- VU1 (MPG at ELF 0xc2534.., lit loop 0x6c4..): fog block = VU qword 12 (upload 0x149870); per vertex
  F = MAX(MINI(intercept + w·slope, F near), F far), w = clip w. Projection (camera +0x2e0) has w = view z, so the
  depth is view-space depth in metres (court centre ≈ 41 m from the match camera). FTOI4 → XYZF2 F = ⌊F⌋.
- Port: gs.rs `court_fog`, GsUniform fog/fog_color, GsKey.fog → GS_FOG in gs.wgsl:
  (⌊F·C/256⌋ + ⌊(255−F)·FOGCOL/256⌋), before the shadow darkening (a later frame-buffer draw on the PS2).
  Test gs.rs court_fog_row (court 10 row vs RAM values).
- Check: context/shots/p17/port_c10_fog.png vs orig_slot5.png: the far props/temple haze to blue-grey as on the PS2;
  near court unchanged (F = 255 below 40 m). An exaggerated test (F far 0 at 60 m) confirmed the depth gradient.
- Ponytails / open: F per pixel from depth, not Gouraud-interpolated per vertex (same below 40 m and above 190 m);
  time of day 0 only (P17e); the other two views' fog (F near 245/229.5 in RAM, set by 0x3484d0/0x3afcb0 from
  per-index tables) is not identified with any draw — players use StandardMaterial and get no fog; whether the
  sky is drawn with the court view (then F ≈ 224) is unconfirmed, the port fogs it like the court.
