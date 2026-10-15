# B305 — default umpire is Random

**Finding:** the local default was already Random before this task — B191 set `Menu::default`'s `umpire` to
`UMPIRES` (Random), and settings.txt's `umpire` line falls back to Random when missing or bad. The TODO line's
"umpire 4" was stale.

**Online lobby:** the host's menu starts on the same saved/Random umpire. When the host opens the lobby,
`online_open` throws `roll` once, and both the `Room` it sends and Start's `Cmd::Venue` resolve the court/umpire
through `Menu::venue` with that roll. So every guest gets the host's concrete umpire and the match seed isn't
needed for it. The `Handshake` compares the resolved `args.umpire` (its test fixture's `umpire: 4` is a resolved
value, not a default), so there's nothing to change there.

**Change:** `online_flow_host_and_code` now asserts: the host menu starts on Random court and umpire; the hosted
`Room` carries a resolved pair; and Start sends that same pair.

## Not verified / not 1:1
- A guest's lobby settings row shows the host's resolved umpire (and court), while the host's row shows "Random".
  `Room` carries only resolved values. This matches how the court already behaved, and front-end design is ours,
  so I left it as is.
- `--umpire` with no flag (direct `--play` runs, not the menu) still defaults to 4 (`main.rs`). That's the CLI
  default, outside the menu/lobby this task covers.

## Why it isn't ticked
The work is done and the menu tests pass (106). The full `tools/check.sh` fails on one unrelated test:
`play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally`. `mods/` (shared with the main checkout)
gained a store install of `hst_umpire_04_anna` at 22:39 today, and `mods/hst-mods/` has it too, so
`mods::list` returns Anna twice. I filed that as B380. Once B380 lands or that duplicate is removed, re-run
`tools/done.sh B305 …` to tick this.
