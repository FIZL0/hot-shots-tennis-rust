# B40h four players, any device on any seat (2026-10-08)

Code: `play/controls.rs` (`Seat`, `PadSlots` ×4, `PadSlots::keys`), `play.rs` (`Pads` ×4 slots + `seats`, `slot_of`,
`read_input` routes the keyboard to its seat), `main.rs` (`--pads a,b,c,d`), `play/main_menu.rs` (assignment).

- `--pads a,b,c,d`: seats 1P..4P, each `k` (keyboard), a gamepad index (sorted, Steam virtual pads skipped, as before)
  or `-` (computer); missing entries are the computer. Slot s = (s+1)P drives match player s in singles, in doubles
  1P→0, 2P→2, 3P→1, 4P→3 (B40's order). With seats, who is human is fixed for the match (sides never change mid-match,
  as the original), even while a pad is unplugged; a seat's pad is re-taken first by its own index, then any free pad
  no other empty seat is waiting for.
- Without `--pads` (dev `--play`): keyboard + pad 1 drive 1P as before; slot s > 0 is human while s+1 pads are in
  (so 3 and 4 pads make the other team human).
- Menu: any device (keyboard included) joins with ✕/Enter into the first free seat, ←/→ moves it to the next free
  seat (1P stays the host's), ○ leaves; the select already gave every seated human its own cursor.
- HUD/markers/stats/pause already label by slot (0..3 → 1P..4P, 4 COM): no change needed.

## Checked
- Tests: `controls::seated_pads` (four seats, keyboard as 3P, unplug/replug), `main_menu::four_humans_any_device_any_seat`
  (pad host, keyboard 2P→4P, three pads, all ready, `--pads 1,2,0,k`; singles keyboard 2P),
  `locked_costumes_and_match_order` (`k,1,-,-`).
- Shots: `context/b40h/four_8.png` (`--pads 0,1,2,k` doubles: markers and panel 1P–4P), `singles_k2p.png`
  (`--pads -,k`: COM 1P, 2P human, camera behind 2P).
- `mods::tests::rerigged_mod_plays_forehand_and_run` fails, and fails the same on HEAD before this change (not B40h).

## Not verified / not 1:1
- No real keyboard/pad input was driven into a seated match (no 3–4 physical pads here): the keyboard → seat routing is
  covered by `PadSlots::keys` tests only, and the camera/pause/stats keys stay global (any device), as before.
- Pad numbering across the menu and match processes is still B40i (10).
- With several humans the camera follows the first human (lowest match player), as before; the original's split of
  camera ownership with 3–4 humans was not checked in PCSX2 (multitap).
- The seat-moving input (←/→ on the assignment screen) is remaster-only; the original seats by multitap port.
