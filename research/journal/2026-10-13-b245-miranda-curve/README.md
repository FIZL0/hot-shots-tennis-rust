# B245 — Miranda's curve-ball trail

Miranda's rally slice (her effect row bends only the rally slice, bend[2] = 0.8) compared between PCSX2 (copy 2,
slot 8 = Miranda P1 via a cross-confirming copy of pick.py, COM rally returns; some flights had ball bend poked to
0.63 when her return came out unbent) and the port (`--play --chars 8,2,1,5 --stage 10`, a temporary hook in
`start_effects` that screenshots 2…40 ticks after a bent kind-1 hit; removed again). Captures: `context/b245/run4`
(original), `context/b245/port1` (before), `context/b245/port2` (after).

## What the game does (decompile)

- **Ribbon**: walks the kept samples newest first joined by *straight* segments (no curve fit) — same as the port.
  Side = normalize((older − newer) × (eye − point)) · max(0.08, 0.005·tan(fov/2)·dist); length = 20·speed (halved
  under a mode count > 2); alpha = colour alpha · max(1 − run/len, 0), last vertex 0; V from 1 down by run/len, no
  scroll. Colour stops by height all get the same colour. All already in `effects.rs draw_flight`. Nothing in the
  hit event handler is specific to curving shots: the bend only changes the ball path the samples follow.
- **Glow disc** (the difference): frame looking along the velocity (side = up × v). If the flight is within 60° of
  the court's length (a = atan2(|vx|, |vz|) < 60°) the frame is turned about world up by min(60° − a, a) towards the
  length. Then it is turned about its **side axis** (not the flight axis) by 90°·life/30: at the hit the disc lies
  along the flight (seen from the camera as a flat ellipse) and it rises to face the flight as it fades (alpha
  life/15 under 15 frames, as before). The port spun it about the flight axis, so it was always a face-on ring.
  Constants read live: fade 15 frames, 90°, 60°, factor 1.0.

## Fix

`effects.rs glow_turn`: the game's frame, tilt and side-axis turn. Side-by-side: the port's disc now starts flat
along the flight and ends as an upright ellipse near the net like the original's (run4 t4 vs port2 p16/p22). The
ribbon's shape, width, colour and length already looked the same; the sparks at the hit too.

## Not verified / not 1:1

- Screenshot timing on the original is not frame-exact (L3+R3 fallback lands ~38+ frames after the press, F8 via
  hyprctl didn't work on copy 2), so frames were matched by ball position, not tick.
- Several original flights had their bend poked (0.63) because the COM-driven return wasn't always bent; the
  natural one (t3) looked the same.
- The rotation signs (which way the tilt and the side-axis turn go) are taken from the shape of the rule (the tilt
  only makes sense towards the court's length) and the sce matrix helpers' usual convention; by eye they match.
- The glow's flag switch by flight facing (towards/away from the camera; a draw-setting word on the glow, value
  0x1a/0x1b) is not ported: its meaning wasn't found and no difference was visible.
- The ribbon's halved length under one of the game's mode counts stays out (unknown mode, as before).
- Port court 10 vs the original's slot-8 court and hands/costumes differ a little; comparison is by eye.
