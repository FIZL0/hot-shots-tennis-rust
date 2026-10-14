# B353 — B348's shared select hint bar, checked by eye

`tools/shot.sh` of the character select (`HST_MENU=…` passed through an `HST_BIN` wrapper script, since
`hyprctl exec` drops the environment) at 960×720 (4:3), 1280×720 (16:9) and 1680×720 (21:9), `--shot-at 4`.
Shots and 2× crops of the bottom strip in `context/b353/run2` … `run4`.

| sample | humans | devices |
|---|---|---|
| `chars` | 1 (2P–4P CPU) | keyboard |
| `chars2` (new) | 2: 1P keyboard, 2P a pad, ready on Carol | keyboard + pad |
| `chars4` | 4 | keyboard ×4 |
| `chars4-pads` (new) | 4: 1P keyboard, 2P–4P pads | keyboard + pads |

Added `chars2` and `chars4-pads` to the menu samples (`play/main_menu.rs`): there was no 2-human sample, and the
existing ones only seat the keyboard, so the bar never showed the wider keycap + pad-button chips.

Result: in every case the panels carry no hints and one untagged bar at the bottom right lists each action once
(Singles / Doubles, Character, Costume, Hand, Ready, Back, Stats), with labels joined where players differ
("Back / Cancel" with 2P ready, "Back / Leave" with 4 humans) and both devices' glyphs side by side
(`Esc K / B`). The widest case (mixed devices at 4:3) fits the strip at full size, right-aligned under the
description bar; nothing clipped or overlapping at 16:9 or 21:9 either. Nothing to fix.

## Not verified / not 1:1
- The pads in the new samples are placeholder entities, not real gamepads: their panels say "Controller 0" and
  their glyphs use the Xbox style (vendor unknown). PS/Deck glyph widths weren't shot; they're the same chip sizes.
- Front-end is ours to design (PLAN.md): no original to compare with.
