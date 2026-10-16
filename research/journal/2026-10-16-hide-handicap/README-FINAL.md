# B482 — hide the Handicap cell on the confirm screen

The confirm screen's strip showed a dead HANDICAP "Off" cell. It's gone until Set Handicap (B40g1) works.

- `frame::spread(&mut Geo, n)`: re-lays the strip's first `n` cells by their shares across the whole strip, so
  the rest are zero wide. `confirm_geo` calls it with 4.
- `frame::strip` takes a slice of cells, and the last one gets the rounded right end (it was fixed at index 4).
- To bring Handicap back: drop the `spread` call in `confirm_geo` and add the Cell again in `confirm::draw`.

Checked: `tools/check.sh -p hst main_menu` passes. `HST_MENU=confirm` at 1280×720 and `HST_MENU=games` at
900×1200 (shots in `context/b482/`) show Court, Umpire, Sets and Games filling the strip, and the picked Games
cell's lit end is rounded and flush.

## Not verified / not 1:1
- none (this is our own front-end, so there's nothing in the original to match)
