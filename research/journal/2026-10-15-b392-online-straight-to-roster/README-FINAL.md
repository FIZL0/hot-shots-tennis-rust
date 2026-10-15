# B392 — Online goes straight to the roster

Task: going online skips the character-assignment (controllers) and singles/doubles screens and goes straight to
the roster; the host gets the Singles/Doubles swap that used to live on those screens.

## What changed
- `online.rs` `Menu::online_enter`: Quickplay, Host, a lobby picked in Find, and a code (typed or pasted) all
  go straight to `online_open` with 1P alone (other seats, computer players, picks and scroll reset). The lobby's
  first line-up then opens its roster (`heard`, unchanged), the same way a local match goes straight to its roster (B331).
  Quickplay and Host start in singles. Find and Code take the mode from the lobby.
- A lobby link's start (`linked`, Assign screen with no device yet): the first press seats that device and joins
  straight away. The controllers screen no longer waits for a second ✕.
- Host's swap on the online roster: `online_swap` / `online_can_swap`. □ / Tab from the host's 1P device (or a
  tap on the top bar's mode pill, `roster.rs`) sends `Cmd::Doubles(!doubles)`. The room comes back and `align`
  re-seats the select. It works only while singles fits everyone seated (the lobby Mode item's `can_switch`); otherwise it plays the refuse sound.
  It is off for guests and while the host is picking for a computer player. The legend shows "Singles / Doubles"
  only when the swap is allowed.
- Tests: `online_flow_host_and_code` (Host → Lobby, singles; host □ → Doubles(true); guest/2P device/four seated
  don't swap), `join_by_typing_a_short_code` (code and pasted link → Lobby; link start's first press joins),
  `find_first_then_pick_in_the_lobby` (Find's pick → Lobby).

## Not verified / not 1:1
- Bringing a second local player online: this machine now enters every lobby with one player. The lobby protocol
  fixes a peer's seat count at join (`Room::take` / `repick`), so a pad joining on the online roster with ✕ needs a
  net change. That is filed as B405. Until then, local co-op online is unavailable.
- Quickplay now always looks for singles lobbies (`quick_pick` filters by mode) and hosts singles if none fit.
  The host can switch to doubles from the roster.
- Front-end only (PLAN: menus are ours to design): checked with unit tests and one `--shot` of the online roster
  (`context/b392/`). The sample's three seats in doubles correctly hide the swap. No PCSX2 comparison is needed.
