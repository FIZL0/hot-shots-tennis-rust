# B188 — matches of more than one set at Games = 1

## The original's Games option

On PCSX2 (copy 1, slot 2 → character select → Start → "Confirm Settings", Sets 1 / Games 4), Games steps by 2 and
stops at the ends: from 4, left → 2, left again stays at 2 (the ◀ arrow goes); right → 4 → 6, right again stays at 6
(the ▶ arrow goes). So the original offers **2, 4, 6** only. Screenshots: `context/b188/g_l*.png`, `g_r*.png`;
slot 9 holds the confirm screen with Games being edited.

Our menu offered 1–6 (`wrap(…, 6)`). It now steps through `GAMES = [2, 4, 6]` (`play/main_menu.rs`) and wraps round
like our Sets row (the menu doesn't have to match 1:1). The default is still 4. The online lobby takes the host's menu
value, so it gets the same range. The CLI still accepts any `--games N`.

## Hunting the crash

None of these runs panicked:

- **hst-sim score rules** (`tests/score.rs every_match_length_ends`): singles and doubles, sets 1–3 × games 1–6, 500
  random matches each, tiebreaks included. Every match ends, the server and receiver stay in range, and the set and
  set counts stay in bounds. This runs in the normal suite.
- **Headless match** (`play/instant_replay.rs every_match_length_plays_out`, MinimalPlugins and FixedUpdate): bot
  singles and doubles at sets 1–3 × games 1–6 all played to `match_over` (`context/b188/grid.log`, 913 s). It's
  `#[ignore]` because of that time. It now defaults to the menu's 2/4/6, and `HST_LENGTHS=2x1,…` picks other lengths.
- **GUI release bot matches**, all with `--windowed` and `HST_AUTOPLAY`:
  - Singles, sets 3 games 1, 400 s.
  - Singles, sets 2 games 1, to the ceremony, the result screen and "Continue, back to the menu".
  - With a temporary hack (reverted, never committed) that forced 1-1 and a tiebreak in every set: singles and doubles
    at sets 2 games 1, each through two tiebreaks, the ceremony and the result screen.
- **GUI debug build** (overflow checks on), singles, sets 2 games 1: a set, a game and the start of a set-2 tiebreak,
  then the match went back to the main menu. That was **not** a crash. The user's own controller (GuliKit, connected
  14:42) drove the hidden window's pause menu, since every match reads every pad. I stopped the GUI runs then so I
  wouldn't take the user's input.

The result board (`popups.rs board`), the pause scoreboard (`menu.rs`), the info panel, the intro banner digits, the
weather schedule, the ceremony plan and `match_stats` were read for indexing or arithmetic that 1-game sets could
break. All of it is clamped or bounded.

The crash itself wasn't reproduced. With the menu now limited to the original's 2/4/6, Games = 1 can't be picked any
more, so the setting the report was about is gone from the menu either way.

## Not verified / not 1:1

- **The reported crash wasn't reproduced.** The sim grid, headless grid and GUI bot matches above (1-game sets,
  forced tiebreaks, ceremony, result screen) don't panic. I ran no human-seat match (the `--pads` path from the menu)
  and no web build. The user was using a controller during the session, and the hidden windows read it.
- **Games wraps round (6 → 2) instead of stopping at 6 as the original does.** It matches our Sets row; the menu
  doesn't have to be 1:1 (PLAN.md).
- **The CLI's `--games N` still takes any value.** It's for testing; only the menu is limited to 2/4/6.
