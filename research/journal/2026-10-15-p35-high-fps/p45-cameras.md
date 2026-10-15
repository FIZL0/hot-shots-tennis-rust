# P45 — cameras and queue submits (2026-10-15)

In Bevy 0.19, every render-pass system that takes a `RenderContext` records its own command buffer, and
`submit_pending_command_buffers` sends them all in one `queue_submit` (`count=18` in the P35 trace). Each active
camera also runs its camera schedule. A Core2d camera runs clear/opaque, transparent, tonemapping and upscaling (a
blit into the window), plus `ui_pass`, which runs for every camera that has a UI view.

The cameras of a match before:

| order | camera | job |
|---:|---|---|
| 0 | scene `Camera3d` | court, players → `scene` |
| 1 | pop-ups `Camera3d` | call models; already inactive unless shown (`popups.rs`) |
| 48/49 | persistence `Camera2d` (one active per frame, Hdr) | full-screen `UiMaterial` node: scene + last frame → `last[i]` |
| 50 | HUD `Camera2d` | the UI → `hud` |
| 51 | out `Camera2d` | full-screen `UiMaterial` node: shown + hud → window (viewport, black clear) |

That is three Core2d schedules a frame just to draw two full-screen triangles. `widescreen.rs` spawns no camera.
The main menu's cursor camera and the touch pad camera are already inactive while they have nothing to draw.

## Change

`hud_gamma/out.rs`: the persistence and window cameras are replaced by one camera (order 51) on a render schedule of
our own, `Out`, which has a single system, `draw`. In one command buffer it:

1. in a match (`Frame::mix`), mixes the scene and `last[1 − i]` into `last[i]` (the old `persist.wgsl`, now the
   `PERSIST` branch of `hud_gamma.wgsl`);
2. clears the window black and draws the composite of `last[i]` (or the bare scene) with `hud`, only inside the
   camera's viewport (B117's fixed aspect ratios).

The pipelines are specialized in `prepare` (`Rgba8Unorm` for the mix, the window's swap-chain view format for the
composite). The weight uniform is written there. The bind groups are cached while the texture views stay the same.
The window's attachment comes from `ViewTargetAttachments`, not from the raw swap-chain view: a Bevy screenshot
redirects that attachment to its capture texture, and taking the raw view gave black `--shot`s. `hud_gamma.rs` loses
the two `UiMaterial`s, their plugins, the persistence cameras, the composite node and the material refresh on resize.
`persist` now only sets `Frame::mix`.

The HUD camera stays a Core2d camera: it draws the UI. It always has something to draw in a match (score, speed,
balloons), so deactivating it when empty would gain little, and would need a composite variant without `hud`.

## Measured (`tools/profile.sh`, 15 s, 4-bot doubles court 1, traced)

Load average was 20–28 on 16 threads, and much higher during the after run (279 traced frames vs 742). The passes
this task doesn't touch (Core3d's `unpack_bins`, shadow, opaque, transparent) came out ~30 % slower in it, so compare
counts, and times only relative to those.

| | before | after |
|---|---:|---:|
| Core2d schedule runs / frame | 3 | 1 |
| Core2d inclusive | 842 µs | 416 µs |
| `Out` inclusive (incl. `draw` 40 µs) | — | 143 µs |
| command buffers per submit | 18 | 13 |
| `upscaling` / `main_transparent_pass_2d` calls | 4 / 3 | 2 / 1 |
| `ui_pass` | 4 calls, 61 µs | under the table's top 80 (< 34 µs); 2 views left (scene, HUD) |
| `unpack_bins` (untouched, for the load) | 296 µs | 389 µs |

Core2d + Out takes 559 µs vs 842 µs, even under the heavier load. Five fewer command buffers and four fewer render
passes per frame. The `queue_submit` time itself (395 → 517 µs) went up with everything else under the load, so it
can't be compared from these runs.

Tried and dropped:
- `CompositingSpace::Srgb` for the UI: Bevy's UI render ignores it.
- Copying the swap chain as the persistence's last frame: its usage is `RENDER_ATTACHMENT` only.
- One pass writing two targets (MRT): the window and `last[i]` differ in size (render scale, viewport).
- Single-threaded executor for the Core2d/Core3d schedules: Core3d got slower in wall time (RenderGraph 4582 vs
  3186 µs). Core2d was inconclusive under the load. The experiment code is removed.

## Checked

A/B `--shot`s, old binary vs new (`HST_WEATHER_SEED=1234 HST_AUTOPLAY=1`, court 1, 4 bots), in `context/perf/p45/`:

| shot | identical | within 2 | > 16 |
|---|---:|---:|---:|
| match at 6 s (persistence on) | 54 % | 98.2 % | 0.2 % |
| `--4x3` at 4 s (viewport, black bars) | 77 % | 99.6 % | 0.02 % |
| `--original --scale 0.5` at 4 s | 90 % | 99.7 % | 0.002 % |
| main menu (no persistence) | 99.9 % | 99.9 % | 0.1 % = the version string |

The larger differences are on moving players and the ball (wall-clock `--shot-at` catches different frames). The
HUD, the pillarbox bars and the blend over the court look the same. `cargo check --target wasm32-unknown-unknown`
builds clean.

### Not verified / not 1:1

- Rendering only: no sim, RNG or match timing is touched, so there's no PCSX2 check.
- The A/B shots aren't pixel-identical: the two runs were caught at different moments (wall-clock `--shot-at`). The
  differences are on moving things; the static scene, the HUD and the bars match by eye.
- The mix writes `last[i]` (`Rgba8Unorm`) directly. Before, the persistence camera drew it through an Hdr
  (`Rgba16Float`) intermediate and an upscaling blit. Both store 8 bits in the end, so at most rounding differs (≤ 1
  level, within the A/B noise).
- Only run on the desktop Vulkan path (Linux). The wasm build compiles, but WebGL2/WebGPU, iOS (Metal), Android and
  Windows weren't run. The composite uses the swap chain's own view format, so an sRGB/non-sRGB swap chain is
  handled like Bevy's upscaling does, but that isn't checked on those platforms.
- The frame-time gain can't be measured cleanly on this loaded machine (see the load note above). The call and
  submit counts are exact.
