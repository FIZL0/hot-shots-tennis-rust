# 01 — clouds from the game's draws (FINAL)

## Findings (decompile + recordings, PCSX2 copy 3, slot 5 = court 10 doubles)
- Spawner: 5 `rand()` draws per cloud, in the order yaw, z, y, x, model. Each float is `t & 0xfffff800` × 2⁻³¹.
  Yaw = 2π·u. Each coordinate = `lo + (hi−lo)·2⁻³¹·t` (`adda`/`madd`). Model = `rand % model count`. Each cloud is
  inserted before the first cloud with a higher y, so the list is sorted ascending by y. R = 50·40.
- Respawner(court, old weather):
  - Does nothing when the old and new weather are both heavy rain (3/5).
  - Does nothing when there are no in-season cloud models.
  - Otherwise the count is the court's envir count, or the exe constant (200; 50, −2, −5) in weathers 3/5. The
    constant is read by `exe::Game::rain_clouds`.
- New-point order, recorded: shared reseed → cloud draws (only when the weather changed) → court reseed → sound reseed.
  `research/p3f3_cloud_log.py` rewrites slot 5's schedule to 3, 5, 1, 0. Recorded draws:
  - 0→3: 1000 (200 clouds)
  - 3→5: 0
  - 5→1: 45
  - 1→0: 45
- Respawns happen in doubles too (the clouds just aren't drawn there).
- Setup: in singles the setup spawns clouds for the first game's weather. Two doubles setup logs had no cloud draws
  (`context/p3f3/setup_*.bin`). P3f's menu logs had 100.

## Port
- `rng.rs`:
  - `Rngs::new_point_clouds(n)` and `Rngs::clouds(n)` added.
  - `setup_rand` returns the cloud draws too.
- `weather.rs`:
  - `cloud_layout` takes the exe's rain layout.
  - `clouds_remade(old, new)` added.
- `clouds.rs`: `spawn(draws, models, y)` is now bit-exact (ps2 ops).
- `main.rs`:
  - The `Sky` static records which weather the clouds were made for.
  - `point_clouds` (called from `play.rs next_point`) draws a respawn on a weather change.
  - The `clouds` system lays out the setup's or the respawn's draws. The xorshift is gone.
- Test `rng.rs clouds_remade_like_the_game` (`context/p3f3/run2.bin` + `.clouds`):
  - The draw counts, plus landing on the court and sound reseeds, match exactly.
  - Each cloud's y and model match the game's list bit for bit, in its order.
  - x and z are within 5 units.
- Test `effects.rs`: asserts `rain_clouds` = (200, [50, −2, −5]).

## Not verified / not 1:1
- **The setup's second spawn.** At its end the setup compares the first game's weather with a byte in the cloud
  object that the constructor never sets (malloc garbage). It can therefore draw a second spawn that depends on heap
  contents: P3f's menu logs had 100 cloud draws, the two doubles logs here had none. The port assumes the byte equals
  the first weather (no second spawn), and spawns at setup in singles only.
- **A heavy-rain first game at setup** wasn't recorded. Its count of 200 comes from the decompile plus the exe
  constant.
- **x and z** are compared only within 5 units. The list was snapshotted a few ticks after the spawn, so the clouds
  had already drifted. y and model are exact. Yaw isn't compared, because its offset in the cloud object wasn't
  identified.
- **The app's game index** is taken as `score.games_played`, the same index `play/weather.rs` uses for the schedule.
  It wasn't recorded across a game boundary in the app.
- **No visual check.** The `--stage 10 --singles` orbit shot (`context/p3f3/shot_w3.png`) doesn't look at the sky,
  so the clouds weren't compared on screen.
