# Shot creation: precomputed trajectory tables (PART — format + lookup verified, inputs partly unknown)

Chain: player swing → (37e420) blend normal/charged params → (37dbf0) table lookup → (37e950) build launch frame
→ (379080) launch wrapper (2 MT19937 draws → ball+0x258/0x25c) → (375da0) write ball state.
Serve toss is separate (37af20): straight up, v = sqrt(2·g·0.9/3600·h), spin 0.1.

## TRAJ/tr_pcNN_<type><n>[_dwN|_upN].dat = 16×16×16 grid of u32 cells (16384 bytes)
cell bits 0-11 signed × π/4096 = elevation (rad); bits 12-23 signed /1024 = speed (m/frame); byte 3 = flight frames.
index = x + 16·y + 256·z, trilinear in f32 like the game; at an axis maximum the game keeps index 15 with fraction 1.0
and so reads one cell PAST the table (adjacent memory) — prototype clamps; decide later.
Axes (hit side mirrored so hitter is at z<0; target z += -0.15):
- x: horizontal distance hitter→net crossing, (d - 0.5)/17.67 for strokes ([-8.885,-17.42] serves)
- y: hit height between per-class/kind/character bounds (37c9a0; uses character heights at 0x2f0ee8.. in RAM)
- z: distance net crossing→target, strokes [3.0, 16.17] (kind 2: 6.4, class 3 kind 0: 6.9425; kind 4: [2.0, 8.22])
Then: flight frames (+0x260) = table frames + 1 (±1 by curve side logic), launch = pitch the hit→target frame by
elevation, velocity = forward × speed. Spin etc. from a per-character param record (0x4287c0 + char*0x451 + kind*0xdd + class*0xd),
in degrees/rad; kinds 2/3/4 rescale spin/first-bounce spin by court position (tables at 0x4108a8..).

## Verification (research/tools/traj_model.py, traj_inverse.py; slot 5 shots)
Inverse search (unknown target distance along launch direction): 7 strokes reproduce speed+elevation to <1e-4
with frames = table+1 (shots 002 004 015 016 020 023 033). Others: speed exact, elevation 0.2–0.7° off.
Suspects: per-character height axis (only one character's 1.1638 used), near-net safety lift (dist 0.5–6 m,
height <1.3 m; 41096c/d set), normal↔charged blend (37e420 param_1).

## Next
- Capture per-player character id + heights to fix the y axis; then the blend; then port to hst-sim::shot.
- Tables resident for pc00/01/02/05 in slot 5 (4 players).
