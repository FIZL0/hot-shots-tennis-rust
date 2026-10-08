# P2 — Aim exactly as the original

Status: FINAL.

The port's human and bot shot aim now follows the original's aim (retail 34ccf0 and its compute 34c250): sweet-hit test, depth and width from the court constants, stick-to-square mapping, button offsets, the angle limit with the incoming-slice multipliers, then the projection and near clamp. A sweet full-diagonal stick lands exactly on the corner (5.485, 11.885) in doubles and (4.115, 11.885) in singles. Every recorded human aim is bit-exact against the game's result.

## Files
- `crates/hst-sim/src/shot.rs`: `AimStats`, `Hitter`, `aim`.
- `crates/hst/src/play.rs`: `aim_target`, `aim_stats`, `Player::aim_from`, `Game::last_sweet`, `Game::aim_stats`.
- `tools/record_aim.py`: records aims from the real game with the virtual pad.
- `crates/hst-sim/tests/aim.rs`: `human_aims`. Fixtures `context/fixtures/aim_singles.bin` and `aim_doubles.bin` (git-ignored).

## Journal
- [1-AIM-FINAL.md](1-AIM-FINAL.md): what the original does, the port, and the verification.
