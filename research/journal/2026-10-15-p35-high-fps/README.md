# P35 — high frame rate on anything (user 2026-10-15)

User: an RX 9070 XT / 16-thread desktop gets ~250 fps in a match while CS2 runs 600–700; monitor is 240/480 Hz.
Goal: "so optimized it can run on anything and it can run HIGH fps". The sim stays 60 Hz; drawing interpolates
between ticks (`overstep_fraction`: ball, players, animation, camera), so fps above 60 is visible smoothness up to the
monitor's refresh.

## Baseline (2026-10-15, main 4ed31e84, `tools/bench.sh`, 4-bot doubles, court 1, 1280×800, defaults)

- ~250 fps, `cpu 8.8 ms/frame` (summed over threads). GPU passes (`HST_PASSES=1`) total ~1.3 ms: **CPU-bound**,
  the GPU is mostly idle.

## Per-system profile (`tools/profile.sh`, Bevy trace_chrome; tracing ~halves fps, compare relative costs)

Pipelined rendering: main world (~2.7 ms traced) and render world (~4.5 ms traced) run in parallel, so the render
world sets the frame rate. Top self time, µs per frame:

| µs | where | |
|---:|---|---|
| 619 | `write_binned_instance_buffers<Shadow>` | 4 shadow cascades × every caster, rewritten each frame |
| 517 | `prepare_preprocess_bind_groups` | bind groups rebuilt every frame (buffer churn?) |
| 410 | `hst::play::control` (FixedUpdate) | ~1 tick/frame here, so ~400 µs **per sim tick**: bot AI / input |
| 277 + 63 + 37 + 34 | `ui_layout_system`, `ui_stack`, `update_clipping`, `ui_focus` | UI tree every frame |
| 252 | `prepare_clusters_for_gpu_clustering` | clustered lights |
| 211 | `per_view_shadow_pass` encode | 80/43/38/33 µs for cascades 0–3 |
| 185 + 118 + 68 | visibility `par_for_each`s, `check_visibility_cpu_culling` | entity count |
| 173 | `hst::vertex_patch::write` | per-frame vertex writes |
| 168 | `write_binned_instance_buffers<Opaque3d>` | |
| 114 | `hst::character::animate` | |
| 108 | `hst::noise::draw` | |
| 91 + 47 | `prepare_skins`, `extract_skins` | |
| 72 | `hst::play::bodyhit::pose` | |
| 52 | `hst::play::foot_fx::tick` | |
| 32 | `hst::play::net_hide::hide_around_eye` | |
| 22 | `hst::play::main_menu::cursor::tick` | menu cursor running in a match |

All `hst::` systems together: ~1.3 ms/frame self (traced). Follow-ups: P36–P48 in `plan/TODO.md`.

Rerun: `tools/profile.sh context/perf/<name>.txt [seconds] [hst args…]` (builds `target/trace`, ~2.5 min first time).
`samply` needs `perf_event_paranoid` ≤ 1 (root); trace_chrome needs nothing.

## P38 — `hst::play::control` per tick (2026-10-15)

Measured headless, release build, 4-bot doubles on court 1 (`crates/hst/src/play/tick_cost.rs`:
`tools/check.sh --release -p hst --bin hst four_bot_control_cost -- --ignored --nocapture`, `HST_TICKS=N`; prints
µs/tick of `control` and an FNV hash of every tick's `state_hash`). Baseline 420 µs/tick, same as the trace.

Timing every call inside `bot` showed that ~99% was `doubles_ai::step` → `path` → `predict`: 180 `Flight::step`s
(~2 µs each). The P23 cache only helped the second to fourth player in a tick; the ball moves every tick, so each
tick still stepped the whole path again. Two fixes, neither changes what the AI sees:

- **Shift the path** (`doubles_ai::Steps`): keep the 181 flights the last path went through. If this tick's start
  (net off, shot, base, court; bincode bytes, so the floats are compared bit for bit) equals last tick's `flights[1]`,
  the new path is the old one a step further on: drop the first, take one new step. Mode = the last flight's call,
  gap = the last None→Out step's gap, the same as `predict`. The flights are `#[serde(skip)]` inside `Built`, so
  they're not in the snapshot or state hash; after a restore the first tick just steps the whole path again.
- **Held ball, stepped only if copied** (`hst_sim::rally::PathFrom`): through the serve the ball sits in the hand
  and moves with it every tick (1752 of the 2006 rebuilds that were left). A flight with `lines: None` can't change
  its call, so mode = its call and there's no gap without stepping it. `path` hands over `World::path_from` instead,
  and `Rally::copy_path` steps it only on the frames it really copies a path.

Result: `control` 420 → 34 µs/tick. The match hash over 20 000 ticks (3 games, every tick's state) is
`a2b7cc03e774bdf0` before and after; `path_cache_keeps_the_match` (cached vs `NO_CACHE`, which now also asserts
each shifted path equals a full `predict`) passes, and so do all 707 tests and `synctest_through_a_change_of_ends`.
What's left (~30 µs) is the copies of the held ball's path and the rebuilds after hits, net touches and points.

### Not verified / not 1:1

- No PCSX2 check: these are only speed changes, and the 20k-tick hash shows the state is bit-identical. The
  `tools/profile.sh` traced run in a window wasn't redone; the timing above comes from the headless harness.
