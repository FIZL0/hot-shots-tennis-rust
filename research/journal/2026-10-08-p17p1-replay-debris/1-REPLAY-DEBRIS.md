# P17p1: instant-replay dive debris and the replay footstep sound

## What gm+0x344 is

gm+0x344 is the instant-replay flag; there is no separate training or close-up mode. Poking it directly stalls the game. A real replay can be forced instead: once a point is over (gm+0x346 set, 0x344 clear), set byte gm+0x32e = 1, gm+0x350 = 1 and byte gm+0x35c = 1. The replay starts about 38 frames later and shows the end of the point (350–470 frames).

Slot 5, lock-step from the load, with every point's replay forced: points end at 8502, 9656, 11185 and 12937. The replay of the 11185 point runs 11223..11694, and in it a dive throws grass at vsync 11512; that group is dropped at 11543. Replays consume state, so the earlier replays must also be forced for the dive to land on that frame. `tools/record_foot.py` does this with `FOOT_DEBRIS=grass|dirt` (dirt also flips the court table's grass byte off and clay byte on) and `FOOT_FROM`. Fixtures: `foot_s05g.bin` and `foot_s05c.bin` (120 frames from 11450).

## Kind and groups (run object)

- **Layout:** slots at +0x71c0 (0x90 bytes each, 20 per group, 3 groups); group count at +0x9ec0; kinds at +0x9ec4.
- **Kind store:** at every dive, after the rings and not gated by the replay, the game stores kinds[count] = 1 if the court is grass (court +0x81), else 0 if it is clay (+0x82), else −1.
  - Kind 0 calls the clod spawn 3985d0.
  - Kind 1 calls the blade spawn 398c30, which is also gated on !wet.
  - Both spawns are gated on 0x344.
- **Allocation:** 397d90 drops group 0 (397c80) when count > 2, then increments count. The spawn uses the row of kinds[count−1], so with three groups up the kind is stored at kinds[3] (overflowing onto the first dash matrix's x) and the new group reads its predecessor's kind.
- **Drop:** 397c80 copies groups 1.. down one; the kinds are not shifted.
- **Aging drop:** in aging, a group with no live bit drops group 0 (not itself), and the same index is then re-processed.

## Spawns

Common to both:
- lunge = (float)((double)pl+0x3f80 · 0.3).
- dir = level unit (player − motion object +0x84 matrix pos) · lunge, with y = 0.08 and w = lunge·0.
- origin = (right toe x, −0.05, right toe z, toe w).
- basis = 328780(dir) (= `effect::impact_matrix`); its row 3 is replaced by base = origin + row2·reach.
- Per bit, the first MT draw turns the basis: basis = rotZ(u·2π)·basis.

Clods (4 MT draws per bit):
- r = u; dv = normalize(base + row0·r·spread − origin), with the length summed y, x, z (madda order).
- mag = √(dir.z²+dir.x²), raised to row +0x10 (1.1) with `powf` (1161c0 → 1121f0) when that is positive, × (lo + (hi−lo)·(u·2⁻³²)).
  - Reading it as a square root missed by a constant ×0.757; the summation and lerp orders came from the asm and fixed the last ulp.
- vel = dv·mag, then y = −|y| and y ≥ lift (−0.03).
- size = ((1−r)(hi−lo)+lo)/2.
- cell = cells[MT>>16 & 3].
- Rows 0..2, spin and colour of the slot are left stale.

Blades (5 MT and 3 C `rand()` draws per bit):
- m = identity; the turn draw is spent (the basis is unused).
- size from draw 2; vel = dir·push with y = 0; cell from draw 3.
- spin: z, y and x from `rand()`, each lo·k + (hi·k − lo·k)·2⁻³¹·(rand & 0xfffff800), k = π/180.
- m = euler·m, where euler = Rz·Ry·Rx (126010).
- pos = origin + (lerp(offset x), lerp(offset y)) from MT draws 4 and 5.
- colour = court +0x10 for bits 0..10, +0x20 after.

## Aging (39a720, after the puffs and prints)

- **Clods:** pos += vel; alive = pos.y < 0; vel *= decay; vel.y += fall·0.0027222224 (0x3b326750); size *= shrink.
- **Blades:** pos = pos + (vel + wind·0.5), in that order (the other orders miss by an ulp); pos.y += sink; vel *= decay; size *= shrink; m = euler(spin)·m; alive = !(y ≥ 0 || size < 0.01).
- The wind is weather +0x1a20, the same vector the puffs drift with.

In the recording another system draws 24 `rand()` before the blades on the dive frame. The test lines the generator up from the recorded end state.

## Draw (3964a0)

Every group is drawn with group 0's kind and the latest spawn's model (run +0x60).
- uv = (cell + 0.5)/W .. (cell + 31.5)/W.
- Clods draw two quads per bit:
  - a camera billboard in the court +0x30 colour, alpha 128;
  - a shadow at y −0.01, (32, 32, 32) with alpha 90.
- Blades draw one quad, pos ± row0·size ± row2·size, in the slot colour.

## Replay footstep sound

In 3993f0, when a foot lands, the game plays bank program 3 at 0x69 at the player's model origin (key as for the dive thud) if all of these hold:
- 0x344 is set;
- the player's state byte +0x3fa5 == 1;
- there are fewer than 3 players;
- camera mode (gm+0xbc)+0x51 is in 0x56..0x59;
- the view test passes. The level bearing from the camera (1e7d20) to the player must be within 50° (cos ≥ 0.64278764) of the camera look (1e7d30/38). It also needs either cos ≥ 0.9961947, or 125 < 1e7ff0 / (player · view 1e7e30).z.

This is ported as `sound::step`, `sound::step_heard` and `Feet::stepped`. It is unchecked: slot 5 is doubles, and no singles replay with that camera has been captured.

## Result

`foot_debris_s05` is bit for bit on both fixtures (every bit's position, velocity, size, cell, and for blades the matrix, spin and colour).

The app (`foot_fx`) draws the debris but never sets `Feet::replay`, because there is no instant replay yet (P0b4d). So nothing shows in play, and the footstep sound is not wired up.
