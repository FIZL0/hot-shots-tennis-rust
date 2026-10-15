# B381 — drop Lily, Anna, Suzuki, Chika from the modded roster

The four are HST's own chair umpires packaged as character mods by HST-MODS `tools/rerig/hst_umpires.py`
(`hst_umpire_00_chika`, `_01_suzuki`, `_02_lily`, `_04_anna`); their stood-up meshes look wrong as players.
Robot Tennis (`hst_umpire_03_robottennis`) and the real player Suzukis (`getagrip_pc12_suzuki`, `oob_pc13_suzuki`,
`gag_ump06_suzuki`) are untouched.

- `mods::dropped(id)` / `DROPPED`: the four ids. `mods::list` leaves them out by the mod.json `id` (so a renamed
  install folder `…-2` is dropped too) → off the character select roster and the Mods screen's installed list.
- `modstore::merge` and the cached listing in `modstore::list` filter them → not offered in the store.
- Tests: `mods::tests::dropped_umpires_stay_off_the_roster` (temp mods dir, Lily copy dropped),
  `modstore::tests::main_then_newest_pr` (store listing), `mod_match::tests::umpires_are_on_the_roster_and_play_a_rally`
  now expects only Robot Tennis; run once with `mods/hst-mods` → HST-MODS `out/mods` (all five umpire mods): passes,
  Robot Tennis plays a rally.

## Not verified / not 1:1
- Not compared with the original: remaster-only feature (mods), nothing in the original to match.
- Online: a peer on an older build could still pick one of the four; this build has it off the roster, so it'd be
  handled as any mod we don't have (stand-in path, `mods_online`). Not exercised in a netplay run.
- `modstore::mine` (your own uploads list) still shows them if you uploaded them, so you can remove them from the store.
