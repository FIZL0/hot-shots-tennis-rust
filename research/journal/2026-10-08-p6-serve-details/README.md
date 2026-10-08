# P6 serve details (2026-10-08)

## Shot effect table
Per-character row (0x7c bytes), read by `hst-data::exe::ShotEffects` (`Exe::shot_effects`):
- +0x24 bend class 0 kind 0; +0x28 bend class 0 kind 1; +0x2c bend class 1 kind 1
- +0x30 lob curve class 1 kind 3
- +0x38 up1 slice-serve switch
- +0x40 turn in degrees (x 0.017453292 -> `Shot::bounce_turn`)
- Enable byte not read (ponytail).

## Special condition (37f660 -> `shot::special`)
Class-0 serve with strong toss and kind < 2 needs |timing offset| < 2; anything else needs grade 1 or 2.

## Scaling (37e950 -> `shot::effect`)
- s = clamp(sqrt(dz^2 + dx^2) / 23.77, 0, 1), with ps2 ops.
- bend x s, negated if flip (branch 1/2 with odd motion id = backhand `sh_pcXX_b_*`), negated again if lefty.
- curve x s; turn negated if lefty.
- Debug force switch and practice exception left out (ponytail).

## Side axis +0x90 (37b110 -> `shot::side_axis`)
FPU flat unit vector, then VU0 normalize, cross(normalize(up), dir), normalize.
`launch_frame`'s row 0 does NOT renormalize the flat vector first: using the renormalized one there broke a
1p3goodcpus volley (vsync 19727, character 10), so the two stay separate.

## Bent serves, slice serve, dw1 weight
- `serve::inside` keeps unscaled service-box margins when the serve has a bend (`bent`).
- Character 10 (up1_slice) uses `tr_pcCC_serv1_up1.dat` + its variant record when a slice serve is special.
  No recorded case, untested.
- The dw1 serve variant weight is per character (`serve_variant_weights`); character 10's is -0.454545, not -0.5.

## Tests (crates/hst-sim/tests/serve.rs)
- Bend/curve/turn bits (+0x250/+0x254/+0x1b0) and side axis on every recorded serve.
- `round1_turned_serves`: 2 turned serves, vsync 25558 and 28213, Will left-handed.
- `rally_effects_like_the_game`: every lob curve bit-exact; bends to sign (round1 has 2 Kaito backhand slices, flipped).

## Lets
judge.rs/score.rs already port the call: NetIn on a serve -> call 4 let, replay same point; NetOut -> fault.
No fixture has a let. round1's four "net, no point" serves (vsync ~17039, 20097, 23099, 27113) were all
classified as net faults, not lets: the following serve's placement (`flow::serve_placement`) only fits
faults=1 (second-serve stance) at vsync 17118, 20176, 23178, 27192. So the fault-after-cord flow is consistent
with the original, but a let has never been recorded. Recording one needs a capture where a serve clips the cord
and lands in (rare in CPU play: 0 in round1's ~40 points).

## Open
- Record a let serve (bot-only slot 5 match, look for call 4 at +0x426) and check the replay flow against it.
- Up1 slice serve of character 10.
