# P14f — court 1's chickens move

## What was wrong
- The app built every trigger creature with no path (`npc::Trigger { .. }` without `path`), so `reset` left
  `moving = false` for all of them and a startle had nowhere to go. And `play/npcs.rs` only copied `t.world` into the
  transform for types 2/6, 15 and 44.
- Types 0/1 (trg_sa-01/02) stay `on = false` until something comes within 2.0 of them (their msg-4 reset); this is
  the original's behaviour too (startle recording `prox_c01`: 3 startles, idle otherwise; RAM: not moving).
- Type 2 (trg_sa-03, the landing-spot creature) is not "on with frame 0" any more: its reset turns it off and
  hidden; message 6 after a game or set won on a plain call with a named point turns it on at frame 1 and it
  animates (clip length 85 on court 1, checked in the app). The brief's observation came from before its reset
  was wired.

## Paths (from the disc)
- A trigger record's link (category 14, index among the category-14 markers) names the start node; the path is
  every category-14 marker with the same `sub` byte, in file order; the start node is the linked one's index in it.
- Each node's radius is the plant record's f32 at 0x30 (court 1: 1.0).
- In RAM the path manager hangs off the layout object (+0x50): a ring of [last, next, id, count, nodes], nodes
  0x70 apart, position at +0x50, radius at +0x10. `creatures_match_ram_on_four_courts` now walks it for every
  trigger object: courts 1/2/4/10 have 8/13/7/13 paths, nodes, radii and start node bit-exact.

## Random point about a node (rows without `exact`)
- The waypoint draws u = 2^-32·roll, a = −π + 2π·u (madd), builds rot_y(a) applied to the identity (decoded the
  VU0 macro code by hand: rows (c,0,−s), (0,1,0), (s,0,c)), and aims at node + row 2 · radius
  (x: add of mul, y/z/w: madd on 0 + node). Ported as `Trigger::node_at`, used by modes 1–3/5.
- Checked: a reset's first target in RAM on courts 1 and 2 (11 resets), x/z/w bit-exact for some draw near
  the target's angle (128k-draw search). y differs by a few µm on court 1's type 1: that row snaps each
  waypoint to the ground (snap −1, not ported), so y is the court mesh height there.
- `startled_creatures_match_the_game` now runs with the disc paths and radii: court 1 4051 ticks moving along a
  disc path, court 2 328, all bit-exact. No leg in those recordings picked a new target on a non-exact row
  ("0 legs aimed about a node"), so `node_at` is proven only by the RAM resets.

## App
- `play/npcs.rs`: passes `c.path`, `c.radii`, `c.start` into each creature; every creature's transform is
  `t.world` each tick (the 2/6, 15 visibility rules unchanged, 44's own copy folded in). Court 1 screenshot
  (`context/p14f/run1/c1.png`) shows the chickens at home before a startle.

## Not verified / not 1:1
- Ground snap of waypoints (row `snap`): not ported, so a snapping row's target y is the node's y, not the mesh
  height (µm on court 1). No recording shows it changing gameplay.
- Start mode 1 with a non-exact row (a random point within the node's radius plus a height jitter from the
  node's 0x0c field) and start modes 2–3: not ported; no court-1 creature uses them (all start 0).
- `node_at` was checked only at reset (RAM), not mid-walk: the startle recordings never retarget on a non-exact row.
- No new PCSX2 recording of court 1: the existing `trig_c01`/`prox_c01` recordings and the court-1 RAM dump
  already show the original's idle (standing at home, no wandering) and startled walk, and the port matches them.
- A startle in the app was not watched on screen (no player went near a chicken in the shot); the sim side is
  the recording-checked `step_near`, the app only copies its world.
