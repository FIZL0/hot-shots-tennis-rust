# P14c — trigger creatures: survey and split

Object 0x290 bytes (vtable 0x1d2180): +0x50 type, +0x5c animation controller (0 = no model), +0x70 home matrix,
+0x1b0..+0x1ef current matrix (position +0x1e0), +200 sound countdown, +0x271 animating, +0x281/+0x135 moving.
Per-type rows: parameters 0x41aec0 + 0x74·type (copied to the stack each tick); callbacks 0x41c6e0 + 0x18·type
(callback, sound id, …); model/animation table 0x41c6d0 + 0x18·type (model pointers, flags). Type 53 is not a
type (the table ends at 52). Callback messages: 0 init, 1 destroy, 2 tick (end of the generic tick 0x3f2d50),
3 draw, 4 reset (new point), 6 ← 0x17 point over, 7 ← 0x19 scoreboard pause over, 8 ← 0xc serve placement; 0xe
saves/restores a replay snapshot; ball-hit creatures broadcast 0x14 (0x18b310).

## Generic
- 0x3f2d50 tick: path motion (circle about +0x150 at radius +0x19c, steer to +0x150, constant velocity +0x140),
  ground snap every N ticks (0x3f5ab0: ray y+100 → y−2 via 0x335ba0), waypoint reached → 0x3f40e0, animation speed
  scaled by distance to +0x160/+0x170 or by +0x27a, idle animation repeat after a random delay (+0x1a0), +200 → 0
  plays the type's sound and draws one RNG value (pan direction bit 16), then cb(2).
- 0x3f40e0 next waypoint: timers +0x13a/+0x13c with RNG jitter; path modes 0 random point, 1 loop, 2 ping-pong,
  3, 4 spline, 5 one-shot. 0x3f23b0 motion reset: clears +0x128..+0x290, start node, one RNG bit → reverse (+0x134).
  0x3f4d00 restarts all animation channels.

## Behaviours (callback: types)
- 0x3f5ba0 (0, 1, 5, 31, 32, 39) startled by a player/ball within 2.0: path + one-shot animation, once per point.
- 0x3f6210 (2) appears at the ball's landing spot after a game/set win (msg 6).
- 0x3f66c0 (3, 30, 49) stream sound every 14400 ticks. 0x3f6990 (4, 7, 11–13, 16, 17, 23–26, 35, 36, 41, 42, 45,
  46, 50, 51) ambient emitter: +200 = int(base·(1 − jitter·r·2⁻³²)) (row +0x54 short, +0x58 float); 36 sweeps pan.
- 0x3f6cb0 (8) stereo loops, random emphasised channel every 1200 ticks. 0x3f7350 (10) sound on an even counter.
- 0x3f74c0 (6) flyover after a game/set win. 0x3f7b80 (9, on no court) appears at the point winner.
- 0x3f8d60 (14, 21) box overlap with a fast player or the ball → animation, sound 0xb/0xa, 0x14.
- 0x3f9200 (15) a 50 % roll at a new point picks one of 4 routes, once per match.
- 0x3f9820 (22) deciding set only, head follows the ball. 0x3f9a10 (27–29) within 4.0 → animation once.
- 0x3f7e70 (19, 33) 50 % roll at serve placement arms a sound at the next new point.
- 0x3fa4b0 (34) random 300–600 tick repeat. 0x3f9d90 (37) one RNG draw per tick while visible.
- 0x3f9fb0 (38) ball overlap at speed ≥ 20 → faces the ball, animation, sound 0xc. 0x3fa750 (40) cheer on a human
  team's game/set. 0x3fa980 (43) ball overlap at speed ≥ 0x41cc60 → animation + sound 0x23 + 0x14.
- 0x3fad20 (44) deciding set only, random sound/idle timers. 0x3fb430 (47, 52) positional sound loop.
- 0x3fb670 (48) frame scrubbing while a player/ball is within 1.0. 18, 20: no callback (generic tick only).

## Split
Too big for one task (21 behaviours, a 3.7 KB tick with trig): P14c1 engine + idle animators, P14c2 sound
emitters, P14c3 proximity, P14c4 ball hits, P14c5 match events (PLAN.md). The shared RNG is the same MT as the
walkers' (P14b), so every draw a creature makes shows up in the walker recordings' draw counts.

Recorder: `tools/record_npc.py <slot> <n> <out> trig` records the trigger creatures instead of the walkers.
