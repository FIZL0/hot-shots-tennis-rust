# B137 — bounce effects bugged on mobile

## Reproduction attempts (none showed the bug)
Court 1, singles bots (`play&singles&stage=1&pads=-,-`), phone preset (scale 67, no MSAA), bursts of 8 shots/s
over a rally (captures in `context/b137/`):
- native Vulkan (RX 9070 XT), 844×390 window: `nat1/`
- web WebGL2 (ANGLE on Vulkan), `MOBILE=1` touch viewport: `run6/`
- web WebGPU, `MOBILE=1`: `run7/`

All three look the same: the yellow fleck ring, a small pale mark (court 1's mark colour from GAME.BIN is
(117,114,98), so pale is right), a small dust puff, and the red landing marker. No real phone was available.

## What differs on a phone: the hardware depth bias
Every effect material is a `StandardMaterial` with `depth_bias` set (`AFTER_SCENE` = 500 for the impacts, sparks,
trails, flight ribbon and bounce effects; `1e3 + rank` for the landing markers' layers). We only want that for
Bevy's transparent sort (`distance + depth_bias`). But `StandardMaterial` also writes it into the pipeline as
`depth_stencil.bias.constant`. The unit of that bias is up to the GPU:
- Vulkan/D3D with float depth: r = 2^(exponent of the depth − 23). With reverse-Z at 20–40 m (depth ≈ 0.005), 500
  units come to about 2e-7: nothing.
- Metal (iOS Safari, both WebGPU and ANGLE-backed WebGL2) and some GLES drivers: a fixed step of about 2^-24, so
  500 units ≈ 3e-5 in depth. With a 0.1 m near plane that is about 0.1 m at 20 m and 0.5 m at 40 m. For the
  markers' 1e3+ units it doubles. The ground-level marks, puffs and markers then pull toward the camera and draw
  through the players' feet and the net.

The earlier phone bugs (B109, B121) were on an iPhone, so this fits the report. It is the only path we found where
the bounce effects can render differently on a phone with the same data and code.

Emulating it natively (`AFTER_SCENE` raised 128× to 64000, `emu/`) showed nothing clear at screenshot size,
because the desktop unit shrinks with depth.

## Fix
`effects::Fx` = `ExtendedMaterial<StandardMaterial, SortOnly>`. `SortOnly::specialize` zeroes
`depth_stencil.bias.constant` after the base material has set it. The sort offset stays the same (`depth_bias()` is
delegated to the base), so the draw order is unchanged on every platform. The hardware bias is now 0 everywhere,
which on desktop is what it already was in practice.
- All in-match effects (`load`, `load_sparks`, `load_trails`, `load_flight`, `load_marks`, `load_bounce`) build
  `Fx` (setup passes a new `Assets<Fx>`; `MaterialPlugin::<Fx>` is registered in `play::plugin`).
- `Shown`, `model`, `look` and `pose` are generic over `EffectMaterial` (default `StandardMaterial`), so the menus,
  pop-ups, intro, match stats, NPCs and tornado keep their `StandardMaterial`s unchanged.
  `character::skinned_parts` is generic over the material's asset type.
- Checked after the fix: native (`fix1/`) and web WebGL2 with `MOBILE=1` (`run8/`) draw every effect as before, with
  no new console errors.

## Not verified / not 1:1
- The bug itself was never reproduced. No phone or Metal device was available, and desktop Chrome has no Metal or
  mobile GPU profile. The mechanism (the GPU's own depth-bias unit) comes from the specs and the Bevy source, not
  from watching it on a phone. Whoever has the phone: check that bounce marks, puffs and landing markers no longer
  show through players or the net.
- The head markers, serve balloons and surprise icons (`markers::LAST` = 1e4, `play.rs` balloons, `surprise.rs`)
  are still `StandardMaterial` with a hardware bias, and are 20× bigger than the bounce effects'. They float above
  the court, so a shift toward the camera mostly keeps them on top anyway, as intended. Left alone to keep this task
  to the bounce effects; the same `Fx` swap applies if they show the problem.
- No new comparison with the original (`tools/screenshot.sh`): desktop rendering is unchanged by this fix, and the
  bounce look itself was already checked when B13/bounce effects landed.
- Bevy's mesh allocator logs "Use-after-free: attempted to copy element data for an unallocated key" every frame
  (native and web alike). It is not related to the bounce look; noted only.
