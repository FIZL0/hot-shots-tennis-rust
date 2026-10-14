# B331: Singles/Doubles and joining on the roster (FINAL)

User 2026-10-14: "go ahead and implement it" (no concept artifact).

- Local (and the Mods page's Play with) goes straight to the roster (`Menu::local_start`): 1P is the device that
  chose it, every other seat a computer player.
- A device not seated presses ✕ on the roster: it takes the first seat held open (B238) or, on a local roster, the
  first computer seat (`join_seat`). Its legend shows "Join"; an empty card says "CPU / Press X / Enter to join".
- ○ off a seat on a local roster gives it back to the computer (was: held open, which blocked Start).
- □ / Tab (pad West) or a tap on the Singles/Doubles bar swaps the mode (`swap_mode`): singles' 2P becomes doubles'
  3P (still across the net); back to singles keeps 1P and the first other human (opponents first). Every card
  unreadies; picks follow their seats.
- 1P's ○ on the roster (not ready) goes back to the main menu.
- Mode and Assign screens stay for online hosting/joining and the pause menu's seat change (B254).
- Same session: Mods opens on Browse, now the first tab (Browse, Installed, Upload, My uploads).

Shot: `context/b331/chars4b.png` (HST_MENU=chars4 via an HST_BIN wrapper).
