# P28 — iOS: GPU frame time in the log

M91 build 2015 showed `prep` 13.7 ms on the palace and 10.5 ms on the one-mesh menu: Bevy's swapchain acquire
(`prepare_windows`, CAMetalLayer nextDrawable) sits inside `prep`, so a GPU-bound frame looked like CPU prep, and
RenderDiagnosticsPlugin gave only CPU pass times on Metal.

## What changed (`crates/hst/src/graphics.rs`)
- Marks 7/8 around `prepare_windows` (Render schedule, `PrepareViews`, before/after it). `acquire` is reported on
  its own and `prep` now excludes it, in the Detailed FPS overlay, its `info!` line and the new log line.
- `HST_FPS=1` (always on on iOS): every 5 s, `frame split ms: frame (worst) game sim prep acquire enc submit idle
  gpu` to stderr (→ `hst-log.txt` on the phone), its own accumulator (`SplitLog`) so the overlay's half-second reset
  doesn't eat it.
- iOS only, `gpu_time`: two tiny command buffers of our own bracket the frame's render-graph command buffers (one in
  `RenderGraphSystems::Begin`, one after `Render` / before `Submit`; both go through `RenderContext::add_command_buffer`
  so they are submitted in order with Bevy's). Each is a raw-API wgpu encoder (`as_hal_mut::<Metal>`; wgpu forbids
  mixing raw and wgpu calls on one encoder, so it records a 4-byte `clear_buffer` through wgpu-hal) with a Metal
  `addCompletedHandler`: first one's GPUStartTime, last one's GPUEndTime → ms, summed into `GPU_MS`, taken by
  `frame_start`. Values outside (0, 1000) ms are dropped. Elsewhere `gpu n/a`.
- Deps (iOS target only): `wgpu` (the patched one), `objc2`, `objc2-metal`, `block2` — the versions wgpu-hal
  already pulls in, no new crates.

## Checks
- `cargo check -p hst` (Linux) and `tools/ios-build.sh check -p hst` (aarch64-apple-ios) build clean.
- Linux release run with `HST_FPS=1` (menu, 16 s): `frame split ms: frame 5.09 (worst 17.1) game 3.59 sim 0.04 prep
  0.21 acquire 0.07 enc 0.90 submit 0.64 idle 0.00 gpu n/a` — the split adds up, acquire is split out.

## Reading it on the phone
GPU-bound: `acquire` large, `gpu` ≈ the frame time. CPU-bound: `acquire` ≈ 0, `gpu` well under the frame. `gpu`
includes any idle gaps inside the frame's GPU span (none when GPU-bound) and not the compositor's work.

## Not verified / not 1:1
- The Metal GPU time has not run on a device: there's no iPhone in this environment. It builds for iOS, but whether
  GPUStartTime/GPUEndTime come back non-zero for these tiny command buffers and the numbers are sane needs the next
  IPA's `hst-log.txt` (look for `gpu` not `n/a` on the `frame split` lines). If it stays `n/a`, the handler never
  fired or got 0 times.
- Metal may overlap the start of one frame's work with the end of the previous one on Apple GPUs; the bracket's span
  then slightly undercounts. Fine for sizing GPU levers (P29–P33), not a precise per-pass profile.
- Gameplay exactness: not affected (render-side timing only, no sim RNG or match timing touched).
