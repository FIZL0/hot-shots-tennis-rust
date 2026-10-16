# Z21 — cover what Z2's sweep missed, strip what stays unused

Follow-up to `2026-10-16-z2-unused-assets`. The sweep was driven with scripted input (`HST_MENU`, `HST_KEYS`), not
HST_MASH, and folded with `research/assets_used.py` over `context/z2/sweep1` and the new logs.

## Runs

`HST_SWEEP=paths HST_SWEEP_SECS=200 tools/asset_sweep.sh context/z21/run1 3` (new `paths` set, not part of `all`)
produced 54 logs:

- every `HST_MENU` screen (press, main, rematch, settings, graphics, advanced, sound, controls, mode, assign, chars,
  chars4, attrs, names, coms, the three confirms, courts, umpires, sets, mods, extras, online, lobbies), each run
  once plainly and once with `--original`;
- the in-match pause → Settings → two pages and back, plainly and `--original` (`pause`, `pause-original`);
- 3-set matches: court 1 singles and court 7 doubles `--original` (`sets3-s1-singles`, `sets3-s7-doubles`).

Fold output: `context/z21/assets_used.txt`. The HD pack's never-looked-up list is in `context/z21/pack_unused.txt`.

**Logging gap fixed:** `Sound::music` (hst-base `audio.rs`) logged nothing, so no BGM bank ever showed as used. It
now logs a `snd` line. With that fixed, BGMM_03, 04, 05 and 06 all play in the menu runs, so none is a candidate.

## Stripped (evidence: static reading + the logs)

- **EFFCT `impactef_flat` / `impactef_drop`.** `Glow::start` maps flat to slice's glow and drop to top's, so glow
  slots 2 and 4 are never selected. `effects::load_flight` now reads only slice, top and lob and fills the five
  slots from those three.
- **In-match pause (`menu.rs`):** `i_dialog_00`/`01` (B141's box, pills and rows replace them) and `hsm_yubi`.
  The 3D hand (`cursor::setup` at Startup) always exists, because the game can't start without the disc; this was
  the user's correction. `Tex::Hand` stands in and draws nothing.
- **Main menu `SHEETS`** went from 15 entries to 3: `wall_playerselect`, `characterselect2_01`,
  `characterselect2_04`. Across every menu run the only sheets drawn were those three plus `Confirmation_00`/`02`,
  the court pictures and `word`.
  - Dropped: the option and title sheets, `characterselect2_00`/`02`, the `CS2_Shots_*` pictures, `info`,
    `Confirmation_01`/`04` and `KeyAssign_CharacterSelect2`.
  - Also dropped: `wall_confirmation`, `Confirmation_03` and `item_umpire00–04`. These belonged to the old
    umpire screen (`umpire_screen`, `name_card`, `strip`, `strip_rects`, `court_meters`, `METERS`). Since B451 that
    screen has been unreachable, because `umpire_picker::draw_over` draws the pick. That code is removed too, along
    with `umpire_mods::card`/`stage`.
  - The 3D hand fallback `hsm_yubi` is gone from the main menu and the pause Settings overlay. The pause overlay now
    makes an image only for the wall.
- **`flame.rs`:** `f_s.tm2` is not loaded and its draw is removed. The flame still ticks, because the match seed
  follows its `rand()` draws.
- **`menu_art.rs`:** the label sheets nothing loads any more are gone: option, select, keyassign, confirmation_01,
  confirmation_03 (with the CONFIRM labels and the VS look), confirmation_04, i_dialog_01, the five titles and info.
  The pill, title box, info icons and shadow machinery went with them.

**A/B check** (`context/z21/ab/`, the old binary is `context/z21/hst-old`): main, `--original` main, settings,
umpires and pause → Settings came out pixel-identical. Chars, attrs and confirm differ only in a few dozen pixels of
the animated portraits. The in-match pause menu differs only through match timing (crowd/players); the menu itself
is the same. Tests: `-p hst menu` 186 passed, `-p hst-play menu_art` 7 passed, `-p hst-base effects` 2 passed.

## Kept

- **EFFCT `cf00-*` bounce words:** loaded for every surface in 1..=15, and the rare surfaces were not driven.
- **`inpane_pause01` (TIEBREAK), `inpane_set` (Set Point) and `result_set` (set won):** ported presentation of the
  original (`popups.rs`), and both are reachable in play.
- **BGMM_10:** the result music (`ceremony.rs`), which plays at match end. No run ended a match, so the fold still
  says "never played".
- **`dvv_vc00d`:** the doubles voice bank (`hst-sim` sound test reads `/DBL/`); not driven by a played point here.
- **HD pack: 247 of 4584 files never looked up.** The top groups are MENU10B 55, SAITOU/HSMODE 30, MENU06 28,
  AZUMA/INPANE 27 and MENU60 18. Most are for disc sheets that the remaster no longer loads, which is now more of
  them after this strip. Nothing was deleted (deletion can't be undone, and the pack is user-facing). The list is in
  `context/z21/pack_unused.txt` for the pack's owner.

## Not verified / not 1:1

- **Hot Shot mode, tournaments, trophies/prize:** the remaster has none of these (our front-end), so there is no
  path to drive. Their disc assets (SAITOU/HSMODE, PRIZE) stay unused by design.
- **Set pop-ups were not seen:** both 3-set runs were still in the first set at their 600 s shot (autoplay rallies
  run long), so the logs show no `inpane_set`/`result_set`/BGMM_10 draw. They are kept on static grounds only.
- **Not driven:** the tiebreak pause board (`inpane_pause01`) and the rare-surface `cf00-*` words.
- **A/B is by eye and pixel count**, not a full screen-by-screen sweep. The online and mods screens weren't A/B'd;
  they draw the same wall.
