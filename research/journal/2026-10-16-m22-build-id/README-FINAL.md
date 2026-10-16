# M22 — online version mismatch check

Before this, the lobby's Hello compared `"hst " + CARGO_PKG_VERSION` and the M2e handshake compared `HST_BUILD` (set
only by `ios-ipa.sh`) or the version, so two builds of one version from different commits met and could desync.

## What changed

- **One build id** for both checks: `hst_base::handshake::build()`, "VERSION COMMIT" (e.g. `0.1.0 2e28939`, `+` after
  the commit for a build with uncommitted changes). `hst`'s `main` / wasm `run` / `android_main` set it first thing
  (`handshake::set_build(BUILD_ID)`) from `HST_COMMIT`, which `crates/hst/build.rs` already made for the version
  corner. A static set by the top crate rather than a build script in `hst-base`, so a commit doesn't rebuild
  hst-base and hst-play (P59's reason for not watching git in dev builds). Unset (hst-base's own tests, dev builds)
  it is "VERSION dev". `net.rs`'s own `BUILD` const is gone.
- **Same stamp on every platform**: build.rs takes the commit as exactly 7 characters (`rev-parse HEAD | cut`, not
  `--short`, which grows on an ambiguous prefix) and honours `HST_COMMIT` from the environment.
  `tools/build.sh` computes it once and passes it into the Steam Runtime container (a worktree's `.git` file points
  outside the mount); `tools/release.sh` sets it for `build.sh all`; `pages.yml`'s web, Linux (container checkout
  may have no usable git) and Windows jobs set `${GITHUB_SHA::7}`.
- **Refused at the door**: the host's Hello check (before seats/picks, already first in line) and the rejoin path now
  say `Refused: a different version of the game (host 0.1.0 aaaaaaa, you 0.1.0 bbbbbbb). Update so you both have the
  same one.` (a rejoin from another build used to say "the match has started").
- **Find**: `Listing` carries the host's `build` (serde default: a pre-M22 listing reads as another build). Lobbies
  of another build draw as a dim grey row, "Other version: update" instead of court/seats; OK on one plays the back
  sound and puts the refusal text in the status/info line instead of joining. The Online card's preview says "Other
  version" for them. Quickplay skips them. The mDNS listing still fits 245 bytes with a 24-emoji name and a
  `10.10.10-rc.10 2e28939+` build (test).

## Verified

- `net::tests::another_build_is_refused_at_the_door` (loopback, relay off): a guest Hello of another build gets
  the refusal with both ids and is never seated; quick_pick/same_build on listings incl. a pre-M22 one.
- `online::tests::another_builds_lobby_is_not_joined`: OK on a greyed lobby doesn't join and says why; the same
  build's joins.
- Two real dev binaries, `HST_COMMIT=aaaaaaa` and `bbbbbbb`, `hst net host` / `hst net join --relay off` on this
  machine: bbbbbbb → `Failed("Refused: a different version of the game (host 0.1.0 aaaaaaa, you 0.1.0
  bbbbbbb)…")`; a second aaaaaaa → `Joined: pick your character`.
- A wasm32 release build (`--features ship`) with `HST_COMMIT=aaaaaaa` contains the identical id string `0.1.0
  aaaaaaa` as the native one.

## Not verified / not 1:1

- **Native ↔ web join not run live**: it needs a relay and the browser harness with a disc pack; shown instead by
  the wasm binary carrying the byte-identical id (same code path, same build.rs).
- **Dev builds**: debug builds are "VERSION dev" (as before, P59: watching git would rebuild on every commit), so two
  dev builds of different commits still meet. Stamp one by hand when testing online across machines:
  `HST_COMMIT=$(git rev-parse HEAD | cut -c1-7) cargo build -p hst`.
- Two dirty builds of the same commit (`abc1234+`) with different edits still meet; shipped builds are always clean
  (release.sh refuses a dirty tree; Actions build checkouts).
- The Find screen's grey row not looked at in a screenshot (`--shot` launches grab other agents' vpads in a parallel
  run); the `lobbies` menu fixture now lists a third lobby of another build for the next shot.
- The pages.yml change only runs on the next push to main.
