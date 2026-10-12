# P17o1 — footstep inputs, last gaps (FINAL, 2026-10-12)

## (b) Motion anchor = the head node
- The spawn's "motion object +0x84" matrix is a node pointer: the anim object's +0x80/+0x84/+0x88 point at Bip01,
  **Bip01Head** and the racket node (0x120-byte nodes, world matrix at +0). This was shown in the s05 dump: walking
  Bip01→Pelvis→Spine→Spine1→Spine2→Neck→Head, every local translation matches its rest value.
- Port: `foot::Runner` lost its `anchor` field; the debris direction is the level unit vector (player − head) · lunge,
  using the head joint the runner already carries.
- Check: `foot_debris_s05` now asserts, every recorded frame, that the recorded +0x84 matrix's row 3 equals the
  recorded head, bit for bit (the g/c/r fixtures pass).

## (a) Puff billboard basis
- The run object's draw builds the basis as `impact_matrix(−look)` from the camera look vector. The quad's centre is
  pos + 0 − look·0.5, d/e = centre ∓ row0·size, up = row1·size·2, and the vertices are d−up, e−up, d, e.
- The look vector equals row 2 of the camera→world view matrix (`g.cam.view.rot[2]`), w 0. The new test
  `puff_look_is_view_forward` checks this in 6 save states.
- `foot_fx::draw` now builds the quad that way with ps2 ops from `g.cam.view`, and no longer uses the Bevy camera.

## (c) Crossfade from the held node values
- In the decompile: bind resets every node's rows to rest but leaves the sampled quaternion (+0xe0) and position
  (+0xf0) alone. Exceptions: single-key lists are set once, and unkeyed lists of a tracked node go to rest. Per-frame
  sampling writes rows and held values for lists of 2 or more keys. The crossfade mix (first 23 tracks) slerps or
  madds from the held values into the **rows only**, with position w forced to 1.
- So a node the base clip doesn't key mixes from what the last clip that sampled it left. Per the disc survey (P17o),
  only hair bones (Bone0002–4 / Bone0202–4) are affected: they are keyed only by pc00/pc02's gu/di reactions and the
  co team clips.
- Port: `hst_sim::pose::Nodes` (new, bind, sample, mix) keeps that state per player. `play/bodyhit.rs`'s `pose`
  system runs after `character::tick`: it rebinds on a motion change, then samples and mixes as `Motion::mix` says
  (held fade: old clip sampled, new clip's frame 0 mixed in). `detect` and `foot_fx::tick` read `Posed`.
  `bodyhit::posed` is gone.
- Check: new test `motion.rs nodes_ram` uses 5 save states, 20 players, 3 mid-fade. It starts from each player's
  RAM nodes, samples the bound clip and mixes the outgoing one. Results:
  - every track's rows match the clip's cache as the game wrote it at sample/mix time;
  - every tracked node's held quaternion and position stay equal to RAM;
  - 1061 nodes, bit-exact.

## Not verified / not 1:1
- (a) The quad's vertices weren't compared to a GS/draw capture. The basis rests on the decompile and on
  look == view row 2, which was checked only at the serve's resting view in the save states, not mid-rally.
- (c) No dump catches a fade where the base clip doesn't key a node the other clip keys (the hair case), so that path
  is checked only by construction (held values unchanged by the mix, shown in RAM). A node never sampled starts at
  its rest pose (the player's creation isn't ported). A restart of the same motion through the point's reset to no
  motion doesn't rebind (`ponytail` in `pose`).
- Footstep/body-hit results don't change: the hair bones feed none of the bones they read.
- `character::animate` (drawing) still mixes unkeyed nodes from the other clip; it is out of this task's files.
- `draw_bits` (debris clods) still faces the Bevy camera.
