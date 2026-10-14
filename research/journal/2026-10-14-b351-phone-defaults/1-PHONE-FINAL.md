# B351 — phone defaults: Low shadows at 100 %, fewer casters at Low

## What changed
- `graphics.rs`: new preset **Phone** (`Graphics::PHONE`): 60 fps limit (ignored in the browser, which paces
  itself), vsync off, render scale 100 %, shadows Low, MSAA 4x (what the old first-visit text left at default).
  Listed in Settings → Graphics → Preset after Low.
- First run on a phone: `main_menu.rs` `first()` picks Phone when the page is played by touch (`touch_page()`:
  `(pointer: coarse)` or `?touch`), after the Steam Deck check. The page no longer writes its own first-visit
  settings.txt (`web/index.html` used to write frame_limit 60 / render_scale 67 / shadows low): the game decides,
  and the rest of a first run's settings (16:9 on a phone etc.) now come from the same place. Existing installs keep
  their settings.txt.
- Shadows Low casts less (`shadow.rs`): a `Minor` marker on
  - every NPC rig except the umpire (`npcs.rs`: walkers and court creatures; court 5's crowd never cast), and
  - every `Caster` prop whose shadow can't reach the court (`main.rs` at the plant spawn, `shadow::reaches_court`:
    the placed bounding box swept along the court's sun to the ground overlaps the doubles court ±(5.5 + 2,
    11.9 + 3) m).
  A `Dropped` marker goes on their meshes; at Low they get `NotShadowCaster`, Off/High take it back (live switch).
  Players (and rackets), the umpire, the net (`znet_*`, its own decal shadow is separate), the umpire's stand
  (`shinpandai`/`shinpan`/`w-shpn`, kept on every court that has one as a caster) and the trees/buildings whose
  shadows cross the court keep casting. High is unchanged.

## Numbers (prop draws casting, plant records with code byte 3 ≠ '0', all 11 courts, from the disc)
Before (all casters) → Low now: c01 78→20, c02 21→8, c03 122→32, c04 144→12, c05 52→10, c06 101→13, c07 4→4,
c08 94→37, c09 44→8, c10 63→19, c11 92→7 (counts are material parts per model, an upper bound on draws).
Court 4's 15 m sakura/midtree ring, the spare nets at x 17 and the bench rows drop; court 8 keeps its jungle trees
(B287: their shadows lie across the court). Plus the players' rigs and the umpire's.
A size cut (≥ 4 m) was tried first and kept 132 draws on court 4: dropped for the reach test.

## Not verified / not 1:1
- Not run on a phone and no fps measured: this session had no logged-in desktop (no Wayland socket), so no window,
  no `--shot`, no `tools/web_check.mjs` run. The prop counts above are computed from the disc with a throwaway test;
  the in-match "N cast shadows" line (Show FPS → Detailed) wasn't read. Measure with `tools/web_check.mjs`
  (MOBILE=1) under B343.
- Look at Low not compared side by side with the original/High: shadows of the dropped props (outside the court
  ±2/3 m) and of the walkers/creatures are gone at Low by design (a remaster setting; High still has them all).
- The 2 m / 3 m margin is our choice, not the game's.
