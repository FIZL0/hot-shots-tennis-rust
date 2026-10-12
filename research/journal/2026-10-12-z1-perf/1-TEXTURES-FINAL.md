# 1 — texture replacements off the critical path (FINAL)

## How it was measured
- `--features bevy/trace_chrome` build in `CARGO_TARGET_DIR=target/trace`, `TRACE_CHROME=…json`, with
  `RUST_LOG='info,bevy_ecs::schedule::executor::multi_threaded=warn,bevy_render::batching=warn,bevy_render::renderer::render_context=warn,bevy_ecs::query=warn'`
  (`RUST_LOG=error` drops every span; unfiltered traces are 8–13 GB a minute). A small streaming script printed, for
  each main-app frame over 30 ms, the spans in it.
- `HST_FPS=1` now also prints every frame over 33 ms as it happens (`slow frame X ms at T s`).
- Stack sampling: eu-stack gave empty stacks (ptrace scope); gdb as the parent (`thread apply all bt` on SIGINT
  every 50 ms) works.
- Runs: menu (`HST_KEYS`, `HST_MENU=chars`), select previews with and without the bake cache, a 110 s bot doubles
  match (`--play --stage 1 --court 1 --pads -,-,-,- --chars 3,0,1,2`), leaving the match back to the menu.
- The machine ran other agents the whole time (load 15–25 on 16 cores): single frame times swing by 2×; the
  per-system spans were used to tell our work from contention.

## Found → fixed
| where | before | after | fix |
|---|---|---|---|
| `textures::ui_rects` in the select | 30–50 ms on many frames | gone | `add` no longer holds the texture `STATE` lock while it decodes a replacement PNG (UI waited on worker threads' decodes) |
| `textures::reload` | up to 144 ms once a second | gone | the mod folder walk (5830 PNGs) runs on the IO pool; the diff/apply is unchanged |
| `previews::bake` | 22–25 ms per baked preview | gone | the preview PNG encode runs on the IO pool; the next bake waits for the file |
| match load frame | 1.24–2.3 s (339 replacement PNGs ≈ 1.49 s decoded one at a time, most twice: lit + raw) | 0.76–1.06 s | `textures::prefetch(&mtl)` decodes a model's replacements on all cores before its `add`s (court models, characters, rackets, effects, tornado); a guard drops them after |
| `main_menu::setup` (startup and every return to the menu) | ~300 ms (60 ms of it without upscales) | 150–380 ms under load; 161 ms vs 149 ms `--no-upscale` on a quiet moment | `textures::prefetch_tim2` decodes the ~50 menu sheets' replacements at once |

Same files and the same pixel conversion (`texhash::Files::load`, the old `Overrides::load` body), so every texture
is the same as before. `tools/check.sh`: 360 tests passed (replay tests included).

## Left as is
- In-match: steady, slowest frame 9–30 ms over 110 s; the one 518 ms spike seen once didn't come back (contention).
- The ~67 ms first-preview-spawn frame from the brief did not show in any trace (cache empty or full).
- Match load is still one blocking Startup of ~0.8–1 s behind the static loading screen (`mode.rs`): 
  `hst::load` ~290 ms, `play::setup` ~310 ms, then the PostStartup setups (panel, npcs, popups, menu, match_stats,
  banners, foot_fx, 30–90 ms each) run one after another. Samples show no single hotspot left (spread over disc
  motions, court world, pose maths, xb inflate).

## Not verified / not 1:1
- The loading screens still freeze for the blocking Startup (~0.8–1 s match, ~0.15–0.4 s menu): moving mode
  Startup off the main thread is a redesign of `mode.rs`, not done.
- All timings are from a machine at load 15–25; before/after numbers are the trace's per-system spans, not clean
  benchmarks.
- The 67 ms first preview frame wasn't reproduced, so it isn't proven fixed.
- Not prefetched: the tornado's raw-image adds outside its model, `mods.rs` character mods' `materials_of`, and
  the match PostStartup `panel::image` sheets (popups, match_stats, banners, panel) — each still decodes in turn.
- Visuals: unchanged by construction (same decode code, same pixels), not screenshot-diffed.
