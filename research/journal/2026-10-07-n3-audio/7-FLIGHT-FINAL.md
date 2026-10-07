# N3c5: flight whistle (FINAL)

## The code
- The hit trail (`0x33ea10`, run at every hit) stops the flight sound object (`*(0x3165c8)`, handle +0x5c, mode +0x60) and restarts it by shot kind (fx+0xd0, 5 for a smash): `0x1a1b60(obj, mode)`.
  - Mode 1 (pitch from height): the hitter's framed flag (player +0x3f06, `0x355620`) for any kind, and always for a lob (the lob branch's mode argument is lost in the decompile; the recordings show mode 1).
  - Mode 0: training flag gm+0x344 (own volume table `0x1bc5b4`, `0x1a2c30`) — not ported.
  - Mode 2 (rush, program 5 key 6/0, volume by km/h·0.9 ≥140 in bands): the serve (rally +0x20 == 1) for topspin/flat, a slice for character 10 under `0x37f660`, any speed-qualified smash or other kind — not seen in the recordings, not ported.
- Mode 1 start: slot 0, program 5, key 2 (5 for characters 5/0xd under `0x37f660`), volume 0x80 at the ball (+0xe0), then speed `0x1a2b30(top 10, 1, low 0, 2)` (low 0.3/top 8 for characters 5, 6, 9, 0xd under `0x37f660`).
- `0x1a2b30(top, min, low, hi)`: h = |ball y|; h ≥ top → hi; h < min → low; else |((hi − low)/top)·(h − min)| + low; clamp to [0, hi].
- Per frame (`0x1a2970`, mode 1): speed `0x1a2b30(10, 1, 0.3, 2)` and re-place (`0x19c960`: falloff + bearing → volume and pan, no Doppler) at 0x80.
- `0x1a10c0` stops it once the live ball's bounce count (+0x224) is ≥ 1.

## Proof
- Test `flight_whistle_matches_the_game` (hst-sim `tests/sound.rs`, hits_s05): the 5 lobs/framed hits start the voice (sample `BASE + 0x9e30`) that frame or the next and nothing else does; 330 later frames each have the scale word `speed_word(flight_speed(y, 0.3, 10))` and L/R = court chain for program 5 key 2 at the ball (this frame's ball or the last, the recording's phase); no update 3 frames after the bounce.
- The first frame's scale is the start's (low 0: 0 below 1 m, as at frame 1771).

## App
- `audio.rs` `Sound::play`/`play_at` return an id; `update(id, play, pos)` re-places and re-pitches its voices; `stop(id)` releases them.
- `play.rs`: `strike` turns the whistle on for a lob (kind 3, serves included), the step turns it off at the first bounce, `play_sounds` starts/updates/stops it at the ball.
- ponytail: framed hits are not modelled (P3), so only lobs whistle; mode 0/2 and the character variants are left out.
