# B477: court NPCs clear of a court mod's geometry

**What stands on a court mod.** A court mod plays on its disc layout court, so the people come from that layout's
plant records: the umpire at the layout's chair, the walkers (the gallery, up to six) at their layout homes, court 5's
crowd (layout 4 only). The trigger creatures are already hidden under a court mod (B436). HST has no separate line
judges or ball kids; a mod's own people (`npcs.json`) stand where its author put them (none on Get a Grip's courts since
M112).

**Umpire.** B433 had already fixed this on its branch (`court_mod::umpire_seat`: court.glb's `umpire_seat` node,
Get a Grip's `pc_locator`; the umpire is drawn on the court's own chair, turning with the other arm on the far side),
but `task/B433` (fba72cf5) was never merged: a slot reset left it dangling. I cherry-picked it here (conflict with
B436's `court_mod` parameter resolved: one parameter, `as_ref()` for the trigger root), so B433 is ticked with this
merge. All 16 Get a Grip courts have an `umpire_seat` (checked in the glbs).

**Walkers.** `play/npcs/court_clear.rs` (new): at match start on a court mod, each walker is checked against
`court_mod::solid` (court.glb's triangles in game space at rest, this match kind's nodes, no `@add`/`@sub` light
pools), binned in a 1 m grid:
- ground = the lowest surface within 0.6 m of the layout's ground (so a bench, table or wall top is an obstacle);
- body = an upright cylinder 0.3 m + 0.1 m room, from 0.3 m (kerbs are stepped over) to 1.7 m above the feet,
  tested exactly (triangle clipped to the slab, seen from above);
- footing: ground within 0.15 m all round inside the body, and nothing dropping more than 0.6 m within 0.8 m (not a
  parapet over a fall);
- not moved further into the players' room (doubles court + 2 m / 4 m) than its home; a moved walker stays a body
  away from the umpire's seat and from walkers placed before it.
Home if it passes, else the nearest passing spot on rings every 0.25 m out to 6 m, else hidden. The walker is drawn
at its match position + that offset (`Moved`, PostUpdate before transform propagation); its moves, cheers and the
court RNG are untouched (`Npcs` isn't in `state_hash`; only the drawn transform changes), so replays and online stay
bit-exact. Stock courts don't run it (`MatchCourt` only).

**Result** (`every_court_mods_walkers_have_room`, all 16 courts, singles and doubles): no walker hidden. Moved
off geometry by 0.25–4 m on c02, c03, c04, c05, c06, c07, c09, c13, c15, c16, c17 and c20 (c05's back-wall walker
was behind the wall; c20's rooftop walker stood on the parapet). The rest only settle onto the mod's ground: c05's
street 0.55 m lower, c15's platform 0.3 m higher, c16/c20 floors 0.5 m off the layout's.

**Seen.** `--shot`s with a held eye on each moved walker (`context/b477/run1/`, `run2/sheet.jpg`): c02, c04, c05,
c06, c07, c09, c13, c15, c16, c17 and c20 (both rooftop walkers), all standing on the floor and clear of walls and
props. Run 1 caught the c20 parapet case (fixed with the lowest ground and the drop test).

## Not verified / not 1:1
- Not compared with the original or with Get a Grip: a court mod is ours (the original has no such courts), and Get a
  Grip runs on the PSP with no capture setup. The check is against our own drawing of the mod courts.
- The walker's spot is fixed at match start. When it dodges (players, ball) it moves from there, so it can brush
  geometry; the 0.1 m extra room covers small steps only.
- Court 5's crowd (layout 4) isn't moved: it's whole stands as single models, not people, and no installed court mod
  uses layout 4.
- Doubles chair positions and walkers were checked only by the test's numbers, not shot.
- c01, c08, c14 and c18 needed no moves beyond settling onto the ground (< 0.06 m), so they weren't shot.
