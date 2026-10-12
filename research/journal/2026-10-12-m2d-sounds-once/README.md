# M2d — sounds and effects out of the tick (for rollback) — FINAL

A rollback restores the match (`play/snapshot.rs`) and re-runs ticks; their sounds must not play again.

## What changed
- `play/heard.rs`: every match tick gets a number (`Heard::tick`, counted first thing in `FixedUpdate`, kept in the
  snapshot so a restore winds it back). `Played` (never snapshotted) remembers what started: a one-shot plays only the
  first time its (tick, Debug text, how many alike before it that tick) is seen, pruned after 120 ticks. A re-run that
  comes out differently (corrected input) plays its new sounds then; mispredicted ones that already played can't be
  taken back.
- `play_sounds`: court/voice sounds (hits, whooshes, shouts, bounces, creature sounds via `g.sounds`), the gallery's
  cheers, the jingles (`Bgm::step`) and the umpire's words go through `Played::first`. Continuous things (flight
  whistle start/update/stop, the BGM's level ramp) are only touched on ticks heard for the first time
  (`Played::fresh`), reconciled from the sim's state then.
- Sim state that lived in the audio system's `Local`s moved into the snapshot (`Heard`): the whistle's last-seen count
  (+ the tick it started) and the umpire word's ticks left. The flight sound's per-tick step (`sound::Flight::frame`,
  which can end `Game::whistle`) now runs from the sim state, also without a sound device, so a muted peer's state
  hash no longer differs there. `Game::umpire_voice` is now 1/0 (sounding or not) instead of the mixer's play id, so
  it's the same on every machine; the id itself is in `Played`.
- `npcs.rs`: creature sounds deduped the same way; the flyover's play id moved from `Npcs` (snapshotted, so a restore
  lost it) to `Played`, updated/stopped on fresh ticks only.
- Effects and balloons needed nothing: their state (`effects::Impacts/HitSparks/BallFlight/BallBounce/LandingMarks`,
  `Player::balloon`) is already in the snapshot and `Update` draws the last tick's; their `commands` only swap a
  material (idempotent).

## Test
`play::snapshot::tests::rerun_ticks_play_nothing_twice`: round1.bin's pads for 900 ticks, once straight and once
with, from tick 300 on, every tick a restore to 8 ticks back and those 8 again (each tick simulated up to 9 times).
The number of plays the mixer started (`Sound::plays`, a no-output mixer on the disc's banks) matches the straight run
after every tick (23 sounds in the window), and the final state hash is equal. With the dedupe switched off it fails
at tick 330 (7 vs 6). Unit test `heard::a_rerun_tick_plays_nothing_again`.

## Not verified / not 1:1
- Not a gameplay port: nothing to check against the original (sound timing on a first run is unchanged).
- The test's app is on court 0 (no sound bank), so it plays court 1's bank and gallery; the umpire word and jingles
  only fire if the 900 ticks reach them (the count covers whatever plays).
- Rain ambience (`weather/rain.rs`) keeps its voices in a `Local`; a re-run doesn't start them again, but its turn
  steps run once more per re-run tick (by-ear drift only).
- The umpire word's length still needs a sound device (`Sound::centre_frames`); a peer without one times the
  post-point differently (M2g's concern, unchanged).
