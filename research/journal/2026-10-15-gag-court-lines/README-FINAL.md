# M8f5a5 — Get a Grip court mods missing their lines

**Cause.** The court textures carry the lines on all 16 courts (a quarter court per 512² texture). On c01, c03,
c04, c05, c07, c15 and c18, the ground material holds **two** texture records (0x26): the court texture and a baked
shadow map (`c01_yamate_shadow00`, `c03_baked01`, `c04_resort_baked`, `c05_GroundShadow`, `c07_baked`,
`c15_baked01`, `c18_china_shadow00`). Each 0x55 batch of the mesh holds one 0x4f per texture, and the second one
draws the same triangles again with the shadow map's UVs. `i3d.read_materials` read only the first texture and
`read_meshes` merged both passes. The shadow pass was therefore a second opaque copy of the ground: the court
texture at the shadow map's UVs (stretched, no lines), z-fighting over the real one. That gave lines only in
fragments, and c04's "low-res" look.

Counted over `200_court`: 0x4f per 0x55 always equals the material's texture count (4828 single, 185 double). The
second record's +0x24 is 0x12 on every shadow map. Those maps are black where lit and bright in shadow, so the pass
subtracts.

**Fix** (../opentennis-mods 01ef8ae): `i3d.Material.passes` and `Prim.pass_`. `export_getagrip_courts.py`
`prims` groups by (material, pass); pass 2 gets its own texture, `@sub` for +0x24 & 3 = 2 and `@add` for 1 (the
standard's suffixes, which `court_mod.rs` already maps to the GS reverse-subtract and add blends). `notes/getagrip.md`
gets a *Two-texture materials* section. The re-export of all 16 leaves every `mod.json` (layout, lines, run-off)
unchanged. Side effects, also real second passes that used to z-fight: c08 gets its shadow map (walls) and an
additive statue reflection map, c09's umpire chair an additive highlight, c16's roof an additive glow.

**Check.** In-match `--shot`s (`--play --court-mod`, HST_WEATHER=0) of all 7 plus c08/c09/c16, before/after for c01
and c04: `context/M8f5a5/run1/`. The full line set is crisp on every court; c04's wood and lines are readable, with
the palm shadows. No lines overlay fallback was needed.

Follow-up: M8f51. Character accessories etc18/51/81 have an additive second pass that `export_getagrip.py` still
merges.

## Not verified / not 1:1
- Shadow strength against Get a Grip itself: no PPSSPP reference shot. I read 0x12 as subtract from the maps' look
  (black = no shadow) and render it through HST's `@sub` (Cd − Cs·As, modulated by the vertex colours like pass 1).
  Whether the PSP modulates the second pass by vertex colour is unknown. c05 comes out quite dark (its street is
  mostly in the buildings' shadow); judged by eye only.
- c08's `c08_refmap02@add` is a reflection map. On the PSP it may use generated (environment) UVs, but it's
  exported with the stored UVs.
