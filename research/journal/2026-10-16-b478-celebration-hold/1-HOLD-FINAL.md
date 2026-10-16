# B478 — mod celebrations hold their last frame

## What was wrong
- The match sets the point/game reactions (`re_gu` 0x2c, `re_di` 0x2d, `re_gu_set` 0x2e, `re_di_set` 0x2f) once,
  not looping (`play.rs` `react` → `set_motion(.., false, ..)`); both the gameplay clock and a mod's drawn clock
  (`character::tick`) already clamp at their clip's end, so the engine held the last frame.
- The loop / snap to idle was in the mods' own `motions.glb`: opentennis-mods `opentee_data.reactions` built Open Tee
  `re_di` as `ku_di` + `wait` and `re_di_set` as `lostNN` + `waitNN`. `motions.py` plays the second anim as a loop
  after the first, so 31 Open Tee / Open Tee 2 characters fell back into their idle and looped it through the
  rest of the transition.
- The loader also rejected any own clip shorter than the donor's, so a hand-made mod with a short celebration
  couldn't load at all (the tools padded to the donor length instead).

## Change
- `crates/hst/src/mods.rs`: `CELEBRATIONS` (0x2c..=0x2f) may be shorter than the donor's clip; the drawn clock
  holds its end until the next motion (other clips still must cover the donor's). Gameplay still reads the donor's
  clips: nothing in the sim changes. A fade out of a short clip samples it past its end, which clamps to the last key
  (`Keys::find`).
- Test `mods::tests::short_celebration_holds_its_last_frame`: hand-built `.glb`, a 30-frame `re_di_set` over a 90-frame
  donor loads and holds its last key through 200 ticks; a 30-frame `sh_f_t` is still rejected.
- opentennis-mods 6c95b99: Open Tee reactions are one clip each (no `wait`); standard §6 says the four reactions may be
  shorter than the donor's. Re-packaged `package.py opentee opentee2` (31 mods): no manifest has "then wait" any more;
  Alia's `re_di_set` now stops moving at frame 69 of 101 and holds (before: the idle moved through the tail).
- Get a Grip's reactions keep intro + `_loop`: that loop is the source game's own designed continuation of the
  celebration (its hold pose), not a repeat of the motion or the idle. Fore!/OOB reactions were single clips already
  (held by `motions.py`'s `min(t, len)`).
- `mods::` tests (48) pass with `context/mods` pointed at the re-packaged `out/mods`, incl. `psp_mods_keep_their_heads`
  (loads every Open Tee 2 costume with its motions).

## Not verified / not 1:1
- No in-match screenshot of a re-packaged Open Tee mod at a lost game: presentation only (PLAN: feel right); checked
  on the clip data (keys constant from the motion's end) and the clock test instead.
- Get a Grip's `_loop` tails kept as they are (see above); if they should freeze too, drop the `_loop` in
  opentennis-mods `package.py` `GAG_REACT` clips.
- Char-select Win/Loss previews (plate_emote) not touched: they return to the pose after the donor's length by design.
