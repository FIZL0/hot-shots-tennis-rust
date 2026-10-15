# 8 — Get a Grip c05 (Street): the turned match eye inside a building (M8f5a4)

## What blocked it
A ray check (eye → HST's 9×9×2 playing-area points, the same lines as `court_mod::blocks`, over every rigid
court.glb triangle) on all 16 exports, both resting eyes (glTF (0, 11.89, ∓39.24)):
- c05 turned: one island of the ground model's `Outside_Building` node (203 triangles, x −8.9..19.5, y 0..12,
  z 31.8..44.2): a street block whose box holds the eye (just under its 12 m roof). It blocked all 162 lines. Also the
  +z low wall `ob16_wall` with its `Graffiti` and ivy (`ob14_tutanaga`), 3.4 m high at z 20 (8 lines). Rest end: the
  chain-link `Fence_SingleSide` at z −20 (9 lines).
- c05 isn't the only one. c02 (indoor): the eye is outside the hall, behind its end wall, with the roof over the
  court (162/162 from both ends). c09 rest: the eye is just above `ob13_house`'s roof and its parapet (162/162).
  c16: the studio's end walls (162/162 from both ends). c17: the additive glass partitions `garasu1`/`kakuheki05`,
  16 m high at z ±20 (162/162, see-through but brightening the whole view).
- The rest is fine: nets and their shadows, ground decals (c01's light pools, c06's snow, c13's stones, c15's
  leaves), and c01's pine bough over the sideline (1 line).

## Decision
I didn't drop anything: a building at one end is the backdrop seen from the other end, so dropping it leaves a hole
there. I didn't name whole nodes `__clear` either: `Outside_Building` is every building round the court in one node,
and its box would hide the far-end backdrop too. Instead the exporter splits the blocking **islands** into their own
`<node>_<k>__clear` children (standard §2 already applies `__clear` to court.glb; HST hides them only while they
block):
- `Scene.blockers`: islands (welded across materials) that a line from either resting eye to the playing area
  passes through. An island whose box reaches into the playing area (c16's half-shells, which span half the court)
  splits first into its flat faces (coplanar, connected), and only the blocking faces outside that area go. Without
  this its box would always block and HST would hide it in every view. The net, ground decals (< 0.5 m) and pieces
  wholly between the baselines (c01's bough) stay.
- `split_blockers`: the node keeps its mesh minus those triangles; each group gets a child mesh with only its own
  vertices (HST boxes the accessor's vertices).
- Result: c02 10 pieces (roof, both end walls, beams, windows, banner, posts), c05 7, c09 2, c16 6, c17 2, the other
  11 none. Listed in each mod.json `todo`. After the split the check finds only nets and decals on all 16 courts.
- opentennis-mods 52baa54 (`export_getagrip_courts.py`, `notes/getagrip.md`). No HST code changed.

## Checks (`context/m8f5a4/run1/`)
`--play --singles --court-mod <export>`, `HST_WEATHER=0`, with `HST_EYE` at each resting eye (game (0, −11.89, ±39.24),
17° down, tan 10°): `cNN-rest.png` / `cNN-turned.png` for all 16, tiled in `sheet-a.png` / `sheet-b.png`.
- c05 turned: the court, the far end's stands, the truck and the ads all show. No wall.
- c02: the hall without its roof and near wall, with the far wall and its windows as the backdrop, from both ends.
  c09, c16, c17: the court is clear from both ends, and the far end's pieces are still drawn.
- The other 11 courts look as before from both ends.

## Not verified / not 1:1
- Not compared with Get a Grip running (no PSP capture). That its camera keeps these pieces behind it is inferred
  from the geometry, as in M8f5a2/a3.
- Only the two resting eyes were checked. The selection is made from those eyes, and a piece that blocks only from a
  match-camera position away from rest isn't split, so it's never hidden. HST's runtime test (box, any camera) does
  hide a split piece wherever it blocks.
- Splitting islands is a choice for an addition, not a port. Hidden near-end walls and roofs (c02, c16) mean the
  match view looks into the hall over cut-open walls, as HST's own indoor courts do.
- c17's glass and c05's chain-link fence are see-through. They're hidden too, because the rule doesn't look at
  materials.
- c05's `__clear` pieces in the turned view, and c02/c16's roofs and walls from inside (cut-aways), weren't shot.
  Those cameras rely on HST's box test, which M8f5a3's unit test covers.
