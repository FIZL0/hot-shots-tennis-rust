# 1 — chat over the peer connection

## What
- `net.rs`: `Msg::Chat { from, name, text }` on the existing peer stream (QUIC / browser RTC data channel, same JSON
  framing), never in the rollback input stream. Star topology: a guest sends its text with empty `from`/`name`; the
  host fills them from the sender's peer id and lobby name (a guest can't speak as someone else), cleans, rate-limits
  and forwards to every other seated peer, in the lobby loop (`host()`) and in the match (`relay()`). `Cmd::Chat`,
  `Event::Chat`; `Net::chats()` holds lines for the overlay, `Net::in_room()` says when chat is open to use.
- `chat.rs` (new plugin in the base app, so it lives across the menu and match modes): cleaning (control and bidi
  characters out, whitespace to spaces, 80 chars), 4 lines per 8 s per sender, overlay of the last 6 lines bottom-left
  fading after 12 s (2 s fade), over the HUD, no input taken unless the box is open. Keyboard: Enter opens/sends, Esc
  closes; `/mute NAME`, `/unmute NAME`. Pads: R2 opens and steps preset phrases (plus Mute/Unmute rows per sender),
  L2 says the picked one. Touch: tap the "Chat" button; in the browser the phone OS keyboard (`web::code_entry`).
  Font size in Vh/Vw like the fps counter, so any aspect, never stretched.
- Settings → Mods → Chat (on/off, `chat = off` in settings.txt) hides it.

## Checks
- `net::tests::chat_in_the_lobby_and_the_match` (3 peers, local testnet, relay off): lobby and mid-match lines reach
  everyone else with the right name, a spoofed `from`/`name` is overridden, a bell/newline is stripped, a burst of 8
  delivers 4.
- `chat::tests` (clean, rate limit), `settings_round_trip` (chat key).
- `synctest_reruns_every_tick_the_same` and `synctest_through_a_change_of_ends` pass (chat isn't in the sim).

## Not verified / not 1:1
- Native + browser crossplay (lobby and mid-match): not run, one machine and an unattended run with no browser to
  drive → PLAN *Needs the human*.
- The overlay wasn't screenshotted: it only shows inside an online room, which `--shot` can't reach alone.
- Browser LAN (WebRTC, M2k) path untested; it uses the same `Msg` framing so should carry chat unchanged.
- The phone keyboard reuses the code-entry input (autocapitalize = characters, covers the canvas while open).
- Enter opens chat while in an online room instead of confirming menus there (J/Space/○ still confirm).
- Mute is local to the player and by peer id; a muted peer's lines are dropped on this machine only.
- Not 1:1: the original has no chat; all of it is ours.
