# B264 — Random seats stay Random until the match

Bug: on the character select a seat on Random showed its ? card only while it was picking (`picking && random &&
!ready`). A ready one (a local COM readied on Random, or online the host's Random computers, which `align` marks
ready) fell through to the 3D preview / name of whoever it drew — or what its preview slot last had. The confirm
screen showed the drawn character's portrait, style and rank.

Fix (`play/main_menu.rs` only, draw code):
- select panel: `random && (picking || ready)` draws the ? tile + "Random"; ready → smaller tile, "Ready to go!".
- confirm screen: a `players[p].random` seat draws the ? tile and "Random" in place of portrait, play style and the
  COM rank pill (the rank would hint at the draw); 1P/COM tags as before.
- `HST_MENU=coms-random` / `confirm-random`: 2P's computer readied on Random, for shots
  (`context/b264/*.png` checked by eye).

## Not verified / not 1:1
- The original has no Random card (B217 is ours), so nothing to compare against; menus are ours to design.
- Online path not run with two machines: it relies on `align` already carrying `pick.random` into `players[i].random`.
