# B77 — sound while paused (FINAL)

## What the original does (slot 5, bot singles, PCSX2 copy 3)
- `research/pause_sound.py <slot> <out> <run_s> <paused_s> <after_s>`: loads the slot, presses Start after `run_s`,
  again after `paused_s`, and logs every sound-ring command (EE 0x305000, index 0x304fc0) with wall time and vsync.
  Audio: the copy's output routed to a private PipeWire null sink (`PULSE_SINK`, `OutputMuted = false` for the run
  only, restored after) and recorded with `parecord`; `research/pause_sound_levels.py` prints peak per 0.1 s.
  Logs and wavs: `context/b77/run1..5`.
- Opening the pause sends nothing but the pause sound's key-on (system bank, two voices). No key-off, no volume
  or core-volume command. Voices already keyed ring out; the crowd's held bed (gallery voice keyed at the load)
  goes on at the same level all through the pause (audio peak ~1000 before, during and after).
- The one exception, run 5 (Start 0.55 s into a lob's whistle, voice 0x2c, ADSR a5ff/9fed): the whistle's
  per-frame update goes on while paused but sends cmd 4 with scale 0 every frame (pitch 0 — the voice stops where
  it is, silent; volume command no longer sent). The whistle is not keyed off. On closing, three more frames of
  scale 0, then the update resumes at the pitch it had (0xef0 before, 0xef3 after) and the whistle ends at its
  bounce as usual. So the game's per-frame sound update runs at play speed 0 while paused.
- The BGM wasn't playing at either pause (a match plays it only around the intro / change of ends), so whether
  pausing touches it isn't seen.

## Port
- `audio.rs`: the mixer remembers the plays the match re-levels every frame (`update`/`turn`: the flight whistle
  and rush, the flyover); `Sound::hold_followed` sets those plays' voices to pitch 0 and their sequences to scale 0.
  Nothing else is touched. The match's next `update` after unpausing re-pitches them (the match's sound system only
  updates on a fresh tick, so nothing re-pitches them while paused).
- `play/menu.rs`: opening the pause (Start/Esc/`HST_PAUSE`, or a peer's pause online) calls it.
- Test: `audio::tests::pause_holds_the_whistle` — whistle sounds, held → no sample-to-sample change for 4 s and
  still playing, next update → sounds again.

## Not verified / not 1:1
- The three extra scale-0 frames after closing: the port re-pitches on its first tick after unpausing (3 frames
  = 50 ms of whistle earlier; not worth a frame counter for a presentation detail).
- Only the whistle was seen held. The rush and the flyover are held the same way on the assumption that they go
  through the same per-frame update (the rush is the same flight code; the flyover wasn't captured paused).
- BGM and jingles during a pause: not captured (none playing at the pauses), left playing.
- Whether a multi-step SE sequence keyed before the pause goes on stepping while paused: not seen (none was mid-
  sequence); the port lets non-followed sequences finish.
- One-shot SFX ringing out through the pause is what the original does, so the port keeps it; if the user still
  finds that annoying, muting them would be a change from the original.
