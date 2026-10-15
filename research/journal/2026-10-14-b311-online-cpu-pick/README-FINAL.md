# B311 — an online computer player added without picking isn't Random

**Cause.** In the host's lobby, ✕ on an open seat added a computer player on a Random draw (B217) and sent that
pick to the lobby at once, then opened the select. Backing out of the select (○), or the match starting while it
was unreadied, kept the Random pick.

**Fix** (`play/main_menu/online.rs`, `bot_added` / `bot_default`). The pick sent when the computer player is
added is now:
- the cell the last pick readied on (B269's `Menu::last`): that character on the first costume no other lobby
  seat holds (a mod: the menu's own free-costume check);
- a Random draw only if that last ready was on the Random card;
- nothing readied yet: the default an open seat gets at Start (`Room::bots`): the first disc character in number
  order nobody holds, on its first free costume.

`edit_bot` puts the select on the costume the lobby got, so the two agree. `net.rs` needed no change: its
open-seat fill at Start was already first-free-character, not Random.

Test: `an_added_computer_keeps_the_last_or_default_pick` (`main_menu/online.rs`) covers all three cases and ○ back
to the lobby.

## Not verified / not 1:1
- Front-end behaviour, ours to design (PLAN.md), so not checked against the original. No `--shot`: nothing
  changes on screen beyond which pick the card shows. The unit test covers it.
