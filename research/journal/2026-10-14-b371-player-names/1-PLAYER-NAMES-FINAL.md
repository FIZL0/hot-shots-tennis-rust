# B371 — player names, Smash Bros style

Spec: `plan/B368.md` § Names. Front-end and in-match presentation only: ours to design, no sim/RNG/timing change.

## What landed

- `play/main_menu/names.rs`: the store (`names.txt` beside `settings.txt`, most recent first, 40 kept, ≤10
  characters from the menu font, no commas), each device's chosen name (the name follows the device to another
  seat), and the picker per seat: △ on your own seat opens it; list = saved names (one another device holds is
  greyed and skipped), *New name...*, *No name*; ✕ picks, □ forgets a saved name, ○ closes. *New name...* opens an
  on-screen keyboard (A–Z 0–9 space - . !, Back/OK); a physical keyboard types straight into it (Backspace, Enter →
  OK). A name another player already has is refused (buzz). Taps/clicks work on rows and keys; the panel swallows
  taps on the roster under it. Legend shows the picker's own actions, else △ "Name".
- The card's toggle for the attribute card moved off △ (it is still on the Stats/Picture pill tap, `Press.stats`).
  The B347 test was rewritten for that (`stats_pill_shows_the_attribute_card`, plus "△ names the player").
- The select card's header tag shows the name instead of 1P–4P.
- Match: `--names a,,b` (match order) → `play::name_tags::Names`. Before each serve, a named player's 1P–4P marker
  is swapped for a UI tag (name in the panel colour, a pointer under it) hung at the marker's anchor and as tall on
  screen as the marker (projected through the match camera; turned for the sideways phone HUD). Point boards'
  plates, the final board and the stats page swap a named player's slot-label quad for the name in the label's tint,
  shrunk to fit the label's room (`name_tags::take` / `labels`).
- Online: each `Live` pick carries `player` (the seat's name); the lobby rows show it (else the machine name as
  before), and `launch` passes `--names` built from this machine's devices + the others' live picks.
- Shots: `HST_MENU=names` / `names-type`. Tests: names file round trip, picking/typing/No name, keyboard typing,
  names to the match by seat (local and online), `take` of named labels, `--names` parse; legend probe covers the
  picker states. `tools/check.sh -p hst`: 357 passed.

Looked at (`context/b371/`, scratch shots): picker list and typing views at 16:9; a 2P doubles match with two names
(tags over both, markers kept on the COMs); final board and stats page with a 10-character name (fits after the
shrink).

## Not verified / not 1:1

- B368–B370 (the per-player-column select rewrite, △ no longer the card toggle) were open in parallel branches; this
  branch builds on today's select, so the runner's merge may need the picker re-hooked into the new columns. The
  picker panel itself is placed from the card's rect only.
- The in-match HUD score panel (the 1P pill beside the face in the corner) still shows 1P–4P: the task names the
  select header, the marker spot and the result screen, not the panel.
- Online name exchange is unit-tested (room → flags, live pick field) but not run between two real machines.
- Name text uses Bevy's default (monospace) font, not the menu's vector font; on the pink stripe the tinted name
  is less contrasted than the original's outlined label art.
- Portrait / sideways phone layout of the in-match tag is computed (`to_hud`) but not looked at in a shot.
- Online remote players' names in a match come only from their `Live` pick; a peer on an older build sends none
  (shown as 1P–4P).
