# B553 — online: after a match, back to the lobby, not the home screen

## Cause
The hand-over itself was in place and tested: the result screen's Continue → `netplay::again` (host sends
`Msg::Lobby`, `LOBBY` holds the connection and room) → the match mode's way back (`main_menu/online.rs` `launch`)
puts a `Kept` menu on `Screen::Lobby` with the `Net` back into the menu app. The menu's `setup`
(`crates/hst/src/play/main_menu.rs`) then overwrote `menu.screen` from `HST_MENU` on every start, kept menu or not:
empty → `Screen::Main`, so `!fresh && screen != Lobby` ran `after_match` (home screen, Local highlighted). The
connection stayed alive under a home screen. Even with the screen kept, the next branch (`else if screen != Main`)
would have filled the kept menu with the `--shot` sample seats and `online::sample`'s demo lobby.

## Fix
`setup`: a kept menu on the lobby (`!fresh && screen == Lobby`) stays on the lobby whatever `HST_MENU` says, and
skips the sample-state branch. Local matches' way back still goes to the home screen.

Test: `play::main_menu::tests::an_online_match_comes_back_to_its_lobby` runs `setup` with such a kept menu: lobby,
room, `me`, status and seats kept; a kept non-lobby menu still lands on Main. Fails on the old code (Main ≠ Lobby).
The net side (`Msg::Lobby` relay, the ready/countdown after it) and the menu's late-`Msg::Lobby` hold for a guest
were already covered (`net.rs` lobby tests, `online.rs` 4096, `netplay::the_result_screen_leads_back_to_the_lobby`).

## Not verified / not 1:1
- No live two-machine run (host + guest windows playing a whole match over the network) in this unattended slot: the
  fix is checked through the menu's own `setup` with the exact kept menu the way back inserts. Front-end, so no
  match to the original needed.
