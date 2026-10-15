# 1 — split, standard, `kind` (M8f1)

## Split
M8f (every court, its NPCs and every umpire from the M8 games, loader, online, store tabs) is far past one
iteration. Split into M8f1–M8f10, `plan/M8f.md`, lines before M8f in `plan/TODO.md`. Order: standard first, then
the umpire end to end (voices M8f2 → loader M8f3 → model M8f4: the smallest slice that reaches the done-when's
"umpire calling the score in their own voice"), then courts (extract M8f5 → loader M8f6 → NPCs M8f7 → online
M8f8), the store (M8f9, after B373 draws the kind tabs) and the golf games' venues last (M8f10).

## Findings that shaped the standard
- HST's own umpire banks are already named `gag_vcNN{a..d}` (`audio.rs umpire_bank`), the same names as Get a
  Grip's umpire banks (`xbdata/sound/game/010_umpire_NN_0V.xb/.../gag_vcNNa|b.sgd`, plus `_rev` copies): the PSP
  sequel kept the umpire bank layout's naming. Whether its programs/keys line up with HST's is M8f2's question
  (the notes name GaG cues `vc_voice_point_*`, `tiebreak_*`, `miss_*`).
- The umpire's word lengths drive the sim: `play.rs` sets `umpire_voice` from the playing voice's frame count and
  `Umpire::step(playing, …)` starts the second word / the next phase off it (`call_match`: the match waits for the
  voice). So an umpire mod must not change timing: the standard has every call timed by the mod's `donor` disc
  umpire, the mod's clip only replacing what is heard. Same rule makes online safe (a peer without the mod hears
  the donor, same ticks).
- Call table (24 keys over programs 0/1/2) taken from `hst-sim/src/umpire.rs` (`call_score`, `announce`,
  `point_over`, `call_match`) and `judge.rs` `Verdict::call` (1 out, 2 fault, 3 double fault, 4 let, 5 net; 6 is
  silent). Program 0 keys 11/12 = the server's / receiver's side after advantage, game and set.
- Court mods: gameplay from a disc `layout` (0–10, `--court` order); surfaces by `play/main_menu.rs` `SURFACE`
  (clay 0 1 2 8, grass 4 5 7 9, hard 3 6 10).
- Get a Grip ships 17 courts (`game/200_court/courtNN{,_gallery,_gimmick,_obj_0,_obj_1}.xb`, 01–09, 13–18, 20;
  `201_court` lighter copies), each with `ground/*.i3r`, a `_col` collision mesh (unused by us: the layout's is),
  `setting/cNN_se.csv` (sounds) and `cNN_areaclip.csv`; `200_court/umpire.xb` and `lensflare.xb`.

## Done (M8f1)
- ../opentennis-mods `standard/HST-COURT-UMPIRE-STANDARD.md` (new): §1 kinds, §2 court mod (`layout`, `court.glb`,
  `sky.glb`, `gimmick.glb`, `light`, sound, `npcs.json` §2b), §3 umpire mod (`donor` timing/fallback, call table
  §3a, `umpire.glb` §3b). Character standard §6 points at it for `kind`; README lists it.
- `crates/hst/src/mods.rs`: `Kind` (`Kind::of`, `Kind::of_dir`); `read` refuses a non-character with
  "`kind` is `court`: not a character mod"; `list` (the character roster) skips other kinds without a warning.
  Test `other_kinds_stay_off_the_roster`.
- Store `Meta` gets no kind field here: B372/B373 says it's shared with M5c and B373 is in flight in another slot;
  M8f9 adds it if B373 hasn't.

## Not verified / not 1:1
- The court/umpire standard is a draft: the court `light` fields, `npcs.json` cues and `umpire.glb` animation
  names are proposals that M8f4/M8f6/M8f7 confirm against what the Get a Grip data actually holds.
- The meaning of umpire program 0 keys 11/12 ("server"/"receiver" side) comes from the port's code, not from
  listening to the disc takes; M8f2 transcribes them.
- The `court.glb` alignment rule ("same origin and axes as the disc court's models") is not checked against a
  disc court export yet (M8f6).
