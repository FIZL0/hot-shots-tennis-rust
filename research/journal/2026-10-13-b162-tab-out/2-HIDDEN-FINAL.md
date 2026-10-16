# B162 (2) — a player who switched away holds the match instead of being dropped

## What was still wrong (user 2026-10-16: "tabbing out shows 'waiting for player to come back'")
B196's `play/recover.rs` hears a peer through its frame loop's keep-alive (`Msg::Alive`, every 0.5 s). A hidden tab
(B260: no frames at all during a match) or a minimised window stops that loop, so after `SILENT` = 3 s the host marked
the player away: "Waiting for X to come back... 20 s", then the bot/lobby question — a 30 s or 2 min tab-out ended
the match. (A host tabbing out ended it for the guests after 13 s.)

## Fix
- `hst-base/src/net/keeper.rs` (new): `Keeper`, a per-match task off the frame loop (a thread natively; a timer on
  the page's event loop in the browser, which the browser throttles to ~1/s while hidden — enough). Each frame stamps
  it (`recover::keep`, called from `netplay::watch`) with the window's focus. When no frame has come for 1 s *and* the
  player switched away (window unfocused at its last frame; in the browser also `document.hidden` / `!hasFocus()`),
  it sends `Msg::Hidden(me)` about once a second. A loop stopped with the window in front (a hang) sends nothing, so
  B196's silent-peer path still covers crashes and freezes.
- `net.rs`: `Msg::Hidden` (a match message, relayed to every peer), `Net::keeper`.
- `play/recover.rs`: `Recover::hidden` (peer → seconds since it switched away; `Hidden` adds, its frame loop's next
  `Alive` removes). A hidden peer isn't away: everyone (host and guests, a hidden host too) shows
  "X switched away from the game. Waiting for them to come back... M:SS" while the match stalls on its missing pads
  (rollback at its window, lockstep at once) — paused for everyone, nothing in the sim or the rollback inputs changes.
  It turns away (B196's 20 s wait, then bot/lobby) only after `LONG` = 5 min, or if even its keeper goes quiet for
  `HIDDEN_QUIET` = 75 s (Chrome's intensive throttling of hidden tabs, 1 timer/min, starts after 5 min). A host
  switched away for 5 min ends the match for the guests ("host switched away for 5 minutes. The match is over.").
- Back, the peer's frames resume (first frame's dt already clamped; B299), its `Alive` clears the notice and the
  existing catch-up (`rollback_survives_a_tab_out`) brings it in step.
- `web/index.html`: the B260 comment says what a match does while hidden now.

## Verified
- `net::keeper::tests::speaks_only_for_a_player_switched_away` (real threads): frames going on (focused or not) or
  stopped in front: nothing; stopped unfocused: `Hidden` each interval; dropped: silent.
- `play::netplay::tests::matches` (three rollback peers, `HST_NETPLAY=1`): `a_player_switched_away_comes_back`
  (guest, 30 s: notice "g switched away… 4:40" on both others, no question, no end, then 600+ ticks in step with the
  same pads/checksums, no restart), `the_host_switched_away_comes_back` (host, 30 s; before: ended at 13 s),
  `a_player_switched_away_for_2_min_comes_back` (ignored, ~40 s: the task's 2 min), `a_player_switched_away_too_long_is_away`
  (forced to `LONG`: B196's wait takes over). The harness drops what a dead peer would have heard (stricter than a
  real hidden tab, whose inbox keeps filling).
- Still pass: `silent` tests (B196), `a_silent_peer_rejoins_and_a_silent_host_ends_it` (ignored), `cargo check -p
  hst-base --target wasm32-unknown-unknown`.

## Not verified / not 1:1
- No live two-instance run (browser+desktop, browser+browser, 5 s / 30 s / 2 min): an unattended run can't drive two
  lobbies and switch a real tab away. The keeper's browser path (`document.hidden`, the throttled timer reaching
  iroh's send while hidden) is compile-checked and reasoned, not observed; worth one manual check with a friend.
- Desktop minimise on Wayland/Windows: relies on winit's focus-lost reaching one last frame before frames stop
  (the stamp). Not observed. If a compositor stops frames before that, the player is treated as silent (old path).
- An unfocused window whose loop stalls >1 s (e.g. compiling pipelines) shows "switched away" briefly on the others.
- Online play has no original to compare against (remaster addition); no sim/rollback/snapshot code changed.
