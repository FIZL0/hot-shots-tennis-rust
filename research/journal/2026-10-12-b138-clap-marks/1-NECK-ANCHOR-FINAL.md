# 1 — neck anchor + base follow (FINAL)

## Found (decompile + RAM over PINE)
- A walker keeps a pointer to its model node named `Bip01Neck` (looked up by name at creation). When it breaks
  into its cheer (anim 5) it registers a mark at that node's translation (the gallery manager's mark list, max 6
  counted; the write happens at the count even when full; delay = 3·slot).
- While it stays in anim 5 it rewrites its kept mark's *base* to its neck every tick, even after the marks
  were zeroed and even if another walker has since taken that slot. The mark position only catches up at its next
  jump.
- RAM dumps on court 10: neck ≈ 1.15 m up; the sprite hangs 0.5 m above the mark (P3e6's draw, unchanged).
- Every stock gallery model has `Bip01Neck` (new assert in `hst-data/tests/npcs.rs`), so the original's
  no-node case never happens.

## Port
- `hst_sim::npc::Cheers::add` returns the mark index; new `Cheers::follow(mark, pos)`.
- `play/npcs.rs`: the walker's neck comes from its rig joint's GlobalTransform (Bevy → game space: x, −y, −z);
  `add` at the neck on react, and `follow` every tick while anim 5. Which walker cheers and when is unchanged.

## Checked
- `research/b138_claps.py SLOT OUT [delay [hold]]`: freezes the mark count after a point so the slow PCSX2
  screenshot still shows the marks; dumps camera/fov/marks (`context/b138/`).
- Court 10 (slot 5): the original's clap sits just above the pink walker's hat; the port at the same camera
  (HST_EYE from the dump) does the same. The original's left mark is hidden under the "B Team" HUD banner there.
- Court 4 (slot 4): the original (taken mid cut-away) shows the clap just above the left walker's head; port shot
  at the match camera: all marks above heads, none in empty space.

## Not verified / not 1:1
- The neck is read from the previous frame's posed joint (Bevy transform propagation runs after `step`); the
  original reads the node as last drawn too, but the one-frame phase isn't proven bit for bit (render only).
- `follow` is skipped while raining; the port still steps the walker animation there. Not checked against a rainy
  match (no save state with rain).
- Match-start marks (the ponytail cheer at setup) use the walker's feet since no pose exists yet; the manager zeroes
  them straight away, so never drawn.
- The original's no-neck-node fallback (mark at the court origin) is replaced by the walker's home position; no stock
  model hits it.
- Court 4 was compared at different cameras (the original shot fell in a cut-away), so by eye only, not at the same
  view.
