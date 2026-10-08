# B25 — pause and end screen "just blue" (FINAL)

## Cause
- The pause screen hides every root UI node (`Without<ChildOf>`) to replace the match HUD. Since B5b the whole frame
  reaches the window through one more root node: the gamma composite (`MaterialNode`, `UiTargetCamera(out)`).
  Hiding it left the output camera's clear colour — the plain blue screen. `context/shots/b25/pause_before.png`.
- The end screen itself drew (forced with `HST_STATS`), but Start opened the pause over it (→ blue), and the match
  HUD (panels, team tags, sets/games bars, COM markers, pop-ups) stayed on top of it. The original's result screen
  shows none of them (`context/shots/b25/orig/s1_t125.png`, `page1b.png`, captured with
  `research/p26b_result_shots.py` on slot 5, team B's games poked to 0 at 3–3 to end it).

## Fix (`play/menu.rs`, `play/markers.rs`, `play/match_stats.rs`)
- `menu::Hud` filter: root HUD nodes without the composite (`Without<UiTargetCamera>`), the pause's and the result
  screen's own roots. Hidden while the pause or the result screen is open, restored after.
- Player markers hidden while a result is up. Start doesn't open the pause while the result screen is up.
- Test `play::menu::tests::hud_roots`. Shots: `context/shots/b25/{pause,end,stats}_after.png` vs
  `context/shots/p19a/pause_0.png` and `orig/`: pause layout matches P19a's check; result and stats pages show only
  their own art over the court.

## Not verified / not 1:1
- The original's ceremony (winners' camera, confetti, "NARROW WIN" etc.) under the result screen: P26c, not in B25.
- Whether Start does anything on the original's result screen: not tested (the port now ignores it there).
- The stats page shows every COM at rank row 0 ("Lv 5"); the original shows each CPU's rank (Lv 4 on page1b) → B25a.
- The port's 3D call models of a running pop-up still show under the pause (P19a's note).
