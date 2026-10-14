# M17: live store check (uploaded on one machine, listed, installed and played on another)

The user ran the in-game upload (2026-10-13, TODO note "uploads work"): the store `FIZL0/opentennis-mods` now holds
50 mods, all as open PRs (#4–#56) from the account `hotshotsgaming`, committed by the game's Hub API path.

## Checked here (2026-10-14, a different checkout, no token)
- `HST_LIVE=1 … live_store`: the listing (main + every open PR, `refs/pr/<n>`) returns all 50 mods. The test's
  preview assertion was stale since B248 keyed previews by `<author>/<id>` (`Entry::key`); fixed.
- `HST_LIVE_INSTALL=fore_pc11_z HST_LIVE_ROOT=context/m17/run4/mods … live_install`: Z (6.7 MB, PR #4) downloads,
  passes the sha256 and `mods::read` checks, unpacks 30 files in 1.5 s, peak RSS 60 MB. `HST_LIVE_ROOT` is new: it
  keeps the install so it can be played.
- `tools/shot.sh context/m17/run4/z-play2.png --court 1 --play --singles --slot-mod 0 context/m17/run4/mods/fore_pc11_z
  --shot-at 25`: a singles match with the store's Z as 1P, bot opponent, HUD with Z's face.
- Browser path: `resolve/refs%2Fpr%2F4/…/mod.zip` and the open-PR discussions API both answer with
  `Access-Control-Allow-Origin: https://opentennis.dev`, so the web build can list and download PR mods.

## Not verified / not 1:1
- The browser build's Browse → install → match flow was not driven here (no browser automation in this run); the
  CORS headers it needs are checked above, and the wasm build compiles. The human items B227/B242 cover phone
  installs.
- Store UI is ours (menus need not match the original), so no comparison against the game.
- `--play` shots in this worktree show no court geometry, also without any mod (`stock.png`): unrelated to M17.
