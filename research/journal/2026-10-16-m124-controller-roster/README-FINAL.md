# M124 — Controller select shows the match roster

Task: the Assign screen (menu and pause "Controllers") shows each seat's character card; characters can't change
there. Local: 1P can leave, swap players between seats, remove players. Online: only swap your own pads between
your local players.

## What changed
- `main_menu/assign.rs` (new): the Assign screen drawn as one card per seat (one row; 2×2 for four seats in a view
  taller than wide). Each card: band with seat number, character and controller (Keyboard / pad / No controller / COM /
  Online), Near/Far, red ✕ to take it off when `droppable`; face, name, hand + costume; play style and the five
  grades; a footer with the seat's status pill, ‹ › swap buttons on the focused card (`host_swap`), + / − COM
  (`can_bot`), "Yours when you leave" (B437 pending) or a join hint. Title tap = back, Done button = ok, the whole
  card is a pointer row. Cards end 34 units over the view's bottom, so they fill tall windows down to the bar.
- `main_menu.rs`: `Menu::net_pause` (the seats another machine plays in an online pause). With it set nothing is
  droppable and the lead is the first local seat (`first_seat`), used by `lead`, `lead_pending`, `host_swap`.
- `pause.rs`: the overlay loads the faces/attrs sheets and the disc's grades, styles and hands; `opened()` fills
  `menu.players` from the match (`Game.chars`, `Args.outfits`, `MatchMod`, `SwapHands`, seat order 0 2 1 3 in
  doubles) via `roster_from_match`, and online sets `bots = !mine` and `net_pause` from `Netplay.lock.mine`.
  The overlay's quad pool went 1400 → 2600 (four cards are ~2000; `assign::tests::four_cards_fit_the_pause_pool`).
- `menu.rs`: the pause dialog offers "Controllers" online too when this machine plays two or more seats.
- Swapping online only moves this machine's input between its own seats (`PadSlots`), never the sim's seats, so
  the lockstep stream is unchanged.

## Checked
- Tests: `assign::tests::{cards_fit_any_aspect, kinds, four_cards_fit_the_pause_pool}`,
  `pause::tests::online_swaps_only_its_own_seats` (lead = first local seat, → swaps across the remote seat, no
  drop, no joining a remote seat, cards = match characters/costumes/hand swap), existing `reassign_in_a_match`,
  menu/legend tests (`-p hst main_menu`, `-p hst menu`).
- Shots (`HST_PAUSE=4 HST_PAUSE_SETTINGS=assign --play --shot-at 10`), `context/m124/run1/`: `doubles3.png` 16:9,
  `tall2.png` 600×1000, `singles_wide.png` 8:3 — checked by eye; Carol/Will show Left. First shots found the 4th
  card blank (pool too small) → pool raised.

## Not verified / not 1:1
- Not 1:1 with the original: the original has no such roster screen; menus are ours (PLAN.md).
- Online Controllers not run over a real two-machine netplay session: only the unit test of the seat rules; the
  input path is the same `PadSlots` the local pause already uses.
- A mod character's grades on the pause card are its base row's disc grades (no per-cell override like the
  select's `mod_grades`), and a mod's face isn't drawn in the pause overlay (no mod icons loaded there).
- The main menu's own Assign screen isn't reachable in a normal local flow, so its card view ("Not picked yet") was
  only checked through tests, not a shot.
- Player (profile) names aren't shown on the pause cards: the match doesn't keep them.
