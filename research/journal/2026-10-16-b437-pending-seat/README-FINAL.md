# B437 — Controllers screen: 1P's own seat move applies on leaving

Before: on the controller assignment the screen's driver was simply `seats[0]`, so 1P's ←/→ moving themself off 1P
handed the cursor to whichever device landed on 1P, and they couldn't switch again.

Now (`main_menu.rs`):
- `Menu::assign_lead`: set by 1P's ←/→ swap; `Menu::lead()` is that device while on `Screen::Assign`, else `seats[0]`.
  `step`'s `host`, the legend (`legend.rs`) and the pointer's device (`pointer.rs`) use `lead()`. The field is
  cleared on the first step off the assignment, so the new 1P drives from the next screen (character select; in a
  match the pause menu's Controllers hands the seats to `PadSlots` as before).
- The device moved onto 1P neither moves nor leaves (its ←/→ and ○ do nothing, legend shows nothing for it); the
  driver can still take it off.
- The driver's Back returns with itself as 1P (the other seats freed as before).
- Shown on screen: "Yours when you leave" under the driver's new seat, and a "Pending: you move to NP when you
  leave…" line in front of the help (or "you give up your seat…" if it also took itself off).
- Tab / ✕ taking 1P off without a move first keeps B238's immediate hand-over (its own feature and test).
- `HST_MENU=assign-moved` shows the pending state (shot: `context/B437/assign-moved2.png`).

Tests: `pause::reassign_in_a_match` (keeps switching, other pad can't act, leaving applies) and
`dropping_1p_moves_the_next_device_up` rewritten for the new rule.

`play::court_mod_match::a_court_mod_is_seen_and_heard_and_never_moves_the_match` fails in this worktree
("0 of the mod's 0 people placed") with or without this change; it touches no menu code (M8f7 data/environment).

## Not verified / not 1:1
- Front-end screen, ours by design (PLAN: menus don't have to match the original); nothing to compare against PCSX2.
- Not driven with a real second pad; covered by unit tests through `Menu::step` and one `--shot`.
