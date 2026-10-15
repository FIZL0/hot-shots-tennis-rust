# B408 — umpire mods: no "calls as <donor>"

User 2026-10-15: an umpire mod has its own model and voice; the donor only fills missing calls and word timing,
so the menus shouldn't name it.

- `mods_umpire::Umpire::game()`: the source game after a `Series: Game` colon, else "Mod" (same rule as court mods).
- Umpire strip (`main_menu.rs` `umpire_screen`): subtitle was "Get a Grip, calls as Lily" → "Get a Grip".
- Confirm screen (`main_menu/confirm.rs`): "Calls as Lily" → "Get a Grip".
- Unit test in `mods_umpire.rs` covers `game()` with and without a source.

## Not verified / not 1:1
- No screenshot taken: a text-only change to front-end screens (ours to design, PLAN.md), and the worktree has no
  umpire mod installed to show it; the new string is shorter than the old one, so it fits wherever the old did.
