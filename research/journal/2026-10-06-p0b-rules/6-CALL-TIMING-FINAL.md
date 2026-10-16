# P0b4c — umpire-call timing (FINAL)

The call show itself (`388190`: settle on anim end + voice idle or countdown, hold > 0x1d, fade 5, chain with a
1-tick pause) was already in `flow.rs` (P12b3). What was missing: when the game *hears* a voice stop, and the
match-over wait.

## Voice-stop lag
`1a03f0(5, …)` asks the sound driver (a virtual call on the slot's sequencer track); the answer comes back through
the IOP, a few frames after the clip has played out. Measured against our mixer's clip length (`centre_frames`,
umpire 4 a = slot 5's):

| reader | clips | heard after play |
|---|---|---|
| her word queue (`3a1c10`), `umpire_s05` | love 40, 15 45, 30 37, 40 40, "game" 33 | clip + 5 (every case) |
| scoreboard call settle (`388190`), `match_s05` | fault 39 (8×), out after net 32 (2×) | clip + 4 from the phase-entry play (7/10), clip + 3 (3/10) |
| gm match-over (`326270`), `match_s05` | match call 58 | clip + 4 (one sample) |

Her countdowns (+0xdc, `0x413e20`) are clip + 6, which is why her second word looked like "gap − 2".
The other two recordings fit: slot 3 / `new_recording` and `1p3goodcpus` are **umpire 2** a (read live from
slots 3 and 4: `0x2ef7de` = 2), whose calls all run on the countdown (out 35 + 5 > 35; net 39 + 5 > its 35).
The old test assumed umpire 4 for `new_recording`; fixed. The countdown table is indexed by `0x2ef7df` (voice
a/b), not the language: `exe::scoreboard_timing(voice, umpire)` renamed.

Port: `umpire::VOICE_LAG = 5` added to the clip's frames where the app times her voice (`play_sounds`). The app
plays the line call a tick before the game (at the verdict, not the next tick's phase entry), so +5 there is the
scoreboard's +4. `Game::umpire_voice` now holds the ticks left, so the match-over wait (which the app also plays
in the same tick the game does) reads it one tick sooner (`<= 1`).

## Match over (`326270`)
Match over (0x4230be): until the scoreboard's +0x192 (pause end) the normal path runs (nothing ends it). On +0x192:
no 0x19 (players don't react), `188800(obj, 0, 0)` deactivates each player and its +0x70 companion (they freeze),
+0x376 = 1, voice program 2 key 4, return. From the next tick: voice still sounding → return; else
`184610(0)` (clears the pause/overlay flag 0x2eefe8) and phase 5. The early return on `0x2ef870 > 0` is the
attract demo's exit timer (main loop), 0 in a match. Port: `PostPoint::calling_match` (instead of `reacted`),
`Next::MatchOver` on the first voice-idle tick after; the show's end and presses are ignored then.

## Tests
- `score.rs::match_s05_match_over`: the last point-over phase of `match_s05` (after the replay): call at tick 31,
  phase 5 asked at 94 = 31 + 58 + VOICE_LAG − 1, as recorded; one tick earlier fails.
- `score.rs::match_s05_post_point`: each fault/net line end found is clip + VOICE_LAG (mode) or − 1.
- `hst-base audio::call_clips`: the umpire 4 a clip lengths used above.

## Not verified / not 1:1
- The ±1 jitter: the original's voice-stop read lands a tick early in ~1 of 4 calls (the IOP isn't locked to the
  frame); the port always takes the mode, so such a call's post-point phase is one tick longer than that run of
  the original. Not reproducible from the EE side.
- Clip lengths are our mixer's render (`centre_frames`), verified only for umpire 4 a (7 clips) against
  recordings; other umpires/voice b assume the same lag (no recording where their voice, not the countdown,
  ends a call).
- The match-over lag rests on one sample (clip + 4, same as the scoreboard's mode).
- The players aren't frozen during the match call (the app doesn't play phase 5; the next match starts with
  fresh placement), presentation only.
- After the match-ending point's instant replay the original re-enters phase 4 (new 30-tick pause, then the
  call); the app's replay resumes at the pause end, so the call starts ~31 ticks sooner there (P0b4d replay
  presentation).
- Without a loaded umpire bank the app has no voice to time, so calls run on their countdown and the match call
  ends at once (as before).
