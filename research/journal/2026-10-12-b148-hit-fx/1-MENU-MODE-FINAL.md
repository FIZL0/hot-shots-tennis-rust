# B148: effects only missing in a match started from the menu

## What was missing
In a match started from the main menu, none of the `effects::Fx` draws showed: the impact model on contact, the
hit sparks, the ball's flight ribbon/glow, the bounce effects and the red (and yellow smash) landing markers.
`--play` drew them all, which is why B137's own check (`--play` shots) passed.

## Cause
B137 (4d37ad1) moved the effects to `effects::Fx` (`ExtendedMaterial<StandardMaterial, SortOnly>`) and registered
`MaterialPlugin::<Fx>` in `play::plugin`. `mode.rs` builds a mode (menu, match) into the world *without the render
app*, so a material plugin added there only creates `Assets<Fx>`; its render side never runs and every `Fx`
mesh is skipped. `--play` adds `play::plugin` to the base app, which has the render app, so it worked there.
No bisect needed: comparing a `--play` run with a menu-started run (`HST_MENU=confirm`, `HST_AUTOPLAY=1`) showed
it, and the mode doc names the rule.

## Fix
`MaterialPlugin::<effects::Fx>` is registered in the base app (`main.rs`, beside `gs`/`shade`/`hud_gamma`);
`play::plugin` no longer adds it.

## Checks
- Screenshots 1, 4 and 8 frames after each of the first hits (a temporary, uncommitted hook), menu-started
  doubles: before the fix no impact, sparks or red markers (`context/b148/Hmenu.jpg`); after it all show
  (`Hmenu2.jpg`). `--play` singles before B137 (`Hpre.jpg`) and now (`Hcur.jpg`) both show them.
- `play/hit_fx.rs` tests: `hits_start_their_effects` runs a bot singles match headless for 3600 ticks and checks
  every hit starts the impact, the sparks and the ribbon, every return past the serve sets the red landing marker,
  and the pre-serve player markers show; `the_match_registers_no_materials` fails if `play::plugin` registers the
  `Fx` material plugin again (checked: it fails with the old line put back).
- The "red indicator" is the red landing marker (`chakudan_p`, `LandingMarks.red_at`); the 1P/COM head markers
  (`markers.rs`, plain `StandardMaterial`) were never affected.

## Not verified / not 1:1
- No new side-by-side with the original (`tools/screenshot.sh`): the effects' look didn't change, they were just
  not drawn; the fix brings back what B13/P3e2/B91 had checked against the original.
- Web build not run: it uses the same `main.rs` start path and mode switch, so the same fix applies.
- The Bevy "Use-after-free" slab-allocator log flood is still there (before and after B137); that's B149.
