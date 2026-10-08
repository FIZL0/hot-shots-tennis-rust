# B16 — head markers drawn behind things (FINAL)

## The original
- The balloon/marker manager builds the markers into a prim object on the `i_playerinfo` texture whose draw
  bucket (+0x44) is 0x1c, like the balloons (default 5): queued after the whole 3D scene.
- Depth: `research/marker_depth.py <slot> <prefix> [y]` loads slot 3 (markers up), screenshots, then pokes every
  marker's anchor height (manager +0x72c, y down-positive) and screenshots again
  (`context/shots/b16/orig_*`, `half_*`).
  - y = 5 (5 m under the court): all markers gone → Z-tested against the court.
  - y = 0.85 (anchor 0.2 m under): only the top strip shows above the court, and the players' shoes cover it → Z-tested
    against players too. No Z write needed (nothing draws after them in the scene).
- So: drawn last, depth-tested. Nothing translucent in the scene ever lands on top of them.

## The port's bug
- Markers are `AlphaMode::Blend` quads; court/scenery `GsMaterial`s with blend or no-Z-write tests are also Blend.
  Bevy sorts the transparent phase by each mesh's AABB-centre view depth, so a big blended scenery mesh (or one
  nearer by centre) drew after a marker and covered it.

## Fix (`play/markers.rs`)
- `depth_bias: LAST` (1e4) on the marker material: sort distance + 1e4 → drawn after every other blended mesh; the
  depth test stays, so the court and bodies in front still hide it as in the original. Bevy also applies it as a
  constant GPU depth bias: ≤ 1e4·2⁻²³ of the depth value (~0.12 % of view depth, ~2 cm at 20 m).
- Checked `--shot` on stages 01–11 old vs new (`context/shots/b16/old_*`, `new_*`, `cmp_*`): markers draw as before
  where nothing covered them. The exact occluding frame the user saw wasn't reproduced in these autoplay frames.
- Balloons (play.rs, same bucket 0x1c in the original) have the same sort issue; left to B21 (balloon placement).
