# B75 — effects drawn behind background elements

## Cause
The backdrop models (`_sky`, `_clo`, `_acc`, `_bg`) stand at the camera eye + a per-court lift (`gs::backdrop`,
lifts −67…800 game units across the courts' `envir_cNN_h01.dat`), and the drifting `cloud` models at eye + their
offset. Bevy sorts blended draws by the mesh's translation plus `depth_bias`/`order`, so these sorted by
`viewz(eye + lift)`: somewhere in ±800 depending on the camera's pitch. Everything blended that writes no Z and has a
smaller key was drawn **before** them and painted over by their blended texels (the bg models are all
`Blend` Ge70/Lt70 pairs): the 3D billboards with no sort offset (balloons, COM pointers, bounce words, rain, the sun
flare at the camera), and the `AFTER_SCENE` (+500) effects whenever the backdrop's key went past theirs.

## The original
GS dump `context/p17s/s5.gs` (court 10, last frame; `research/p17v_gsprims.py` + a run grouper): the sky is the first
draw of the frame (untextured strips, Z constant 0x100 under ZTST GEQUAL), then the court's opaque draws (Z ~0x40xxxx),
then the cloud quads at Z 0x400–0xad2 (ALPHA 0x44, Z written but at the far end), then the court's blended draws, and
the effects much later. So the backdrops never cover anything drawn after them.

## Fix
`main.rs`: every backdrop part's `GsMaterial::order` and each cloud's own material get `BACKDROP` (−2e6), below the
hole's −1e6: the backdrops sort first, before everything blended. A constant (not per-kind ranks) keeps their order
among themselves exactly as before (by lift): a first try ranking sky<clo<acc<bg changed how `_bg` and `_clo` layer.

## Checked
- `--shot` before/after (context/b75/before, after): court 10 singles, match camera, cut-aways, and a fixed low
  `HST_EYE` past the far player (wrapper scripts, hyprctl drops env): layering of players, pointers and clouds the
  same where nothing overlapped; nothing lost from the backdrops.
- Original (`tools/screenshot.sh`, slot 5, context/b75/orig): cut-away billboards (CLAP words, balloons) over the
  backdrop, as ours after the fix.

## Not verified / not 1:1
- No frame-matched pair showing the old bug itself: `--play` bot runs aren't repeatable frame for frame and the cloud
  layout is random per run, so no shot happened to put a no-Z effect over a backdrop texel; the cause is from the sort
  keys above and the GS dump's order.
- No same-moment original vs port pair (the original's slot 5 is doubles, ours singles court 10 with clouds); compared
  by eye at comparable moments (render-not-exact rule).
- The original's relative order of `_clo`/`_acc`/`_bg` among themselves wasn't read from the dump (only the sky and
  cloud quads have the far Z there); kept the port's previous by-lift order.
- Backdrops still write their real depth where the original writes a far constant Z; only matters if a backdrop
  were nearer than an effect, which none is at court distances.
