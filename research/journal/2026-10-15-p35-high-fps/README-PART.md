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

## P40 — per-frame mesh writes (2026-10-15)

`noise::draw` (costume sway) and `vertex_patch::write` (its GPU writes):
- **Sim rate**: the deformers step once a tick and the motion moves once a tick (every `Motion` writer runs in
  FixedUpdate), so a rig's sway is redrawn only after its motion changed (`Ref<Motion>::is_changed`, kept in
  `Sway::stale` across a Sway-LOD skip) or its noise stepped. Between ticks the vertices stay put. At ≤60 fps every
  frame has a tick, so the output is the same as before. Above that, a vertex's bind-space offset ignores the pose's
  sub-tick interpolation; that only reaches it through the blend of a multi-joint skin, under a millimetre.
  Checked with a temporary counter at ~300 fps: about 60 redraws per second per rig.
- **Per recompute**: each joint's posed rotation and its skin term (rotation × inverse bind) are computed once per
  joint, not once per vertex entry. A run of vertices sharing a skin reuses its inverse. The math is the same, in the
  same order.
- **Uploads**: the queued writes share one byte buffer (no `Vec` per write, none per frame in the render world). Runs
  up to 2 KB apart join into one `write_buffer` (it was 16 vertices). Frames with nothing queued return at once.

Traced (`tools/profile.sh`, 4-bot doubles, court 1, 1280×800; shared machine, so take the ratios, not the fps):

| | before (~80 fps traced) | after (~190–270 fps traced) |
|---|---:|---:|
| `hst::noise::draw` µs/frame | 169.6 | 32.8 |
| `hst::vertex_patch::write` µs/frame | 125.6 | 25.7 |

An intermediate run at ~80 fps (batching only, no joint cache) gave 159.9 / 80.3. Tables are in `context/perf/p40/`.
`--shot` of the match (`context/perf/p40/match.png`): ponytails and skirts look right.

### Not verified / not 1:1
- Sway between ticks above 60 fps holds the tick's vertices: not compared side by side with the old every-frame
  version in motion. It's rendering only and the difference is sub-mm by the argument above; no sim state is touched.
- fps numbers come from a machine shared with other agents' builds and PCSX2 copies: only the before/after ratio of
  the two systems is meaningful.
## P39 — UI layout every frame (2026-10-15)

Cause: Bevy 0.19's `ui_layout_system`, `ui_stack_system`, `update_clipping_system` and `ui_focus_system` walk
**every** UI node each frame, hidden (`Visibility::Hidden`) or not; taffy's cache only saves the measuring. So their cost
is the node count, and a match carried the overlays' quad pools all the time. New `HST_UI_CENSUS=1`
(`crates/hst/src/ui_census.rs`): every 5 s, nodes per UI root and per-frame changes (Node, Text, ImageNode, …).
In a 4-bot doubles match: **3790 nodes**:

| nodes | root |
|---:|---|
| 1601 | online notice pool (`pause.rs` `NoticeRoot`), empty unless a netplay notice is up |
| 1403 | pause Settings overlay pool (`pause.rs` `Root`), hidden |
| 409 | result screen pool + name tags (`match_stats.rs`) |
| 201 | pause menu pool (`menu.rs`) |
| 176 | the HUD itself (panel, pop-ups, speed, key hints, …) |

Per-frame changes were few: the panel/pop-up/speed quads re-set `ImageNode` (~12/frame), the result screen's fade
re-set its `BackgroundColor` every frame, the FPS counter rewrote its text on each smoothed-fps change.

Fix:
- `crates/hst/src/ui_pool.rs`: `keep` spawns a pool's slots under its root (ahead of the root's other children, so the
  result screen's name tags stay on top) while its screen is up and despawns them when it goes; chained before the
  draw, so the first frame is drawn. Used by the pause menu (`menu.open`), its Settings overlay (root shown), the
  result screen (`match_stats.result` set: ceremony + pages) and the online notice (text set or `HST_NOTICE`).
  `paint` writes a quad's image/rect/tint only when one changed: panel, pop-ups, banner, serve speed, notice.
- result fade: `set_if_neq`.
- `main_menu::cursor::tick`: no hand on screen → no wag stepping/posing of the 3D hand (it restarts on showing, as before).
- FPS counter (On): text refreshed 4×/s instead of every frame.

Result: **190 UI nodes** in play (3790); per-frame changes ~0. Traced (`tools/profile.sh`, 15 s, the same busy
machine for both, other agents building): `ui_layout_system` **435 → 48 µs/frame**, `ui_stack` 93, `update_clipping`
53, `ui_focus` 48, `cursor::tick` 34, `propagate_inherited<ComputedUiRenderTargetInfo>` 33 → all under the table's
28 µs cut-off. Profiles: `context/perf/p39/{before,after}.txt`.

Checked by eye (`tools/shot.sh`, `context/perf/p39/`): pause menu (`HST_PAUSE`), its Graphics page
(`HST_PAUSE_SETTINGS`), the online notice with pills (`HST_NOTICE`/`_ASK`), the ceremony and the stats page
(`HST_STATS`, `_PAGE`) all draw as before; stats page compared with the old binary: same (incl. its existing
"pts" overlapping the rating digits, not from this change).

