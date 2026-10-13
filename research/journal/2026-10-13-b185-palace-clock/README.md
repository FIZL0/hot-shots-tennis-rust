# B185: Grand Palais "inside of a wall" at the near-end camera

## Finding
- The leftover piece is the **clock on the end house's façade** (`w-house` material 5 "clock", and its copy in
  `w-house2`), not a wall.
- The house (HOL01, plant at z 26.25) spans game z 26.25..42.85. The match camera behind your baseline after the
  change of ends (eye 0, −11.89, 39.24) sits inside it.
- From there all of the house's walls are back faces and are culled, as in the original: flipping the cull fills the
  screen with wall. But the clock's casing is 0.36 m deep (local z −0.09..0.27) and pokes 0.27 m through the façade.
  Its side faces behind the wall face the eye, so the port drew a ∧ over mid-court. A mirrored copy under the floor
  showed as two slivers lower down.
- Hiding material 5 alone removes all three pieces (`context/b185/flip/hide.jpg`).
- The original's GS output at that view (`context/b185/gs/plus.gs`, slot 9) has no prim nearer than the court floor
  at the clock's screen spot: it doesn't draw the clock from in there.
- Its house node is drawn through VU1's clip pass (eye inside the bound sphere). The pass's reject test accepts
  those triangles (all in front, inside the guard rectangle), so the original's actual rule wasn't found.

## Fix
- `main.rs` `HideInside`: a `w-house*` plant isn't drawn while the eye is inside its model-space box.
- From inside, the clock is the only part of the house that can draw, so this matches the original by eye at the
  change-of-ends, replay and cut-away cameras on that side.

## Check (screenshots in `context/b185/fix2/`)
- `cmp.jpg`: before / after / original (`orig/end0.png`) at the near-end view. The ∧ and the slivers are gone; the
  rest is unchanged.
- `cmp2.jpg`: B169's low camera and the far-end match camera are unchanged. The houses still draw from outside.

## Not verified / not 1:1
- **The rule is by eye, not the original's.** The original's own reason for not drawing the clock wasn't found: its
  VU1 clip pass would accept those triangles. Rendering only has to look right (PLAN).
- **The rule only applies to the `w-house` models.** No other stage was seen with the same problem.
- **The instant replay and cut-away cameras weren't screenshotted on that side.** They weren't driven to that side in
  the original. The rule depends only on the eye being inside the house's box, the same case as the change-of-ends
  view that was checked.
