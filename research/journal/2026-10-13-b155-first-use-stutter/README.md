# B155 — first-use stutters on hits and effects (desktop, cold cache)

## How it was measured
- Release build, `MESA_SHADER_CACHE_DISABLE=true` (cold driver cache every run; Bevy keeps no pipeline cache on
  disk), `HST_FPS=1` (every frame over 33 ms), started like a player does: `HST_MENU=confirm HST_KEYS="3:Enter"`
  through `mode.rs` and its loading screen. A temporary hack (removed) set `--pads -,-,-,-` so four bots rally.
- Temporary render-world logger (removed): every new `PipelineCache` entry with its time, label, shader defs,
  targets and sample count. Temporary main-world logger (removed): time of each contact (`Game::shots` change).
- One `bevy/trace_chrome` run (8.8 GB, deleted) for the slow frames that remained.
- Logs and shots: `context/b155/run*.log`, `*.png`. Machine load 5–15 (other agents' builds) throughout.

## Found
After B110 every effect mesh compiles under the loading cover, but 3–4 pipelines still compiled mid-match on
desktop, at the first umpire call ("Fault", "Out" …) and later at the first call model with morph targets:
- skinned `pbr_alpha_blend_mesh_pipeline` (and a `MORPH_TARGETS` variant),
- two `blit pipeline`s (MSAA writeback Rgba16Float ×4, blended Rgba8Unorm output).

Cause: the call models (`play/popups.rs`) sit on their own render layer, drawn only by `CallCamera`, which is
inactive except while a call is on show (P23). `warm` made their meshes visible, but no active camera drew them,
so their view's pipelines (and that camera's blit pipelines) were left for the first call. Same for any other
camera kept off until its moment (`hud_gamma.rs`' glare mix cameras).

Sounds are not it: the synth's banks are decoded at setup. No disc reads on contact (B110 checked). Meshes and
images spawned at setup are uploaded on extraction whether shown or not.

## Fix
`mode.rs`: while the loading screen is up, `warm` also turns on every inactive camera (marked `WarmingCamera`),
and `unwarm` turns them off again at the start of the next frame, before their systems run. The pipelines compile
behind the cover and `loaded` waits for them as before (98 pipelines under the cover vs ~89–94 before).

| run (cold cache, 4 bots) | new pipelines after the cover | frames > 33 ms near first contacts |
|---|---|---|
| before (run3/run4/run5) | 3 at the first call (~40 s), 1 more at ~75 s | clusters around the first call (load ~14) |
| after (run6, run7, run9) | 0 in 110–125 s each | none: first ~20 contacts (13.7–30 s in run9) all ≤ 8 ms frames |

The 33–42 ms frames left after the fix come in whole 5 s windows where the frame rate drops (≈190 → 60–110 fps)
while other agents build; they fall long after the first contacts, and the trace shows the render schedule slower
as a whole, no single span. Not first use.

## Not verified / not 1:1
- Not measured on a quiet machine: other agents' builds ran throughout (load 5–15), so "never over 33 ms on
  contact" is shown for the first contacts of three runs, not for every frame of a long match.
- `--play` from the command line has no loading screen, so no warm-up (unchanged since B110); the menu path is
  what players use.
- A match of 110–130 s doesn't reach every court/weather or a tiebreak/match-end call; those cameras and meshes
  exist from setup, so the warm-up covers them the same way, but not every one was seen drawn.
- Web build not re-measured; the change is the same code path B110 measured there.
- Seen in passing, not part of B155: every match frame logs `bevy_render::slab_allocator: Use-after-free:
  attempted to copy element data for an unallocated key` (thousands per minute), before and after this change.
