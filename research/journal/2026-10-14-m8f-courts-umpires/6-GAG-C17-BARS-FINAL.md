# 6 — Get a Grip c17 (Shadows): black bars over the match view (M8f5a2)

## What it was
- The bars are `c17_ob13_NORMA__00` (haichi-placed, ships a motion → `gimmick.glb`), not `c17_gi09_earth`. Its
  `c17_ob13_code01` primitive holds floor cables and 17 m-tall cable strands rising around NORMA's core (`dogma`) at
  game z −27…−31 m. HST's resting match camera (`hst_sim::camera::base`: 41 m at 17°, eye ≈ (0, −12, −39.2)) sits
  behind them and 9–12 m away, looking through them to the court.
- The strands are opaque in Get a Grip too: `code01`'s texture record is the most common opaque pattern (626 records
  share it: +0x10 0.5, +0x24 0x10, +0x31 1), and the texture is black with a soft light stripe, alpha 255 throughout.
  So this isn't a material state. The earth globe stands behind the *other* end's wall (z +26…+48, mostly below the
  floor) and isn't in either resting view.
- How Get a Grip keeps them out: its PSP camera sits closer to the court than HST's, so NORMA (30 m behind the
  baseline) is behind its back in a match. Inferred from the geometry; Get a Grip itself wasn't run (no PSP capture).

## Fix (../opentennis-mods d22e1cc)
- `export_getagrip_courts.py`: `Scene(clear_view=True)` for `gimmick.glb`. Each rigid mesh's triangles are welded
  into islands (shared or coincident corners, 1 cm) in world space. An island with a corner inside HST's resting match
  view (either end: the camera turns when the human is on +z; 7°+1.5° up/down, 21:9 wide) and within 20 m of the eye
  is dropped. Each drop is listed in the mod's `todo`. Over all 16 courts it drops only NORMA's 288 strand triangles
  (264 of `code01` left: floor cables; core, plates and the security cameras untouched). Other courts re-exported
  unchanged in content.
- `notes/getagrip.md` *Courts*: why, and what the 20 m rule doesn't catch.

## Checks (`context/m8f5a2/run1/`)
- `c17-before.png` / `c17-after.png`: `--play --singles --court-mod`, `HST_WEATHER=0`. Before: the bars. After: the
  clear room, security cameras and NORMA's plates still drawn.
- `c17-turned.png`: the +z resting view via `HST_EYE` (0,−11.98,39.2 …): clean. NORMA's core is the far backdrop
  there, now without its strands.
- `c08.png`: gaslamps flagged by a looser test only sit low at the view's corners: fine.
- `c05.png`: the basket goal blocks the view's middle. Same cause (Get a Grip's closer camera), but it's ~21 m from
  the eye and a signature piece of that court, so it's a separate decision → new task **M8f5a3**.

## Not verified / not 1:1
- Not compared with Get a Grip running (no PSP emulator capture): where its camera sits, and so that NORMA's strands
  are never in its match view, is inferred from the geometry.
- The strands are gone from every view, including HST's turned (+z) view and cut-aways where they'd be harmless
  backdrop: a static glb can't keep them for one camera and not the other.
- The 20 m / view-cone rule uses each gimmick's rest pose. A motion that carries a piece into the view isn't caught
  (NORMA's plates only bob in place).
- c05's basket goals are still in the view (M8f5a3). Other near-end props below the court's line in the view
  (c16 balloons, c20 tools, c14 cheerleaders) are left as they are: they're seen from behind the baseline, as any
  camera there would see them.
