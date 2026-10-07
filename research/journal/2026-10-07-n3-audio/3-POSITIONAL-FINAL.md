# N3c1: positional play (FINAL)

N3c split into N3c1–4 (positional play; racket hits; swing whoosh; bounce/net/roll/serve, flight sound, footsteps).

## The positional play
- Play call `19fa70(slot, program, key, &pos4, mode, volume, prio)`; clamps slot 0..24, program 0..127, key 0..65535. Mode 1 = positional (court sounds); mode 0 (gallery, the `0x1b0098` stereo pair) has no falloff and is left to N3d.
- Listener: the library context (pointer at EE `0x312540`) has mode +4 = 0 in every save state, so the listener is fixed: identity orientation at (0, 0, −10) (entry 2 of a runtime table of three: (0,0,0), (0,0,−4), (0,0,−10)). Yaw 0.
- Bearing: `msub((π + atan2f(dx, dz)) · 57.29578, π + yaw, 57.29578)` truncated, +360 if negative, clamped 0..359 (0 = straight ahead, +z). Distance: `sqrt(madd(dz·dz, dx, dx))` truncated, clamped 0..128. The context keeps the last call's bearing (+0xc), distance (+0x10) and volume after falloff (+0x14).
- Falloff (curve flag 0): unchanged up to 10 m, else `msub(v, v / 118, d − 10)` truncated, clamped 0..128.
- Stereo: i = (bearing + 90) % 720 into two 720 × i32 tables (2048 · cos over half degrees, −4096 = zero; L at EE `0x1bdb78`, R at `0x1bd038`, both in the exe). Sequence volume = v · |g| >> 11. The signs (phase inversion) apply only in surround mode, which the game forces off. Play pan 0x40.
- Bank volume: `0x1ce970` gives each of 24 slots a category, `0x1bbf00` = [118, 125, 115, 95, 92] the volume per category; slot 0 (court SE) 118.
- Play speed: a post-play call clamps a float to 0..2 and sets the scale word (int)(f · 4096); hit sounds use 0.9..1.2.

## Recording
- `tools/record_sound.py <slot> <frames> <out>`: record_live.py's sample, the live ball's first 6 contact records (ball +0x8f8, 0x50 each), the context's (bearing, distance, volume), and the ring commands written since the previous frame (ring EE `0x305000`, write index `0x304fc0`). `context/fixtures/sound_s05.bin`: 3600 frames from slot 5.
- Only each frame's last play is visible; when two plays land in one frame (a hit and the flight sound, a bounce and a footstep) the earlier one's key-on cannot be checked.

## Proof
- Test: hst-sim `tests/sound.rs`. Every live court bounce (first or second, material 1; 22) keys a court voice, this frame or the next, whose cmd-1 L/R equals our chain from `place(contact position)` and `falloff(0x80, d)`. 83 court key-ons (co_se10, scale 0x1000) whose play is in the recording get cmd-1 L/R = `Level::volume` with `stereo(volume, bearing)`, bank volume 118, the key-on velocity and the tone; 39 more belong to plays the recording overwrote.
- Dead-ball bounces (3rd on) play something else (a centred voice, or nothing); walls/net materials have their own keys: N3c4.
- Play speed (scale word) is left to N3c2, with the hit sounds that use it.
- The bounce sound uses the contact record's position, not the ball's frame position (which differs by up to 3°).

## Seen, for later
- Frame 486 on: one voice of the court bank whose volume L/R and pitch scale follow the ball every frame (flight sound) — N3c4.
