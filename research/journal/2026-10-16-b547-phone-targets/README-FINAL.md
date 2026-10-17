# B547 — iPhone Safari: render targets, lobby pre-warm, low-power cap

User log 2026-10-16 (iPhone 414x896 CSS, DPR 3, Safari, Low Power Mode, online match with a mod): ~60 s load with
3–10 s frames, 28 pipelines compiling at the match start, 145 MB of textures; the menu held four 3840x2160 targets
(135 MB) at start.

## Findings
- The four targets are `hud_gamma`'s scene, HUD and the persistence's two `last` images. 3840x2160 is Bevy's default
  1280x720 window × DPR 3: on the web the canvas size only arrives with winit's ResizeObserver, so the first frames
  made (and uploaded) targets for a window the phone never had.
- `last[0..1]` were kept at the scene's size in the menus too, where nothing uses them (only `Persist`, in a match).
- The targets followed the full DPR: on a DPR-3 phone the HUD at 3× (2688x1242) and the scene at scale × 3×.
- Pipelines: the menu feeds the P55 record one a frame only after 1 s without input; in a lobby players tap around
  until the host starts, so little got fed before the match.
- Low-power 30 fps rAF cap: index.html measures the display rate (`hstRefresh`) from rAF before the game loads, so in
  Low Power Mode it reads 30; `auto_tune` times against min(refresh, 60), and `auto_timing` already tests that a 30 Hz
  display holds at 30 without stepping down. Checked, no change. Tried re-measuring on page re-show to catch Low Power
  Mode switched mid-session; reverted: a menu frame just over 16.7 ms is also exactly 30 Hz by vsync, so a slow game
  would be taken for a capped display and Auto would stop stepping down (never-30-fps rule).

## Changes
- `crates/hst/src/main.rs`: on the web the window starts at the page's CSS size (`web::page_size`), not 1280x720.
- `crates/hst-base/src/hud_gamma.rs`: images capped at 2 device px per logical px (`MAX_DENSITY`; DPR ≤ 2 untouched:
  every desktop); the scene at min(scale, cap); `last` images 1 px outside a match. On the reported phone at Phone
  preset: HUD 2688x1242 → 1792x828, menus hold 2 window-size targets instead of 4; at start none at 3840x2160.
- `crates/hst-play/src/mode.rs` `warm_up`: in an online lobby (`Net::in_room`) the record is fed one a frame without
  waiting for idle controls.
- Follow-up added: mod characters' .glb loaded on the async pool from Start (`preload.rs` only does disc characters).

## Verified
- `tools/check.sh -p hst-base hud_gamma` (new `density_capped_at_two`), `-p hst-play warm_up`; `cargo check -p hst
  --target wasm32-unknown-unknown --features ship`.
- Desktop shots (context/b547/menu.png, match.png) look as before (match.png = match-base.png from HEAD; the bare
  `--play` run draws no court either way).

## Not verified / not 1:1
- Not run on an iPhone: no device here; the DPR-3 cap and the start-up size are checked by the unit test and by
  reading winit/Bevy's window creation (logical size → canvas, CSS `!important` keeps the page's size).
- Lobby pre-warm not exercised end to end (the feed-one-a-frame path only exists where pipelines compile in place:
  browser/macOS); the change is one condition on the existing path.
- On a DPR-3 phone the Scale setting's 100 % now means 2× CSS px (67 % of device pixels); intentional, noted here.
- Gameplay untouched (rendering targets and pipeline timing only); no PCSX2 needed.
