# N3c2: racket hit sounds (FINAL)

## Path
- The effects manager (`0x33e5d0`, object at `*(0x423f80)`) handles event 1, the hit. It sets fx+0xd0 to the shot kind (0 topspin, 1 slice, 2 flat, 3 lob, 4 drop), fx+0xbc from player+0x3f06 (framed mis-hit) and fx+0xbd from player+0x3f0c (dull mis-hit), then calls the hit effect `0x340860(obj, 1, player)`.
- Per-player records at fx+0xd8+8p, filled by `0x33e060`: +0 timing offset (player+0x3fa0); +4 grade (player+0x3ee8; 1 and 2 are clean); +5 branch (player+0x3ec1: 0 serve, 1 ground, 2 volley, 3 dive, 4 smash).

## Hit effect logic
Ported as `hst_sim::sound::hit_sounds`; slot 0 = court bank, program 6.
- Framed: slot 9 program 0 key 4 at 0x80. Dull: key 3 at 0x80.
- Player power gap > 3 (`0x3555f0`/`0x355600`): key 8 at 0x46 (gap < 7) or 0x5a, played twice; the second play gets speed 0.9 (`0x1a04d0`).
- Main sound, skipped when framed or dull:
  - smash: key 5 if slice; otherwise key 1 at 0x80 when |offset| < 2, else 0x6c, with speed 1.05 / 0.95.
  - drop: program 5 key 4 at 0x80.
  - clean grade: key 0 at 0x76 if |offset| < 2 and not a dive, else key 2 at 0x76.
  - otherwise a random bit: key 2 at 0x76 or key 3 at 0x80.
- Extra key 4 at 0x62 when all hold: no gap or gap < 4; not a lob or drop; clean grade, |offset| < 2, not a dive; and solo (`0x422fa4` == 1), or hit count `0x423060` != 1, or the server's toss (+0x3ea0) is strong (X).
- Character 9's lob after the serve plays slot 9 key 7 (left to N3d).
- The ball-speed whooshes (keys 6-8 at speed >= 100 / >= 70 km/h, speeds 1.2/1.1/1.0) are dead code in practice: gated on the float table at `0x3fc1f0` indexed by the court, which is all zero in the ELF and in RAM and never written.

## Position
The sounds play at court marker 8 (`*(gm+0xb8)+0xd130+8*0x40`, translation +0x30), which equals the ball position (ball+0xe0) every frame. So they play at the ball, at the hit frame or the one before.

## Driver behaviour seen in the ring
- A hit voice can key on with an unplaced (centred) cmd-1 volume; its positional L/R follows in the next frame's cmd 1.
- The play speed (cmd 4 scale word, (int)(f*4096), e.g. 0.95 -> 0xf33) lands the frame after key-on. It reaches only the last voice of the play: smash key 1 has two tones and only the second gets 0xf33. When that tone keys a frame later, it starts with the scale already set.
- The recorded "last positional play" triple is often the flight sound that starts right after the hit (pitch words 0x4941/0x4944, varying scale, N3c4), so it cannot be read as the last hit play. The test uses it only for bearing and distance.
- Other non-0x1000 scales (pitch words 0x4330..0x4d37, scale 0xe74-0x12e0, in groups 18 frames apart) are likely footsteps or crowd (N3c4/N3d).

## Recording and proof
- `tools/record_sound.py` gained an optional 4th argument `hits`. It appends a 0x330-byte hit block after the play triple: fx+0xb8 (0x48), 0x422fa0 (0x20), 0x423040 (0x40), gm+0x340 (8), marker 8 (0x40), players 0/1 +0x3e90 (0x120 each). It also fixes a layout bug that wrote 4 extra bytes. Players 2/3 are not recorded, so their power gap is unknown and taken as none.
- `context/fixtures/hits_s05.bin`: 3600 frames, slot 5 doubles bot match, court 10.
- Test `hit_sounds_match_the_game` (crates/hst-sim/tests/sound.rs): all 28 hits get their court key-ons (40 key-ons) with bit-exact L/R from falloff + stereo at the ball, and the smashes' 0xf33 scale. Random keys 2/3 are taken as whichever keyed. Slot 9 plays are not checked (character bank not in the recording).

## App
- `play.rs` `strike` builds the `Hit` (framed/dull/power gap not modelled, P3) and queues the plays at the ball.
- `play_sounds` (FixedUpdate) plays slot 0 through `audio::Sound::play_at` with the court bank (`CourtBank`, loaded from SND/COURT/C_SNDnnA.XB0).
- `play_at` applies the play speed as the driver's scale word to every voice. ponytail: the game scales only the last voice.

## Open
- Why the speed reaches only the last voice.
- Slot 9 and character-9 lob: N3d.
