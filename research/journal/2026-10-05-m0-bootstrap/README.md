# M0 bootstrap — research base + asset container

## Binary
- Metrowerks CodeWarrior MIPS 2.4.1, stripped. ELF holds engine/libs (0x100000–0x322d00); game code in MWo3 overlays.
- MWo3 header (0x40 bytes): magic, id, load addr, text size, data size, bss size, ctor, dtor. Loaded whole at load addr.
  GAME 3796 fns, MENU/MOVIE decompiled too (context/decomp).
- RNG: Mersenne Twister (state 0x9c4 words + 0x9908b0df) allocated in game init next to `cmn/game.xb` load.
- BMP loader exists (14+40 byte headers, palettes ≤8bpp, CLUT swizzle for 8bpp).

## XB container — SOLVED, crates/hst-data/src/xb.rs
- Verified: all 629 archives / 8433 entries decode with exact sizes; IRX.XB modules byte-identical to MODULES/*.IRX.
- Game over-reads Huffman bitstream past EOF on last entry → zero padding.

## Inner formats (inventory)
TIM2 .tm2 (548), BMP (44), MIDI BGM (33) + Sony .hd/.bd banks (293), MDL/MTL/MTI/MOR/MTA (models, ~1000 each),
ANI/ANI2 (animations), TRAJ/*.dat (shot trajectories), .HMP (court?), CSVs (Shift-JIS):
- PCDATA/.../Data/AIParam.csv — per-character AI tuning (reaction frames, error frames, net-dash %...)
- PCDATA/.../Data/TParam.csv — per-character stats (height, handedness, power stats...)
- MENU/.../text/*.csv — challenge text etc.

## Next
1. TIM2 → RGBA decoder (well documented format).
2. MDL/MTL/MTI layout — find the parser in decomp (search for ".MDL" strings / callers of archive lookup).
3. Bevy model viewer (crate `hst`).

## Update — M1/M2 progress (same day) → 2-ASSETS-VIEWER-PART
- TIM2: crates/hst-data/src/tim2.rs, 582/582 decode; CSM1 8-bit swizzle confirmed visually.
- MDL: crates/hst-data/src/mdl.rs, layout rebuilt from the game's loader (node tree → per-material batches → packets
  carrying raw VIF/VU1 data). 1152/1152 files consumed exactly; ~1.06M verts / 684k tris.
  Vertex layout: GIF tag, V4-32 pos, V4-16 normal (/16384, w bit15 = no-kick strip restart), colour (STROW or V4-8), V4-32 UV.
- MTL/MTI: crates/hst-data/src/mtl.rs, 1171/1171 exact. Mat header +0 RGBA f32 (128=1), +0x20 tex idx.
  8-bit CLUT CSM1 swizzle confirmed statistically (73 vs 8 textures smoother).
- Game space is Y-DOWN (ground y=0, objects negative y). Viewer rotates 180° about X.
- Viewer: `cargo run -p hst -- <iso> COURT/01/HOL01.XB COURT/01/GRD01.XB --radius 120 [--shot out.png]`.
  Court 01 renders textured and upright.

## Open
- Props are authored at their local origin → placement comes from layout data (candidates: COURTSET/xx/hole01.xb,
  COURT/xx/*.txt "entry_c01.txt", .HMP). Find the reader in decomp.
- One untextured white volume in court 01 (probably a helper/occluder; check material +0x18..0x1f flags).
- Skinning: packet bone list + per-packet matrices (+0x38 × 32 bytes) not yet used; ANI/ANI2 not started.
- TRAJ/*.dat shot trajectories + AIParam/TParam CSVs = the physics/AI data path.
