# M92 — Get a Grip pc14 (Helghast) voices

M8f2's SEQD cue decode gave `sgv/dgv_PC14` cue names. `out/voices/getagrip/pc14/` already had the
`sgv_pc14a_pc_voice_<cue>.wav` aliases (33 cues incl. `smash_0-2`, `st_ji_0-4`, `point_get_0/1`, ...), so in ../HST-MODS:

- `tools/voices.py` rerun: 0 new links (aliases current).
- `tools/rerig/package.py getagrip`: 15 mods; `getagrip_pc14_helghast` now has `voice/` (27 wavs) and
  `voice_counts` {0:3, 1:5, 2:3, 3:3, 4:2, 6:3, 7:2, 8:2, 9:2, 10:2}, the same as the other 14. pc13's mod.json
  unchanged by the rerun. No duplicate wavs; spot-checked 0_0/1_0/6_0/7_0/10_0 are byte-equal to the
  smash_0/st_ji_0/start_0/point_get_0/set_lost_0 aliases (regions 001/004/018/022/031).
- HST-MODS `notes/audio.md` updated (commit 45bc846). Mod output is under `out/` (game data, not in git).

## Not verified / not 1:1
- No whisper transcript of Helghast's packaged wavs: `faster-whisper` isn't installed on this machine. The
  cue→wave decode itself was checked by transcript in M8f2 (umpire banks, `dgv_NPC_m06a`), not on PC14.
- Programs 1/2 (st_ji/st_nb) stay an approximation, as for every Get a Grip mod (manifest todo).

## Side fix
`play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally` failed on main too: `mods/` (symlink to the
main checkout's user mods) holds one store-installed umpire (`hst_umpire_04_anna`, has `store.json`) and the test
asserts all five packaged ones as soon as any is present. It now skips when the umpire mods came from the store.
