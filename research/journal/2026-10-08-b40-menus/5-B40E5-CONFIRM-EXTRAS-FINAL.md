# B40e5: the confirm screen's extras (2026-10-12)

Original walked on PCSX2 copy 5 (save slot 2 sits on the doubles character select: four ✕ presses → confirm; saved
to scratch slot 9). Shots: `context/b40e5/run1/d0.png` (confirm), `run1/e.png` and `run2/` (court picker, all 11
courts), port: `context/b40e5/port1_s.png` (`HST_MENU=confirm`). Walker: `context/b40e5/walk.py`.

## What the original shows
- Per player: play style title above the tag, the tag (1P / 2P COM …), and for a computer a pill in the player's colour
  with its **Tennis Lv** title (`Confirmation_04`: 17 titles, nine to a column: Lv 5 … Lv 1, Hopeful, Tier II–V,
  Master, Elite, Guru, Legend, Hero, King, Superstar). The title is the AIParam.csv row's rank (`AiParams::rank`):
  Jun and Cody rank 0 → "Tennis Lv 5", JJ rank 1 → "Tennis Lv 4", as the screen showed (doubles rows, costume 1).
- Top right: "Umpire" (`Confirmation_03`), the round face (`Confirmation_00`, 46×44 at x 63 + 48u, y 272), the name.
- Under the court name: the surface title (`Confirmation_01`: Grass / Clay / Hard, tinted) and two ball meters: a lob
  arc (bounce height) and a dashed line (ball speed), out of 4 (`Confirmation_00` lit/dark balls at (72,192)/(72,216)).
  Over all 11 courts the meters depend only on the surface: grass 2/4, clay 3/2, hard 4/3. Surfaces by court k
  (message 276 + 2k order): clay 0 1 2 8, hard 3 6 10, grass 4 5 7 9, which is also what `hst_sim::ball::COURTS`
  plays (spin_kick 0.003 = grass, spin_relax 0.1 = clay).
- The original's court picker has no random entry (11 courts, wraps), though `Confirmation_02` has a "?" tile.

## Port (`play/main_menu.rs`)
- Style title over the portrait's foot; a COM's tag is "2P COM" with the Lv pill under it (rank from
  `Choice::new(menu_row(char, costume), char, doubles).row`, a mod by its `ai_row`).
- Umpire block as the original; court row adds the surface title and both meters.
- Random court / umpire (ours): one more value after the last court / umpire; "Random Court" / the "?" tile + "Random".
  Pressing Start throws `Menu::roll` (std's per-call hasher keys, never the match RNG), `Menu::venue` resolves both for
  `--court/--stage/--umpire` and the online lobby's `Room`. The menu keeps "Random" for the next match.
- Set Handicap / Offbeat Rules: still not on the screen; the user deferred both to stretch (B40g1/B40g2, `2-B40G-DEFERRED.md`).
- Test: `random_court_and_umpire`.

## Not verified / not 1:1
- Layout and sizes are ours (menus needn't be 1:1): positions, tints of the surface title and the pill, the 3D preview
  in place of the original's in-scene models, no court backdrop behind the players.
- The rank → title mapping is checked on three COMs only (Lv 5, Lv 5, Lv 4); higher titles (Tier, Master …) only follow
  the sheet's order, not seen on screen (they belong to the harder costume blocks).
- Random court / umpire isn't in the original; no original behaviour to match.
- Set Handicap / Offbeat Rules not added (deferred by the user).
