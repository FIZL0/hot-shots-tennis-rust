# Shadow-texture draw: GS state and leaf alpha

## Game (GS dump `context/p17v/cap0.gs` / `.pkl`, VRAM in `context/p17r/GS.bin`)

- Casters are drawn into a 256² CT32 target: FRAME FBP 0x8c, FBW 4, FBMSK 0xffffff00 (only R is written),
  SCISSOR 4..251, XYOFFSET 0x7800. It is then copied local → local into 128² PSMT4 textures (TBP 0x3308 + i·0x20,
  TBW 2), keeping every other pixel. The 4-bit index is R >> 4. The receiver CLUT is i·0x20, clamped to 0xff.
  - `GS.bin` is a GS freeze. Its VRAM starts at file size − 4 MiB − 84 (offset 425). With that offset, no column
    rotation is needed; P17r's "rotation 8" came from a wrong offset.
- The GS state templates hold two shadow variants. The material's normal TEST picks between them:
  - **Solid:** TEST 0x3460b (ATE GEQUAL 0x60, AFAIL KEEP, DATE). PRIM is a plain triangle with RGBA
    (0, 0, 0, 0x80), so it writes R 0x80 → index 8.
  - **Textured:** TEST 0x3401b (ATE GEQUAL **1**). It is used for alpha-tested materials (seen with normal TEST
    0x5370b) and keeps the material's PRIM (TME|FGE…).
    - Other registers: TEX0 TFX 3 (HIGHLIGHT2) with TCC 1, TEX1 0x…061 (bilinear, MXL 0, top level only), CLAMP
      region 0..127, ALPHA 0x44 = (Cs − Cd)·As >> 7 + Cd.
    - The vertex colour is black, so Cs = Af = 0x80. Fog with F = 0xff gives 0x80·255 >> 8 = 0x7f, so a leaf
      texel blends R toward 0x7f by its bilinear texture alpha.
- The decoded game tree textures confirm this. Their shapes match the port texel for texel. Their leaves are
  index 7 (0x7f) and the trunks are 8. The trunks win over the leaves, so solid triangles are drawn last.

## Port (`hst-sim/src/shade.rs`)

- `Shape::new(model, mtl)`: a material with TEST mode 10..29 and a texture becomes a `Cutout`. A cutout holds
  the triangles with their UVs, the raw PS2 alpha, the wrap mode, and a red of 0x7f when fogged (else 0x80).
  Every other triangle is solid.
- `texture` keeps red as u32:
  - Cutout triangles blend toward their red where the bilinear alpha (4-bit fraction) is ≥ 1.
  - Solid triangles then write 0x80.
  - The texel is red >> 4.
- `sample` reads index·0x20 (clamped) bilinearly.
- `hst/main.rs` builds casters from `Shape::new` with each model's materials.
- Test `court10_map_near_game`: IoU 0.9752 ≥ 0.975.

| Variant | IoU |
| --- | --- |
| Solid (P17r) | 0.947 |
| Binary cut, alpha ≥ 1 | 0.967 |
| Binary cut, alpha ≥ 64 | 0.974 |
| Blend toward 0x80, solid first | 0.9747 |
| Blend toward 0x7f, solid last (shipped) | 0.9752 |

Exact tree texels per caster went from about 14.6 k to 15.2–16.1 k of 16 384.

## Gap (→ P17v2)

The court 10 seat (zseat, parasol) and statue (sekizo) have mode-15 textured materials. In the game they come
out at index 8, not 7: no fog, or a solid draw. Their coverage matches the game, so this only changes the
level, not the shape. No header byte, colour, prim, category or plant code separates them from the tree leaves.
Header +0x25 = 1 matches the seat but not the statue, so it is not used.
