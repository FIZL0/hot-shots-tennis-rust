# P31a1 — iOS: confirm the load stall is gone (after P31)

Needs a device log from a build with P31 (commit 6ca032b3, 2026-10-15 04:03 −0400) in it. None exists yet.

## What's there
- `~/Downloads/hst-log.txt`: build 2012, written 00:05 −0400. Before P31.
- `~/Downloads/hst-log (2).txt`: build 2013, written 00:19 −0400. Before P31. It has no `game stall` /
  `render stall` / `pipelines ready` lines, and its `loading screen down after N frames` lines have no `at T s`.
  That's the pre-P31 format. Covers at 04:16:30 (13 frames) and 04:17:41 (29 frames).

So neither log can say whether a stall still comes after the cover lifts, or what the first unpaused frame costs.

## What unblocks it
The human step in plan/HUMAN.md (*iPhone: load stalls (for P31a1)*): build and install the IPA from main with P31,
start a match from the menu, play to the result screen, go back to the menu, and send hst-log.txt (the first line
should read build ≥ 2014 with P31 in it). Then read the lines around `loading screen down … at T s`:
- `render stall` after the cover, with `images +` / `ready +`: work still reaches the screen late. Extend `unsettled`
  in mode.rs to cover whatever it is.
- `game stall` on the first unpaused frame, with nothing uploading: the unpause itself is the cost. Its breakdown
  comes from the 10-frame watch.
- Nothing over 50 ms after the cover: P31 fixed it. Tick it.

## Not verified / not 1:1
- Everything. There's no device and no post-P31 log, so nothing is measured on iOS.
