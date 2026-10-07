# P17a — court materials with the PS2 GS state

- Scope: material GS state of court models. P17 split into P17a–h (fog, VU1 lighting/highlight, clouds, sky/seasons, MTA/UVA, shadows, mipmaps).
- Material setup 0x17b730 (from MTL loaders 0x17bad0/0x17bda0; A+D order in constructor 0x17b4e0: CLAMP, TEX0, TEX1, MIPTBP1/2, ALPHA, TEST).
  - ALPHA default 0x8000000044 `(Cs−Cd)·As+Cd`; name `@add` → 0x48 `Cs·As+Cd`; `@sub` → 0x42 `Cd−Cs·As` (tested second, wins).
  - TEST (helper 0x17b620) from MTL header +0x1e (short mode): base 0x5000b (ATE, GEQUAL, AREF 0, ZTE, ZGEQUAL); 10..19 AREF 0x40; 20..29 AREF 0x70 AFAIL RGB_ONLY; `@add`/`@sub` ATST NEVER AFAIL RGB_ONLY (colour, no Z).
  - TEX0 TFX HIGHLIGHT2 when (int) material alpha (header +0xc) == 0x80 and name lacks `@vert`, else MODULATE and header +0x14 zeroed (likely the highlight term); TCC 1.
  - CLAMP 0x0A region clamp; MDL material header +8/+0xa give wrap. TEX1 MMAG linear, MMIN 4, MXL = header +0xf.
- Batch header +0x31 = GS PRIM bits (0x20 FGE, 0x30 TME|FGE, 0x70 +ABE); +0x30 is 0x0c/0x0e. GIF PRIM = +0x31|0xc (0x15de80). ABE is OR'd in at runtime only by the object fade path (0x15df30 via 0x15dce0, from 0x14f8d0/0x15c0a0), cleared by 0x15dfc0.
- Court stats (all courts): only modes 5, 15, 25. 25 always ABE; 15 mostly unblended alpha cut-out; 5 mostly opaque. Some translucent materials have batches without ABE (wafu `$water`, `line@add` on greece/wafu/western).
- Port: new crates/hst/src/gs.rs + gs.wgsl (GsMaterial, custom Bevy Material, key-specialised blend/depth-write/shader defs); main.rs gs_models (raw Rgba8Unorm texels, Repeat/Clamp per wrap, one mesh per prim group, two draws for mode 20..29); mdl::Packet.prim; Tonemapping::None; colour computed in gamma space, then sRGB→linear.
- Side by side: context/shots/p17/orig_slot5.png (PCSX2 slot 5 = court 10), port_c10.png (`HST_AUTOPLAY=1 hst <iso> --stage 10 --play --shot … --shot-at 3`), port_c01.png (court 1).
  - Found: court lines (`line@add`, mode 15, colour alpha ~50/128, batches without ABE) vanished: non-Z-writing draws sat in Bevy's opaque pass and the ground overwrote them.
  - Fix: non-Z-writing draws go to the transparent pass; `@add`/`@sub` materials always blend (the PS2 shows faint additive lines, so ABE ends up on for them; exact runtime path unconfirmed). Courts then match closely.
- Remaining differences: no player/ball shadow blobs, tree shadow patches lighter than PS2 (P17g); PS2 distance blurrier (mipmaps, P17h); no fog (P17b); highlight term taken as 0 (P17c); blending in linear not gamma space; transparent order by Bevy distance sort, not file order; alpha >1 clamped; no clouds (P17d).
- Gotcha: `--shot` exits immediately if the output file already exists (auto_shot waits for the file); delete it first.
- Check: `cargo test -p hst --bin hst for_batch`.
