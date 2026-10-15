# 4 — M8f6 court mod loader (FINAL)

A `kind: court` mod (standard §2) now plays: `--court-mod DIR`, or Select Court, where installed court mods follow
the disc eleven.

## How it works
- `crates/hst/src/mods_court.rs` reads and checks `mod.json` (layout 0..=10, `court.glb` required, optional
  `light` with sun/fill/ambient/fog, `sky.glb`, `gimmick.glb`, `sound/bgm.wav|ogg` + `bgm_loop`,
  `sound/ambience_N.wav` + `ambience_loop`, `lens_flare`); `list` for the menu.
- `main.rs`: `--court-mod DIR` defaults `--stage`/`--court` to layout + 1. `load` builds the layout court as ever
  (collision world, lines, net, chair, people, weather, court generators) but parents its models under a hidden
  `court_mod::DiscCourt` entity, so nothing the match reads changes.
- `crates/hst/src/court_mod.rs` (PostStartup `setup`): spawns the glbs' node trees under GameSpace (undoing the
  scene's `game_space` node), drops the other match kind's `__singles`/`__doubles` nodes, skinned gimmicks with
  their joints, glTF animations looping on their own clocks (`animate`); the manifest light is baked into vertex
  colours (ambient + sun·max(n·l, 0) + fill·max(n·l, 0), clamped at 1.0); materials go through `GsMaterial`
  (`@add` in a material name → additive); the fog becomes the GS fog ramp; `keep` re-applies the mod's light and fog after
  `weather::apply`; the sky stands at the eye as a `Backdrop`. The shadow `Sun` follows the manifest sun (raised to
  ≥ 50° like the game's); the layout court's ball/figure `Shade` is removed (its casters are hidden).
- Sound (`sounds`, PreUpdate, once the banks are in): the BGM becomes a streamed `SoundBank` (`audio::Track`, its
  loop span, linear resampling to 48 kHz, levelled to `Track::BGM_RMS` = 0.069, the median RMS of the eleven disc
  court BGMs at level 55, measured by the ignored `court_bgm_loudness`: 0.051–0.080). It is the match music
  with `--music`, and the pause menu's music toggle gets it back (`court_mod::bgm`). Ambience layers loop in the
  SFX group; court bank program 7 (the creatures' and emitters' sounds) is swapped for one silent sample, so its
  plays still start and stop on the same ticks.
- Lens flare: the disc flare sprite is drawn when the mod has a `lens_flare`, none without (`FLARE_OFF`).
- Menu (`play/main_menu/court_mods.rs`): court values past Random are mods; tile = layout court's tile, name +
  game under it; the name card says "<game>, plays as <layout court>"; surface/meters/picture are the layout's;
  `args` adds `--court-mod`; Random draws disc courts only. Long strips now show at most 17 slots around the
  pick (`STRIP_SHOWN`). With 16 court mods the 28-tile strip ran out of the menu's 3200-quad pool and the last
  tiles weren't drawn. With mods on the strip, only the hovered tile's names are outlined. Not saved to
  settings.txt; a pick no longer installed goes back to Random. Online rooms leave court mods out (M8f8).

## Checks
- `a_court_mod_is_seen_and_heard_and_never_moves_the_match` (`play/court_mod_match.rs`, ~5 s): bot singles on
  Alpine Heights with and without `gag_c01_yamate`; > 400 more meshes, BGM is the match music, ambience loops,
  state hash identical every tick through two points. Skipped without the ISO or `mods/hst-mods`.
- `reads_and_lists_court_mods`, `court_mod_flags`, `court_mods_follow_the_disc_eleven`,
  `a_hundred_court_mods_fit_the_strip` (every aspect, pool), `a_court_mod_stream_loops_at_the_bgm_loudness`.
- Shots (`context/m8f6/`): run1/yamate.png (match camera, singles), run3/sky.png (low eye, doubles: sky, clouds,
  lamps, gimmicks), run3/courts-mod.png (16:9), courts-mod-tall.png (600×900), confirm-court-mod.png.

## Not verified / not 1:1
- No original to compare against: court mods come from other games; their look is checked by eye only.
- Lighting: the PSP's per-vertex lighting is approximated by a baked Lambert sum (ambient + sun + fill), clamped
  at 1.0; gimmicks are lit at their bind pose, not per frame.
- Yamate's `c01_yamate_light` (sun dapples) draws as a black octagon left of the net: on the PSP it's additive
  but the exporter doesn't record GE blend state, so the loader can't know. Not guessed here: task **M8f5a1**
  (HST-MODS exporter names additive materials `@add`, which the loader already honours).
- Gimmick reactions (pigeons flying off, baskets tipping) aren't ported (the PSP program's, encrypted); they
  only loop their idle animation.
- The mod's own lens-flare sprite isn't drawn; the disc's flare stands in at the sun when the mod has one.
- The layout court's NPCs/creatures and their placement stay as on the disc (they're match-side people).
- Ambience replaces only court bank program 7; level 0.7/√layers relative to the BGM loudness is a choice, not
  measured. BGM loudness is matched by RMS over the first 20 s with a linear volume; resampling is linear.
- Pause-menu music toggle back on with a court mod: wired (`court_mod::bgm`) but not tried in a running match.
- Online: not carried (M8f8); the strip hides court mods while online.
- The hidden disc court is still loaded (memory/time), kept so collision/props/generators stay untouched.
