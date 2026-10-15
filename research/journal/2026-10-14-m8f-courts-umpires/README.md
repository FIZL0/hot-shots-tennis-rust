# M8f — other games' courts, court NPCs and umpires as mods

One README per task; iterations below, newest last.

- `1-SPLIT-STANDARD-PART.md` — M8f split into M8f1–M8f10 (`plan/M8f.md`); M8f1 done: court/umpire standard,
  `mod.json` `kind`, roster skips other kinds. Next: M8f2 (Get a Grip umpire voices).
- `2-GAG-UMPIRE-VOICES-FINAL.md` — M8f2: SGXD cue→wave decoded (SEQD note in decimal digits), 8 Get a Grip umpires
  packaged as umpire mods with all 24 calls, donors by sameness/overrun. Next: M8f3 (loader).
- `3-GAG-UMPIRE-MODEL-FINAL.md` — M8f4: Get a Grip's 8 umpire models + motions → each mod's `umpire.glb`
  (HST-MODS `tools/psp2gltf/export_getagrip_umpires.py`), drawn on the chair with `--umpire-mod DIR`
  (`mods/umpire.rs`, `play/npcs.rs`). Turn direction checked against the disc clips. Next: M8f3 (loader/menu).
