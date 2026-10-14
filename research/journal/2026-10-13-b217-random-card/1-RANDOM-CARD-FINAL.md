# B217 — Random in the character select

- A Random card (the confirm screen's "?" court tile, `RANDOM_TILE`) sits in the intermediate group's empty
  bottom-right corner (`RANDOM_CELL`, 435,295), cell `CHARS` of `grid_step`: ↓ from Ken (the top row's right end via
  Kyoko), → from the cluster's bottom-right, tap/hover too (`pointer.rs` `Act::Cell(CHARS)`). Disc page only; Tab
  clears it for players not ready.
- `Player::random`: while hovered and not ready the player's card shows the ? tile and "Random"; ✕ draws the pick
  (`Menu::draw` / `roll_random`): a character uniformly among every one with an open costume (14 disc + every mod that
  isn't a costume pack; a pack's costumes go with its disc character, as on L1 / R1), then a costume uniformly among
  its open ones (B66 `free`: not readied by anyone else, not a failed-to-load mod costume). The pick is set on the
  player's cursor and they're ready — the card reveals it. Picking again (○) shows the ? card again.
- The dice: `dice()` (std's `RandomState` hash), never the sim RNG. Online the picker draws and the result travels
  as any pick (`lives` / `picks`), so peers and the handshake see the same character and costume.
- Local computer players: 1P drives them on the same select, so Random works for them the same.
- Online computer players (the lobby's Add Computer): `bot_random` draws a free disc character and costume
  (taken = `bot_taken`, split out of `bot_pick`); ←→ / L1 R1 still change it.
- Tests: `random_card_draws_disc_and_mods` (3000 locks: every one of the 146 open picks comes up — disc costumes past
  1, both mods' costumes, the costume pack's — none taken; a COM's Random); `shared_select_and_host_bots` (the
  lobby bot draw covers all 140 − taken disc picks, never a taken one).
- Shot: `HST_MENU=chars-random` → `context/b217/random.png`, `random-wide.png` (16:9+: the card stays in the grid).

## Not verified / not 1:1

- Remaster-only screen (no original to compare; menus are ours, PLAN.md).
- Online computer players draw disc characters only: bots can't play mods online until B216 (their mod picks and
  handshake); `bot_random` has a ponytail note. They start random when added, not "on the Random card" in the select,
  since online bots are picked in the lobby until B216 moves them into the select.
- Another peer sees a remote player's hovered character (its cursor underneath), not the ? card, until they're
  ready: `Live` has no Random flag (adding one changes the wire format).
- "Only mods allowed in the lobby": there's no per-lobby mod filter today; Random draws from the same roster a manual
  pick can take.
- Not tried in a two-machine online match (no second peer here); covered by the unit tests only.
