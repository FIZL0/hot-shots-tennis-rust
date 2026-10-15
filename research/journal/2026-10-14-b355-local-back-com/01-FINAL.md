# B355 — local select: ○ after 1P's pick readied the CPU instead of going back

## Cause
Once 1P is ready on the local roster, 1P's device (the keyboard's on a phone; the title tab is its ○) drives the open
2P seat's COM card (`Menu::driven`). ○ there hit B258's branch "leave an open seat's COM card to the computer": it
readied the card on `auto_com`'s pick and went to Confirm — a pick for the CPU, not a step back.

## Fix
- `main_menu.rs` Chars ○: the B258 branch is gone; an open seat's COM card falls through to the computer-player branch
  that already existed for added bots: unready the last readied player (singles: 1P picks again; doubles: 4P's ○
  goes back to 3P's card). A second ○ on 1P's unready card leaves the select (B331) as before.
- `legend.rs`: the "Leave COM" hint is gone; the COM card now shows ○ "Back".
- Tests: `an_open_seat_is_a_com_card` updated (○ undoes, doubles ○ steps back to 3P); new
  `touch_back_after_1p_picks_undoes_it` drives the title tab's Back through `Menu::point` as a phone does.

To leave a COM card on the computer's own choice, ✕ keeps its preset pick (B346: 1P's character, next costume).

## Not verified / not 1:1
- Not tried on a real phone; the touch path is covered by the unit test through `Menu::point` with the drawn title-tab
  hit (the same act `pointer` sends). Front-end, so no original comparison needed (PLAN: menus are ours).
- The old ○ "computer's own pick" (first character nobody has) is no longer reachable from the select; ✕ keeps the
  preset pick instead. Intentional per the user's report.
