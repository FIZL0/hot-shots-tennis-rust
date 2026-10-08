# B5b — HUD blended in gamma space (FINAL)
- `crates/hst/src/hud_gamma.rs` (+ `.wgsl`): every 3D camera that drew to the window draws into a `scene` image;
  the UI has its own camera (`IsDefaultUiCamera`) drawing into a `hud` image cleared transparent (= UI
  premultiplied); a last camera's full-screen `UiMaterial` writes `dec(enc(hud/a)·a + enc(scene)·(1 − a))`, the GS's
  blend on the encoded values. Both images follow the window's physical size and scale factor.
- Pitfall: `Assets::get_mut` only raises `Modified` on a mutable deref; the composite's bind group kept the 1×1
  textures (black frame) until it was re-prepared with `into_inner()` (done the resize frame and the next).
- Check: `HST_AUTOPLAY=1 hst <iso> --play --stage 11 --shot … --shot-at 1.5` before/after vs
  `context/shots/b5/orig_100.png` (`context/shots/b5b/cmp_bottom.png`). Grey profile down the strip at x 104/640:
  original 202 212 179 153 152 152, now 201 183 153 159 152 153 (its texture's top-to-bottom gradient shows), before
  a flat ~175. Viewer mode (`--shot` without `--play`) unchanged, pixel-identical.
- Left: UI pieces overlapping each other still blend in linear among themselves (only their sum goes over the court
  in gamma) — `ponytail:` note in the module.
