# P22 — widescreen, high frame rate, input polish

- **High frame rate**: already done by F0 (uncapped present mode, visuals blended between 60 Hz ticks). Nothing new.
- **Widescreen**: the 3D camera was already Hor+ (`play.rs camera` keeps the game's vertical field of view and widens
  with the window). The HUD (panel, pop-ups, banners, pause menu) used a full-window root, so its 640×448 layout
  stretched on wide windows (round score badges went oval). Now each root is `widescreen::screen_43()`: a node whose
  left/width a PostUpdate system keeps at the centred 4:3 part of the window. Auto margins don't centre absolute root
  nodes in Bevy's UI, hence the system. Checked on screen at ~16:9: the HUD sits at x ≈ (W − 4/3·H)/2, unstretched.
- Known: the pause menu's dim quad covers only the 4:3 part. `--shot` on the pause screen comes out solid blue with the
  old binary too (pre-existing, not P22). `--shot` sometimes logs "Unknown window" while other PCSX2 windows are open;
  grim on the window works instead.
- **Rebindable controls**: `controls.txt` beside the ISO (git-ignored), written with the defaults on first run;
  `action = Name …`, keys by Bevy KeyCode name (`J`/`KeyJ`, `1`/`Digit1`, `Space`), buttons as `PadSouth` etc.
  Unknown names are warned and skipped; an action with nothing valid keeps its default. The help line shows the
  current bindings. Left/right sticks stay fixed; Esc/Start pause and menu navigation stay fixed (menu.rs).
- **Hot-plug**: `controls::PadSlots` keeps each pad's slot while it's connected; unplugging pad 1 leaves pad 2 driving
  slot 2 (player 3 / the singles opponent stays human), a newly plugged pad takes the first free slot.
- Tests: `play::controls::tests` (bindings round trip/parse, slots across unplug), `play::widescreen::share_of_width`;
  `tools/check.sh` all 185 pass (hst-sim untouched, so the replay suite is unchanged by construction).
