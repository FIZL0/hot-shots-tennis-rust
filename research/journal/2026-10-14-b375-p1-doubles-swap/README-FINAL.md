# B375 — 1P can swap Singles ↔ Doubles again

B361 took the manual swap away from everyone on the local roster (mode followed the devices only). The user wants
1P to keep it: P1's □ (pad West) / Tab, or a tap on the Singles/Doubles bar, swaps the mode again.

- `Menu::can_swap(dev)`: local roster and `dev` is seat 0's device. Only that device's □/Tab swaps; other players'
  presses fall through as before (they still change the mode by joining/leaving, B361's automatic rules stay).
- Swap uses `swap_mode(true)`: humans' ready picks stay ready; 2P goes across the net to 3P and back.
- The mode bar is a tap target again (the mouse is the keyboard's device, so it swaps when the keyboard is 1P).
- Legend shows "Singles / Doubles" on □ for 1P only.
- Home's Local card says Doubles "1-4 players" again (1P can pick doubles alone, with COMs).

Tests: `one_roster` (1P swaps both ways, 2P's □ does nothing), `pointer` taps test (mode bar swaps). `check.sh -p hst main_menu`: 91 pass.

## Not verified / not 1:1
- No screenshot of the legend/mode bar: the change restores B331's drawing as it was, and front-end is ours to design (PLAN.md); unit tests cover the input.
- Going to singles by hand with 3–4 humans seated drops the extra devices (as B331 did); they can rejoin, which turns it back into doubles (B361).
