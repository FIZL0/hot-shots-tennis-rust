# P17h — mipmaps and TEX1 LOD for court models

- Task P17h: mipmaps and TEX1 LOD for court models.
- Material setup 0x17b730 sets TEX1 = (texture header +0xf MXL)<<2 | 0x120: MMAG linear, MMIN 4 (linear-mipmap-nearest), LCM 0, L 0, K 0 in the base material.
- The per-draw copy 0x143160 (called from 0x15f0a0 ← 0x14a150, from model constructors 0x149fb0/0x14a0b0) puts K = param & 0xfff into TEX1 bits 32–43.
  - The asm shows K = MDL material header +4 (lw 0x4); wrap u/v bytes are +8/+0xa.
  - MXL is capped by a constructor argument; we assume it is uncapped for courts because RAM MXL equals the texture header MXL in the slot-5 dump (court 10).
- K is signed 12-bit with 4 fraction bits. Court 10's files match the RAM draw records exactly (clouds −6.0625, bg −11.4375, jungle_h01 −15.25).
- LOD: VU1 writes Q = 1/w (UV qword z = 1), w = view depth in metres (the same depth fog uses). GS LOD = log2(1/Q)·2^L + K; nearest-mip rounds, so level = clamp(⌊log2(w) + K + ½⌋, 0, MXL).
- The MTL texture header +0x10 holds 7 × {u32 size, u16 w, u16 h}. Levels halve exactly; level count = MXL+1 on every court.
- Court ground textures have MXL 0 (unaffected). Plants, trees and structures use mips.
- Found: first try with K=0 turned far grass tufts into dark blobs. With per-material K they are feathery like the PS2 (shots in context/shots/p17h, not committed).
- Port: mtl.rs Texture.mips, mdl.rs Model.lod_k, gs.rs uniform lod_k, gs.wgsl textureSampleLevel with the level above. The shadow pass (gs_prepass.wgsl) samples level 0.
- Test: crates/hst-data/tests/mipmaps.rs.
- Open: MIPTBP is not ported (the GS addresses do not matter here); characters (StandardMaterial) are out of scope.
