# P47 — match-start hitch: pipelines compiled behind the loading screen, a frame at a time (2026-10-15)

## Where the ~950 ms frame went
Measured again first (desktop, release, `tools/bench.sh`, `HST_FPS=1`; logs in `context/p47/`):
- **`--play`, default (async compiles):** started in 17 frames / 449 ms, the cover came down after 15 more; every
  pipeline (145) had landed before it; in 150 s of match nothing compiled after it and no frame over 33 ms. The old
  953.9 ms frame was the one-frame start that P50 sliced up.
- **Cold driver cache** (`MESA_SHADER_CACHE_DISABLE=true`): start 47 frames / 1029 ms, worst 225 ms, all behind the cover,
  nothing after it.
- **Menu → match → menu → match** (`HST_MENU=confirm`, `HST_KEYS`, script `context/p47/menu.sh`): the second match
  compiled only 4 new pipelines (another stage) and its cover came down after 6 frames: Bevy's pipeline cache already
  keeps them across matches in one run.

So on Linux/Windows/iOS (async compiles) `mode.rs`'s warm-up (`warm`, P30) already does what the task asks. Where Bevy
compiles **on the render thread** (the browser and macOS: `create_pipeline_task`'s cfg), it doesn't: the warm-up queues
all the match's pipelines at once and `process_queue` compiles every one in that frame. Reproduced on Linux with the new
`HST_SYNC_PIPELINES=1` (`RenderPlugin::synchronous_pipeline_compilation`, `main.rs`) and a cold Mesa cache:
**one 1107 ms frame** (`pipeline queue 1020.6 ms`, 65 pipelines) and one 507 ms frame (`queue+prepare`: the
upscaling node's `block_on_render_pipeline` runs the whole queue too). The loading screen stands still meanwhile
(B415's "Not responding" on the web).

## Fix
- `third_party/bevy_render` `PipelineCache` (already patched): `set_compile_budget(Some(d))`. When pipelines compile
  in place, a queue pass stops starting new ones once `d` is spent (at least one per pass), and the rest stay
  waiting for the next frame. `block_on_render_pipeline` still compiles the one it waits for if the budget left it
  queued. With no budget, or with async compiles, it behaves as Bevy does.
- `mode.rs`: sets an 8 ms budget (`COMPILE_BUDGET`). The cover already waits for the waiting list to drain
  (`Compiling`), so the deferred pipelines still land before it comes down.

## Result (sync + cold cache, `--play`)
| | worst frame | worst pipeline-queue pass | cover down at |
|---|---:|---:|---:|
| before | 1107 ms | 1020.6 ms | 3.12 s |
| after | 170 ms (a game-side start update, P50/B415) | 78–95 ms (one cold shader) | 2.51 s |

Menu → match → menu → match under sync + cold: both matches drew everything (`context/p47/s1.png`, `s2.png`).
The second match compiled nothing and its cover came down after 7 frames. Default async run after the change: same as before
(start 424 ms, cover down after 21 frames, nothing after). `cargo check --target wasm32-unknown-unknown` passes.

## Not verified / not 1:1
- **Not run in a browser or on macOS.** Their synchronous compile path is reproduced on Linux/Vulkan through
  `synchronous_pipeline_compilation`. WebGL2/WebGPU and Metal per-pipeline compile times differ: a single slow
  shader still takes a whole frame (one pipeline can't be split; ~80–95 ms cold here).
- No automated test: the budget needs a GPU device and real compiles. Measured from the `HST_FPS` logs instead.
- Mid-match on the web, a first-time pipeline now shows a frame or more later instead of stalling that frame. This
  is rendering only and doesn't touch the sim. None appeared after the cover in these runs.
- Not persisted across launches: Bevy 0.19 has no on-disk pipeline cache. The drivers' own shader caches (Mesa,
  Metal, the browser's) do that part. "Cache across matches" holds in-process (second match: 0–4 new compiles).
- The menu compiles a few preview-bake pipelines just after its cover (38–48 ms frames under sync + cold). These are front-end
  pipelines, not the match's, and are left as they are.
- The remaining 100–230 ms frames behind the cover are game-side start systems (court load, `main_menu::setup`),
  P50/B415's area, not pipelines.
- Loading screens/presentation don't have to match the original (PLAN.md), so there's no comparison against the PS2.
