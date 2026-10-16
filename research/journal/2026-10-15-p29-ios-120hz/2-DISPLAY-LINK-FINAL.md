# P49 — iOS still at 60 fps: a display link of our own

## What the device log said
Build 2316's `hst-log.txt` (user raised the cap; `context/p49/hst-log-build2316.txt` in the main checkout): steady
16.67 ms frames for 240 s with GPU 2–5 ms, game 2.5–5 ms and 'acquire' 6–12 ms, i.e. the frame waits on a 60 Hz
vsync. So the Phone preset's 60 was not the limit; P29's Info.plist key (`CADisableMinimumFrameDurationOnPhone`) only
*allows* more than 60 Hz. iOS picks a ProMotion display's rate from what's on screen asks for (display links,
animations); wgpu-hal 29's Metal surface just `presentDrawable`s (wgpu-hal `metal/mod.rs`, no CADisplayLink) and
winit's iOS backend has none either, so nothing asked and the panel stayed at 60.

## Change
- `ios.rs` `frame_rate(fps)`: on the main queue (like `haptic`), makes once a `CADisplayLink` with a no-op target
  class (`HstFrameRate`, `tick:`) on the main run loop in the common modes and sets its `preferredFrameRateRange`
  (iOS 15+, our minimum): cap 0 (uncapped) or > 60 → {80, 120, preferred 120}; cap ≤ 60 → {cap, cap, cap}, so a
  60 cap keeps the panel at 60 Hz and its presents on 60 Hz vsyncs (P29's worry about uneven 8.3/25 ms pacing).
  Logs `display link: min–max Hz, preferred p` to hst-log.txt each time.
- `graphics.rs` plugin: a PostUpdate system calls it whenever `Graphics` changes (first frame included).
- `Cargo.toml` (iOS only): `objc2-quartz-core` (CADisplayLink, CAFrameRateRange) and `objc2-foundation` (NSRunLoop),
  the 0.3.2 versions wgpu-hal already builds.
- `main.rs` iOS log filter: `winit::platform_impl::ios::app_state=error` drops winit's "processing non
  `RedrawRequested` event after the main event loop" warning (14.5k of the log's 15.8k lines). The rest of winit
  stays at warn.
- `maximumDrawableCount`: left as wgpu sets it (from the surface's frame latency, 2–3); with 2–5 ms GPU frames it
  isn't what limits the rate.

Built: `tools/ios-build.sh check -p hst` clean; `tools/ios-ipa.sh` release build OK, IPA in `context/p49/` (Info.plist
key still true; the binary has the class and filter strings). Desktop/Android/web untouched (all `cfg(target_os = "ios")`).

## Not verified / not 1:1
- On-device frame rate at 60 / 120 / uncapped: no iPhone here. `plan/HUMAN.md` *iPhone 120 Hz* updated: install the
  IPA on the 15 Pro, Show FPS → Detailed, check ~8.3 ms frames at 120/uncapped (acquire should drop) and smooth 60
  with the Phone preset; the log's `display link:` lines show what was asked.
- If a display link alone still doesn't raise the rate (iOS ignoring a link whose target draws nothing), the next
  step is `presentDrawable:afterMinimumDuration:` in wgpu-hal's present (would need wgpu-hal patched into
  third_party) or driving the frame from a `CAMetalDisplayLink`.
- Not a gameplay change (no sim/RNG/timing code touched), so nothing to check against the original.
