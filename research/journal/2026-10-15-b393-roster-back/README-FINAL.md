# B393 — Back from the online roster goes to the screen it came from

## Problem
Online, OK on the controllers screen (Assign) opens/joins the lobby and the room's first line-up drops you straight
on the character select (`pick_next`, B195); the lobby screen (seat cards + court/sets/games/umpire settings row) is
never shown in between. Back (Esc / Backspace / ○ / the title's pointer hit) on that select sent you to
`Screen::Lobby`, a screen you'd never seen, which reads as a "match settings" screen. A probe test confirmed the
model path: Chars → Back → Lobby (sel on Seat(1)).

## Fix
- `View::first` (online.rs): set when the first line-up opens the select, cleared as soon as the lobby screen takes a
  press (`online_step`) and by `back_to_online`.
- Chars Back for 1P online with `first` → `Menu::chars_back`: sends `Cmd::Leave` (as the lobby's Leave), unreadies,
  clears the room state but keeps `net.go` / the code / the mode, back to Assign. OK there opens or joins again; Back
  there goes on to Mode / Code / Lobbies as before.
- A late `Event::Room` arriving on Assign (from the lobby just left) is ignored.
- Opened from the lobby (Characters button, a computer's seat card) Back still returns to the lobby.

Test: `online::tests::back_on_the_select_goes_where_it_came_from` (host: Chars → Back → Assign + Leave, a late Room
ignored, reopen, lobby, Characters → Back → Lobby; guest by code: Chars → Back → Assign → Back → Code).

## Not verified / not 1:1
- Front-end only (ours to design, PLAN.md): nothing to compare with the original; not checked against PCSX2.
- Not exercised over a real network: the Leave → Assign → reopen path is tested in the menu model only (the net side
  already handles `Cmd::Leave` as the lobby's Leave button does).
- Interpretation: "match settings screen" taken to be the lobby (its settings row) shown for the first time on Back;
  "correct previous screen" taken to be the controllers screen the lobby was opened from.
