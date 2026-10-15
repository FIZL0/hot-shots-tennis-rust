# B342: re-measured on main's merger (B328's `merge.rs`) — no gain, stays off by default

## Why a second pass
B342 itself landed in a08655e9 (part 3-MERGE-PART). The merge into main kept B328's `merge.rs` (per-plant rebuild)
instead of this task's own one, and left the toggle default **off** going by this task's numbers. Those numbers were
taken with the dropped implementation, though. B328's own journal (3-MERGE-FINAL) reported that merging by itself
took (cpu − sim)/sim from 8.3 to 7.45. The merge commit also dropped the TODO tick. So this pass measures the merger
that ships now, using the toggle (`?merge-meshes=on|off`, `--merge-meshes on|off`), and closes the task.
There are no code changes in this pass.

## Web (`tools/web_check.mjs`, MOBILE=1 THROTTLE=4, Vulkan ANGLE flags as part 1, `?play&stage=1&show-fps-detailed`, frames 25–50 s)
The build is current main (wasm, no wasm-opt), served from `context/b342v2/web` on :8774. Off and on alternated run
by run. Load average was 17–24 the whole time (other agents). Per-run numbers are in `context/b342v2/summary.txt`
(script `context/b342v2/sum.py`, runner `runs.sh`).

| backend | off fps per run | on fps per run | render/(game−sim) off → on |
|---|---|---|---|
| WebGPU | 22.1 / 36.8 / 16.8 / 47.6 / 28.0 | 27.0 / 25.2 / 33.2 / 44.6 / 32.3 | 1.66 → 1.66 |
| WebGL2 | 15.9 / 21.5 / 30.9 / 22.8 | 21.0 / 21.1 / 22.7 / 23.8 | 2.06 → 2.06 |

Meshes drawn 547 → 276, shadow casters 170 → 127 (so the switch does take effect).

The single-ratio comparison B328 used, (cpu − sim)/sim, follows the machine's load: the sim's ms per frame swung
1.4–7.3 between identical runs, and the ratio moves with it. Instead I fitted non-sim CPU ms = a·sim + b + c·[on]
per backend over all runs, with one shared load slope. Results:

- WebGPU: c = **+0.13 ms** (residual sd 0.73 ms; mean non-sim CPU 29.6 ms/frame)
- WebGL2: c = **−0.11 ms** (residual sd 0.93 ms; mean 41.1 ms/frame)

So no gain at the 0.5 % level. B328's 8.3 → 7.45 was most likely load drift between runs.

## Native (`tools/bench.sh … 32 --stage 1 --merge-meshes off|on`, fps samples from 10 s, three alternating pairs)
off 83.6 / 55.8 / 45.6 fps (CPU 15.95 / 18.05 / 20.44 ms/frame), on 66.8 / 49.1 / 49.9 fps (17.02 / 19.42 / 19.82).
No gain. Bevy's mesh/material pairs seen go *up* with merging (226 → 241), the same as part 3 found. Bevy already
draws each prop's copies as one instanced batch.

(`tools/bench.sh` needs `HYPRLAND_INSTANCE_SIGNATURE`. Unattended shells don't have it, so export it from
`/run/user/1000/hypr/`; without it the bench silently writes no log.)

## Result
Merge Court Meshes stays in as an option (Settings → Display, `--merge-meshes`, `?merge-meshes=`) and **off by
default**: no measurable gain on WebGL2, WebGPU or native. Visuals are unchanged from part 3's on/off shots; this
pass changed no code.

## Not verified / not 1:1
- No real phone measurement (no iPhone here). Desktop Chromium was throttled 4× on a machine at load ~20, so the
  effect bound is only as tight as the regression (~±0.3 ms/frame).
- Not compared against PCSX2. This is a renderer-internal switch, so on vs off was compared in our own build only.
- No new screenshots this pass. Part 3's on/off shots (same picture, ≤25/255 on a few net-tape pixels) still apply,
  since `merge.rs` is unchanged since then.
