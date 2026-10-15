# M8f10a4 — the laid court's surface on the golf court mods (FINAL)

## What
The user's screenshot (Out of Bounds' Highland Country Club, c01) showed the laid court as a flat one-colour slab:
a plain blue court on a plain green surround. Fore!'s were plain striped green. Of the task's two routes I took
the exporter one: the slab now wears the hole's own textures (the other route, drawing the layout disc court's
ground under the mod, would put Park's/Jungle's stadium floor under a golf course). No HST code changed: the
loader already draws textured court.glb materials. The code is in opentennis-mods (`tools/fore2gltf/src/court.rs`,
`tools/oob2gltf/court.py`, `notes/venues.md` → *Slab surface*; opentennis-mods cb99baa), and all 14 Fore! and 6 OOB court mods in
`out/mods` (= `mods/hst-mods`) were re-exported.

- **Which texture.** For a given area, the hole model's textured material that covers the most ground seen from
  above: the green-lie cells (Fore! 4, OOB 6), the fairway-lie cells (1), or the ground under the slab. The greens
  found are the materials named `Green`/`グリーン` where a course names them. The ground under the slab is no good
  for run-off on its own: it is stone, bunker sand or a cart path on several Fore! courses.
- **Grass** (all of Fore!, OOB c02/c03/c04/c07): the court plus an apron (1.8 m beside the doubles sidelines,
  3 m behind the baselines) in the green's texture, with the mown stripes kept as an 8 % darker vertex colour. The
  rest of the slab and its skirts use the fairway's texture. Fore! c15 (mini golf) has neither lie on its spot's
  hole, so its turf under the slab serves for both.
- **Hard** (OOB c01 `cart00`, c06 `asphalt`): the paving under the slab, tinted to the old blue court and green
  apron (vertex colour = target ÷ the texture's mean colour). The apron is the ITF's recommended 3.66 / 6.40 m.
  Beyond it the run-off is that paving as the hole draws it, so the slab runs into the car park / helipad.
- **Scale and colour.** Each surface is tiled at the hole model's own texel scale (area-weighted √(world/UV area):
  greens 0.6–1.1 m a repeat, fairways 1.8–2.7 m) and drawn at the mean vertex colour the hole gives it. It is
  opaque: the hole materials draw blended, and a blended slab would sort against them.
- **Open Tee 2** (M8f10a1, Nakagawa h07) needs nothing: its court is the venue's real asphalt court with OT2's own
  textures, not a slab.

## Checks
I took HST release shots with `--play --singles --court-mod <dir>` and `HST_WEATHER=0`, using `tools/shot.sh` with
held `HST_EYE` views (play view at 41 m / 17°, high above, low from the side). They are in `context/m8f10a4/run1`
(OOB c01 before and after the first tint) and `run2` (OOB c01 and c06 with the paved run-off, Fore! c01 before and
after).
- Fore! c01: the court is the hole's mown green texture with stripes, and the run-off is fairway that meets the
  hole's own fairway with no visible seam. From the side the slab edge is a faint step only.
- OOB c01 (Highland): a textured blue court with a green apron, on the car park's own paving, which runs on into
  the car park. OOB c06: the same on the helipad's asphalt.
- No HST code changed; `tools/done.sh` runs the suite.

## Not verified / not 1:1
- None of the golf games has a tennis court, so there is nothing to match. The surfaces and the apron sizes are
  our choices, taken from the hole's own textures. They were checked by eye in HST only, from held eyes, not
  through a whole match.
- The hard court's blue and green are the old plain colours put on the paving's grain. They are not taken from
  any game.
- Only Fore! c01 and OOB c01/c06 were looked at after the change. The other 17 courses were re-exported with the
  same rules and not shot again. The exporter logs which surfaces it picked for each course.
- Lighting: no manifest light (as before), so the slab is lit by the layout court's weather the same way as the
  hole ground around it.
