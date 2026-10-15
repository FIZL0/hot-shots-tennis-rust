# B411 — Settings → Graphics → Advanced; Sway LOD removed (2026-10-15)

## Sway LOD re-measured: no gain, removed

The machine wasn't idle: load average 24–30 from the other parallel agents. So, as in B389, I used numbers that
don't depend on load. Runs and scripts are in `context/b411/` (`run.sh`, `fit.py`, `webrun.sh`, `webfit.py`).

- **Native** (`tools/bench.sh`, 1280×800, court 3, 4 bots, 45 s; `tools/instr.py PID 25 all` over s 15–40; fitted
  instr/s = a_cfg·fps + b with b shared), 6 interleaved runs per setting:
  Sway LOD on **38.38 ± 0.28** M instr/frame, off **38.40 ± 0.37**. No difference.
- **Web, phone class** (`web_check` MOBILE=1 THROTTLE=4 WEB_TRACE=25:25, Vulkan ANGLE flags from part 1,
  `?webgl2&play&stage=3&court=3&chars=0,1,2,3&sway_lod=on|off`, WebGL2 build with the switch). The same fit on page
  instructions:
  - Set 1 (12 runs, the first 6 overlapped a wasm build): on 132 ± 15, off 145 ± 13 M/frame.
  - Set 2 (6 runs, quiet): on 81.7 ± 6.0, off 78.0 ± 6.7 M/frame.
  - GPU busy was slightly higher with it on in both sets (1.48 vs 1.37, 1.43 vs 1.23 ms/frame).
  - The two sets point opposite ways, each within its own error.

  The expected saving is small anyway. B344 measured about 30 µs a frame native, around 0.3 % of the page's work,
  which is far below the run-to-run spread.

Verdict: it doesn't help anywhere, so it's removed the B389 way:
- `noise.rs`: `LOD`, `LOD_FLAG`, `set_lod`, `due`, `Cams`, and the camera/frame inputs of `draw`. The once-per-tick
  rewrite B344 also made stays.
- `main.rs`: `--sway-lod` / `--sway_lod`. This also drops the browser's `?sway_lod=` switch, which goes through the
  same args.
- `main_menu.rs`: the `sway_lod` key. `Settings` keeps 8 switches and `UPDATE_CHECK` is now 7. An old settings.txt's
  `sway_lod` line is ignored (tested).
- `options.rs`: the `SwayLod` row.

## Advanced page
- Display now has only Display/Renderer, Aspect Ratio, Menu Size, Show FPS, Key Hints and Button Icons.
- Graphics is Preset, Resolution, Shadows, Anti-aliasing, Frame Limit, VSync, Original Mode, Upscaled Textures, then
  **Advanced** (value `>`, legend "Open").
- ✕ on Advanced opens `Screen::Advanced` (Mesh, Fewer Shadows, Skip Off-Screen). The Graphics tab stays lit there, and
  L1 / R1 go to Graphics' neighbours. ○ goes back to Graphics' Advanced row. The footnote reads "Graphics > Advanced:
  changes apply right away". `HST_MENU=advanced` opens it.
- Pause-menu Settings uses the same pages, so the change applies there too.
- Tests: `every_page` walks into Advanced, `advanced_under_graphics` is new, `pages_fit` covers Advanced, and the
  legend test covers it. Screenshots at 1280×720 and 600×1000 look right: `context/b411/menu/`.

## Not verified / not 1:1
- **WebGPU** wasn't measured. The headless Chrome page fell back to WebGL2 even without `?webgl2`. Sway LOD's work
  happens on the CPU and doesn't depend on the renderer, and the CPU side showed nothing on native or WebGL2.
- **A real phone** wasn't measured. A 4×-throttled Chrome on a loaded desktop stood in for one (as in B328/B389).
- The menu is ours to design, so it isn't compared against the original.
