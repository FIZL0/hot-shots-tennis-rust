# M8f — other games' courts, court NPCs and umpires as mods

One README per task; iterations below, newest last.

- `1-SPLIT-STANDARD-PART.md` — M8f split into M8f1–M8f10 (`plan/M8f.md`); M8f1 done: court/umpire standard,
  `mod.json` `kind`, roster skips other kinds. Next: M8f2 (Get a Grip umpire voices).
- `2-GAG-UMPIRE-VOICES-FINAL.md` — M8f2: SGXD cue→wave decoded (SEQD note in decimal digits), 8 Get a Grip umpires
  packaged as umpire mods with all 24 calls, donors by sameness/overrun. Next: M8f3 (loader).
- `3-UMPIRE-LOADER-FINAL.md` — M8f3 done: umpire mods on Select Umpire, `--umpire-mod`, calls over the donor's
  bank timed by the donor, online by id. Lily answers 0_13 (for M8f2).
- `4-GAG-COURTS-FINAL.md` — M8f5 done: 16 Get a Grip courts → court mods in ../opentennis-mods (court/sky/gimmick glb,
  lights, fog, BGM + ambience loops, lens flare, layout by surface + run-off); standard §2 gets singles/doubles
  variants. Next: M8f6 (court loader).
- `3-GAG-UMPIRE-MODEL-FINAL.md` — M8f4: Get a Grip's 8 umpire models + motions → each mod's `umpire.glb`
  (opentennis-mods `tools/psp2gltf/export_getagrip_umpires.py`), drawn on the chair with `--umpire-mod DIR`
  (`mods/umpire.rs`, `play/npcs.rs`). Turn direction checked against the disc clips. M8f3 already landed: the menu/online pick sets the model too.
- `4-COURT-LOADER-FINAL.md` — M8f6 done: court mods draw (court/gimmick/sky glb, baked light, fog, sun) and sound
  (BGM stream at the disc BGMs' RMS, ambience loops) over a hidden layout court; Select Court lists them; replays
  unchanged. Yamate's additive light material → M8f5a1.
- `5-GAG-ADDITIVE-FINAL.md` — M8f5a1 done: GaG's additive flag (texture record +0x24 bit 0) checked over all
  2558 court records; flagged materials export as `@add` (opentennis-mods 27c618f), yamate's black octagon is now sun
  dapples. c17's black bars come from a gimmick → M8f5a2.
- `5-COURT-NPCS-FINAL.md` — M8f7 done: Get a Grip galleries → `npcs.json` people (billboard cards with front/back/
  down/clap frames, c09/c16 stands, crowd sounds; opentennis-mods exporter + standard §2b); HST places them, cues them on
  the disc gallery's ticks (cheer/clap/groan/win/lose, serve/rally/change_ends, line judge point), plays their
  sounds; replays unchanged.
- `6-GAG-C17-BARS-FINAL.md` — M8f5a2 done: c17's bars were NORMA's 17 m opaque cable strands 9–12 m in front of
  HST's (further-back) match eye; the exporter drops gimmick.glb islands in HST's resting view within 20 m of the eye
  (opentennis-mods d22e1cc). c05's basket goals have the same cause → M8f5a3.
- `6-ONLINE-COURTS-FINAL.md` — M8f8 done: rooms carry the court mod (id, name) over its layout court;
  `--court-mod-id` draws it where installed, else the disc court plays; host's strip lists mods online.
- `6-STORE-FINAL.md` — M8f9 done: court and umpire mods upload, list (Courts / Umpires tabs) and install with
  `kind` in `meta.json` (`mods::read_any`); umpire heads on the Mods pages (B400). The parent still waits on M8f7
  (NPCs), M8f8 and M8f10.
- `7-GAG-C05-GOALS-FINAL.md` — M8f5a3 done: standard §2 `__clear` (hidden while it stands between the eye and the
  court, any camera; `court_mod.rs` `clear_view`); the exporter names c05's basket goals (and other pieces behind
  the near baseline in HST's view) so (opentennis-mods 68606db); break-only gimmicks stand still. c05's turned eye is
  inside a building → M8f5a4.
- `7-GOLF-VENUES-SURVEY-FINAL.md` — M8f10 done (survey): Fore!/Open Tee 2/Out of Bounds height maps and placements
  decoded, court spots per hole (opentennis-mods `tools/venues/`, `notes/venues.md`); no umpires in any; Open Tee 2
  Nakagawa h07 has real tennis courts. Exports → M8f10a1 (Nakagawa), M8f10a2 (Fore!), M8f10a3 (Out of Bounds).
- `8-GAG-C05-TURNED-FINAL.md` — M8f5a4 done: HST's turned eye stood inside c05's street block (court.glb
  `Outside_Building`); the exporter now splits every court.glb island between either resting eye and the court into
  a `__clear` child (opentennis-mods 52baa54). The survey of all 16 courts also caught c02, c09, c16 and c17: 32 resting
  views are now clear.
- `9-OT2-NAKAGAWA-COURT-FINAL.md` — M8f10a1 done: Open Tee 2 Nakagawa h07's tennis court exported as a court mod
  (opentennis-mods e05ece6, `tools/psp2gltf/export_opentee2_court.py`). The near court sits on HST's origin, lines exact at the
  outer edges, surface clay (earth courts), Western layout. HST now draws untextured glTF vertex colour at full
  brightness (`court_mod.rs`).
- `10-FORE-COURTS-FINAL.md` — M8f10a2 done: `fore2gltf court` (opentennis-mods 5411330) exports 14 of Fore!'s 15 courses as
  court mods: the flattest survey spot on drawn ground, hole ground + plants + sky moved onto HST's court, a laid
  grass court (slab, lines, net, chair), Jungle/Greece by run-off. Course 14 (mini golf) has no spot on drawn ground.
- `11-OOB-COURTS-FINAL.md` — M8f10a3 done: `oob2gltf/court.py` exports 6 of Out of Bounds' 7 courses as court mods
  (static `.rem` reader, plants merged per nine, hole textures from the disc's loose `data/`), hard on paved ground
  (c01 Extra, c06 Park) else Jungle. Course 05 has no clear flat spot.
- `12-SLAB-SURFACES-FINAL.md` — M8f10a4 done: the Fore!/OOB laid court wears the hole's own textures (exporters in
  opentennis-mods): grass courts in the green's texture with a fairway run-off, hard courts (OOB c01/c06) in the
  paving tinted blue/green with the paving as drawn beyond. Open Tee 2's court is real and needed nothing.
- `13-PARENT-DONE-WHEN-FINAL.md` — M8f done: the parent's done-when re-checked on the current build. Get a Grip
  court + Get a Grip umpire in a `--play` match, Browse Courts 16 / Umpires 8. Court NPCs on Get a Grip courts
  were dropped by the user (M112), so the layout court's people stand in.
