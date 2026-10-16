# B495 — Find and Join by Code titled "Online"

The Lobbies (Find) and Code (Join by Code, also the relay-URL entry) screens drew `frame(0, 5)`, the
"Main Menu" title band. Both now draw `frame(8, 5)`: the blue "Online" band, same as the Online screen they
open from (`crates/hst/src/play/main_menu/online.rs`, `layout`).

Checked with `HST_MENU=lobbies` / `HST_MENU=code tools/shot.sh` (shots in `context/b495/`): blue "Online" tab
over the graph-paper wall, nothing else moved.

## Not verified / not 1:1
- None: the front-end is ours (PLAN.md); the original has no online screens to compare against.
