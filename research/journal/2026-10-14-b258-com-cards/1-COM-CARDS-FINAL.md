# B258 — pick the COM's character in a solo Local match

Before: a seat nobody joined (and no △ computer added on the assignment screen) was filled at Start with the first
character nobody had (Ashley when 1P took someone else), with no chance to change it.

Now (`play/main_menu.rs`):
- Every seat without a device is a COM card on the select (`Menu::bot`), the open ones included, not only the △-added
  ones (B216). Its label says "COM" (was "Open").
- Once every human is ready, the open seats' cards start on that same automatic pick (`preset_coms`, the old
  `fill_open` rule as `auto_com`), and the select stays up: the COM cards are picked one after another, like the
  original's 1P-picks-the-COM flow.
- Any ready human drives the current COM card (was 1P only): ✕ keeps the shown pick (so ✕ ✕ is the old behaviour),
  moving/costume/hand change it, ○ "Leave COM" puts the computer's automatic pick back and readies it.
- Last card ready → Confirm, as before. Confirm's ○ unreadies everyone (COM cards too) as before.
- △-added computers (B216) behave as before (cursor from the grid start, ○ goes back to the last readied card).

Tests: `an_open_seat_is_a_com_card` (singles: preset, Leave COM, picking another; doubles with two humans: both drive
the COM cards, picks distinct), `locked_costumes_and_match_order` (the open 4P now needs its ✕).

## Not verified / not 1:1
- Front-end only (PLAN: menus are ours), so no PCSX2 comparison. The original asks P1 to pick COM characters after its
  own; the automatic pick and ○ = Leave COM are ours.
- Not looked at in a `--shot`: the drawing only changed the "Open" label to "COM"; the card itself is the existing
  B216 computer card.
- Two humans share one cursor on the current COM card (whoever presses moves it); not split per human.
