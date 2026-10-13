# B111 — match sounds stop with pause / leaving the match

The port's sounds are not Bevy audio entities: one `AudioPlayer` streams the mixer (`audio.rs` `Mix`), and every
SFX, voice, crowd, umpire, jingle and weather loop is a play id inside it. So the tag lives there, not per call site:

- `Sound::begin_match()` (the match's `menu::plugin`, PreStartup): from then on every new play id is tagged as the
  match's (`Tags.ids`), unless the pause screen is up.
- `Sound::hold(bool)`: the pause screen opening (Start, or a peer's pause online via `set_paused`) freezes tagged
  plays — no sequence steps, no voice samples — and closing it resumes them where they were. Plays started while
  held (the pause screen's own blips) aren't tagged. The BGM isn't tagged (the mode switch already stops it).
- `Sound::end_match()` in `mode.rs` `teardown`, the one place every exit passes (pause → Return to the Menu, the
  result screen's exit, the online disconnect's `AppExit`, all via `back`): drops every tagged play and voice.
- Tests: `mode.rs` `match_sounds_end_with_the_match` (a match's sounds, held ones too, are gone after its quit; the
  menu's and pause blip survive; the next mode's plays aren't tagged), `audio.rs` `holds_match_sounds`.

## Not verified / not 1:1
- Not checked against the original whether its pause also freezes looping weather/crowd or cuts them: in-match
  presentation only has to feel right (PLAN.md), and the task asks for pause/resume.
- Not heard in a running build (unattended run, no listening): covered by the tests above only.
