# B554 — black box behind the net on Edge (blocked)

Report: context/reports/2026-10-17/1.png (online doubles, room court 3 = disc court 4, four browser peers, host
playing the Anya mod as 1P; desync dump in the same folder). Camera is the normal +z end (umpire on the left),
3P/4P near. The object stands on the far half just behind the net, x ≈ 0..1 m: a black upright box (~0.9 × 1.7 m,
a thin line near its top), a tilted dark bar with a thin arc above it, three or four translucent teal panels with
dark posts, a shadow on the court. The net and the 4P player occlude it, so it is depth-tested scene geometry at
that spot, not a screen-space or behind-camera artefact.

## What was checked (all native, `tools/shot.sh`, captures in context/b554/)
- Disc court 4 natively (both camera ends, 4-player doubles, `--pads -,k,-,-`): no object there.
- Every court-4 plant placement (layout + plant dat, season 0): nothing within the court but the net, umpire
  chairs, light4/ry/uekomi at the neighbouring court's x ≈ 8.7. Far-end props (vending machine `jihanki00`,
  `light5` lamp, `blackdust` bin at z ≈ -29.6) have the right sizes but the wrong colours (red machine) and place.
- Court 4's trigger creatures: type 14 (`trg_M-04`, a wheeled chain-link ball-stop panel, four of them, placed at
  the origin in the layout) are posed by their clip to x ≈ 9.2–9.7, z ±12/±16 and shown correctly natively; type
  15 (soccer ball) is off. Types 16/17 have no model. If that clip pose were skipped the panels would stand at the
  origin — the closest match found (chain-link reads as translucent teal at distance), but it doesn't explain the
  black box and was not reproduced.
- Court mods with layout 3 (Studio A, indoor, shadows, city gag courts): none has this at the centre; the room
  had no court mod. Anya mod: no props. Backdrop/sky/cloud models, znet/netmoto: not it.

## Why blocked
No reproduction: the web run through tools/web_check.mjs never got past the new landing page's ISO import
(status empty, no canvas), so Edge/Chromium WebGPU could not be tried, and nothing native draws it. Without a
repro or a way to identify the mesh, any fix in gs.rs/textures.rs/stage.rs would be a guess.

## What unblocks it
- A Chromium/Edge run of the web build that gets into a match on court 4 (fix web_check's ISO import for the new
  landing page), or
- From the user: does it appear offline on Edge too, on WebGL2 (`?webgl2`) too, from the first point or only after
  the change of ends (the screenshot is at 15-0 in game 2, right after one), and a second angle/screenshot.
- First thing to try with a repro: force type-14 trigger creatures' clip off and see if the panels land at the
  origin (`npcs.rs` `hold_triggers`/`step`, animation of hidden-at-spawn rigs on web).

## Not verified / not 1:1
Everything: nothing was changed.
