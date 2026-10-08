# B40a: custom characters in the character select (2026-10-08)

Blocked: the character select doesn't exist yet (B40 is being built on its own branch in parallel) and mods can't
go into a match yet (M1c open). Done so far: the roster scan the select will call. Details: `1-ROSTER-PART.md`.

- `crates/hst/src/mods.rs`: `list(root)` reads every folder under `root` with `read`, sorted by folder; a broken
  one is left out with its `<path>: <reason>` logged (`warn!`).
