# B140 — select preview PNG size and swap flash

## Causes
- **Size**: the PNG's `ImageNode` used the default `NodeImageMode::Auto`. That drew it about 0.94× and centred in the
  card, while the live `MaterialNode` filled it. The baked PNG itself already matched the live render 1:1 (same
  alpha bounding box).
- **Wrong face in the PNG**: the baker read back after 4 render frames. Faces and poses settle on FixedUpdate ticks,
  and fast menu frames outrun those ticks, so Big Chief's mouth was caught mid-change. Waits of 6 and 10 frames were
  still wrong; 60 was right.
- **Black frame at the swap**: the select's systems ran after `inspect::place`. On its first frame the new preview
  camera rendered without the −1 alpha clear, so it copied zeroes as opaque black.
- **Darkening while covered**: the live node drew under the PNG, and showed through the PNG's semi-transparent racket
  strings.
- **Blank frame**: a `MaterialNode` that was Hidden draws nothing on its first visible frame. The cover was removed on
  that same frame.

## Fixes (`previews.rs`, `inspect.rs`)
- The PNG is drawn `NodeImageMode::Stretch`.
- The baker reads back after `SETTLE_TICKS` (6) FixedUpdate ticks as well as its frame count. `RENDER` goes 4 → 5,
  so the old PNGs re-bake.
- `(receive, sync, bake, buttons)` run `.before(inspect::place)`. `inspect::target` also pre-fills the target with
  (0,0,0,−1).
- The live node spawns Hidden under the cover, already flipped for a swapped hand. After `COVER` frames and
  `SETTLE_TICKS` ticks it is shown. The cover is despawned one frame later.
- New knobs: `HST_SHOTS=dir` now works without `HST_STRESS`, saving frames 1–299. An `HST_SELECT` entry can end
  with `!` to swap that player's hand.

## Verification (captures in `context/b140/`)
- Scale/shift search of the live card against the PNG card (`HST_PREVIEW=live|png`): best fit is scale 1.0, offset 0
  at 1494×840, 1280×720, 1000×900 and 1500×600.
- Per-frame capture `frames7` (`HST_SELECT=0,3!,7,10/2!`): each card has one PNG → live step, with no blank, black
  or unflipped frame. The distance to the final frame drops in one step: 1.34 → 0.12, 0.72 → 0.11,
  1.44 → 0.40 → 0.21, 1.03 → 0.27 → 0.08 (mean abs per channel).
- Big Chief's face in the r5 bake matches the live model.

## Not verified / not 1:1
- The PNG is softer than the live model when the card isn't 384×480 physical pixels (it is bilinearly resampled), so
  the swap still shows a small sharpness step. This is inherent to a cached bitmap.
- Big Chief's shorts sway live; a still PNG can't match that (about 300 px differ).
- `COVER` + `SETTLE_TICKS` is a fixed wait, not a check that the live target has drawn. A very slow first pipeline
  compile is untested.
- The browser (OPFS) path is not tested.
- No comparison with the original: the select menu is ours to design (PLAN.md).
