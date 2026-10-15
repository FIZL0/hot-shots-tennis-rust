# M8f10 — Fore! / Open Tee 2 / Out of Bounds venues and umpires (2026-10-15)

Research and tools only. Everything is in ../open-tennis-mods (453c8f8, 68606db: `notes/venues.md`, `tools/venues/`);
nothing changed in this repo's code. The court-mod exports are split off as M8f10a1–a3.

## What was found

- **Umpires: none.** None of the three golf games has a chair umpire, line judge or referee (no such files or
  models). Caddies exist (Fore! `KC/`, Out of Bounds `kikkawa/caddie`) but aren't umpires. No umpire task.
- **Height maps** (`*.HMP`) decoded, same layout in all three (Out of Bounds big-endian): x0, z0, nx, nz, then
  nx·nz f32 heights and nx·nz u8 lies, x-major, 1.3 m cells. Checked against the placed pins/cups and tees
  (Fore! ≤ 3 cm, Open Tee 2 ≤ 18 cm, Out of Bounds ≤ 0.1 m) and Fore!'s ground mesh (median 2 cm). Lie 27 = water
  everywhere; green 4 (Fore!, OT2) / 6 (OOB); fairway 1.
- **Placements**: Fore! and Open Tee 2 use HST's 0x40-byte plant record; Out of Bounds has its own big-endian
  record, one file per kind per hole.
- **Survey**: a 40 × 20 m footprint (doubles court + HST's smallest run-offs), ≤ 0.5 m height range, no water,
  clear of trees/props/structures/billboards, yaw every 15°. Most courses have several holes that fit (tables per
  course with names in `notes/venues.md`); Open Tee 2's unnamed nine-hole "western" course (19) has none.
- **Real tennis courts**: Open Tee 2 Nakagawa Public G.C. hole 7's scenery model has two tennis courts with nets,
  posts, asphalt surfaces and a wire fence — the best first court mod (M8f10a1). Only that one in the three games.

## Not verified / not 1:1

- Footprint, flatness threshold (0.5 m) and blocker radii are judgement calls, not from any game; a spot that
  "fits" still needs a look in HST once exported (M8f10a1–a3).
- Lie codes other than water (27), green and fairway (1) aren't identified (rough/bunker/path guesses only).
- The height maps cover only the playable hole; flat scenery beyond them (e.g. Nakagawa's courts) isn't surveyed.
- Nakagawa's court position is in psp2gltf's raw mesh space; how that maps onto the hole's height map isn't
  confirmed (the height map at ±z there isn't flat, so the courts lie outside it or the axes differ).
- Course names for Fore! 14/15 (likely mini-golf), Open Tee 2 19 and Out of Bounds 07 weren't found.
- Not compared with the running games (no emulator needed; read from the extracted files only), nothing drawn in
  HST yet.
- The "no other tennis courts" check is a search for the string `tennis` over the extracted files; compressed
  files inside them weren't searched.
