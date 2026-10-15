# M8f10a3 — Out of Bounds golf venues as court mods (FINAL)

## What
`../HST-MODS/tools/oob2gltf/court.py` (HST-MODS 121715a): `python3 tools/oob2gltf/court.py out/files/oob
out/venues/oob.json <HST iso> out/mods [course…] [--end -] [--data DIR]` writes `out/mods/oob_cCC_<stem>/`
(`mod.json`, `court.glb`, `sky.glb`) for 6 of Out of Bounds' 7 courses, ~25 s each. It follows M8f10a2's rules
(spot, run-off, ground height, laid court, `__clear` within 45 m). The full method and per-course table are in HST-MODS
`notes/venues.md` → *Out of Bounds*. The format notes are in `notes/oob.md` → *Static course meshes*.

New in HST-MODS: `static.py` reads static (unskinned) `.rem` meshes, which `rem.py` skipped. `dds.py` now decodes
uncompressed RGB(A). `fore2gltf chairs <iso>` prints the layout courts' umpire chairs for the Python exporter.

- **Space.** OOB is Y-up and its height map sits in the hole model's own space. `oob_spot` turns it 180° about Z into
  HST game space. The front nine (`_p10`) and the back nine (`_p01`) are separate worlds over the same
  coordinates, so every lookup stays in the spot's own nine.
- **Placements.** Each hole's plant files only cover what that hole shows. The spot therefore takes every plant file
  of its nine (plus `_out_`), deduplicated, wherever this hole's ground model is drawn. Record: u16 category
  (0x14 tree, 0x15 prop, 0x16 structure, 0x17 billboard), index into `entry.txt`'s non-LOD list, position, yaw at
  0x24, scale at 0x28.
- **Blockers.** Placement circles, plus prop/structure vertices and hole-model vertices standing above the height
  map. Walls, steps and kerbs are part of the ground model, so they count too.
- **Surface.** Hard when the ground under the slab is mostly paved, else grass. That gives c01 Extra and c06 Park
  (hard); the other four are Jungle (grass).
- **Textures.** The hole models' textures are loose `.dds` files under the disc's `PS3_GAME/USRDIR/data`, outside the
  xb archives (`--data`). The billboards are in `plantmodel01.xb0`.

Bugs found while checking in HST:
- The venue matrix applied the y-down ground top as if it were y-up, which put the hole ground about twice its
  height above the court. c02's court ended up under the island.
- Without the loose textures, the ground was drawn as vertex colour only (beige).
- Placements from the other nine were merged in.

## Checks
I ran HST release with `--play --singles --court-mod <dir>` and `HST_WEATHER=0`, and took `tools/shot.sh` shots with
held `HST_EYE` views: play view, the far resting end, high above (50 m), low beside the court, and from the side.
The shots are in `context/m8f10a3/run4`; run1–3 predate the fixes.
- All six courses: the slab, lines, net and chair sit on HST's court, the umpire sits on the chair, and the
  players stand on the slab. The hole's ground runs level around the slab and the venue stands upright.
  - c01: car park with the clubhouse terrace.
  - c02: cliff-top practice fairway with the sea and clubhouse behind.
  - c03: windmill behind the far end.
  - c04: round green in the savanna.
  - c06: helipad by the manor house.
  - c07: rough between red rock hills.
- Plant yaw sign: structure/prop base vertices sit on the height map better with the yaw used here for c06 h01 (mean
  0.13 vs 0.22 m). c03 and c01 can't tell the two signs apart.
- No HST code changed; no tests touched (`tools/done.sh` runs the suite).

## Not verified / not 1:1
- Not compared with Out of Bounds running: there's no PS3 emulator capture. Checked by eye in HST only, from held
  eyes, not through a whole match.
- Course 05 (fjord) isn't exported: all 41 survey candidates are blocked or not flat. There's no own spot search
  beyond the survey's 3 per hole.
- The surface (hard/grass by paved material names) and the laid court are our choice; OOB has no courts.
- Plant record words 0x14–0x20 aren't decoded (small angles, maybe leans), so placements use position, yaw and
  scale only. The yaw sign was confirmed on one hole (c06 h01) and not contradicted elsewhere, but it isn't proven.
- Sky/backdrops sit at the eye with no lift, and the envir files (light, fog, sky yaw) aren't decoded. There's no
  manifest light, so the weather role applies. From high above at a cliff edge, the backdrop's under-cap shows
  as streaky sea.
- Materials use one colour map each: no multi-layer ground blends, water shaders, specular or normal maps.
  Vertex colour is /255.
- No creatures, galleries, drifting clouds, pin/flag or course music. Sound and ambience come from the layout
  court.
- Course 07's display name is a placeholder ("Out of Bounds course 7"); the name wasn't found in the data read.
- Front and back nines as separate worlds was inferred from height maps (h12 vs h01 at the same x, z), not from the
  game's code.
