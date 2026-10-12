# M2e: match seed and handshake

## Seed
Every match generator already follows from two values: `rand()`'s state when the setup starts and the shared MT's
seed. The AI generator is reseeded from shared draws as each AI object is made (`Rngs::new_ai`), the court's and the
sound manager's from `rand()`; the "fixed AI value" the line mentions is only `Rngs::new`'s placeholder before the
first `new_ai`. So the match seed is `hst_sim::rng::Seed { rand, shared }`:
- `Seed::after(skip)`: the menus drew `skip` `rand()`s, the next one seeds the shared MT (what main.rs did inline
  with the clock-picked skip);
- `Seed::shared(s)`: `HST_WEATHER_SEED`, `rand()` at boot (as before).
`load` (main.rs) takes `handshake::MatchSeed` when it's there (a peer takes the host's), else picks as before, and
inserts the match's `Handshake`. Offline play draws exactly as before (same skip, same order).

## Handshake (`crates/hst/src/handshake.rs`)
`Handshake::of(args, pack hash, mods, seed)`: build (`HST_BUILD` at compile time, else the crate version), disc pack
hash (`Iso::pack_hash`), rules (court, singles, sets, games, umpire), seats (peer per seat, `None` computer),
characters, costumes, each slot's mod gameplay hash (FNV-1a of donor, hand bits, TParam base + overrides, AI row:
same on every platform, looks excluded), and the seed. `check(&host)` returns a sentence for the player naming what
differs. Tests: `handshake::tests` (joins; each difference refused; mod hash moves only with gameplay data),
`tests/rng.rs seed_after_skip`.

## Not verified / not 1:1
- Nothing to compare against the original: online play is the remaster's own; no PCSX2 run needed.
- Not yet sent over a wire: the transport is M2f, which inserts `MatchSeed` from the host's handshake and calls
  `check`; `seats` are filled per peer by the lobby (offline: human seats = peer 0).
- Build identity is the crate version unless `HST_BUILD` is set at compile time: two dev builds of one version with
  different code would pass; M2g's state-hash exchange catches the desync.
- An ISO run has no pack hash, so ISO vs pack (or ISO vs ISO) isn't hashed: both passed the SCUS-97610 1.00 disc-ID
  check at start.
- `HST_WEATHER` (a forced weather) isn't in the handshake: a debug override, not a menu choice.
