# B425 — Roster "Get more" ghost chip (FINAL)

With no mods installed at all (no character, court or umpire mod), 1P's section rail ends in a dashed ghost chip
"+ Get more characters" in the seat's soft colour. A tap or click from 1P's own device opens the Mods page's
store (`mods_open`, as the main menu's Mods row does); ○ there goes back to the main menu like it always does,
where B205's Reload popup picks up anything installed. Mods change the roster only after a reload, so going back
to the select would show a stale roster.

- `Menu::store_chip(p)` (mods_screen.rs): 1P only, on the local select (not online or in a lobby), no mods of
  any kind. 2P–4P never get the chip, and an `Act::Store(0)` from another device does nothing.
- The chip keeps its own width. Its words are the longest of "+ Get more characters" / "+ Get more" / "+ More"
  that fits after the section chips (`more_label`), else one leaving the sections at least half the rail, which
  scrolls for the rest. It sits right after the last section chip, and the rail's own room shrinks by its width.
- `HST_NO_MODS=1` (shot fixture): drops the installed mods right after loading them, so roster shots show the
  chip on a machine that has mods.
- Tests: `the_get_more_chip_takes_the_longest_words_that_fit` (roster.rs),
  `get_more_is_1ps_only_and_only_without_mods` (tabs.rs: only 1P's hit, inside the rail and past the section
  chips, only 1P's tap opens Mods, gone with one mod).
- Shots: `context/B425/run1/` roster1/2/4 at 16:9, roster2/4 portrait 500×900 ("+ More" in the narrow column),
  roster2-favs 4:3, roster4-favs 21:9-ish 1680×720.

## Not verified / not 1:1
- Not 1:1: the chip is our own front-end design (PLAN: menus are ours), so there's no original to compare it to.
- The chip is only reachable by tap or click. A pad or keyboard player can't select it (L2/R2 stop at the last
  section, B462). The main menu's Mods row covers them, and the concept describes the chip as a tap target.
- The online case isn't covered by a test or a shot. `store_chip` returns false there by code (`online()` /
  `select_online()`), because opening the store would leave the lobby.
