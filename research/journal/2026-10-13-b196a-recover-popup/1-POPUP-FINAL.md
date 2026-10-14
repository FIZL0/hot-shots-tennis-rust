# B196a — online recovery notices in the pause dialog's window

B196's notices (waiting for an away player, the bot-or-lobby question, "didn't come back" / "host stopped answering",
and every other online notice that used the plain `Say` text: stalls, desync notes, end reasons) now draw as a
pop-up like the pause dialog: dimmed court, our box (INK/WHITE/PANEL), the lines in the game's font outlined, and
while asking two pills — "A bot plays for them" / "Everyone to the lobby" — with the 3D hand (`cursor.ani`).

- `main_menu/pause.rs`: `OnlineNotice` resource, `notice_layout` (main menu's `Draw` helpers: `box3`, `pill_at`,
  `label`, `outlined`, `Tex::Cursor`), its own 1600-quad pool (`NoticeSlot`), placed with `widescreen::place/vplace`
  (any aspect: checked at 16:9, 1:1, 2.8:1). Mouse hover picks a pill, click/tap answers; sound 0 on a move.
  `HST_NOTICE=<text, | between lines>` (+ `HST_NOTICE_ASK=1`) shows one for `--shot`.
- `netplay.rs`: the `Say` text node is gone; `watch` fills `OnlineNotice` (the question resets to the bot pill).
- `recover.rs`: `ask_me` says this machine is being asked; ✕/Enter/tap answers the chosen pill (up/down or d-pad
  moves), ○/Backspace is still the lobby. The question's text no longer spells the buttons.
- Test: `a_bot_takes_a_silent_peers_seat` now also checks both remaining peers get the question (`ask_me`) after the
  wait and not before. Passed: `silent` (2), `a_bot_takes` (2), `a_silent_peer_rejoins…` (ignored, run).

Shots (context/b196a/): ask.png, ask_tall.png, ask_wide.png, wait.png, end.png, pause.png (pause menu, same match).

## Not verified / not 1:1
- Not shot from a real killed peer: the window was shot through `HST_NOTICE`; the kill → countdown → question on
  every remaining peer is covered by the headless netplay tests (same text and flag the window draws). A two-machine
  run wasn't done (unattended).
- Stick up/down doesn't move between the pills (keys and d-pad do); the window is hidden under the pause screen while
  it's open, as the plain text was. Ours by design (remaster-only UI), no original to match.
