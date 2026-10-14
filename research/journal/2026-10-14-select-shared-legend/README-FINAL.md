# B348 — character select: one shared hint bar

The select's legend (`play/main_menu/legend.rs`) built a group per seated device and only merged groups whose
items were identical. On the select the players' items nearly always differ by a label or two (1P "Back" vs 2P
"Leave", a ready player's "Cancel", an unseated device's "Join"), so every player got their own "1P …", "2P …"
group repeating Character / Costume / Hand / Ready along the bottom.

Now `groups` folds the select's groups into one untagged group (`shared`): every driving device's glyphs (keycaps
and pad buttons side by side as before, e.g. `Esc / ○`), each action once in the select's order
(□ page, move, costume, hand, ✕, ○, chat), and where players' labels for an action differ they're joined
("Back / Leave", "Ready / Join"). Seat assignment keeps its per-player groups (the host really does different
things there).

Test: `select_legend_is_one_shared_bar` (two humans, one or both ready: one untagged group, both devices, each
action once, every player's label in it). `legend_lists_what_each_screen_handles` still passes (it checks `acts`).

## Not verified / not 1:1
- No screenshot of the new bar: this run had no Wayland/Hyprland session (no socket under /run/user/1000), so
  `tools/shot.sh` couldn't open a window. Layout code is unchanged; the bar is the same strip with fewer chips.
- Front-end is ours to design (PLAN.md), so there's no original to match.
