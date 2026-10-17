# B548 — online lobby at 4 fps with a big mod picked (iPhone Safari)

## Cause
The lobby's draw (`online/lobby.rs`: the top bar and the strip) built `Menu::room()` every frame, twice, even
when a room had come and wasn't used. `room()` → `picks()` → `who_pick` → `mods_online::Wire::of`, which parsed the
costume glb (`mods::skeleton`) and packed + FNV-hashed the whole mod (`pack`, 23.8 MB for oob_pc15_alex). That
starts once the mod has been read in (before then `Wire::of` failed fast on the still-lazy files), which matches
the log: 20 s read-in, then ~220 ms per frame until the match.

Measured here (release, native, fore_pc00_phoebe 14 MB, temporary test, not committed): `room()` 32.7 ms cold,
0.04–0.08 ms once cached. Twice a frame at that size, plus wasm and a phone in low power mode, fits ~220 ms.

## Fix
- `lobby.rs`: `room()` only built when it's drawn (no room yet; the strip: the host before the lobby opens).
- `mods_online::looks(dir)` (pack hash) and `kept(m, costume)` (skeleton + looks for `Wire::of`): kept per folder
  (and costume file) while `modfs::stamp(dir)` stays the same. The stamp walks the folder's file list: path, size,
  and on disk the modified time, in memory the buffer's address, still-lazy as its own kind; a reinstall, a
  browser read-in or an unload changes it, so the old court-mod "reinstalled mid-run keeps its old hash" ponytail
  doesn't apply to picks. `mods_online::find` uses `looks` too.
- Test: `pack_unpack_checks_what_arrives` checks one pack per unchanged folder and a re-hash after a same-size
  in-memory rewrite. B530's read-in test still passes (lazy → loaded changes the stamp).

## Not verified / not 1:1
- Not re-measured on an iPhone (no device here): the per-frame cost is gone from the code path and measured native.
- `modfs::stamp`: an in-memory file rewritten with the same size into a freed buffer's address would keep its stamp
  (noted as a ponytail); the hash would then be stale until the next change.
- Gameplay untouched (menus only), so no PCSX2 check needed.
