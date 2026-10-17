# B550 — mod characters' .glb preloaded on the async pool

B547's iPhone online match with a mod spent its first frames loading the mod's `.glb` costume on the main thread
(`mod_match::load` → `mods::load` in `play::setup`). Now it loads the way P50 loads disc characters.

- `preload.rs`: `mods(world, iso, &[(Mod, costume)])` starts `mods::load` on the pool (`jobs::spawn`) into `Staged`
  stores, keyed by the costume's `.glb` path; a costume-less mod (online stand-in) preloads its donor through
  `characters`. `load_mod` takes the finished job (waits if it's still running), falls back to loading in place.
  `busy`/`clear` cover both queues.
- `mod_match::load` goes through `preload::load_mod` (and the stand-in through `preload::load_disc`).
- `mode.rs`: `Mode::early` adds to an earlier `early` instead of replacing it (the online launch adds its own).
- `main.rs`: `--play` and `match_mode` (menu Start) preload `--slot-mod`/`--mod` mods (`mod_lineup`, costume = the
  slot's `--outfits`); `online.rs`'s launch preloads the lobby's `MatchMod` (`online_mod_lineup`).
- The skeleton swap for online mods still happens after the load, in `mod_match::load`, unchanged.

## Check
- Test `preload::tests::a_mod_loaded_early_is_the_one_the_start_loads` (getagrip_pc00_emi): the early load is
  taken, matches a direct load (joints, parts, motions, asset counts), and its handles resolve in the world's stores.
- Desktop release, `--play --singles --slot-mod 1 context/mods/getagrip_pc00_emi`: the `start: play::setup` log
  drops from 159 ms (preload off) to 53–74 ms; the mod draws on court (shot in `context/b550/`).

## Not verified / not 1:1
- Not run on an iPhone or in the browser (no device here); in the browser `jobs` runs it between frames, as for
  disc characters. Expect the same seconds-long frame to move to the loading screen there.
- Online launch path not run end to end (needs two peers); it only adds the same early call to the same mode.
- Gameplay unaffected (same loader, same data): no PCSX2 check needed.
