# B2: net volleys flew long

## Cause
Three gaps in the port's net-area launches, all checked against live-ball recordings (match_s05, new_recording):
1. Volleys (ground branch 2) and dives used character 0's **stroke** (`strk`) tables. The game launches them as
   class 2 from the hitter's own **volley** tables `tr_pcNN_voly{kind}` (voly0/1 in TRAJnnA, voly2..4 in TRAJnnB).
   On the same recorded volleys, the stroke tables miss speed by 1e-2..0.46. That is the "lands out": e.g. Kaito's
   (c10) near-net volley v6174 came out 0.46/frame too fast.
2. Volley bounds differ from strokes: height top = −(1.6·1.3 + 0.5) = −2.58 (strokes −3.85). Kind 1 starts the
   height axis at −radius; the others use the stroke's z-scaled low end. Every low end is clamped to ≤ −radius (0.064),
   and so is the hit's own height.
3. **Near-net speed correction** in the cell lookup (flags on in retail): when the hitter is 0.5–6 m from the net
   crossing and ≤ 1.3 m high, speed is pulled toward the slowest corner of the cell's far-target plane:
   `speed += k·high·near·(slowest − speed)`, near = cos(fz·π/2) with fz = (to_net−0.5)/5.5, high =
   cos((1−h/1.3)·π/2), rise = elev − min corner elev (far plane only, the game's quirk), w = min(rise/7°, 1),
   k = w·(1 − min(|rise|/1.05, 1)). Without it v16538 misses by 8e-2 and v6174 by 2e-3; with it they're exact.

## Verified
`volleys_launch_like_the_game`: all 9 unscattered class-2 launches in the two recordings are exact (speed/rise < 4e-6,
flight frames equal). That includes Carol's (c6) kind-1 volley v8163 and kind-0 v9831. 6 use the base table,
3 use the dw1 table. Strokes still match (csv test), and the correction doesn't fire on them in those fixtures.
Rows with flight frames 0 (branch byte 5/1) aren't table launches and are skipped.

## Left out
- The up1/dw1/dw2 mode pick (from the timing selector, per-character thresholds) is P3. The port uses base tables.
- The game also makes some ground strokes class 2 (branch 1, kind < 2, |z| < 6.4, a stance value ≥ 0.6). That's not
  ported.
- The late/side timing scatter (lookup from hit−scatter, launch toward target+scatter) is P3.
- No on-screen vpad repro: the recordings already contain Carol net volleys and they're now exact.
