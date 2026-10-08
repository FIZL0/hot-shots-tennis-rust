# B32 Cody's face — FINAL

- Symptom: in the viewer (`--character 1`, camera on the face) Cody's face showed brown hair-textured patches,
  holes and an open "teeth" mouth. Ashley's (0) closed eye and JJ's (5) cap had stray patches too.
- Not the morphs (weights zeroed: same), not UVs. Hiding the hair material left holes in the face: face
  triangles were missing. With back-face culling off the face rendered right, so the triangles' winding was wrong.
- Cause: B7 made rigid meshes take each triangle's winding from its closing vertex's UV `w` (≥ 0 strip order,
  else reversed; VU1 culls by it), but `mdl::Model::skinned` still alternated by strip parity. Parity puts
  ~half of each character's skinned triangles against their vertex normals; the UV-`w` rule leaves 0–18 of ~2500
  (thin slivers vs smoothed normals). Non-two-sided materials (Cody's `kao1` face) then lost them to the cull.
- Fix: `Packet::uv_w` kept per drawn vertex; `skinned()` winds by it. NPC/umpire rigs share the path.
- Test: `hst-data/tests/characters.rs skinned_triangles_face_their_normals` (all 14 characters, fails with parity).
- Original not screenshotted this run (the user looked at the port and accepted it).
