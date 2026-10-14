# B342: merge static court/prop meshes (B328 lead) — no gain, off by default

## What it does
`crates/hst/src/merge.rs`: when a court loads, the copies of each prop model (grouped by stem and whether they cast a
shadow) get baked into one mesh per unblended material (`Mesh::transformed_by` + `Mesh::merge`). These are children of
the court root and are drawn in place of the copies' parts. Not merged: blended parts (they are sorted per copy),
mirrored copies (negative scale), models placed once, the net and the `w-house*` models (`net_hide` handles both).
Every frame `switch` shows the merged model only while the setting is on and none of its copies is hidden.
`net_hide`'s NearBox and HideAroundEye can hide a copy, and so can the loading warm-up putting visibilities back,
so the per-copy parts come back whenever that happens.

Toggle: Settings → Display → **Merge Court Meshes** (`graphics.merge_meshes`, not part of a preset, **default
off**; it takes effect from the next match because the meshes are only built when it is on). There is also a
command-line flag `--merge-meshes on|off` and a browser URL switch `?merge-meshes=on|off`. Both override the menu
setting for the whole run (`graphics::MERGE_FORCED`).

## Counts (court 1, `--stage 1`, 4 players)
- 578 court GsMaterial meshes, 521 visible, 222 distinct mesh + material pairs; 138 tree + 27 structure placements
  (court 4: 67 + 52).
- Merged groups on court 1: sugi00 ×17, sugi02 ×13, c01watergrass ×13/×31 (split by caster), c01watertree ×7/×13,
  05stan ×13/×2, manytree ×9, sugi-yuki00 ×8, c01house02 ×5, glass ×3, c1glasses ×3, netvolley, shinpandai,
  goalpost, c01kanban ×2.
- Visible `Mesh3d` entities in a match: 588 off → 324 on.
- **GPU draw calls per frame** (WebGL2 `drawElements*`/`drawArrays*` and WebGPU `draw*` hooked in the page,
  `context/b342/draws.js`): WebGL2 405 off → 417 on, WebGPU 742 off → 758 on. Bevy already batches copies that
  share a mesh and material into one instanced draw, so merging removes entities but not draws. Splitting by caster
  even adds a few.

Note: `tools/bench.sh` runs `--court 1`, which loads no scenery (about 104 court draws, all unique). For this task
the bench used `--stage 1`, which loads the scenery, same as the browser's `?play&stage=1`.

## Before/after
The machine was shared with other agents' builds the whole time (load average 13–35), so absolute numbers are well
below parts 1–2 (WebGL2 about 23 fps here vs 56–58 there) and the spread between runs is wide. On and off were
alternated run by run.

Web (`tools/web_check.mjs`, MOBILE=1 THROTTLE=4, frames 25–50 s, five runs each, `context/b342/web/`):

| build | fps per run | median frame |
|---|---|---|
| WebGL2 off | 23.3 / 23.4 / 22.9 / 22.2 / 20.4 | ≈ 43 ms |
| WebGL2 on  | 23.0 / 23.2 / 22.3 / 18.0 / 24.7 | ≈ 43 ms |
| WebGPU off | 38.5 / 14.4 / 20.0 / 15.3 / 27.4 | — |
| WebGPU on  | 20.7 / 26.4 / 21.0 / 21.7 / 27.4 | — |

WebGPU CPU profile (named build, `WEB_CPUPROFILE=30:15`, `b328_inclusive.py`, three runs each):
camera driver 15.2/15.8/16.3 % off vs 15.5/17.4/17.1 % on; extract 7.7/5.8/7.5 vs 7.8/7.1/7.4; check_visibility
≈1.2 vs ≈1.0 %.

Native (`tools/bench.sh` with `--stage 1`, 32 s, fps samples from 10 s, three alternating pairs): on 57–72 / 86–104 /
47–102 fps, off 76–114 / 85–119 / 114–148 fps; CPU about 16–20.6 ms/frame on vs 15–19.8 off.

**Result: no measurable gain on any backend.** If anything, on is slightly worse, but that is within the noise. As the
task says, it stays in as an option and is **off by default**.

## Visuals
Shots with `tools/shot.sh` (HST_WEATHER_SEED=1 HST_AUTOPLAY=1, `--play --stage c --court c --chars 0,1,2,3
--shot-at 2/9`, courts 1 and 4, `context/b342/shots/`, final build `context/b342/final/`). On and off show the same
picture. Pixel differences come only from animated characters/spectators (run timing; two off runs differ the
same way), plus 97 pixels on the net tape row that differ by at most 25/255. Those come from transforms baked on
the CPU instead of on the GPU, and are invisible.

## Where the camera driver time goes instead
Merging doesn't shrink the per-frame work because the 222 distinct mesh + material pairs are already one batch
each. Better next leads: fewer distinct materials/textures (an atlas per court would let batches span models), the
crowd/NPC rigs, and shadow casters (the shadow pass repeats every batch).

## Not verified / not 1:1
- Merged picture vs per-copy picture: up to 25/255 on single pixels of the net tape row, from CPU-baked transforms.
  Invisible. Not chased, per PLAN's rendering rule.
- Not compared against PCSX2. This is a renderer-internal change, so it was compared on vs off in our own build only.
- Not measured on a real phone. Only the emulated MOBILE=1 THROTTLE=4 desktop Chromium was used.
- The measurements are noisy because the machine was under heavy load from parallel agents. A gain smaller than about 10 %
  would not show. The draw-call count (unchanged or higher) is the solid evidence.
