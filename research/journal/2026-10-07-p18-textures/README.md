# P18: PCSX2 pack and mod-folder textures

**Lookup order:** `mods/textures/` → `replacements/` (the PCSX2 pack) → disc. Both folders sit beside the ISO and are git-ignored (`/mods/` added to `.gitignore`).

- Code: `hst-data/src/texhash.rs` (names, `Overrides`, dump) and `hst/src/textures.rs` (Bevy side: `add_mtl`/`add_tim2` at every image load, 1 s hot-reload poll of mods, UI rect scaling).
- Tool: `texnames <iso> <replacements> [-v]` reports matches per archive.

## PCSX2 2.9 names

`<tex>-<clut>-<bits>` for paletted textures, `<tex>-<bits>` otherwise. Hashes are lower-case hex with no zero padding.

- `bits` = PSM | TW<<6 | TH<<10. For 16/24-bit formats it also carries TA0<<15 | AEM<<23 | TA1<<24.
- `tex` is XXH3-64 (seed 0):
  - CT32, T8 and T4 textures of at least one GS block hash every 256-byte block in its swizzled layout, blocks in raster order.
  - Smaller textures and CT24/CT16 hash rows of 8-bit indices for paletted formats, RGBA32 otherwise.
  - With mipmapping, levels basemip..maxmip go into the same digest. The pack names every level range.
- `clut` is XXH3 over 16 or 256 RGBA32 entries: the logical order (CSM1 swap undone) with raw PS2 alpha.
- Swizzle formulas, checked against PCSX2's tables (unit test `swizzle_columns`):
  - CT32: `(y>>1)<<4 | (x>>1)<<2 | (y&1)<<1 | (x&1)`.
  - T8/T4: `c=y>>2, r=y&3, rot=(r>>1)^(c&1), x8 = rot ? (x+4)&7 : x&7`.
    - T8 = `c*64+(x8>>1)*16+(x8&1)*4+((x>>3)&1)*2+(r&1)*8+(r>>1)`.
    - T4 = `c*128+(x8>>1)*32+(x8&1)*8+((x>>3)&3)*2+(r&1)*16+(r>>1)` (nibble index, even nibble = low).
- 2D TIM2 sheets (INPANE, menus) have palette alpha authored 0–255. The game uploads it halved as `(a+1)>>1`; the pack's CLUT hash matched only with that.
- Pack PNGs hold raw PS2 alpha (0x80 opaque, upscaler overshoot up to ~161). We expand on load with min(a*255/128, 255). Mod files use straight alpha.

## Verification

- `cargo test -p hst-data --test texnames` checks court 05 and the HUD. Every pack file whose texel hash belongs to one of their textures matches by full name: court 05 has 166, HUD 43.
- The only exception is `hole/wim_h01_s1111.22`, which the game re-palettes at runtime.
- Disc-wide: 5831 textures, 2316 of the pack's 2978 files matched.
- Unmatched pack files:
  - 178 are 256x512 `-00002613` files sharing one CLUT. The game declares TH larger than the image, so PCSX2 hashed leftover VRAM; they can't be matched.
  - The rest are runtime-composed, absent from the disc, or region (`-r`) names.
- Court 05 also has never-drawn 16x16/2x2 collision textures. HUD inpane_duce01/result_set/inpane_set and the prize screens are not in the pack (the user never reached them).

## In the port

- `HST_AUTOPLAY=1 hst <iso> --stage 5 --play --shot` replaced 226 textures from the pack. The HUD came out sharper with correct layout once ImageNode pixel rects were scaled by the replacement/disc size ratio (panel and popups pick sheet pieces in disc pixels).
- `--dump-textures` wrote 5830 PNGs (1.5 GB, ~5 min) to `mods/textures/<archive>/<path>.<index>-<key>.png`. Where the pack had a texture, the dump wrote the pack's PNG with alpha expanded.
- Channel-swapped copies of COURT/05/GRD01's 34 files in mods turned the court blue.
- Rewriting them during the run turned it purple within a second: 31 reloaded. Screenshots are in the scratchpad only, not committed.

## Limits (ponytail)

- A deleted mod file keeps its image until restart.
- The dump is single-threaded.
- Replacements have one mip level.
- play.rs balloons (tim2) are not routed through textures.rs.