### Not verified / not 1:1
- No pixel diff for the pause, Settings, notice and ceremony shots: checked by eye only (front-end/presentation, not
  gameplay; `--shot-at` is wall-clock). Only the stats page was put next to an old-binary shot.
- Opening the pause menu / Settings now spawns 200 / 1400 entities that frame (and despawns them on close): not
  timed separately; not seen as a hitch in the shots' runs.
- Absolute fps not compared: other agents loaded the CPU (bench ~100–160 fps both builds); only traced relative cost.
- Not run on wasm/iOS/Android; the change is plain ECS, no platform code.

## P43 — characters' per-frame CPU (2026-10-15)

Traced (P35): `character::animate` 114 µs, `bodyhit::pose` 72, `foot_fx::tick` 52, `net_hide::hide_around_eye` 32
per frame. What each did and what changed:

- **`bodyhit::pose`** (sim, FixedUpdate) sampled every node of every player's clip with VU0 math and built world
  matrices for all of them, though `detect` and foot_fx only read 7 bones (head, neck, spine, spine1, pelvis, both
  toes). New `bodyhit::Bones` resource caches the 7 node indices + a keep mask (those chains and their ancestors)
  per character data; `pose` now samples only kept nodes (`Nodes::sample_kept`/`mix_kept` in hst-sim `pose.rs`, the
  same arithmetic per node; `sample` also stops computing the Hermite twice). `Posed` holds only those chains up to
  date — a new reader of another bone must add it to `Bones::SEEN`.
- **`foot_fx::tick`**: the cost was not `Feet::tick` (~1.3 µs) but the VU0 `node_world` chains for toes, pelvis,
  spine and head of every runner every tick. `Feet::reads` (hst-sim `foot.rs`) says which of them this tick's
  `Feet::tick` actually reads (toes: running / dive / a puff aimed for that player; pelvis: unspent burst or dive;
  spine: the one character+motion that uses it; head: dive); the rest are skipped.
- **`character::animate`** sampled every clip every frame (VU0/ps2 math, i.e. slow software float) and recomputed arm
  IK and face weights. Now, per rig, the pose is sampled once at each sim tick's two ends with host f32 + musl
  (`Clip::sample_drawn`, drawing only) and frames in between slerp/lerp between those two poses by the overstep
  fraction; arm IK turns are cached per tick, face weights only written when the face clock moves, joints written
  with `set_if_neq` (fewer changed transforms to propagate).
- **`net_hide`**: `reach` computed once per frame instead of per prop/player, an exact AABB early-out before the
  segment test (`in_front_rejection_is_exact`: 20k random cases agree with the full test).

### Proof
- Sim unchanged: `four_bot_control_cost` (headless, `--ignored`) match hash `9e33bf44b069eeab` at 6000 ticks before
  and after; 20000 ticks `9f0d2c0f57dfe3b0`. Temporary shadow checks (removed again): the restricted `Posed`'s kept
  nodes bit-equal to full sampling over 80,000 player-ticks; foot_fx with lazy bones gave bincode-identical `Feet`
  and sound RNG vs. full bones over 20k ticks.
- `drawn_sampler_close` (hst-sim tests/motion.rs): host sampler vs. the VU0 one, chars 1 and 6, all ANI2 clips,
  330,720 track samples: worst quaternion component 2.43e-4 (~0.03°), position 3.4e-7.
- Headless per tick: bodyhit::pose 110 → 43 µs, foot_fx::tick 91 → 40 µs.
- Traced profile (`tools/profile.sh`, machine at load ~20–27 from other agents; relative only), µs/frame
  before → after: animate 290 → 63, bodyhit::pose 121 → 36, foot_fx::tick 111 → 38, net_hide 54 → 44 (before the
  reach hoist), prepare_skins 126 → 105, extract_skins 82 → 78, propagate_parent_transforms 355 → 269.
- Shots at 120 fps (`context/perf/p43/shot_{6,9,13}.png`): poses, racket arms, faces, ponytails look normal.
- Skins (prepare/extract_skins ~180 µs) left alone: every part lists all joints; compacting them touches cull/shade/
  lod/mods indices → follow-up **P51**.

### Not verified / not 1:1
- Drawn poses between ticks are a slerp between the two tick-end poses, not the clip sampled at the in-between
  time, and use host float instead of VU0 math (≤2.5e-4 per quaternion component): rendering only, sim untouched.
  Not put next to an old-binary shot frame-for-frame (wall-clock `--shot-at` can't hit the same frame).
- `Posed` (in snapshots) now holds stale values for bones outside `Bones::SEEN`; sim outputs are identical, but
  snapshots differ byte-wise from older builds' (not a wire format between versions).
- foot_fx's lazy bones: the 20k-tick check covered 30 dive runner-ticks and no pelvis burst; the rule follows
  `Feet::tick`'s reads, the rare branches aren't separately exercised.
- Profiles on a heavily shared machine; no idle-machine fps numbers (P35's umbrella re-bench).
- Not run on wasm/iOS/Android; no platform code changed. No PCSX2 comparison: no gameplay change (hash identical).
