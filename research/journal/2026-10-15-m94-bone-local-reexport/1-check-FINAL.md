# M94 — check (FINAL)

Scanned every `.i3r` under Get a Grip's xbdata for `MeshPart.bone_local`: besides the umpires (M93), there are the adhoc
bodies 32, 111, 152, 157, 185, 186 and 206, the full body157, c03 `gi06_NIWATORI` (head on `Neck`) and `gi09_Suzume`
(left wing on `L_Wing`), and c16 `gi09_kusudama_b01` (the four half-ball pieces on `L_BALL_1`/`R_BALL_1`).

## What the exporters write and what changed
- `export_getagrip.py --only parts`: exports only `data/chara/model/` bodies, not the `adhoc/` LODs, so of the bodies only
  `parts/body/body157.glb` changed. Before: the tennis-ball costume (the bone-local group) was a huge ball over the
  origin that hid the whole body. After: it sits on the torso, with arms, legs and shoes outside it. No character set in
  `characters.json` uses body 157, so no `pcNN_*.glb` changes.
- `export_getagrip_courts.py 3 16`: both `gimmick.glb` came out byte-identical to the ones on disk. c03's was already
  re-exported after the fix (open-tennis-mods d22e1cc, M8f5a2, was committed after c79756e). c16 places the rigid
  `c16_gi09_kusudama.psp.i3r` (looping the `_b01_Break` motion, which doesn't move a rigid model), not the
  bone-local `_b01` model, so nothing changes.
- The adhoc bodies and the three gimmick models alone were built into separate glbs (`build.build` for the bodies,
  the courts exporter's `Scene` with the court's texture dirs for the gimmicks) and rendered with render.py →
  `context/m94/standalone/`, `context/m94/montage.png`. All adhoc bodies are whole (hoods, antlers, skirts and boots in
  place). The chicken's head, beak and comb sit on its neck. The sparrow's left wing mirrors its right. kusudama_b01 posed
  by `_b01_Break`: closed ball at t=0, in the same place as the rigid model; split open with the banner hanging from
  0.3 s.
- Nothing to commit in open-tennis-mods: the code fix is M93's, and `out/` is git-ignored.

## Not verified / not 1:1
- No in-game comparison: Get a Grip (PSP) can't be run here. The checks are render.py pictures of the exports, compared
  with the same models' rigid/mirrored counterparts.
- The adhoc LODs and kusudama_b01 aren't written by any exporter, so they were only checked as standalone builds.
- c16's `gimmick.glb` still loops the break motion on the rigid ball, so it never opens. That's the existing placement
  choice, not part of this fix, and is left as it is.
- No Rust changed, so nothing was checked in a remaster match. c03's gimmick.glb, which a match loads, is unchanged.
