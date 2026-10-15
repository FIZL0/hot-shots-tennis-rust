# M8f — other games' courts, court NPCs and umpires as mods

One README per task; iterations below, newest last.

- `1-SPLIT-STANDARD-PART.md` — M8f split into M8f1–M8f10 (`plan/M8f.md`); M8f1 done: court/umpire standard,
  `mod.json` `kind`, roster skips other kinds. Next: M8f2 (Get a Grip umpire voices).
- `2-GAG-UMPIRE-VOICES-FINAL.md` — M8f2: SGXD cue→wave decoded (SEQD note in decimal digits), 8 Get a Grip umpires
  packaged as umpire mods with all 24 calls, donors by sameness/overrun. Next: M8f3 (loader).
- `3-UMPIRE-LOADER-FINAL.md` — M8f3 done: umpire mods on Select Umpire, `--umpire-mod`, calls over the donor's
  bank timed by the donor, online by id. Lily answers 0_13 (for M8f2).
- `4-GAG-COURTS-FINAL.md` — M8f5 done: 16 Get a Grip courts → court mods in ../HST-MODS (court/sky/gimmick glb,
  lights, fog, BGM + ambience loops, lens flare, layout by surface + run-off); standard §2 gets singles/doubles
  variants. Next: M8f6 (court loader).
- `3-GAG-UMPIRE-MODEL-FINAL.md` — M8f4: Get a Grip's 8 umpire models + motions → each mod's `umpire.glb`
  (HST-MODS `tools/psp2gltf/export_getagrip_umpires.py`), drawn on the chair with `--umpire-mod DIR`
  (`mods/umpire.rs`, `play/npcs.rs`). Turn direction checked against the disc clips. M8f3 already landed: the menu/online pick sets the model too.
- `4-COURT-LOADER-FINAL.md` — M8f6 done: court mods draw (court/gimmick/sky glb, baked light, fog, sun) and sound
  (BGM stream at the disc BGMs' RMS, ambience loops) over a hidden layout court; Select Court lists them; replays
  unchanged. Yamate's additive light material → M8f5a1.
- `5-GAG-ADDITIVE-FINAL.md` — M8f5a1 done: GaG's additive flag (texture record +0x24 bit 0) checked over all
  2558 court records; flagged materials export as `@add` (HST-MODS 27c618f), yamate's black octagon is now sun
  dapples. c17's black bars come from a gimmick → M8f5a2.
- `5-COURT-NPCS-FINAL.md` — M8f7 done: Get a Grip galleries → `npcs.json` people (billboard cards with front/back/
  down/clap frames, c09/c16 stands, crowd sounds; HST-MODS exporter + standard §2b); HST places them, cues them on
  the disc gallery's ticks (cheer/clap/groan/win/lose, serve/rally/change_ends, line judge point), plays their
  sounds; replays unchanged.
- `6-GAG-C17-BARS-FINAL.md` — M8f5a2 done: c17's bars were NORMA's 17 m opaque cable strands 9–12 m in front of
  HST's (further-back) match eye; the exporter drops gimmick.glb islands in HST's resting view within 20 m of the eye
  (HST-MODS d22e1cc). c05's basket goals have the same cause → M8f5a3.
- `6-ONLINE-COURTS-FINAL.md` — M8f8 done: rooms carry the court mod (id, name) over its layout court;
  `--court-mod-id` draws it where installed, else the disc court plays; host's strip lists mods online.
- `6-STORE-FINAL.md` — M8f9 done: court and umpire mods upload, list (Courts / Umpires tabs) and install with
  `kind` in `meta.json` (`mods::read_any`); umpire heads on the Mods pages (B400). The parent still waits on M8f7
  (NPCs), M8f8 and M8f10.
