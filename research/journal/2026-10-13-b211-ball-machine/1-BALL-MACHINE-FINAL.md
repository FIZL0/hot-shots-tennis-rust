# B211 — ball machine (court 5 `ballboy01`) under the grass: FINAL

## Cause
The cause wasn't placement or depth. MDL packets are stored in their node's space, and `main.rs` drew court props'
packets straight from the file, so every non-root packet sat at the model origin. The machine's launcher wheels
and balls ended up at ground level: the wheel rims went through the grass and flashed. Packets weighted over
several nodes (court 6's horses) were torn apart for the same reason.

## Fix
`main.rs mesh()` now draws each packet at the model's bind pose:
- A packet on one node is transformed by that node's bind matrix (`Model::node_bind`).
- A weighted packet is skinned through `mdl::Model::skin_packet`, the per-packet body of `skinned()`, split out
  unchanged.

## Checked
- Court 5, match camera of the serve set-up. `research/b211_shot.py` captures the original's eye and screenshot from
  scratch slot 8 (confirm screen, Queen's selected); our `--shot` uses the same `HST_EYE`. The machine matches the
  original: yellow launcher wheels on top of the blue body, feed tube above, wheels clear of the grass, no flashing.
  Shots: `context/b211/run2/{orig_s,orig_crop,ours,ours_crop}.png`; before the fix: `context/b211/ours_mg*.png`.
- Court 6: the horses are whole (`context/b211/c6_horse_skin.png`).
- `tools/check.sh`: 509 passed.

## Not verified / not 1:1
- The original's frame is tighter than ours at the same eye and fov (probably its widescreen/aspect setup). The
  comparison is by eye on the machine only, which PLAN allows for rendering.
- `ballboy01.ani` isn't played. The prop is drawn at its bind pose, so if the original turns its wheels, ours stay still.
  The prop was out of scope here.
- Court 6's horses weren't compared with the original (no court 6 capture). They're only checked as whole and standing,
  drawn at bind pose with no animation.
