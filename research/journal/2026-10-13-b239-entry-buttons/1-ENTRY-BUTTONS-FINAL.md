# B239: text entry pad buttons

On the on-screen keyboard (`Screen::Code`: Join by Code and name entry), `online.rs` `online_step`:

- □ (pad West, the menu's `Press::page`) deletes the last character (sound 1, or 0 if empty).
- ○ (pad East, `Press::back`) leaves the entry at once (before: it deleted while there was text, only leaving
  when empty). Esc is `back` too, so it also leaves.
- Backspace keeps deleting: `typing` pops and clears its just-pressed, so it never reaches `back`.

The menu reads pad buttons by position (`GamepadButton::West/East`) for every pad, so it's the same on any
controller layout (Xbox X/B, PlayStation □/○, Switch-style pads as gilrs maps them by position).

Test: `join_by_typing_a_short_code` now checks □ removes one character and ○ leaves straight to the Online menu.

## Not verified / not 1:1
- Front-end, ours by design (PLAN.md): no comparison with the original needed.
- The keyboard's Tab (also `Press::page`) now deletes on this screen too; harmless, left as is.
- The on-screen hint text doesn't name the buttons; that's M14's legend.
- B233 (cursor stuck on E) not touched here: its own task.
