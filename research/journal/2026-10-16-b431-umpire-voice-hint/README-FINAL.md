# B431: umpire select hints at the voice preview

Already delivered by B451 (its journal: "this covers B431"); nothing left to port.

- `crates/hst/src/play/main_menu/legend.rs` (Confirm, `picking`, row 4): the footer legend adds `△ Hear voice`
  only when `umpire_picker::axes(m).2`, i.e. `Menu::umpire_voiced(m.umpire)` — a disc umpire, or a mod that ships
  any of Preview Voice's calls (`umpire_mods::has_voice`). Random and a mod without its own calls get no hint
  (△ would play nothing / only the donor).
- Same legend row as every other button (the footer's own style, laid out by the legend at any aspect); the online
  lobby's Umpire pill opens the same picker on `Screen::Confirm` (`online.rs` `pick_venue`), so it shows there too.
- Test: `umpire_picker::tests::voice_only_where_there_is_one` checks hint ⇔ △ actually plays, for Random, disc
  umpires, a voiced mod and an unvoiced mod. Re-run 2026-10-16: passes.

## Not verified / not 1:1
- None: the umpire select is front-end (ours to design); the original has no mod umpires to compare against.
