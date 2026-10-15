# 7 — Get a Grip c05 (Street): the basket goals over the match view (M8f5a3)

## Decision
The goals (`c05_gi12_BasketGoal__00/01`, haichi-placed at game z ±19.075 m, 7.2 m behind each baseline, 4.8 m tall)
stand ~21 m in front of HST's resting eye, square over the near half. Of the three options:
- drop the near-end one: either end is the near one once the camera turns at a change of ends, so both would go,
  and with them the court's signature piece;
- move them back: out of the view means past the eye or below the view's bottom edge (≥ ~30 m out), which is the
  street and the buildings behind c05's 7.9 m run-off;
- **hide it while it blocks** (chosen): loader-side, any camera. It is what HST itself does with court objects in
  front of the lens (`play/net_hide.rs`: the net, buried props, B303's props in front). Hidden, not faded: the
  same as HST's own pass, and no blend pass to add to GS materials.

## What changed
- Standard §2 (../opentennis-mods 68606db): a node whose name ends in `__clear` (before any `__singles`/`__doubles`) isn't
  drawn while a line from the eye to a point of the playing area (doubles court + 2 m behind each baseline, ground
  and 2 m up) passes through its box at rest, or the eye is inside it. Its skinned meshes go with it.
- HST `court_mod.rs`: `scene` collects each `__clear` node's box (rigid meshes by their node, skinned ones by each
  vertex's heaviest joint at bind) and its entities to hide; `clear_view` (PostUpdate, before transform propagation)
  tests the drawn camera's eye (`blocks`, 9×9×2 segment-vs-box slab tests) and sets their Visibility. Draw only:
  nothing reads or writes the match. Unit test `a_piece_between_the_eye_and_the_court_blocks_it`.
- `export_getagrip_courts.py`: every gimmick.glb instance (haichi or csv) with a rest vertex inside HST's resting
  view (either end) behind the near baseline gets `__clear` (`Scene.near_end` / `mark_clear`), listed in `todo`.
  c05: both goals and two flags by the +z goal. Over 16 courts 0–9 pieces each (pigeons, baskets, penjor poles,
  c16 panels, c17 NORMA's core…); HST only hides them while they actually block. The 20 m island drop (M8f5a2)
  is unchanged.
- Found on the way: haichi gimmicks with only a break motion looped it (`sorted(names)[0]`): c05's goals fell apart
  and stood up again every 2.8 s. Break motions (`_bNN_`) are no longer rest motions; the goals stand still.
- `notes/getagrip.md` *Courts* updated.

## Checks (`context/m8f5a3/run1/`, `--play --singles --court-mod`, `HST_WEATHER=0`)
- `c05-before.png` (the `__clear` names patched out of a copy): the near goal's backboard over the near half.
  `c05-after.png`: gone, court clear; the rest identical. `c05-eye-near.png`: same through `HST_EYE` (vectors check).
- `c05-from-net1.png`: an eye over the net looking at the −z goal: drawn, upright, hoop and net.
- `c05-turned.png`: the +z resting eye is inside c05's brick building: the whole view is wall. Not the goals'
  doing (court.glb) → new task **M8f5a4**.
- `tools/check.sh -p hst --bin hst court_mod`: passed.

## Not verified / not 1:1
- Not compared with Get a Grip running (no PSP capture): that its camera has the goals behind it is inferred from
  the geometry, as in M8f5a2.
- In HST the near goal is gone from the match view rather than shown somewhere the original's camera would show
  it: a choice, not a port (court mods are additions).
- The box is the rest pose's; a piece whose motion carries it into the view isn't caught. Sampled lines: a piece
  thinner than the gap between two (~1.4 m at the goal's distance) can slip through.
- Pop: a piece appears/disappears at once when the camera changes (as HST's own net/prop hiding).
- The turned (+z) view on c05 wasn't checkable here (inside the building, M8f5a4); the +z goal's hiding is covered
  by the unit test's mirror case only by symmetry of the code, not by a shot.
- The other 15 courts were re-exported with their new `__clear` names but not shot.
