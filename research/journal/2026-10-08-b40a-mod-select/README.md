# B40a: custom characters in the character select (2026-10-08)

Remaster-only feature. Iteration: `2-SELECT-FINAL.md` (done). Roster scan: `1-ROSTER-PART.md`.

- `mods::list` reads `mods/` beside the disc image (folders without `mod.json` skipped quietly, broken ones logged).
- The select's second page (Tab / pad Select, shared by all players) lists the mods; cards show the mod's 3D
  preview, name, play style and (△) TParam numbers; picks go to the match as `--slot-mod N DIR`.
