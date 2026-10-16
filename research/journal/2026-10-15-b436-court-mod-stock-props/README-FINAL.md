# B436 — Get a Grip courts show HST's own props

**Cause.** Under a court mod `load` (main.rs) already spawns the layout court's models (ground, plant-record
props, backdrops, clouds) under the hidden `court_mod::DiscCourt` node, so the plant props were not it. The stock
objects standing on the mod court were the layout court's **trigger creatures**, which `play::npcs::setup` spawns
straight under `GameSpace`. Found by `--shot` of gag_c06_canada with its `court.glb` swapped for its sky (only the
disc's leftovers show): three lattice fences behind the far baseline, at stock court 6's spots. Logged with a
temporary print: court 6 = six `trg_M-07` fences (type 21) + the bear `trg_BA-01` (type 22).

Trigger models on the layouts the Get a Grip courts use (singles and doubles alike):

| layout (stage) | gag courts | trigger models |
|---|---|---|
| 0 (1) | c03 c05 c09 c15 | trg_sa-01 ×2, -02 ×2, -03 |
| 2 (3) | c13 | none |
| 3 (4) | c02 c16 c17 c20 | trg_M-04 ×4, trg_M-05 |
| 5 (6) | c01 c06 c08 c14 c18 | trg_M-07 ×6, trg_BA-01 |
| 6 (7) | c07 | trg_M-08, -09, trg_M-10 ×2 |
| 10 (11) | c04 | trg_M-14 ×2 |

**Fix.** `npcs::setup` takes the match's `MatchCourt`; with one, trigger creatures are spawned under their own
hidden `DiscCourt` child of the game space. They are still made and stepped as before (their draws from the court
generator, timers, sounds), so the match is unchanged; their `Visibility::Inherited`/`Hidden` toggles in `step` stay
under the hidden parent. Umpire, walkers and court 5's crowd (people) stay shown, as M8f6 intended.

**Checks.** Bare-mod shots at layouts 0, 3, 5, 6, 10 after the fix: only the umpire and walkers remain. Full
gag_c06 shot: the stock fences are gone; the red lattices by the kayaks and the ice rock are the mod's own (absent
from the bare shot). `court_mod_match` test: on layout 5 the seven trigger creatures sit under `DiscCourt`, and the
match still hashes the same as on the bare layout court through two points.

Test side note: the mods repo's current `out/mods/gag_c01_yamate` build has no `npcs.json`, so that test's M8f7
people check failed regardless of this change; it now skips that part with a message when the build has none.

## Not verified / not 1:1
- **Stock collision still applies** under a court mod (`hst_sim::court::world`: net, posts, plant-record props).
  Deliberately left: a court mod "changes what's seen and heard, never the match" (M8f6) and an online room mixes
  peers with and without the mod (M8f8), so the collision world has to stay the layout court's for replays and
  netplay to agree. A ball can bounce off an unseen stock prop on the mod court. Removing that would need a design
  decision (e.g. mods carrying their own collision and the room requiring the mod), not a quick fix.
- Layouts 0/3/6/10 checked by bare-mod shots only, not with each gag mod's full court drawn.
- Nothing to compare against the original: court mods are additions.
