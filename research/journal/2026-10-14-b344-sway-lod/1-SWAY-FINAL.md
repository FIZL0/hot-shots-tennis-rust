# B344: character sway (noise::draw) cheaper (B328 lead)

## What the sway costs
Four-bot doubles on court 1: 4 costumes with sway, 15 swaying part meshes (32–987 vertices, 3 750 in all), 466 moved
vertices with 661 position entries. In-process timer around `noise::draw` (temporary, native release, 1280×800):
**~260–300 µs per drawn frame**, ~13 mesh writes per frame (each one a whole-mesh re-extract/re-upload). ~80 % of
the time was `hst_sim::noise::deform` (the VU noise math in software `vu0` arithmetic), not the upload.

## Changes
1. **Always on, identical output** (no toggle: it can't cost any device anything, so there's nothing to turn off):
   - the entries' bone-space offsets are worked out once per deformer step (`Sway::offsets`, keyed on the deformers'
     state) instead of on every drawn frame. The pose still applies every frame;
   - the drawn vertices of a strip share position entries: each distinct (deformer, position, weight) is moved once
     (661 → 383 entries for these four costumes);
   - only the moved vertices are compared and written (in place via `attribute_mut`), not a clone of the whole
     position array.
   Checked bit for bit: a temporary build ran the old algorithm next to the new one and asserted every mesh's
   positions were equal (bit patterns), and that "changed" agreed, over ~50 000 mesh updates with Sway LOD off and
   ~26 000 with it on. Result: **~110–140 µs per frame** (from ~260–300), same writes.
2. **Sway LOD** (Settings → Display, `sway_lod` in settings.txt, `--sway-lod on|off`, browser `?sway_lod=on`): a rig
   that no active 3D camera on its render layers sees (a 2.5 m ball round the root against last frame's frustum)
   keeps its last sway. A far one (its 2 m height under a sixth of a perspective view) updates every other frame,
   alternating by entity. In the match view that's the far team (shares 0.14–0.16; the near pair 0.20–0.25). Measured
   in-process: 12.7 → ~10 mesh writes per frame, draw time ~110–140 → ~80–115 µs.
   **Default off**: no frame-rate gain could be measured (below).

## Frame rate (B328 setup: tools/web_check.mjs, MOBILE=1, THROTTLE=4, `?play&stage=1`, frames 25–50 s)
base = HEAD before B344 (built from `git archive`, served on its own port), on/off = Sway LOD in the B344 build. Four
interleaved rounds (a–d), fps / median frame ms:

| round | WebGL2 base | WebGL2 on | WebGL2 off | WebGPU base | WebGPU on | WebGPU off |
|---|---|---|---|---|---|---|
| a | 45.7 / 21.7 | 40.3 / 19.3 | 49.6 / 19.8 | 36.6 / 26.6 | 36.4 / 26.1 | 36.6 / 26.7 |
| b | 26.2 / 38.1 | 28.7 / 26.0 | 27.7 / 34.0 | 22.2 / 46.3 | 31.3 / 30.0 | 31.3 / 30.9 |
| c | 19.4 / 48.7 | 23.3 / 38.7 | 25.4 / 38.7 | 24.9 / 38.8 | 27.7 / 37.0 | 27.5 / 33.2 |
| d | 18.7 / 49.7 | 36.6 / 26.5 | 34.6 / 26.9 | 40.1 / 24.6 | 37.6 / 26.2 | 52.8 / 18.6 |

The machine was shared with the other parallel agents (load average 6–48 during the runs; B328 measured 56 fps
WebGL2 on the same setup idle). Runs swing ±30 % run to run, so the expected change (~0.15 ms of a ~4.7 ms
unthrottled frame for the rewrite, a third of that again for the LOD) can't be resolved. The B344 build is at least
not slower than base in any round. Native `tools/bench.sh` (4 bots, 1280×800, 45 s), LOD on/off: 120/121/120 vs
114/116/124 fps and 120/121/110 vs 134/134/133 fps: noise again. Runs and logs are in `context/b344/`.

So, as the task says: Sway LOD stays **off by default**. The identical-output rewrite halves the system's measured time
and is kept on.

## Looks
`context/b344/shot-{on,off}.png` (native `--shot-at 8`, LOD on/off): the players look the same. LOD's worst case is
a far player's hair/cloth offset one frame stale (a few mm in bone space, sub-pixel at under a sixth of the view's
height), or an unseen player's sway frozen until it comes back into view (its shadow keeps the last sway).

## Not verified / not 1:1
- **Frame-rate gain in the browser**: not resolvable on a loaded shared machine (table above). Measure again on an
  idle machine or on the phone (Safari timeline) before turning Sway LOD on by default.
- **Sway LOD visuals**: by eye only, on one shot pair (needed: rendering only has to look right). No moving
  side-by-side of a far player's hair.
- **Vertex-shader sway** (the task's alternative): not done. The offset depends non-linearly on the deformer phase
  (value noise), so morph targets can't carry it. It would need per-vertex entry data and per-rig deformer uniforms
  in `gs.wgsl` (no storage buffers on WebGL2). The upload part it would remove is ~13 small mesh writes a frame, now
  that the CPU part is halved.
- Mod costumes' entries (`mods.rs`) go through the same code; checked only with disc characters.
