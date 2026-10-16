# B485 — huge flat polygon fanning across the screen in Firefox/Zen (WebGL2)

## Report
User 2026-10-16, a friend's screenshots: Zen on Linux, WebGL2 (the page's fallback for Gecko). A huge flat grey-green
polygon fans out from a point near the net/a player, in rallies and in the score cut-away. Some frames are worse than others.

## Cause
B157's court guard band (`gs.wgsl` `GS_GUARD`) drops every triangle with a vertex outside VU1's guard band. It does
this by setting that vertex's clip position to NaN (`vec4(z / z)`). D3D/Vulkan-style drivers drop such a primitive,
but GL leaves a NaN position undefined, and WebGL2 in Firefox runs on the system's GL. A driver that rasterises it, or
folds `p.w * 0.0` / `0 / 0` to 0, gets a w = 0 vertex projected at the middle of the screen. The match camera's
middle is by the net, so the triangle becomes a flat fan in its vertex colour.

These vertices are mostly off screen. Shrinking the band showed that no on-screen pixel comes from a flagged
triangle in the usual match views, and guard on vs off differs only by animation noise. On a well-behaved driver
nothing shows. On a bad one, every camera move changes which off-screen court vertices fall outside the band, which
is why some frames are worse than others. Cut-aways put the camera close to the scenery, so many vertices fall
outside the band.

## Fix
WebGL2/browser builds (`cfg(target_arch = "wasm32")`, new def `GS_GUARD_DISCARD`, vertex and fragment): the flagged
vertex keeps its real position and writes `guard = 1` (others 0) to a new varying. The fragment stage discards when
the interpolated value is > 0, which covers every pixel of a triangle with a flagged vertex. Native builds keep
the NaN, which their Vulkan/Metal/D3D backends drop as specified.

`tools/web_zen.mjs`: `SHOT_EVERY=N` takes a screenshot every N s (default 10, as before).

## Checked
- Repro attempts in headless Zen 1.23 WebGL2, `?play&stage=1&pads=-,-,-,-`, a screenshot every 1–2 s over 60–90 s,
  on this machine's AMD GPU (Mesa) and on llvmpipe (`LIBGL_ALWAYS_SOFTWARE=1`): no fan. Both Mesa drivers drop NaN
  triangles. A test build with `vec4(0)` in place of the NaN didn't fan on Mesa either, because Mesa clips w = 0. So
  the bug could not be reproduced here (see below).
- Same triangles dropped: native `--shot`s with the discard path forced on vs the NaN path. To make sure flagged
  triangles were on screen, the band was shrunk to 0.3× for this test only. Court 8: shrinking changes 11k pixels
  (dropped foliage), and NaN vs discard differ by 650 pixels, the same as run-to-run noise. Court 7: the difference
  is only in the animated players. Shots are in `context/b485/` (`small_*`, `nan_*`, `discard_*`, `noguard_*`).
- Fixed web build in Zen WebGL2, court 8 all-CPU match, 60 s: the shaders compile, no console errors, court and
  cut-aways draw normally (`context/b485/run3/sheet-fix.png`).
- `tools/check.sh -p hst-base gs`: 11 tests pass.

## Not verified / not 1:1
- Not reproduced: both drivers on this machine (Mesa radeonsi, llvmpipe) drop NaN triangles, so the fan never
  appeared. The cause is inferred from the description (a fan from the middle of the screen, scenery colour, worse
  at some camera angles and in cut-aways) and from this being the only NaN the renderer makes on purpose. The
  friend's GPU/driver is unknown. To confirm, someone with the bug reloads after this build deploys.
- The discard keeps a pixel whose centre lies exactly on the edge between two unflagged vertices of a flagged
  triangle (interpolated value exactly 0). This is rare, and the neighbouring triangle covers that pixel anyway.
- WebGL2's flagged triangles are now rasterised and then discarded instead of being dropped before rasterisation:
  some extra fill on screens with large flagged triangles. Not measured; the usual match view has none on screen.
