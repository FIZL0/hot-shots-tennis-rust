# B238: ✕ to drop a device from its seat

- Join step (`Screen::Assign`, local and online's take-your-controllers step): every joined device but the menu's
  driver (this machine's first seat) has a red X button on its row's right end. A tap/click on it (`pointer::Act::Drop`)
  frees the seat; ○ from the device itself still does too. Another device's ✕ joins the free seat as before.
- Character select (local and online): the same X on each joined card's top-right corner. Dropping holds the seat
  open (`Dev::Open`, a human's seat with no device: "X or Enter joins" on the card) instead of handing it to the
  computer, so online the lobby's held seats and `align` don't change. The next unseated device's ✕ takes it. A
  joined device's own ○ (not ready) also drops it. Locally 1P backing out to the assignment turns open seats free.
  An open seat is never ready, so the select can't go on to the confirm screen / online ready while one is open.
- Test: `main_menu::tests::a_dropped_device_frees_its_seat` (join two, ✕ one, another joins it; same on the select).
- Shots at 16:9 and 4:3 (`context/b238/`): the X sits on the card corners at both aspects.

## Not verified / not 1:1
- Remaster-only UI (no original equivalent), so nothing to compare against PCSX2.
- Online: no two-machine run; the open seat keeps this machine's seat count, so the lobby protocol is untouched.
- No touch-device run; touch uses the same hit rects as the mouse (B221).
