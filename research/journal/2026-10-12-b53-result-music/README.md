# B53 — result (stat) screen music (FINAL)

## What the original does
- **One track for a win and a loss: `bgmm_10`** (`SND/BGM/BGMM_10.XB`, menu BGM index 7 in the game's BGM table
  `bgmm_03..10, bgmg_01..11, bgmg_14`). The result object's step function, on its first ticks in a normal match
  (game mode ≥ 2), hands a loader thread a routine that loads BGM 7 at no volume, starts it, clears the music
  director's mute flag and sets the level to 0; the next tick's thread loads another sound bank. Nothing there
  branches on the winner or the shown side. What differs between a win and a loss is only the match-end jingle
  (key 3 / 4, already ported) and the ceremony.
- **The court BGM is gone before the ceremony**: the match teardown (mode ≥ 2) frees the BGM handle; in the
  recording that was 82 ticks before the result object appeared.
- **Level**: the match-end director message (0x1a) sets level 0, target 0, step 55/60, mode "fade". Each tick the
  director raises the level by 55/60 to 55 once the match-end jingle (slot 8) has ended, and lowers it toward 0 while
  it plays. Recorded: the new BGM started at ceremony tick 44 (when the loader thread got to it), held at 0 until
  tick 176, reached 55 at tick 235 and stayed there on the result page (state 1 from tick 330). Leaving the page
  fades it from 55 to 0 over 30 ticks; a rematch reloads the court BGM.
- Recording: `research/b53_result_music.py 5 …` (SMF slots: slot 5 runs a track whose bytes are `bgmm_10.mid`)
  and `research/b53_result_volume.py 5 …` (handle volume, director level), slot 5 bots doubles, team 0 poked to
  one game short; logs in `context/b53/`.

## Port
- `play/match_stats/ceremony.rs`: with `--music`, `begin` loads `bgmm_10`; `tick` stops the court BGM at tick 0,
  starts `bgmm_10` silent at tick 44, then each tick raises the level by 55/60 (to 55) unless the match-end jingle
  is still playing (then lowers it). The ceremony ticks on under the result page, so the music keeps playing. This
  replaces the old 270-tick fade of the court BGM for a winner.
- `play.rs` `Bgm::step` records the match-end jingle's play id (`END_JINGLE`) for the ceremony to wait on.
- `ceremony::end` (Continue) starts the court BGM again from the top, as the rematch does.
- Checked: `hst … --play --music --stage 10 --pads -,-,-,-` with `HST_STATS=6` runs through the ceremony to the
  result page without errors.

## Not verified / not 1:1
- Loss case not recorded: only the bot-only save (slot 5) ends unattended, and it always shows the winner. The
  decompile has no win/loss branch around the load, so a loss plays the same track; not checked by ear/recording.
- Not listened to in the port (no audio capture here); only the run's log was checked.
- Start tick 44 is the original's disc-load latency in one recording, used as a constant.
- The 30-tick fade-out on leaving the page is not ported: the port leaves the page at once.
- When the port's match-end jingle is played (at the verdict) differs from the original's director cue (N3e), so
  the fade-in may start at a different tick than the original's 176.
- Modes below 2 (no `bgmm_10`, court BGM kept) don't exist in the port; all its matches are treated as normal.
