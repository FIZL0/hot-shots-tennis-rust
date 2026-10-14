# 1 — port the end stands' crowd cull (FINAL)

## Findings
- Court 5 (Queen's Tennis Garden) is the grass court with stand crowds (`npc::Crowd`, roster types 61..66,
  `creatureMdl/wim-0N`). P14d had left one thing out: the crowd draw skips types 63 and 66 (the two end stands)
  unless a point by them passes the game's view test. Point (0, −7.997, −49) for type 63, (0, −7.997, +49) for 66;
  the test transforms it by the view matrix and draws only when depth > 0.001 and focal/depth < 1000 (focal =
  projection[0][0], 240 / tan of the horizontal half-angle). Read from the disassembly (compare directions checked
  on the `c.ole.s`/`c.olt.s`, not Ghidra's C).
- The same view test also gates the walkers (limit 300) and the umpire (1000); neither changes anything on court 5
  (no walkers there in doubles; umpire is always in view of the match camera) — not ported here.
- Original RAM on court 5 (doubles): 8 crowd objects, types 61,62,61,62 at (±5,−1.65,±10), 64/65 at
  (±3,−3.9,3.85), 63 anchored at z −24.01, 66 at z +27.4; umpire (−7.03,−1.78,0).

## Fix
`crates/hst/src/play/npcs.rs`: `CrowdCull` on types 63/66, `cull_crowd` (PostUpdate, before transform propagation)
hides them while their point fails `crowd_shown` against the drawn camera (any camera mode: match, own camera,
cut-aways, replays). Unit test `crowd_shown_skips_behind_and_too_near`.

## Checks (screenshots in `context/b283/`)
- `research/b283_ends.py` (from scratch slot 8, court picker on Queen's): original match-camera eye and screenshot
  at both ends (`orig/near|far`). Remaster at those eyes (HST_EYE), cull on/off, 4:3 and 16:9 (`run2/`): matches the
  original by eye at both ends; no stands in frame either way, so the cull makes no visible difference there.
- `research/b283_glide.py`: original cut-away eye (`glide/g0`, from inside the court towards the side stand) vs the
  remaster at that eye (`run3/g0_on|off.png`): same crowd blocks visible as the original, on = off.
- HST_EYE framing matched the original only with t = tan(fov/2) unscaled (B211's ×4/3 for PCSX2's 16:9 stretch
  made it too wide); noted for whoever uses HST_EYE next.

## Not verified / not 1:1
- The reported view wasn't reproduced: at the original's match eyes (both ends) and a cut-away eye the remaster
  already showed no extra audience; the ported cull is the original's only stand-crowd cull and is the fix by
  reading, not by a before/after picture. If the user still sees it, a screenshot of the view (camera mode, end)
  is needed. Earlier guessed eyes (z = ±24) showed two figures on top of the far wall at the +z end; not traced
  (likely the trigger creatures), not compared with the original at that eye.
- The change-of-ends glide wasn't captured: the script's ○ presses ended on a cut-away and P1 (human) then held up
  play; stopped after ~15 min.
- Depth/focal come from the drawn Bevy camera in f32 (focal from fov and aspect), not the game's VU0 view matrix:
  a point right at the 0.001 / 1000 edges may flip a frame apart (rendering only).
- Walker/umpire view-test gating not ported (no visible effect on court 5).
- Other grass courts (6, 8, 10) have no stand crowds of this kind; not screenshotted.
