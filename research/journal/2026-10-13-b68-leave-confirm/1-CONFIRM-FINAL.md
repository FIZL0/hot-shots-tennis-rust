# B68 — confirm before leaving a scored match

Ours (the original's "Return to the Menu" fades out at once), `crates/hst/src/play/menu.rs`.

- `scored(&Score)`: any point, game or set so far (`points`, `games`, `sets`, `set`). `step` copies it into the menu
  each frame.
- Choosing "Return to the Menu" with nothing scored fades out as before. With a score it puts the same dialog into
  ask mode: its rows become "Leave the match?" (white, no pill), "Yes", "No" (pills, chosen one bright, the hand
  `hsm_yubi` swinging beside it). No is chosen first. The same input timing as the menu: up/down with the 32/8
  repeat (sound 0). ✕ chooses
  (sound 2), ○ / Backspace / K or Start count as No (sound 1, the main menu's back sound). No goes back to the pause
  menu on "Return to the Menu"; Yes starts the 60-tick fade (then quits, as before). Clicking or tapping a pill works.
- Text: `pause::label` (word.tm2, or the remastered font), the same as the other rows; "?" is in the font.
- Test: `play::menu::tests::confirm_return` (0-0 quits; after a point it asks; No/○ goes back; Yes fades; the drawn
  rows and the hand's position).

## Not verified / not 1:1
- The whole confirm is ours (the original has none), so there's nothing in the original to check it against. It uses
  the pause dialog's box, pills, hand, sounds and input timing.
- I didn't take a `--shot` of it open. The layout test pins where every quad goes, and the rows use the same frame
  and pills as the menu that B141/B171 already checked by eye.
- Sound 1 for ○/No comes from the main menu's back sound. The original's pause screen ignores ○, so it has no
  cancel sound to copy.
