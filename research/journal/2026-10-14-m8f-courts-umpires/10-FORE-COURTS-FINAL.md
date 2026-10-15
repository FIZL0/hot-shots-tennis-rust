# M8f10a2 — Fore! golf venues as court mods (FINAL)

## What
`../open-tennis-mods/tools/fore2gltf/src/court.rs` (open-tennis-mods 5411330): `fore2gltf court <out/files/fore>
<out/venues/fore.json> <HST iso> <out/mods> [course…] [--end -]` writes `out/mods/fore_cCC_<stem>/` (`mod.json`,
`court.glb`, `sky.glb`) for 14 of Fore!'s 15 courses in ~1 s each. Full method and the per-course table are in
open-tennis-mods `notes/venues.md` → *Fore! (`tools/fore2gltf court`)*.

- **Spot.** The flattest fitting survey spot whose ground the hole model actually draws (≥ 95 % of 0.5 m points
  under the slab). The mini-golf height maps run on as flat green far past the drawn holes. Course 14 has no
  such spot (30 candidates tried), so it isn't exported. Course 15 falls back to h04.
- **Venue.** The hole ground model (GRD, read by path), plant categories 17–20 (trees, props, structures,
  billboards) placed with HST's `world::place`, all moved so the spot's centre lands on HST's origin, the court's
  length along HST's z. Anything within 45 m of the court is `__clear`.
- **Laid court** (standard §2: Fore! has no court): a striped grass slab over the run-off with skirts down to the
  ground, lines at HST's judging edges, a net panel with tape and strap, posts at ±6.4 m, and a simple umpire chair
  at the layout's own chair position (from the HST ISO). The slab goes 3 cm above the highest ground under it plus
  2 m round it, taken from the height map and from hole-model triangles within 1 m of it.
- **Sky.** The hole's first in-season sky, plus the course's cloud/accessory/backdrop models from the right nine.
  Yaw and lift follow HST's own backdrop rule from Fore!'s envir files (same format as HST's).
- **Layout.** Surface grass everywhere. Of HST's grass courts, the one with the closest run-off (OT2's rule): Jungle
  for 8 courses, Greece for 6.

Bugs found while checking: GRD's `data/arrow.MDL` sorted ahead of the hole model, and `HOL…/holeend/` reuses the
hole model's stem. Both made some courses export the aim arrow or the hole-end card as "ground" (c09's slab
floated 12 m up). Whole holes draw blended, so the blend bit can't separate ground from decoration; the height map
does that job now.

HST change: `modstore.rs` test `package_then_install` packaged the alphabetically first mod in
`mods/hst-mods` and failed once `fore_c01_hakone` (a court mod) sorted first. It now picks the first character mod.

## Checks
HST release, `--play --singles --court-mod <dir>`, `HST_WEATHER=0`, `tools/shot.sh` with `HST_EYE` held eyes:
- `context/m8f10a2/run1`: c01/c08 play view. Slab, lines, net and chair sit on HST's court, the umpire sits on the
  chair, and the players stand on the slab. The run1 c07/c09 shots predate the arrow fix.
- `context/m8f10a2/run2`: all 14 courts from high above (`high0.jpg`, `high1.jpg`). Each court sits on its hole's
  ground with the venue around it: c01 fairway by the pond, c02 above the lagoon, c04 between town houses, c09 on
  the fort wall, c13 on the beach. `ends.jpg` shows both resting views of c04/c07/c09/c13: clear, with no building,
  fence or wall in front of the court.
- Run-off grows on the height map. The slab's height range ≤ 0.5 m on every course (`mod.json` `layout_why`).

## Not verified / not 1:1
- Not compared with Fore! running: no PS2 Fore! capture. Checked by eye in HST only, from fixed eyes (play view,
  both resting views, high above), not through a whole match.
- Surface **grass** for every course is our choice (a court laid on the lawn); Fore! has no courts. The slab
  texture, lines, net, posts and umpire chair are procedural, not taken from any game.
- Spot = flattest survey spot on drawn ground, an exporter rule. Course 14 (mini golf) gets no court: no 40 × 20 m
  spot on its drawn ground.
- c13's slab overhangs the drawn sea edge: the height map has no water lie there. c07 lies in a fenced road
  cutting, c09 on top of a fort wall with battlements beside it. They fit the rules, but they're odd venues.
- c04's town-house backs and roofs show flat grey. They're drawn as the model has them; whether Fore! ever shows
  them isn't checked.
- Left out: Fore!'s drifting clouds (CMN cloud models), creatures, galleries, pin flag, hole-end scenery, course
  music/ambience. There's no manifest `light`: the envir light isn't mapped, so the layout court's light is used.
  The layout court's NPCs (Jungle's tourists and crocodile, etc.) still appear, as for other court mods.
- p10 = front nine and h00, p01 = back nine: assumed by analogy with Out of Bounds, not confirmed.
- Holes without a hole envir (h00) get HST's default backdrop lift of 370. Not checked against Fore!.
- Courses 14/15 names unknown (SYSTEM.BIN has a "Mini-golf 2" string): exported as "Mini-golf course 15".
