# B277: huge stray brown triangles on the web

## What the screenshot shows
`context/B277/lines.png` (online doubles on grass, during "Waiting for Player to come back"): a smooth-shaded, lit,
skin-coloured surface (darkened by the pop-up's dim) fanning from the bottom-left to a single vanishing point past the
top-right corner. That is a lit character part with some vertices pushed out to a far point. It isn't a court prop,
which would be drawn flat in the GS colours. The frozen match is what keeps it on screen.

## Cause: Bevy 0.19 morph descriptors on the WebGPU (storage-buffer) path
Read from `bevy_pbr-0.19.1/src/render/{morph.rs,mesh.rs,morph.wgsl}`:
- On a frame where a morphed mesh becomes visible again (unhidden, or back in the frustum after a camera cut), its
  render instance is removed while hidden and then re-created. `prepare_morph_descriptors` needs the instance's
  mesh id from `RenderMeshInstances`, but `collect_meshes_for_gpu_building` only fills that in after it has run. So
  that frame the mesh has no descriptor, and its `MeshInputUniform.morph_descriptor_index` is `u32::MAX`.
- `MeshInputUniform`s are only rebuilt when the entity changes (transform, visibility, Mesh3d, ...). While the
  character moves that happens the next frame (one bad frame). In a frozen match it never happens, so the bad index
  stays on screen.
- The shaders don't check for `u32::MAX`. Native Vulkan reads zero out of bounds (the part is drawn unmorphed; that
  is B253's flash). Browsers' WebGPU (Tint/WebKit robustness) clamps the index to the last descriptor, which belongs
  to another mesh. That mesh's weights and target offsets are then applied to this part's vertices, and the face's
  vertices fly off. The WebGL2 path (uniform buffers + morph texture) has no descriptors, so the user's browser was
  almost certainly on WebGPU.

## Fix
- `gs.wgsl` `morph_vertex` (characters, mod characters, everything drawn by `GsMaterial`): on the storage path, a
  mesh with no descriptor (`u32::MAX`) is drawn unmorphed.
- `gs.rs` `reextract_morphed` (PostUpdate, after visibility): marks every shown `MeshMorphWeights` mesh's
  `GlobalTransform` changed, so it is re-extracted every frame and has its descriptor from its second visible frame
  on, also while frozen. Only a handful of entities (faces, effects).

## Checked
- Native release `--play --stage 1 --pads -,-` shot (`context/B277/native-play.png`): match renders as before.
- Web WebGPU (headless Chromium, Vulkan): `context/B277/gpu1/`. The match plays with no console errors, and a
  cutaway's face morphs (wink, smile) are right (`visit2-40s.png`).
- Web WebGL2: `context/B277/gl1/`. The match plays with no console errors (the guard is compiled out there).

## Not verified / not 1:1
- The stray triangles themselves were not reproduced. Getting them needs a morphed mesh shown again during a frozen
  online wait on a browser whose out-of-bounds read lands on a far-flung descriptor, and the reporter's browser/GPU
  is unknown (an unattended run can't ask). The cause comes from reading the Bevy source and matches B253's native
  symptom of the same state. Whoever sees it again: note WebGPU or WebGL2 (the console's `hst: WebGPU` line).
- Effects drawn with Bevy's own `mesh.wgsl` (`Fx`/`StandardMaterial` morphs) get only the re-extraction. On the one
  frame they show again they can still read the wrong descriptor on WebGPU. B253's always-drawn `Collapse` already
  keeps the impact effects from being hidden. That one-frame gap may be B137a's WebGPU-only flat quads: noted there.
- Rendering only, no sim/RNG/timing touched; nothing to compare with the original.
