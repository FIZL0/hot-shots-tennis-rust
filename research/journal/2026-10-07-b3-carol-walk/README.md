# B3 — P1 Carol's walk after a point (BLOCKED)

The bug report: when P1 is Carol and the point ends with her at the edge of the camera, her walk back to position
slides the wrong way. I couldn't reproduce a mismatch against the original. Everything below matches it.

## Checked against the original
- **Handedness.** Players can swap hand at character select (Select/−), so the slot 3/4 Carol is right-handed by
  choice. The default left-hander is Will (character 11). The lefty sim is exact. The new test `lefty_s04`
  (`crates/hst-sim/tests/player.rs`) runs Will's pelvis rows with hand −1 over `p7_carol_s04` / `p7_kaito_s04`:
  416 motions and 510 facings, 0 off. The app's reaction-root rows `hand·(fz,0,−fx) / up / (fx,0,fz)` match the
  game's mirrored model matrix.
- **Facing at the reaction.** In every fixture (`match_s05`, `p7b_c*`, `p7_carol_s04`), +0x3d60 is unchanged on the
  frame the reaction motion starts and 30 frames later. The original never turns a player round for the
  reaction, and neither does the app.
- **gu_set (0x2e).** Only characters 5 and 13 ship `re_pcNN_gu_set_dummy`. Carol's (and Will's) `gu_set` has no
  root path. Her clip keeps the pelvis within ±0.1 m, so it plays in place. That isn't a walk.
- **Team reactions 0x30–0x34.** The path z is negative, so the reaction carries the player backward, away from
  their facing. The app matches the original here: live slot 5 showed p2 0x34 at z −4.03 → −5.03 and p3 0x33 at
  10.06 → 10.46, and the app moves the same way.
- **Phase 4 before the reaction.** In the original, a running player keeps dashing (live slot 5: p0 m7 for about
  1 s, facing its run direction), then reacts in place. The app's bot dashes toward `home`, also facing its run.
- **Stick flip.** The original's pad matrix flag (0x2f0730+0x80) follows the camera eye z ≥ 0 within a frame. That
  includes the post-point close-up, whose eye moves to z 0.2–14.5, so the stick reverses during it, the same as
  the app's `pad_run` (live eye z). The flag is latched in 0x323990 only while gm+0x344 is 0.

## Not resolved
- I couldn't tell which "walk back" the report means. A human P1 in Post moves only by the stick. The bot's
  dash home is the closest candidate.
- To unblock: a short app capture or screenshot of the slide, with frame, P1 human or bot, singles or doubles, and
  stick input. Then compare a slot 3/4 run driven with tools/vpad.py to the same spot.
