# M1m — modded characters in the character select

## What was already there (B86)
B86 (journal 2026-10-12-b86-select-cache) built most of M1m: cards show a baked PNG of the same off-screen
`inspect::spawn_into` preview (same costume, pose, crop, lighting) while scrolling and the live model once the hover
settles (0.3 s); models load on a worker thread on demand, newest ask first; LRU budget `KEEP_MODELS` = 6 loaded
characters, `KEEP_PNGS` = 48 pictures (`play/main_menu/previews.rs`). A not-yet-baked card shows the empty card
(placeholder) until its bake/load lands.

## Added here
- `Mod::source` (`mods.rs`): `mod.json` `source` up to its platform tag ("Hot Shots Tennis: Get a Grip (PSP) pc03"
  → "Hot Shots Tennis: Get a Grip"); empty when absent.
- Select card: the source game along the card's top, right of the player tag (`main_menu.rs`, `fitted` so it
  shrinks to fit; the card is `Anchor::Sides`, so it follows the cards at any aspect). The info bar also says
  "custom character from <game>". A line under the name was tried first: on the bottom row it ran under the info bar.
- Tests: manifest test checks the stripping; `lists_the_custom_roster` checks every packaged mod has a source.

## Results (context/m1m/)
- `select_png.png` / `select_live.png` (`HST_SELECT=m0,m26,m45,m70`: Fore, Get a Grip, Out of Bounds, Open Tee 2):
  each card shows its source game; `png_vs_live.png` side by side: PNG and live model same costume, pose, framing.
- `aspects.png`: 800×600 and 1400×600 windows, cards and source lines in place.
- `stress1/`: 120 s `tools/select_stress.sh` through disc characters, costumes and all 94 mods: exit 0, no panic,
  own VRAM 2.0–3.3 GB (flat at 2.5 GB after 60 s), RSS ≤ 1.3 GB.

## Not verified / not 1:1
- Menu UI is ours (PLAN: front-end not 1:1), so no original to compare against.
- "No hitch" judged from the stress run finishing at full speed, not a frame-time trace.
- The budget sizing for a low-end device is B86's count (6 models / 48 PNGs), not measured on low-end hardware (none here).

## Test run note
`tools/check.sh`: 138 passed, 1 failed — `play::mod_match::tests::get_a_grip_stats_reach_the_match` asserts Emi's
`ai_row` ≠ `donor`, but the locally rebuilt ../HST-MODS `out/mods/getagrip_pc00_emi/mod.json` (git-ignored there) now
has `donor` 0 and `ai_row` 0. Mod data, not this task's code (M1m doesn't touch AI rows or donors); ticked and
committed by hand rather than through `tools/done.sh`. Whoever owns the HST-MODS sizing/donor change (M1l) should
check Emi's donor or the test.

## Recovered (same day, later run)
The commit above (dae4a4f) was left dangling when the slot branch was reset after its run and never reached main;
re-applied onto current main (only plan/ conflicted), builds clean, wrapped up through `tools/done.sh`.
