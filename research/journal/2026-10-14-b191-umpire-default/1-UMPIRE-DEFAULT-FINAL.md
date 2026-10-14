# B191 — umpire defaults to Random, last pick remembered

- `Settings` got a 7th field: the confirm screen's last umpire pick (`UMPIRES` = Random), written as
  `umpire = N` in `settings.txt` (localStorage on web through `crate::store`, same as every other setting).
- Parse: a missing line, a non-number or a value > `UMPIRES` → Random.
- `Menu::default()` starts on Random; the menu load copies the saved pick into `Menu::umpire`.
- ✕ in the umpire picker saves (`SaveSettings`) when the pick changed. ○ puts the old value back and saves nothing.
- The pause menu reads and writes the whole `Settings`, so the line survives a save from mid-match.
- The sim RNG isn't involved: a Random umpire is still thrown by the menu's own `roll` in `venue()` at Start.
- Tests: `random_court_and_umpire` (default Random → pick 4 saved → Random saved; fallback for "", 9, "lily"),
  `locked_costumes_and_match_order` (the default `--umpire` is a thrown 0..5).

## Not verified / not 1:1
- Not checked in a browser. The web build uses the same `store::write` path as the other settings, so it should work.
- Not compared with the original: the front-end is ours to design (PLAN.md), and the original has no saved umpire.
