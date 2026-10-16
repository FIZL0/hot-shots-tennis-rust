# B523 — host kicks and moves players' cards in the online lobby

Built on the current lobby (B449 plates). M120's roster lobby is not on main yet (task/M120 holds an older
attempt; M120 itself is still open), so the controls attach to the lobby's plates. When M120 lands they carry over:
they're keyed on seats, not on how a card is drawn.

## What it does
- `crates/hst/src/play/main_menu/online/arrange.rs` (new): host-only controls, available once the host's own seat
  isn't Picking and no countdown is running.
  - ✕ on any other person's card (a guest's, the host's own 2P..4P; not the host's first seat) picks it up.
    Computers' cards aren't picked up (user 2026-10-16): their ✕ picks their character, as on the roster (B216);
    they can still be dropped onto (the person and the computer trade seats).
    While it's carried, the arrows move only between cards, ✕ drops it onto the focused card (swapping the two
    seats; an open seat means the player moves there), ✕ on the card itself puts it back, and ○ cancels.
  - △ on a guest's card, or the plate's corner X, opens a "Kick NAME?" prompt (Kick / Cancel, cursor starts on
    Cancel, ○ or a tap outside = Cancel). Kick sends the existing `Cmd::Kick`.
  - Computers: △ still removes at once (B216); ✕ is still Pick.
- `net.rs`: `Cmd::Swap(a, b)` is handled only in the host's loop (guests never act on it). `Room::can_swap` refuses:
  the same seat twice, two open seats, two seats of one peer, and any swap that changes a peer's seat order. Each
  peer's k-th player is its k-th seat (`live_of`, `repick`, `align_to`), so reordering would quietly hand one player
  another's pick. A refused drop says "NAME's players keep their order: move the other one."
- The host applies a swap to its own room right away and keeps the cursor on the card where it landed; the echoed
  Room confirms it.
- Legend: Move (✕) and Kick (△) on a focused card; Drop / Put Back / Cancel while carrying; Choose / Cancel on the
  prompt. Guests' legends are unchanged.
- Drawing: the carried card is dimmed with a navy "Moving" pill; the focused target shows a gold "Swap",
  "Move here" or "Can't swap" pill. `rows_popup` height now follows its row count (4 rows still = 230).
- Shots: `HST_MENU=lobby-move`, `lobby-kick` (context/b523/: 1280×720 and 600×1000, both lay out).

## Tests
`arrange::tests::host_moves_and_kicks_cards` (carry onto an open seat, onto a player, refused same-peer, kick
confirm/cancel, a computer's card (✕ = Pick, not carried; a person dropped onto it), nothing before the host has picked or during
the countdown, guests can't), `net::tests::seats_swap_keeping_each_peers_order`, and the updated
`lobby_start_ready_kick_and_name` / `shared_select_and_host_bots`. netplay suite passed (HST_NETPLAY=1).

## Not verified / not 1:1
- Not tried live between two machines (host + guest over iroh). The swap path is the same broadcast as Bot/Doubles,
  and the unit tests cover the room logic.
- The host can't kick its own local 2P..4P: there's no way to drop one local player from a lobby seat without
  reworking how devices bind to seats. Those cards can be moved.
- The host can't swap its own 1P with a computer (neither card is picked up); with a guest in, it works through
  them. Left as is per the user's call that computers aren't picked up.
- Computers are removed without a confirm (kept from B216). Only people get the prompt.
- No original reference: this is our own front-end (PLAN: menus are ours).
