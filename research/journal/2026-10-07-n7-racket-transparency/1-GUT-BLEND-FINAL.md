# N7 — racket transparency

- Every racket (`Rkt/pcNN/racket_NN.MTL`, all 10 characters, same layout): `shaft` and `grip` TEST mode 5 in
  PRIM 0x30 batches (opaque); `gut` TEST mode 25 (header +0x1e = 0x19) in a PRIM 0x70 batch (ABE on), material
  colour 0x80, vertex colour 0x80 everywhere. Its 128×128 texture's alpha is only 0 (10197 texels) or 0x80 (6187).
- GS state per P17 (`2026-10-07-p17-court-rendering/1-GS-MATERIALS-FINAL.md`): mode 20..29 = ATE GEQUAL 0x70,
  AFAIL RGB_ONLY; ABE ALPHA `(Cs−Cd)·As+Cd`; HIGHLIGHT2 so As = texture alpha. Alpha-0 texels blend to Cd: the
  holes between the strings show what is behind. The port drew every character material opaque.
- Fix (`character.rs` load_disc): racket materials with mode 20..29 in an ABE batch get `AlphaMode::Blend`
  (StandardMaterial: base α 1 × vertex α 1 × texel α, `src·a + dst·(1−a)`). Body materials untouched (their
  texture alpha is shading masks; the untextured `Standardmaterial` mode 11 parts stay opaque as before).
- Not exact: the A ≥ 0x70 half writes no Z here (the GS writes it); nothing of the racket sits behind its strings
  but the frame. Checked in the viewer (`--character 0 --motion 32 --radius 3.5`, shot in `context/shots/n7-m32.png`):
  background visible through the string mesh. No PCSX2 side-by-side (strings are a few pixels in play).
