# M1h — modded characters' hair physics (R1 pass) — FINAL

Every mod's hair now moves the way its source game moves it, or stays rigid where the source has no dynamics.

## Per source game

- **Fore (PS2)**: `.NOI` noise, as HST's own (done before, §3a). The two mods without noise on their c00 are
  faithful: Regis (pc07) has a `.NOI` only for costume 4 (c04–c07 carry `_HST_NOISE`, c00–c03 have none on the
  disc either); Ratchet (pc22) has no `.NOI` in any of his 72 models.
- **Out of Bounds (PS3)**: the `.rem` tail after the node tree is collision primitives (ellipsoid/box/cylinder,
  0x88 bytes) and spring chains (`u32 n`, then n × 16 floats + bone name), interleaved; all 102 models parse to
  EOF. Format, parameter table and chains per character in open-tennis-mods `notes/oob.md` *Dynamics*. 11 of 17
  characters have chains (ponytails, braids, ribbons, earrings, ears, Dino's scarf).
- **Get a Grip / Open Tee 1/2 (PSP)**: no dynamics. Skeletons are Bip01 + rigid attachment nodes
  (`Bip01Head9pony`); no `.i3m` track in any of the three games names hair, ponytail, ribbon, earring or skirt
  (open-tennis-mods `notes/psp-formats.md` *No hair dynamics*). Rigid hair is the source's.

## What changed

- open-tennis-mods `8d6bc53`: `rem.py` parses the dynamics, `export.py` writes standard §3b `extras.spring`
  `{limit, stiffness, gravity, damping, oob: raw 16 words}`, `rerig.py` keeps it, standard §3b written. OOB mods
  rebuilt (`export.py`, `rerig/batch.py oob`, `package.py oob`: 102/102 pass).
- HST `crates/hst/src/spring.rs` (new): a mods-only mass-spring. Each spring joint is a mass at the joint, its
  parent turns to point at it; a chain's first record (its anchor: stiffness 1, limit 0–5°) is pinned so the
  head is never turned. Whole 60 Hz steps after `character::animate`, before `noise::draw`. Only the motion
  across a segment counts (a pull along it made the renormalised turn overshoot: an upright 8 mm earring tip
  swung rim to rim for ever), and a step is at most half the segment. `HST_NO_SPRINGS=1` turns it off to compare.
- `noise.rs` (`Costume.springs`, `draw` pub(crate)), `mods.rs` (loader reads springs; test), `main.rs` (plugin).
  Stock characters have no springs: no system runs on them, the sim never reads joint transforms it writes.

## Verified

- `mods::tests::oob_hair_swings_and_settles`: Jasmine's c00 loads 18 masses (23 records − 5 anchors); a 0.2 m
  sideways whip over 10 frames swings her hair up to 50° off the calm pose, it settles within 0.5° by frame ~140,
  the head joint is untouched. `spring::tests` cover one mass trailing within its limit and a 0° limit staying rigid.
- `tools/check.sh -p hst`: 114 passed.
- Shots in `context/shots/m1h/` (direct `--shot`; `tools/shot.sh`'s hyprctl launch timed out in this run):
  `oob_jasmine_m3_2.0_{on,off}.png` — springs off, the ponytail sticks out stiff; on, it hangs down her back;
  same for 2.1. `oob_pc11_dino_m3_2.{0,1}` (scarf hangs and trails), `fore_pc00_phoebe_m3_*` (noise),
  `getagrip_pc00_emi_m3_*` (rigid, as the source).

## Not verified / not 1:1

- **The OOB solver is not ported.** It sits in the program's cloth library (REClothSim/REClothPrim, DSCloth*
  classes), AltiVec-vectorised and unreadable in the decompile. Ours is a plain mass-spring that looks the part
  (in-match presentation only has to look right).
- **Parameter meanings are read from the values, not the code**: word 0 limit (degrees), 1 stiffness, 2×3 gravity,
  7 damping. Words 4, 5, 6, 8, 10, 11 are unknown and unused.
- **Collision primitives are parsed, not exported or simulated**: hair can pass into the body/shoulders on big
  swings (the 50–60° limits keep it mostly clear).
- **OOB cloth meshes** (`skirt`, `uwagi`, `hane1`, `h_mimi` between mesh blocks) are not decoded: skirts and
  jackets stay rigid.
- **Rate and units assumed**: per-step constants taken as 60 Hz; gravity in m/s² not rescaled by rerig's size
  factor (OOB characters are scaled to ~0.95 m, so their hair sags a little more than at source scale).
- No side-by-side with the running PS3 game (no PS3 emulator setup here).
