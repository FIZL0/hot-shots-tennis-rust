# P0b4d6: instant replay step sound (FINAL)

The footstep play N3c6 found (`8-DIVE-THUD-FINAL.md` in 2026-10-07-n3-audio) is now wired up, because P0b4d4's
replay cameras exist.

## What the game does (from N3c6)
In the run object's per-player loop, when a toe lands (`Feet::stepped`), the game plays court bank program 3 at
volume 0x69, using the dive thud's key (3 in weather 2/3, else 0). It only plays when all of these hold:
- an instant replay is showing (gm+0x344);
- the player's state byte (+0x3fa5) is 1, i.e. running;
- fewer than 3 players (singles);
- camera mode 0x56..0x59;
- the player is in front of the camera (`sound::step_heard`).

## Port
- `replay_camera.rs`: `ReplayCamera::close_up()` is true while a replay shot 0x56..0x59 is on. In the replay picks
  those shots are only ever closing shots (`ReplayPicks::end`), so footfalls sound only after the replayed point's
  last shot, while a player is still running.
- `foot_fx.rs`: after `Feet::tick`, `steps()` queues `sound::step(weather::now())` at the player for each landed
  foot. That happens only when a replay is on, in singles, on a close-up, with the player's state byte 1 and
  `step_heard` true for the match camera view at that moment (eye, rot row 2, the world→camera matrix, 240/tan fov).
- `reaction_voices::matrix` is now `pub(super)`, so foot_fx can use it as well.
- No sim RNG is drawn and nothing in the sim changes; the code only pushes to `whooshes`.

## Check
`foot_fx::tests::replay_steps_sound_on_close_ups_only` runs singles bot matches (characters 6 and 1, autoplay,
every plugin and `instant_replay::run`). Result: 3584 replay ticks, 201 on close-ups, 22 footsteps, and no footstep
off a close-up.

## Not verified / not 1:1
- **Not compared with the original by ear.** This needs a singles match whose replay closes on shot 0x56..0x59
  while a player is still running. The only bot-only save state (slot 5) is doubles, where the game never plays this
  sound. Slots 3/4 need a human to play out points until a replay picks that closing shot, and no human was around
  in this unattended run. PLAN treats in-match SFX as "by eye/ear", and the gate and the sound come from the decompile.
- **The point tested is the player's position.** The game tests the player's model origin. P13-era reaction voices
  use the same stand-in (`reaction_voices.rs`).
- **The camera view comes from the previous replay-camera step.** `foot_fx::tick` and `replay_camera::step` have no
  set order, so the view used for `step_heard` can be one tick old. That can only make a footstep at a cut sound or
  not sound one frame early or late.
- **`sound::step` itself is still unchecked against a recording** (its ponytail note stands), for the same reason as
  the first point.
