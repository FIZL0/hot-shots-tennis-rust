# B159 — Enter finishes text entry

Bug: in the character-grid entry (Name, Join by Code) the keyboard's Enter fell through to the menu's OK and typed the
hovered grid key.

Fix (`play/main_menu/online.rs` `typing`, runs before the menu step): a pressed `Key::Enter` (main and keypad both
send it) moves the cursor onto Save / Join, so the menu's OK in the same frame takes that button: the name saves, a
full code joins, an incomplete code just buzzes on Join. `main_menu.rs`: keypad Enter now also counts as the keyboard's
OK everywhere in the menus. The pad's ✕ still picks the hovered key (typing only sees keyboard events).

Test: `online::tests::enter_finishes_entry` — "SUNNY" with the cursor on A, Enter and NumpadEnter each put the cursor
on Save; OK then saves "SUNNY" (no A) and closes the entry.

## Not verified / not 1:1
- Not checked in a running window (no driving of the keyboard in the app); the test runs the real `typing` system on
  synthetic `KeyboardInput` messages. The original has no keyboard entry, so nothing to compare against.
- Browser on-screen keyboard (`web.rs` `typed()`) isn't touched: it sends characters only.
