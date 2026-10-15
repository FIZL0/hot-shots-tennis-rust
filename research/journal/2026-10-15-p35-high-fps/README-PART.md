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

## 2026-10-15 — umbrella check (parallel run)

Picked up P35 itself while none of its parts had landed: P36–P48 are all still open in `plan/TODO.md`
(nothing in `plan/DONE.md`). The umbrella's done-condition (≥480 fps in a 4-bot match at 1440p on the 9070 XT,
P23's Deck numbers, replay tests bit-exact, no look change) can't hold before they do, and porting them here would
mean doing 13 other tasks under one ID (and colliding with the agents taking them in parallel).

- No new bench/profile run: this run shares the CPU with other agents' builds and PCSX2 copies, so a number taken now
  would not be comparable to the baseline above. Re-measure on an idle machine when P36–P48 are in.
- Moved P35 after P48 in `plan/TODO.md` and marked it blocked on them.

Next (agent): once P36–P48 are ticked, `tools/profile.sh context/perf/p35-after.txt` and `tools/bench.sh` at
`HST_WINDOW='2560 1440'` on an otherwise idle machine, log before/after here, then tick P35 or open follow-ups for
whatever still keeps it under 480 fps.

### Not verified / not 1:1
- Frame rate target: not measured this run (machine shared with parallel agents; parts P36–P48 not landed yet).
- P36 (shadow casters): research/journal/2026-10-15-p36-shadow-casters/
## P37 — per-frame bind-group rebuilds (2026-10-15)

Cause: not our code (`vertex_patch::write`, `noise::draw` and skins don't touch these buffers). Bevy 0.19's
`prepare_preprocess_bind_groups` (bevy_pbr `render/gpu_preprocess.rs`) creates every mesh-preprocess, indirect
parameters and bin-unpacking bind group again each frame, for each view × phase. Counted with a temporary
counter: **~507 bind groups per frame** in a 4-bot doubles match (views with 3D phases: the match camera + 4 sun
cascades; the HUD/gamma/cursor cameras and the UI sub-views have none). Most are bin-unpacking groups, one per bin job.

Why they couldn't simply be kept:
- the buffers they bind are two sets that take turns: `collect_buffers_for_phase` (bevy_render) swaps the per-phase
  instance/work-item/indirect buffers with last frame's to reuse allocations, so frame N binds frame N−2's buffers;
- each bin-unpacking job bound the shared metadata uniform at its own static offset, and the offsets shift whenever a
  bin is added or removed in any view (≈150 misses/frame from that alone).

Fix: bevy_pbr vendored in `third_party/bevy_pbr` (patched in the workspace `Cargo.toml` like bevy_render):
- `PreprocessBindGroupCache`: bind groups keyed by layout id + every entry's resource (wgpu buffer identity, offset,
  size; texture views; samplers), holding the bound buffers so their identities can't be reused; one unused for 3
  frames is dropped. Every `create_bind_group` in that system goes through it.
- the bin-unpacking metadata binding is a dynamic-offset uniform (offset passed at `set_bind_group`), so a job's bind
  group no longer depends on where its metadata sits.

Result (temporary counter, 25 s bench): new bind groups per frame **507 → 0–3** (a buffer grew or a view's visible
mesh count changed the bound work-item size); ~1,000 held. Traced profile (`tools/profile.sh`, same 15 s bench;
other agents were loading the machine, so compare relative costs): `prepare_preprocess_bind_groups` **950 → 355
µs/frame**. The rest is building/hashing ~500 keys and Bevy's own per-view bookkeeping.

Picture: A/B `--shot` (old vs new binary, `HST_WEATHER=0`, court 1, 8 s in) both draw all players, net, shadows,
umpire and HUD correctly; the shots are different moments (`--shot-at` is wall-clock), so no pixel diff. wasm32
`cargo check` passes (WebGL2 doesn't use GPU preprocessing; WebGPU supports dynamic uniform offsets).

### Not verified / not 1:1
- No pixel-exact A/B: `--shot-at` fires at a wall-clock time, so the two shots show different moments; checked by eye
  (rendering only has to look right).
- Absolute fps not compared: parallel agents were using the CPU (bench gave ~100–130 fps vs P35's 250 baseline on an
  idle machine), so only the traced relative cost is reported.
- Not run on WebGPU in a browser / Android / iOS; only the native Vulkan build ran (the dynamic-offset change is
  standard WebGPU).
