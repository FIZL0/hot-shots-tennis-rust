# B115 — Start in the online lobby

Most of it was already there from B151: the host's lobby has **Start** (labelled `Start +N COM` while seats are
open), which sends `Cmd::Start` once every seated guest is Ready; `net.rs` fills the open seats with `CPU` seats
(characters nobody picked) on every peer (test `net::tests` start-with-bots). Guests already saw every seat (name,
character, Ready/Waiting, `open`/`COM`).

What B115 added (`play/main_menu/online.rs`, one line in `play/main_menu.rs`'s plugin):

- **Guests are told the host can start**: the Ready hint and a guest's default line say the host starts when
  everyone in is ready and the open seats are played by the computer.
- **Mouse and touch in the lobby**: `online::pointer` maps a click/tap (through `pause::pointer_press`, so the
  same 4:3-in-any-aspect mapping as the pause settings) onto the item `layout` draws there: the mod question's rows,
  the host's guest seats (tap = take out), the bottom buttons (Start / Copy Link / Leave, or Ready / Leave); the
  right button is Back. It goes through `pause::Click`, now also a resource of the main menu, as the keyboard's ✕.
  Pad and keyboard were already wired (step's arrows + ✕/Enter).
- Test `online::tests::lobby_taps_hit_items`.

## Not verified / not 1:1

- Taps weren't driven on a real window/touch screen; `lobby_taps_hit_items` checks the hit boxes against the rects
  `layout` draws. Reason: an unattended run can't click or tap a window. The host → join → Start-with-COM flow is
  B151's (CLI instances + `short_code_on_a_testnet`) and B115 didn't change it.
- Shot of the guest lobby with the new hint: `context/b115/lobby-guest2.png` (`HST_MENU=lobby-guest`).
- The lobby's tab title reads "Confirm Settings" (from before B115, not touched here).
- The lobby is our own screen (no original to compare; front-end is ours per PLAN.md).
