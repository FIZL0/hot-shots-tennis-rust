# B540 — swapped P1 keeps menu control after the match

**Cause.** The front-end's `Menu` is cloned into the match's way out (`Kept`, inside the `Mode` closure built at
Launch) when the match starts. The pause menu's controller select (B254) only rewrote the match's `PadSlots`, so on
the way back the front-end came up with the seats and the splash's 1P (`Menu::p1`, B516) from before the match: the
original 1P pad drove the main screen and the next Local/Rematch seated players the old way.

**Fix.** `main_menu/pause.rs`: when the assignment's Start hands changed seats to the match it also records the
human seats' devices in a static (`SWAPPED`; the kept menu lives in a closure nothing else can reach).
`main_menu.rs` setup, back from a match (`!fresh`), calls `pause::seats_back`: the human seats take those devices,
the computer's stay, and `p1` becomes 1P's device (if 1P has one). The static is taken, so a later match with no
reassignment leaves the seats alone.

The result screen and the pause dialog already read any pad (no 1P gate), so nothing changed there.

**Test.** `pause::tests::swapped_seats_drive_the_front_end_after_the_match`: singles, keyboard 1P / pad a 2P,
traded over the pause menu; back on the front-end pad a is 1P and `p1`, drives the main screen, an unseated pad b
doesn't.

## Not verified / not 1:1
- Front-end only (ours to design, PLAN.md), no comparison against the original needed or done.
- Not tried by hand with real pads; covered by the unit test of the menu's state and input logic.
- If 1P took itself off in the pause assignment (no device on 1P), `p1` stays the splash's device; the B388/B516
  lost-1P rules take over from there.
