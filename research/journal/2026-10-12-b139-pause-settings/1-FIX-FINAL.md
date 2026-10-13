# B139 fix

**Cause.** B133 (custom page face grid) added `icons: Res<Icons>` to `main_menu::draw`. The main menu's plugin
initialises `Icons`; the pause overlay (`main_menu/pause.rs`, B126) runs the same `draw` but its plugin never did, so
the first frame with Settings open failed parameter validation: `Parameter Res<Icons> failed validation: Resource does
not exist` → `Encountered a panic in system hst::play::main_menu::draw`.

**Fix.** `pause::plugin` initialises `Icons` (empty: no mod icons in a match).

**Test.** `pause::tests::open_change_back_in_a_match`: an app with MinimalPlugins + input + `controls::plugin` +
`pause::plugin` only (what a match has, no main-menu plugin), real disc art via the ISO; opens Settings, toggles
Menu Music with a click press (not Widescreen: that sets a process-wide flag the parallel widescreen tests read), checks `settings.txt` was written (to a temp dir holding a symlink to the ISO, so the
user's own settings are untouched), Back closes the overlay. Without the fix it fails with the exact panic above.

## Not verified / not 1:1
- Not clicked through in the windowed `--play` build: the headless test runs the overlay's systems (step, draw,
  ticker) with the disc's art, which is where the panic was; window/render paths unchanged.
- Remaster-only screen, no original to compare against.
