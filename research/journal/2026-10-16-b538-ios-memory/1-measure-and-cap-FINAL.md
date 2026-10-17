# 1 — measure, cap the replacement textures on phones, log the phone's memory

## Measuring (desktop, RX 9070, `--stage 1 --play`, 2048×1152 window)
`context/b538/mem.sh` samples the process's RSS and the GPU memory (amdgpu `fdinfo`, per DRM client) every 0.5 s.
The new `textures held` line (`stall.rs`, every 5 s with `HST_FPS`) adds up the render world's textures, all levels.
Trap: the runner's `HST_BG=1` forces the upscaled textures off (`main.rs`), so the first runs measured disc textures
only. Run with `env -u HST_BG` and a `settings.txt` beside the ISO that has `upscaled_textures = on`.

| match, stage 1 | textures held | GPU (VRAM) | RSS peak |
|---|---|---|---|
| disc textures only | 127 MB | 567 MB | 379 MB |
| HD pack (`mods/hst-hd`, iOS default: upscaled textures on) | 506 MB | 1157 MB | 633 MB |
| HD pack, capped at 1024 | 445 MB | 1157 MB | 654 MB |
| HD pack, capped at 512 | 278 MB | 895 MB | 542 MB |

- On a phone the GPU's memory is the app's memory: iOS counts both together against its limit. So the HD pack alone
  puts a match near 1.8 GB of footprint here, before the phone's own costs (bigger window, Metal).
- The HD pack has 4584 PNGs: 1911 at 512, 961 at 1024, 161 at 2048. Mods' upscaled sets go further: B227 measured
  4K-class PNGs, up to 512 MB of them for one store character. A mod character's images are uploaded twice too (lit
  and raw, `mods.rs`), so one 4096² texture costs 134 MB of GPU memory.
- Upscaled Textures and Upscaled Downloads are both on by default (`main_menu.rs SETTING_DEFAULTS`). On iOS that
  includes the store's upscaled sets. Only the browser leaves them out (B227).
- No leak across mode switches: six menu → match → menu cycles (`cyc3`) stayed at 105–230 MB of textures. The
  menu's count varies with the cards it shows, and it doesn't keep growing.
- The disc stays on disk (`Iso` reads by offset), and images keep no CPU copy (`RenderAssetUsages::RENDER_WORLD`;
  3.9 MB of image data on the CPU in a match).

## Changes
1. **Phones cap replacement and mod textures at 512 px a side** (`textures.rs` `max_side`/`fit_pixels`/`fit`;
   halved with a 2×2 box filter). Covered: texture packs and `texture-replacements/` (in `Files::load`, so the
   prefetch holds the smaller pixels too, and the UI's pixel-rect scale follows the new size), and mod characters',
   faces' and courts' glb or upscaled images (`mods.rs`, `court_mod.rs`). Desktop and the browser are unchanged.
   `HST_TEXTURE_MAX=N` sets the cap anywhere (0 = none) for testing. A/B at a phone-shaped 1280×590 window (HD pack,
   uncapped vs 512, `context/b538/ab/ab.png`): no difference I can see, and the HUD's replaced icons sit right.
   Unit test `textures::tests::halves_to_the_cap`.
2. **The phone's memory goes into `hst-log.txt`** (`ios.rs`): `memory()` reads the phys footprint, the number
   iOS's limit counts, from `task_info(TASK_VM_INFO)` (and its kernel peak), plus `os_proc_available_memory()`
   (what's left before iOS closes the app). Logged at start, every 5 s, on every lifecycle change, at each new high
   64 MB above the last, and on iOS's memory warnings (`UIApplicationDidReceiveMemoryWarningNotification`). The
   next log shows the peak, where it happened, and how close it came.
3. `stall.rs` `textures held` (HST_FPS, always on for iOS): the textures' total and the six biggest every 5 s.

## Not verified / not 1:1
- **Not run on an iPhone**: no device or Mac here. The iOS code compiles for `aarch64-apple-ios`
  (`tools/ios-build.sh check`), but `memory()`, the warning observer, and whether the cap keeps the 15 Pro under its
  limit are unchecked on a device. The cause (jetsam over the memory limit) is inferred from desktop numbers and
  iOS's settings defaults. The next device log's `memory … MB left` lines will confirm it or point elsewhere.
- **Mod characters' 4K upscaled sets not measured**: none are installed here (only courts' sets, ≤ 41 MB). The cap
  applies to them through the same `fit`.
- **Not done**: textures added twice (lit + raw, disc and mod) could share one GPU texture through a view format.
  That would save up to half of it again, but it changes how GS materials bind their textures, so I left it for a
  measured need. Window-sized render targets and the 2048² sheet (~75 MB) are unchanged.
- Rendering only has to look right (PLAN): the A/B shot is the check. No gameplay code touched.
