# B41 Will's eyes / Carol's second eye (2026-10-08)

Not a texture/CLUT problem: the eye textures, materials and UVs load right (the viewer drew Will c00/c09 and both
of Carol c09's eyes at every motion). Two draw bugs in play together left the eye sockets empty. Through the empty
sockets you saw the inside of the head, which reads as black at court distance (`will_before.png`).

## 1. Frustum culling on bind-pose bounds
Bevy culls a skinned part by its mesh's bind-pose AABB under the rig root, and never by the posed one. Will c09's
eyes (mat5 `9 - Default`) are skinned to the `eye` joint (13) only. Their box is about 20×5×5 cm, so once a play
motion moves the head off its bind-pose spot, the part is culled whenever the box is out of view. Close-ups
(cutaways, a near camera) lose the eyes. The racket's gut (another small part) was culled the same way in a
medium shot.
Fix: `NoFrustumCulling` on every character part (`character::spawn`, the racket parts, and
`shade::own_materials`' second-draw siblings). The GS has no per-part culling either.

## 2. Mirrored left-handers culled their one-sided faces
Left-handers are drawn with x scale −1 (`play::draw`). That flips the screen winding, and `Face::Back` culling then
dropped the faces towards the camera of every one-sided material. Will's eyes are one-sided on c00 (`eye1`) and c09.
Carol c09 has two eye materials: `lambert1` (two-sided, drew) and `eye02` (one-sided, vanished). That was the
"only one of two eyes" report.
Fix: `GsKey::mirror` culls `Face::Front` instead. `shade::light` sets it from the sign of the rig's
GlobalTransform determinant (part of its change key).

Both fixes are needed: with only one of them, Will's eyes stay missing in a play close-up.

## Checked
`--shot` play close-ups (a temporary camera override, removed) on stage 4, chars 6,11,2,10, outfits 9,9,4,9:
- Will (`will_after.png`): both eyes draw.
- Carol c09 in slot 1 (`carol_after.png`): both eyes (blue and green) draw, mirrored as a left-hander.
- `tools/check.sh` passed.

## LOD (the user's note)
Character textures have one level (no mips), so the GS texture LOD (`lod_k`) changes nothing on them, and LOD
played no part in the missing eyes. A character model LOD (if the game has one) was not looked for.

## Not verified / not 1:1
- No pixel comparison with the original (`tools/screenshot.sh`). Getting a matching close-up of Will in
  costume 9 needs a cutaway at a known frame. The original draws the eyes (user report), and this checked
  that they now draw, not that they match pixel for pixel.
- How the game keeps a mirrored model's one-sided faces (its VU1 cull-sign handling) was not traced. Flipping the
  culled face reproduces the visible result.
- B32 (Cody's face, already done) was not re-checked against this cause.
