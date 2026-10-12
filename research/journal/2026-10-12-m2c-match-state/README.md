# M2c — match state you can copy and hash

`play/snapshot.rs`: `save(&mut World) -> Snapshot`, `restore(&mut World, &Snapshot)`, `state_hash(&mut World) -> u64`.

- `Game` is `Clone + Debug`. Its per-match constants (serve/smash/rally tables, rally records, specials, timing,
  serve data, collision world, pelvis, `CharacterData`) sit behind `Const<T>` (an `Arc`): copies share them, `Debug`
  prints `_` so the hash skips them, `DerefMut` only works while nothing shares them (setup, mod overrides) and
  panics otherwise.
- `Snapshot` = `Game`, `Pads` (latches + seats), `TickPads`, every `Rig`'s `Motion` (players, umpire, walkers,
  creatures), and the `play/` modules' tick resources: cutaway `Director`, `FootFx`, match_stats `Serving`, panel
  `Clock`, surprise `Pops`, `Weather`, reaction_voices `Frames`, bodyhit `Hits`, `Npcs`, popups `Calls`, and the
  effects (`Impacts`, `HitSparks`, `BallFlight`, `BallBounce`, `LandingMarks`: the hit sparks draw `rng.sound` while
  they live — without them the rerun diverged 36 ticks in).
- Two `Local`s became resources so they can be copied: reaction_voices' frame counter (`Frames`), match_stats'
  serving latch (`Serving`).
- `state_hash`: FNV-1a 64 over the `Debug` text of `Game` (constants out), the players' motions sorted by player,
  and `TickPads`. f32 prints shortest round-trip, so any bit change shows (except NaN payloads).

## Verified
- `snapshot::tests::save_run_restore_rerun` (round1.bin's 1P pads, singles Carol(6) vs (1), the app's FixedUpdate
  chain incl. the ten modules and effects): at ticks 300, 600, 900 save → 300 ticks → restore (hash back to the saved
  one) → the same 300 ticks: identical hash every tick. The 1200 ticks cover serve, rally and post-point twice.
  Then idle pads from the last snapshot give different hashes (the hash sees the input).
- `tools/check.sh`: 327 tests pass.

## Not verified / not 1:1
- `Game.umpire_voice` is an audio play id the sim reads (`post.tick > 0 && umpire_voice == 0`): the sim depends on the
  audio system finishing a clip. Not in the test (no sound). Needs fixing for rollback/netplay (M2d/M2g): drive it by
  the clip's length in ticks.
- `MatchRng` is only read at setup; the tick's generators live in `Game.rng`, so they're in the snapshot.
- Draw-only state not snapshotted (doesn't feed the tick): tornado wind, rain particles, popups' result banner and
  speed readout, noise sway, court_anim, rain's ambience `Local`s. weather's `clear` `Local` is taken at the first
  tick and constant after.
- `flare_tick` and `play_sounds` aren't in the test's schedule.
- Entities/handles inside `Npcs`/`Shown` are restored as ids: fine for rollback in the same world, not for a copy
  into another world (netplay sync sends inputs, not state).
- Hash cost: Debug text, ~0.1 ms a tick. A field walk if it ever matters.
