# M1a: standard character mod loader (2026-10-08)

Remaster-only feature (nothing in the original to compare against); the yardstick is the disc character a mod
was rerigged from. Details: `1-LOADER-FINAL.md`.

- `crates/hst/src/mods.rs`: `read(dir)` checks `mod.json`, `load(iso, &mod, costume, ..)` fills `CharacterData`
  from a costume `.glb` and the donor's disc motions (`character::disc_motions`, shared with `load_disc`),
  `tparam` gives the TParam row (base + overrides), `voice` the wav bank.
- `audio.rs`: `SoundBank::wavs(dir)` (`<program>_<key>.wav`), played as PCM voices in the mixer.
- Viewer: `hst --mod DIR [--motion N]`.
- Gaps: PLAN M1c (mods in a match), M1d (lighting/normals/noise), M1e (wav voices vs SPU).
