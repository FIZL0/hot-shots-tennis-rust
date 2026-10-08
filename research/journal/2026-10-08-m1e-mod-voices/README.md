# M1e: mod voices like the SPU's (2026-10-08)

Remaster-only feature (mods); the yardstick is the disc voice the wav replaces. Iteration: this file is FINAL.

## Survey
Every take in all 14 characters' voice banks (singles/doubles, a/b) is one key-on of one tone: ADSR 0x80ff/0x9fe0,
centred (pan 64, centre gains), set volume 127. What varies per take: key-on delay (0–58 frames), velocity
(100–127), tone volume (100–127), root note/fine (the sample's rate). No take loops.

## What changed
- `snd::Voice` can play PCM (`Voice::key_on_pcm`): 4 samples a queue fill like an ADPCM quad, ends on the fill that
  reaches the end (as at an end-flagged block), then the same gaussian interpolation, envelope and volume.
  `snd::adpcm(bd, start)` decodes a disc sample to its end block.
- `audio.rs`: wav takes keep their own rate (mono, channels averaged; no resampling) and play as SPU voices in
  `Mix::voices`: pitch register = round(rate·4096/48000) × play-speed word >> 12 (as `snd::pitch` scales a disc
  tone), the disc voice tone (`VOICE_TONE`: that ADSR, centred, volumes 127) at velocity 127, so `Sound::stop`
  releases them and `Sound::update` re-pitches them. The separate full-envelope PCM path is gone.

## Verified
- `audio::tests::wav_from_a_disc_take_plays_like_it`: character 0's singles-a takes keyed on at once at velocity
  and tone volume 127 (10 takes) are decoded and written as mono wavs at the rate their voice plays; through
  `Sound::play` each renders bit-identical to the disc take as recorded, at play speed 0x0c00, and stopped after
  10 frames (release). Mutating the wav pitch by 1 or the end-of-sample stop makes it fail.
- `tools/check.sh`: 293 pass.

## Not verified / not 1:1
- Not compared with the running game (PCSX2): the disc path it's checked against is the existing `snd::Voice`
  port, not a fresh SPU capture.
- A mod take always plays at velocity 127, tone volume 127, with no key-on delay: disc takes with velocity/volume
  100–127 or a delay have no wav equivalent (the wav itself must be quieter / start with silence). By design
  (M1a: a take is the sequence's full level).
- Keys past a program's takes still wrap round them (the game asks for keys its disc bank has; a mod may ship
  fewer). Remaster-only, nothing to match.
- Wav rates above ~192 kHz clamp at the SPU's maximum pitch (0x3fff); stereo is downmixed by integer average.
