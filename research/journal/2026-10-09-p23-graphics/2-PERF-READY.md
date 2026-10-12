# P23 part 2: profile at Deck class, fix the waste, fps numbers

## Method
- `tools/bench.sh LOG SECONDS [hst args…]`: 4-CPU doubles on court 1 (`--play --court 1 --chars 0,1,2,3`,
  `HST_AUTOPLAY`), floating 1280×800 window, prints the adapter and the `HST_FPS` lines. The fps line now also prints
  `cpu X ms/frame` (process utime+stime from `/proc/self/stat`, summed over all threads).
- Deck stand-in: `HST_GPU=igpu` (Granite Ridge iGPU, 2 CU RDNA2, about a third of the Deck's 8 CU) plus `HST_SLOW=1`
  (2 cores, 120 % quota, vs the Deck's 4 Zen 2 cores). It's weaker than a Deck on both GPU and CPU.
- Profile: Bevy `trace_chrome` (trace binary built from a separate target dir; about 4.7 GB per 25 s run), aggregated per
  span (total, count, median) with a scratch script in `context/p23/`. samply and perf are blocked here
  (perf_event_paranoid 2, no sudo).

## What the frame went to (dGPU, 1280×800, default settings, ~170 fps before)
- `hst::play::control` ~1.8 ms per fixed tick: the doubles AI re-stepped the ball's 180-step path once per bot per
  tick (358 µs each), always the same path. **Fixed**: `doubles_ai::step` caches the path keyed on everything it's
  stepped from (flight, shot, path base, court; Debug output, so the float bits are exact). The cache field's Debug
  is empty, so `snapshot::state_hash` is unchanged. Test `path_cache_keeps_the_match`: 3600 ticks of a 4-bot
  doubles match give identical per-tick state hashes with and without the cache. A mutation check (key without the
  flight) fails it at tick 254.
- The match's call-overlay camera (`popups.rs` `CallCamera`, a second Camera3d) stayed active while idle, which
  doubled shadow cascades, light clustering and GPU preprocessing every frame. **Fixed**: it's active only while a
  call is on show.
- Render-world `batch_and_prepare` / GPU-preprocess bind groups / shadow `write_binned_instance_buffers`: the
  Render schedule had a median of 7.5 ms. `PbrPlugin { use_gpu_instance_buffer_builder: false }` (CPU batching) cut it to
  2.9 ms, about 2.5× fps everywhere, **but skinned characters disappear** (only rackets and shadows draw; the
  screenshot comparison caught it). Reverted. Worth a follow-up: find why skinned meshes don't draw on the CPU
  batching path in Bevy 0.19 (our skinning setup or a Bevy bug). That's the biggest remaining win.
- Scene: 226 meshes, 120 shadow casters. Neither is a problem by itself.

## Numbers (fps, 1280×800, 4-CPU doubles, 20 s average; machine load average 29–37 from other agents)
| setup | before | after (cache + idle call camera) |
|---|---|---|
| dGPU default | ~170 (quiet), 68–181 | 95 / 117 |
| dGPU low (`--scale 67 --shadows low --msaa off`, uncapped) | 88–143 | 97 / 134 |
| stand-in default | 44 / 67 | 72 / 77 |
| stand-in low | 83 / 90 | 74 / 76 |
The stand-in rows are interleaved A/B runs under the same load. The same binary swings ±30 % between reps, so the
after-vs-before gap is within the noise except where the CPU-bound doubles AI dominates. With CPU batching (not
shippable, see above), the stand-in went to 138–146 default and 157–240 low.

Look check: `tools/shot.sh` court (`--court 1`), character viewer (`--character 0`) and match (`--shot-at 25`), before
vs after. Court identical; character differs only in a 100×200 patch from the animation phase; match same layout,
players and HUD (the run isn't deterministic frame for frame).

## Not verified / not 1:1
- 120 fps at low / ≥60 at default on a Steam Deck: there's no Deck here, and the stand-in is weaker on both GPU (2 CU vs
  8) and CPU. ≥60 at default holds on the stand-in; 120 at low doesn't (74–76).
- Numbers are noisy: 30+ other agents loaded the machine (load average 29–37); a quiet re-run is needed.
- The low preset's frame cap (60) was left off for the bench (uncapped) to see headroom.
- The look check is by screenshot, not pixel-exact (render-only, as PLAN allows).

## Needs the human
Run `tools/bench.sh context/p23/deck-default.log 25` and `tools/bench.sh context/p23/deck-low.log 25 --scale 67
--shadows low --msaa off` on the Deck (or this machine when idle). Next lever if short: the skinned-mesh issue on
CPU batching (above).
