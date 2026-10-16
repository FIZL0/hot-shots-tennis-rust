# M91 — native iOS IPA built on Linux

User 2026-10-14: the browser build runs poorly on an iPhone 13 Pro; wants a native IPA they sign themselves. No Mac.

## Toolchain (all on Linux, no sudo)
- `tools/ios-sdk.py Xcode.xip` → `~/.local/share/hst-ios/iPhoneOS.sdk` (Xcode 27.1 RC → iOS 27.1 SDK, ~2 min).
  Two traps in the .xip, both handled there:
  - hard links: the bytes sit in one entry (usually MacOSX.sdk's); every other link is a same-size `NULLcanary`
    placeholder, so plain `cpio` gives 2338 garbage headers. The odc inode field (18 bits) wraps, so links are matched
    on dev + inode + size (an inode-only match put a .swiftinterface into stdint.h). Exits if two different files share a key.
  - linker stubs: Xcode 27's .tbd files list only `arm64e-ios, arm64e.x1-ios`; no lld (rustup's is 23.1) knows
    `arm64e.x1`, and there is no arm64 entry at all. arm64 apps link against arm64e system libraries (Apple's ld allows
    it), so `arm64e.x1-ios` is renamed `arm64-ios`.
- `tools/ios-build.sh [cargo args]`: clang + rustup's `gcc-ld/ld64.lld`, SDKROOT, iOS 15.0 minimum.
- `tools/ios-ipa.sh` → `target/HotShotsTennis.ipa` (unsigned, 35 MB; binary stripped 175 → 106 MB). Bundle id
  `io.github.fizl0.hst`, landscape, full screen, Files-app sharing on, icon = `web/icon-180.png`.

## Code changes for iOS
- `libc` pinned to 0.2.189 (Cargo.lock): 0.2.190 made `in6_ifreq` and the `RTM_*`/`RTAX_*` routing constants
  macOS-only, which breaks `netdev`/`netwatch` (iroh) on iOS. Unpin once they adapt.
- `bevy/system_clipboard` only off iOS (arboard has no iOS backend; the in-app clipboard remains).
- `rfd` only on desktop; iOS's Add mod says to use the Files app (`modstore::pick`).
- `disc.rs`: data dir = `$HOME/Documents` on iOS; no import before the window (launch watchdog): runs from the pack
  if present, else straight from the first `*.iso` in Documents.

## First device runs (iPhone 15 Pro, iOS 27.0.1)
- Build 2006: black screen, closes. A folder the user makes in Files isn't the app's; iOS lists the app's own folder
  only once the app has written a file there. Fix: `ios.rs` writes `Documents/hst-log.txt` (stderr, panics, the
  engine's log through `LogPlugin::custom_layer`) every launch, and shows a "where the ISO goes" page instead of exiting.
- Build 2007: the log reached Metal (A17 Pro) and stopped; the crash report (`hst-….ips`) is EXC_BREAKPOINT in
  `_UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption`: iOS 27 refuses apps without the UIScene lifecycle.
  winit 0.30.13 (bevy 0.19's) has none. Fix: Info.plist `UIApplicationSceneManifest` (no delegate class) and
  `third_party/winit` puts its windows in the scene (`window.rs` adopt_scene / scene_connected on
  `UISceneWillConnectNotification`; Bevy makes its window before the scene connects). Waiting on the next device run.

## Not done / next
- Input: gilrs has no iOS backend and the touch controls are `web/touch.js` (browser only), so the app reaches the
  menu but can't be driven. Next: a GameController-framework bridge (Xbox/PS/MFi pads), then native touch controls.
- No ISO in Documents → the app exits (message only on stderr). Wants an in-app screen.
- LAN discovery (mDNS) needs Apple's multicast entitlement, which free signing can't get; internet play should work.
- Not yet run on a device: **waiting on the user** to sign, install and launch it.

## Build 2010: gamepads
Build 2008/2009 open on the 15 Pro (scene fix works) but had no controller input: gilrs has no iOS backend. Added one
in `third_party/gilrs-core/src/platform/ios/`: polls `GCController.controllers` → extended gamepad each frame, emits
gilrs events with the default codes (A/B/X/Y = south/east/west/north, triggers as 0..1 axes). No-disc hint shrunk
(VMin 3.5, shorter text). Waiting on the user: does a paired controller drive the menu and a match?

## Build 2012: perf log, no-disc page fullscreen
Controller confirmed working on the 15 Pro. The `HST_FPS` log is on by default on iOS (fps, slow frames, assets, draws
every 5 s into hst-log.txt), with CPU ms/frame from getrusage and NSProcessInfo thermal state. Settings → Show FPS →
Detailed adds the frame split (game/sim/render). The no-disc hint was centred off-screen (the page's default 1280×720
window is bigger than the phone): it now uses borderless fullscreen like the game; the → glyph (missing from the font) is now >.

## Build 2014: first device profile (15 Pro, 100% scale, Low shadows, 2x MSAA)
hst-log.txt from build 2012: a match holds 60 fps (game ~5 ms, render ~9.5 ms of 16.7). Every drop is one of the
umpire's call models (Fault, Out, Change Sides; `play/popups.rs`, overlay camera on CALL_LAYER): each one, ~1.1 s
of 30–40 ms frames, render +5–7 ms, game +1.5 ms, 12 more meshes. Ruled out: MSAA mismatch (the call camera is
a `hud_gamma::Scene` camera, same samples), sun shadows (filtered by render layer), CPU morph (a few dozen
vertices). Big stalls only at loads (match start 1.5 s, result → menu 0.9 s). winit logged every redraw at info
(37k lines): now winit=warn. Added RenderDiagnosticsPlugin on iOS, logging the 8 slowest passes every 0.5 s
(GPU time if Metal gives timestamps), and prep/enc/submit in the frame split. Next log says which pass the call
model costs.

## Build 2015: second profile (Grand Indoor Palace, stage 7, same settings, fps cap 120)
40 fps through the match (25 ms frames): game ~5 ms, render 19 ms = prep 13.7 + enc 3.1 + submit 1.9. `prep`
(main app end → render graph begin) includes Bevy's swapchain acquire (`prepare_windows`), i.e. waiting for the GPU:
the menu, one mesh at 60 fps, shows prep 10.5 ms too. CPU work is ~11 ms a frame, so the palace is GPU-bound at 100%
scale (2556×1179, 2x MSAA, HDR scene image). Metal gave no GPU timestamps through RenderDiagnosticsPlugin (CPU-only
pass times, all <0.1 ms): removed again. The 20.8 s "frame" was the app suspended (lifecycle Suspended/Running).
Next: A/B on the palace at Scale 50–70% and MSAA off to size the GPU levers.

## P28 (2026-10-15): acquire split out of prep, GPU time from Metal
The `HST_FPS` log now has a `frame split ms` line every 5 s with `acquire` (the swapchain wait, no longer in
`prep`) and `gpu` (Metal GPUStartTime→GPUEndTime over the frame's command buffers). Untested on a device:
journal 2026-10-15-p28-ios-gpu-time.
