# N3c7: the lob whistle's tail after the bounce (FINAL)

Report: the lob sound "reverbs weirdly after it should be done". Verdict: the tail is the original's; no code change.

## What the game does
- The flight object's per-frame step stops the whistle once the live ball's bounce count is ≥ 1; the stop goes through
  the sound library to the driver as a **key-off**, not a hard cut.
- `context/fixtures/hits_s05.bin` (ring commands per frame, `tools/record_sound.py 5 … hits`): all five whistles
  (sample BASE + 0x9e30) are keyed on with ADSR1/ADSR2 `0xa5ff`/`0x9fed` and end with ring cmd 2 word 8 (key-off) on
  their voice at the bounce (or the next hit); no pitch/volume updates after it. Scratch parser:
  only a few lines over `samples()`' layout in `crates/hst-sim/tests/sound.rs`.
- `0x9fed`: sustain held at full level (increase, shift 31 = no change); release exponential, shift 13.
  On the SPU2 envelope (ours, `hst_data::snd::Voice`, PCSX2's formula) that release takes 0.93 s (56 frames) from
  full level to silence, −20 dB at 0.65 s, at the last pitch the whistle had. That ringing tail is what the report
  hears, and the original plays it the same way.

## App
- `play_sounds` stops the whistle at the same tick (`Sound::stop` → `Voice::key_off`), so the release is the same.
- Test `audio::tests::whistle_rings_out_as_the_game_releases_it` (hst-base): the app's whistle keys with the game's
  ADSR, holds at 0x7fff, and after `stop` releases for exactly 56 frames, above −20 dB at frame 38 and below at 40.

## Not verified / not 1:1
- No side-by-side listen against PCSX2's audio output: the comparison is by ring commands (key-off timing, ADSR) and
  the envelope model, not an audio capture.
- The original's SPU2 reverb/effects send is not modelled (`audio.rs` render: dry voices only); if the game sends
  court SFX to reverb, the original's tail is if anything longer/wetter than ours. Masking sounds (crowd, other
  SFX) also make the tail less noticeable in the original; that's presentation, not checked here.
