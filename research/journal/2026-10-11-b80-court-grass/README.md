# B80: court grass (Nakagawa River, Western Prairie) and how court models draw

## What was wrong

- **Nakagawa (stage 9)**: the hole model's decal batches (`abb_yogoreglass`, `abb_zassou`, …) carry vertex alpha
  0..0x80 to fade grass and dirt into the ground. The port drew the hole like any model: an opaque-colour material is
  TFX HIGHLIGHT2, so alpha came from the texture alone (opaque). The result was hard-edged grass patches.
- **Western (stage 3)**: `ygrass` (the hole's green decal around the court) was drawn but then painted over by
  `ground02`'s Lt80 (blended) draw. A constant vertex alpha 0x80, interpolated as a float, came out as 0.99999994, so
  `floor(a·128)` = 127 passed A < 0x80. Bevy's sort (mesh AABB centre) then put that draw after `ygrass`.

## The game's rules (decompile + RAM)

Read from the material setup, the model loader, the batch loader and the hole fade on/off. Then checked against
each batch's GS registers in RAM (`context/b80/allram.py`: start every court from slot 8, dump RAM, compare PRIM
and TEST per batch; script in the scratchpad). Result: 11 courts, 8,821 batches, 0 mismatches.

1. A material whose TFX is MODULATE gets ABE on every batch at load. MODULATE means: colour alpha·128 ≠ 0x80, or
   the name has `@vert`.
2. The batch loader clears FGE when the material's ALPHA is add (`@add` without `@sub`; `@sub` wins and keeps fog).
3. The court's hole model (`hole/<court>_h01_s….mdl`) is in "fade" mode for the whole match. In fade mode:
   - every batch has ABE;
   - every material is TFX MODULATE with TEST 0x5180b (A ≥ 0x80 → colour + Z, else FB_ONLY), `@add`/`@sub` too;
   - the vertex alpha is the MDL's (not the highlight).
4. GS MODULATE alpha is `(At·Af) >> 7` on integers. The shader now computes it the same way, so an opaque 0x80
   stays 0x80.

## Port

- `gs.rs`: `GsMaterial::for_court_batch` applies rules 1–3. A hole batch becomes two draws: Ge80 opaque with Z,
  then Lt80 blended. `GsMaterial::order` feeds `Material::depth_bias`.
- `main.rs` `gs_models_anim`: court models use `for_court_batch`. The hole's parts get order −1e6 + material index·1e3,
  so its blended parts draw before every other blended part, in its material order (the GS's order within the model).
- `gs.wgsl`: MODULATE alpha on GS integers (rule 4).
- Test: `gs::tests::for_court_batch_keys`.

Shots: `--stage 3/9 --play --shot` vs `tools/screenshot.sh` from the same moment (`context/b80/orig_c03.png`,
`orig_c09.png`, `port_c0{3,9}_fix3.png`). Grass now fades into the dirt on stage 9 and shows around the western
court. Every other court was shot too (`context/b80/all_*.png`): no regressions.

## Not verified / not 1:1

- **Order of the hole versus other models' blended draws.** The port draws the hole's blended parts before all other
  blended parts. The game's model draw order wasn't traced. The hole is the ground under everything else, so this
  order is the likely one, but it is unproven.
- **Order within one hole material.** Its parts still sort by distance in the port, while the GS goes in packet order.
  Normal blending of overlapping packets of the same decal could differ slightly.
- **`@add`/`@sub` hole materials under fade.** The RAM check covers their TEST and PRIM, but no court showed one on
  screen to compare.
- **Fade outside a match.** Whether fade stays on in menus or replays wasn't checked. The port always uses it.
- **AFAIL.** The port treats AFAIL RGB_ONLY (0x5370b) and FB_ONLY (0x5180b) alike. They differ only in the
  framebuffer's alpha channel, which nothing in the port reads.
- **Vertex alpha truncation.** The GS's fixed-point interpolation is approximated by truncation plus 1/64.
  Exact at the vertices and for constant alpha; between vertices it can be ±1 off.
- **Stage 7 draws as a brown screen** with `--play --shot` at 4 s. This happens on main too, without B80's changes
  (`context/b80/s7_main.png`), so it isn't part of this task. It probably comes from B74's sky follow; it needs its
  own task.
