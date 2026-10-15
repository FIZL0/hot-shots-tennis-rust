# M8f10a1 — Open Tee 2 Nakagawa h07 court mod (FINAL)

## What
`../open-tennis-mods/tools/psp2gltf/export_opentee2_court.py` (open-tennis-mods e05ece6; it reuses the Get a Grip exporter's
scene/model/run-off code) writes `out/mods/ot2_c13_h07_nakagawa/`, with `mod.json`, `court.glb`, `sky.glb`,
`sound/bgm.wav` and `sound/ambience_0.wav`. Run it with `cd tools/psp2gltf && python3 export_opentee2_court.py
[--court ±] [--end ±]` (~2 s).

- **Courts.** Hole model node `polySurface3` holds the whole ground. The courts are bare earth (`c13bare@mud`). The
  lines are 0.1–0.2 m `breakwator02@asphalt` strips, with a 1.6 m `redge01@rough` border. There are two courts side by
  side (±9.75 m), with collinear 11.74 m nets (`net` node). The `+` court is the flatter one (+0.14/−0.04 m within
  20 m), so it goes on HST's origin with the net along HST's net. The mapping is a rigid rotation plus a flip (det +1)
  in the `ot2_cm` node.
- **Lines.** In game metres the rows are x 0.05, 4.015/4.115, 5.385/5.485 and z 6.2/6.4, 11.585/11.685/11.885. The
  outer edges match HST's to the mm, and the court is flat within 1.8 mm.
- **Plants.** HST's 0x40 plant records map to mesh space as (x, y, z) → (−x, −y, z); the tee marks confirm it to
  within ~2 m. Placement uses HST's `world::place`. Placed: 54 trees, 2 tee marks, a prefab hut, 2 kei trucks and the
  hole flag.
- **Sky.** hol07 `kasen00_sky` dome, plus the bg0 `p10` photo panorama and clouds, dropped 386 m.
- **Sound.** BGM is course 13's `at_c_07` (ATRAC3+, decoded with vgmstream) with its loop points. Ambience is the
  hole's `co_40` loop.
- **Layout.** Western (2), by the closest-run-off rule among clay courts (run-off +z 13.6, −z 7.7, +x 40, −x 22.3).

HST change: `court_mod.rs` `primitive()` scales the vertex colour of untextured primitives by 255/128. `gs.wgsl`
draws untextured colour at the GS's 128 = 1.0, so glTF vertex-coloured skies came out at half brightness. Sky pixel
before: (0, 27, 104); after: (0, 53, 208), against a vertex colour of (0, 54, 209).

## Checks
HST release `--play --singles --court-mod …` shots (`tools/shot.sh`, HST_EYE held eyes) are in `context/m8f10a1/run1`
(play, rest, turned, high, low, lowback, side, up, sheet.jpg) and `run2` (sky after the brightness fix):
- Court, lines and net line up under HST's own court, and both ends' match views are clear.
- Fence, second court, railway bridge, embankment steps, trees, hut, kei trucks, river and panorama all show.
- The sky is a blue gradient with clouds over the embankment.

## Not verified / not 1:1
- Surface: **clay**, not the task line's "hard". The courts are bare earth with asphalt line strips and look like
  dirt courts in HST.
- Not compared with Open Tee 2 running. No PSP emulator capture was taken; checked by eye in HST only.
- bg0 `p10` taken as the front-nine backdrop by analogy with Out of Bounds' `half_p10`/`p01`; not confirmed.
- SKY_DROP 386 m judged by eye, so the panorama's far-bank waterline sits at eye level.
- Sky dome kasen00 chosen over the paler kasen01 by eye. Which one the game uses for hole 7 isn't decoded.
- No manifest `light`: the hole's `envir_c13_h07.dat` isn't decoded, so the layout's light is used.
- Ambience one-shot `co_15` (0.3 s) left out; when the game plays it isn't known.
- The net is 0.87 m tall at the centre against HST's 0.914 m, and the posts are at ±5.87 m against HST's ±6.4 m. The
  mod draws Open Tee 2's net; HST's net physics are unchanged.
- Inner line strips are 0.1–0.2 m wide as modelled, not HST's widths. Only the outer edges are exact.
- Plant categories 7–15 and 23 are skipped (not models). The billboard (cat 20 → `hio`) and prop (cat 18 →
  `c13teemark`) index→model mapping is assumed, since each has one model.
- An earlier low-eye shot showed the cloud band (`c13clo`, alpha blended) as a hard-edged tall rectangle. After the
  brightness fix it no longer shows in run2, but other eyes weren't re-shot.
- Shot only from the resting eyes plus a few held eyes, not through a whole match.
- The `court_mod.rs` brightness change also brightens Get a Grip mods' untextured pieces (skies, shadow cards). That
  is correct per glTF, but those mods weren't re-shot.
