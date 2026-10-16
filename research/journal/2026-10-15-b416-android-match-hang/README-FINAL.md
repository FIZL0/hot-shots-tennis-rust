# B416 — Android APK hangs loading into a match

Whose move: the human (phone retest). No code change.

## What the 2026-10-15 log says (context/android/hst-log-previous.txt, SM-S931U, Adreno 830)

- Match start at 22.9 s: `--court 5 --shadows high --msaa 4x`. Loading itself is fine: disc reads and spawns take
  50–90 ms steps, `hst::mode: started in 140 frames, 1635 ms` at 25.0 s, frames at 8–9 ms with 469 meshes and 101
  shadow casters per cascade. The applause plays because loading has finished and the match is running.
- About 0.3 s later one frame's `graph+submit` takes 10.6 s, the next one 20 s. Queue/prepare/assets stay at 1–5 ms,
  so the CPU isn't the problem. The GPU stops responding: Adreno logs 'Fence didn't signal in 3000 ms', then DeviceLost,
  then wgpu's 'SwapchainAcquireSemaphore still in use' panic on quit. The ANR comes from the stuck main thread.
- That APK (15:11) still used Bevy's GPU-built indirect draws on Adreno. On a non-bindless GsMaterial that means one
  `bin_unpacking` compute dispatch per batch set (~350 a frame), followed by multi-draw-indirect-count.
  The same build also lost the ball FX/outline on Adreno, so the driver was already mishandling that path.

## Status at HEAD

- P48 (merged, f0f32ff5) puts `NoIndirectDrawing` on every 3D camera (`crates/hst/src/graphics.rs`, `direct_draws`)
  unless `HST_INDIRECT=1`. So Adreno no longer gets indirect draws at all.
- The user's c19cb239 APK (after P48) plays well with the Phone preset, and the ball FX/outline are back. That's
  the same build, so the indirect path is the likely cause of this hang too.
- Left to do: play shadows=high + msaa 4x on court 5 with a post-P48 APK. No phone was attached to this run
  (`adb devices` was empty), and the emulator isn't an Adreno, so it can't reproduce a driver hang.

## If it still hangs with a post-P48 APK

Bisect on the phone as the task says: shadows off, then msaa off. The log's `pipeline at` lines show which pipelines
first get used around the hang. One suspect is the msaa-4 pipelines that finish compiling right at match start
(`compiling 9` at 25.31 s, `compiling 2` at 36.20 s).

## Not verified / not 1:1

- The fix isn't verified on the device: no phone attached. Retest request in plan/HUMAN.md.
- Nothing to match 1:1 here: rendering/perf only.
