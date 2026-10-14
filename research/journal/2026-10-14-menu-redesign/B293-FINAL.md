# B293 — pause menu shows the court and umpire

The only change to the pause menu (user 2026-10-14). The top bar's right end now carries the match's court name
(the rules' yellow, e.g. "Nakagawa River Courts") on the bar above its rim and "Umpire <name>" (white, a little
smaller) just under it. Set with the dialog's own `word.tm2` labels (`main_menu/pause.rs label`), names from
`message0.dat` as the Confirm screen reads them (`pause::venue`: court `276 + 2k`/`277 + 2k` for `--stage`/`--court`
1–11, umpire `669 + 2u` for `--umpire`); a viewer court outside 1–11 leaves the court line empty. Pinned to the
window's right (`Anchor::Sides`), so at 16:9 or wider it stays under the bar's right end, scaled uniformly.

Presentation only: textures built at setup and two quads in `menu.rs layout`; no sim, RNG or timing touched.

Checked: `HST_PAUSE=6 --play --stage 9 --court 9 --umpire 2 --shot-at 8` at 1280×720 and `--stage 4 --umpire 0` at
960×720 → `context/b293/pause_16x9.png`, `pause_4x3.png`. Test: `menu::tests::slot3_pause` (positions), `art_order`.
