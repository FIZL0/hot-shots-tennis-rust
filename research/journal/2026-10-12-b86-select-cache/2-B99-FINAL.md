# B99 — select previews from the PNG cache don't load properly

## What the user saw (reproduced)

`tools/select_stress.sh SECS DIR [env…]` drives the real select (`HST_MENU=chars`, four keyboard seats; `--play`
is not needed and goes straight to a match). Two new aids:
- `HST_SHOTS=DIR` saves every frame 60–299 as a PNG (a frame-by-frame series while scrolling).
- At exit, the stress run logs how many card-frames showed the hovered key, showed nothing or another key though
  its PNG was there, or showed nothing because no PNG was baked yet.

Before the fix (`context/b99/base/`), 11397 of 43536 card-frames (26%) didn't show the hovered key. In the shot
series, all four cards are **blank on almost every frame while scrolling**. A picture showed only when a card held
still. Causes, all in `sync`:
1. A key change took the old card down at once, and the new PNG was read on a worker thread a few frames later. So
   the card was blank on every scroll step (the suspect named in the task).
2. A new node was spawned unplaced: the placement query couldn't see it until the next frame. For one frame it drew
   at the image's own size in the top-left corner (a flicker).
3. 87 mod costumes (costumes 4–7 of most Hot Shots Golf Fore! mods) **never loaded at all**. Their shared
   `motions.glb` keys joints those costumes lack (`Body`, `node10`, `nina_skirt`…), and `mods::own_motions` rejected
   the whole costume. So those cards stayed blank forever, and the costume couldn't load in a match either.

The bakes on disk were fine, as the task said. (`Loader::png` keeping a `None` for an unbaked file was real but
harmless: the baker's `forget_png` cleared it.)

## Fix

- **The PNG is read on the frame its key is hovered** (`Loader::png`, synchronous: a 384×480 PNG decodes in a few
  ms). The worker thread for PNGs is gone, and so is `forget_png`. An unbaked file isn't cached, so it is checked
  again each frame until the baker writes it. B86's 48-PNG LRU is kept.
- **Nodes are placed and given their `ZIndex` as they spawn.**
- **A key with no PNG yet asks for its live model at once** (no 0.3 s settle).
- **The PNG stays over the live model that replaces it** for `COVER` = 10 frames. It becomes a full-size child of
  the live node, then goes in `Gone`. This covers the new camera's first frames.
- **The baker now queues every mod's costumes** after the disc's, by default (before, only `HST_BAKE=all` did), so
  the cache fills in the background on the select.
- **`own_motions` skips channels for joints this costume lacks** instead of failing. Test:
  `mods::tests::motions_skip_joints_a_costume_lacks` (Phoebe's 8 costumes load with their own motions). The
  ../opentennis-mods standard §6 is updated to match (opentennis-mods 866158e). `RENDER` is not bumped: nothing about the drawing
  changed, and the costumes that had baked loaded the same before.

## Results (`context/b99/`)

| run | card-frames showing hovered / wrong or blank / no bake yet | blank cards in the shot series (pixel check) |
|---|---|---|
| base (old code, 25 s) | 32139 / **11397** / (counted in wrong) | 487 over 240 frames |
| fix1 (25 s) | 39242 / **0** / 2530 | 0 |
| long (180 s) | 131288 / 0 / 16052 (mods not baked yet), 151 baked meanwhile | — |
| mods3 (12 s, mods page, cache full: 501 files) | 20424 / 0 / 0 | 3 (all the "Regis" mod, see below) |
| long2 (240 s, full cache) | 433060 / 0 / 0 | — |

- No panic in any run. Own VRAM max 2.9 GB in long2 (B86: 3.1–3.5 GB).
- `HST_BAKE=all` baked 87 more after the `mods.rs` fix, with no failures: 140 disc + 361 mod pictures.
- The pixel check (`blank.py`: stddev in each card's figure area) flags 3 frames on the mods page. All three are the
  "Regis" mod: its picture shows, but the figure is tiny and sits at the bottom of the card. That is the mod's
  framing or scale (M1k territory), not the select.
- Frame rate is the same as before (card-frames per run, within 5%).

## Not verified / not 1:1

- **Cover length.** `COVER` = 10 frames is a fixed count. The shot series show no blank frame at a PNG→live swap,
  but a slow GPU compiling pipelines for a first-ever live preview could take longer.
- **No picture for an unbaked costume.** It still shows a blank card until its live model loads (~130 ms) or the
  background baker reaches it. The first launch with a new mod has this until the bake finishes. With a full cache,
  there were 0 such frames.
- **Regis mod framing.** Drawn small and low on its card (see above). Not looked into here.
- **No reference in the original.** The select previews are our own design (menus are "similar, not exact"), so
  there is no PCSX2 comparison.
