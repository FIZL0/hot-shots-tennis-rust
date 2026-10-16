# M8f51 — Get a Grip accessories' second (additive) texture pass

Change in ../opentennis-mods (commit a52bc55, `tools/psp2gltf/build.py`, `notes/getagrip.md`):
`build._gl_prims` (used by export_getagrip.py for parts and characters) now groups a part's GE prims by
(material, `Prim.pass_`), as export_getagrip_courts.py `prims` does since M8f5a5. A pass > 0 takes its own texture
record (`Material.passes[k]`); +0x24 & 3 == 1 → `<name>@add`, == 2 → `<name>@sub`, BLEND, untextured → white
`vertex_colour` so the vertex colours show. `TextureCache.material(name, add, sub)` builds those variants.

Survey of all 564 character i3r files: only etc18, etc51, etc81 have two-texture materials; the second record has an
empty texture name, +0x24 = 0x11 (additive), and the pass-1 prims carry the same UVs with vertex colours 0x59 / 0xb2
(etc18), 0xff (etc51), 0 (etc81). No character material has an additive/subtractive pass 0.

Result: etc18/etc51/etc81.glb now have a `vertex_colour@add` primitive per second-pass batch (8+22, 46, 62 triangles)
in place of the duplicate drawn opaque in the pass-0 texture. Parts without a second pass group with the same keys
in the same order as before, so their output is the same. None of the three is a costume-set accessory in
characters.json; they are only in `parts/etc/`. The game already reads `@add` names from mod glbs
(`mods.rs` `materials_of` → `gs::GsMaterial::for_batch`).

## Not verified / not 1:1
- How the pass looks in Get a Grip itself: not compared in PPSSPP. The untextured additive pass may be GE-lit
  (specular/sheen) rather than a flat vertex-colour add; exported flat, etc51's white vertex colours add full
  white. The +0x08/+0x0c floats of the second record (1/30, 29/30 on etc18/51; ±inf on etc81) are not decoded.
- No in-game shot of these parts as a character mod: no packaged character uses them.
