# B61 — Queen's Garden: near half missing the ball shadow and red dots

Already fixed by B80 (36b4d1a, 2026-10-11). No code change here.

## Cause
Queen's Garden's hole model (`*_h01_*`) holds blended wear decals (brown grass patches) over the court. Before B80,
Bevy sorted those blended draws by mesh centre like any other. The near-half decal's centre is nearer the camera than
the ball shadow, so the decal drew after the shadow and covered it. The red landing marker had the same problem
(B91 later fixed its layer order too). The far-half decal sorts behind the shadow, so the far half was fine. B80 gives
the hole's parts `order = -1e6 + material·1e3` (`main.rs`, court models), so they always draw first, as the game does.

## Check (context/b61, release `--shot`, `HST_BALL=2,-3,±6,0,0,0` via an HST_BIN wrapper, `--stage N --play --singles --pads -`)
- Queen's Garden (stage 5), current build: the ball shadow draws on both halves, and the red rings draw on the near
  half (`zg.jpg`, `grid.jpg`).
- The same build with B80's hole order turned off (`old.jpg`): the near-half shadow is washed out under the decal.
  That reproduces the reported bug.
- Stages 1 and 8 still draw the near-half shadow (`other.jpg`).

## Not verified / not 1:1
- No `tools/screenshot.sh` of the original on Queen's Garden. The save states aren't on that court, and getting
  there means driving the menus. The original draws the shadow on both halves (the user reported the missing near
  half as a bug), and rendering only has to look right (PLAN).
