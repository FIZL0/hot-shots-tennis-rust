# 5 — Get a Grip additive court materials → `@add` (M8f5a1)

## What landed
- The exporter change was already in ../opentennis-mods (27c618f, made while B401 was being worked on): `Scene.prims` reads
  each material's texture record (`i3d.Material.raw`, the type-0x26 record) and passes `add = raw[0x24] & 1` to
  `material()`, which names it `<name>@add`, BLEND. The standard (§2) defines `@add`. This task checked that the flag is
  right and that HST draws it, and documented the format in opentennis-mods `notes/getagrip.md` *Courts* (c8698cc).
- The flag, over all 2558 material records in `200_court`: +0x24 is 0x10 or 0x11; every 0x11 record also has +0x31 = 2
  and +0x33 = 0x10 (38 records, no mixed cases). +0x31 = 2 also appears without the flag on 4 records (yamate's court,
  ground03, stairs01, c16's roof01): plain surfaces, so +0x24 bit 0 is the test, not +0x31.
- Flagged (re-exported glbs): c01 `yamate_light`; c02 `sunlight01`, `window03add`, `LampRay` (court + gimmick);
  c03 `ob18_mizu`; c04 `waterAdd`; c06 `rainbow01`, `water01/02`, `fog01`; c08 `gaslamp02`; c13 `light01`
  (+ the pool's `HAMON` ripples); c15 `lightmask` (+ the fireworks); c17 its plates, wall/map glow, a `court` copy that
  lies over the opaque `c17_shadows_court` (bbox checked: a glow layer, not the floor), `ob13` pieces and the
  switch; c20 `bg_cloud02` (sky). The names say the same thing (`…add`, `light`, `Ray`, `mizu` = water).
  c05, c07, c09, c14, c16, c18 have none.
- Loader: `court_mod::materials` passes the glTF material name into `GsMaterial::for_batch`, which already
  blends `@add` as the GS does (`Cs·As + Cd`, no Z write). Nothing changed in this repo except the journal.

## Checks (`context/m8f5a1/`)
- run2 `eye-pair.png` / `match-pair.png`: `--play --singles --court-mod gag_c01_yamate` from a fixed `HST_EYE` over the
  net's left side, with the material's suffix blanked (top: the black octagon) and with `@add` (bottom: light
  dapples on the grass, no black). run1 `eye-red-play.png`: the same pieces painted red and lifted 1 m to find them.
  Pitfall: without `--play` the court mod isn't loaded at all (no `court mod …` log line); the viewer shows the
  disc layout court.
- run3: denen, canada, c17 match views; denen's water and canada's water/rainbow show no dark patches.

## Not verified / not 1:1
- Not compared with Get a Grip running: there's no PSP emulator capture here, so how bright the dapples, glints and
  rays are is judged by eye only. Reading byte +0x24 as the GE's additive blend is an inference from the 38 flagged
  records (all light/water/glow textures, mostly black) and from how they render, not from the PSP program
  (encrypted).
- Additive draws ignore the PSP's exact blend factors (the record may also select fixed or src-alpha factors);
  HST draws them all as the GS `@add` (colour × alpha + dest).
- c17 Shadows: solid black vertical bars fill the match view. They come from `gimmick.glb` (gone without it), not
  from an additive material (black added to the scene is invisible). New task **M8f5a2**.
- Not every flagged material was looked at in a match: c02, c04, c08, c13, c15, c20 were only checked in the data.
