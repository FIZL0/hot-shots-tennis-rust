# B167 — preview faces differ between browser builds

**Cause.** Bevy 0.19 has two morph-target paths. WebGL2 (no storage buffers) passes each draw's weights as a uniform
and was right. Native and WebGPU look the weights up through a morph *descriptor* whose index is baked into the
mesh's `MeshInputUniform`. That descriptor only exists while the entity is visible (`extract_morphs` skips hidden
ones). The previews' game space spawns `Visibility::Hidden` until posed (B120 `Unposed`/`reveal`), so the uniform was
built with "no morphs" and kept it after the reveal. Native and WebGPU then drew every preview face neutral: Big
Chief with a closed mouth instead of anger_mouth 0.7's gritted teeth, Mifune with a shut mouth instead of
doki_mouth 0.6. A phone falling back to WebGL2 showed the real expressions, which is why it looked "different from
desktop". The CPU weights at the held pose (a temporary test printing `faces[POSE]` at `pose_frame`) matched the
WebGL2 render.

**Fix.** `inspect.rs reveal`: on the frame after a preview shows, mark its meshes' `Mesh3d` changed, so Bevy rebuilds
their mesh uniforms with the now-allocated descriptor. B120's hide-until-posed stays. `previews.rs RENDER` 6 → 7 so
the cached cards re-bake. `tools/web_check.mjs WEB_CARDS=1` copies the browser's OPFS-baked cards out for comparison.

**Verification.**
- Baked all cards (`HST_MENU=chars HST_BAKE=all`) on three builds:
  - native;
  - WebGPU and WebGL2, in headless Chromium on the desktop GPU (`MOBILE=1`, Vulkan ANGLE).
- Faces now agree on all three: d07, d13, d01 and d00 show the expressions their weights give.
- A control run that simply spawned the space visible baked the same disc cards as the fix.
- Images are in `context/b167/` (`fix.png`, `fix-web.png`).

## Not verified / not 1:1
- **No real phone.** The phone was emulated with headless desktop Chromium (WebGL2 and WebGPU on a desktop GPU), so a
  mobile-only GPU limit can't be ruled out. The bug as reported is fully explained by the WebGL2/storage split.
- **Not compared with the original's inspect screen.** The faces now follow the disc's face motion at the held pose,
  the same data on every backend. That data was not checked against a PCSX2 shot (rendering only has to look right).
- **Face-icon crops on d06 and d01 (costumes 5–9) still differ between native and web.** The full cards are identical
  there. `face_square`'s alpha heuristic picks a slightly different square from near-identical alpha. This is B176's
  open crop task, so it was left alone.
- **Mod costumes' (HST-MODS) face cards vary slightly between two native bakes,** e.g. Hubert's mouth opening. Both
  bakes show expressions; not chased (mods, not disc data).
