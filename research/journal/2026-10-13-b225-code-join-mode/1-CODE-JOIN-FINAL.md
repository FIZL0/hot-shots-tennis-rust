# B225 — Join by Code no longer asks singles/doubles

- Join by Code (typed or pasted code/link) now goes straight to the controllers screen (`online.rs join_code`), skipping
  the singles/doubles choice. Up to 3 local players may take controllers; the lobby refuses more than its open seats
  ("not enough open seats"), as before.
- Every `Event::Room` sets `Menu::doubles` from the room, so the joiner's character select is the host's mode (and a
  mode vote is followed too). Lobby links (`linked`) already skipped the choice; they now allow up to 3 players too.
- Find already took the mode from the listing (B195), unchanged.
- Back on the controllers screen after a code join returns to the code entry (was: the mode screen).
- Test: `join_by_typing_a_short_code` (code → Assign, Back → Code, join → a singles room → 2-seat character select).

## Not verified / not 1:1
- Not tried across two real machines (needs the human's two-network setup, M2f); covered by the menu unit tests only.
- Front-end, so no 1:1 with the original required (PLAN.md).
