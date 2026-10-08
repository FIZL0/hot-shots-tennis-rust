# P17s5: the net's wire and pole take VU1's unlit path

**Question.** The net's wire and pole draw at a constant ⌊vc·1.8⌋, and house and light4 didn't fit (P17s4 gap).

**The net: VU1's unlit path.**
- VU1's rigid normal routine (microcode 0x13d) tests object flag bit 1 (VU mem 206.x, the per-draw flags qword the EE
  uploads to VU addr 1). When it's set, the routine jumps to 0x15f and skips the normal transform: N' = (1, 0, 0).
- So diffuse = 1, the second light is 0 and the highlight is 0, and colour = ⌊vc · mat · (ambient + light)⌋.
- The flags come from the draw record (vtable 0x1d0e68). The function 15de80 copies +0x10's low byte from the MDL batch
  header +0x30.
- In the files, +0x30 is 0x0e only on the net's `pole1:wire` and `pole1:pole` batches (znet_s1000, netmoto). Every
  other court-4 batch has 0x0c.
- Slot 4's RAM: the wire and pole records have 0x10e/0x20e. znet's use the models block (0.8, 1.0), so 1.8 = A + L,
  matching the dump's K ∈ [1.7979, 1.8).
- The sky, bg and cloud records (park00_sky, park_bg, park_clo) also have bit 1 at runtime: 0x10e/0x30e, own light
  blocks (0.7, 0.5, 0.02). Their file batches have 0x0c, so something sets it at load. The flag setter chain is
  15df30 ← 15dce0 ← 15c0a0 ← 155220, a vtable method. Its caller for skies and bg was not found.
- Bit 0 is OR'd at runtime too (the ground 0x10d, alpha batches); it is not this task.
- `research/p17s5_draws.py` maps every draw record in s4.ram to (model, material, light block, flags).

**House and light4: they fit; the check was wrong.**
- Both use the court block (0.7, 0.5, 0.02) with flags 0x10c, like ry and shinpandai.
- The old matcher (p17s_light_gs.py / p17s2_check.py) pairs each packet with the only GS run of the same kick pattern.
  Packets with repeated kick patterns (the house's mirrored halves) landed on another packet's run.
- `research/p17s5_check.py` instead picks the run whose GS/vc ratio is most constant over equal normals, and pairs
  greedily, one-to-one. With it, light4 fits 390/394 (fitted rotation, max error 1).
- The house's one node matrix in RAM is a ≈180° yaw (rows (−1, 0, 0.0059), (0, 1, 0), (−0.0059, 0, −1); normals n·R).
  With it and no fit, batch groups 2, 3, 5 and 6 are exact: 358/358.
- The two tiny packets left (tesuri, 8 verts; ueko, 12) each have an exact candidate run that the greedy pairing gives
  away on a tie.
- So nothing new lights them. The P17s4 note "fit neither, even with a fitted rotation" came from the mismatched runs.

**Port.**
- `mdl::Packet::flags` (batch +0x30).
- main.rs `gs_models_anim` gives unlit batches their own mesh with `GsUniform::unlit = 1`.
- `weather::apply` marks every sky and background (`look.skies`, `look.bg`) material unlit.
- gs.wgsl: when unlit, diffuse = 1, the second light and the highlight are 0, and glare still applies (baked into the
  EE colours).
- Test: crates/hst-data/tests/unlit.rs (court 4: exactly the four net wire/pole batches have the flag).

**Gaps.**
- Which load code sets the sky/bg/cloud unlit bit (the port takes it from the bg dir).
- Sky and bg draws are not checked against a GS dump (s4.gs has no unique match for them).
