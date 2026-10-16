# M120: Online Host goes straight into the lobby

## What changed
- **Host → select at once** (`online.rs` `online_open`): the host lands on the lobby's character select straight
  away (`first` set, so ○ leaves the lobby). It no longer sees the "Creating lobby..." screen. The room arriving
  later leaves the host where it is. If the host readies before the room exists, its picks queue behind `Cmd::Host`
  (the host's task reads commands in order). A failure still drops to the Lobby screen, which gives the reason.
- **Corner status chip** (`online::corner`, drawn by `roster::chars` while online), right-aligned in the top bar.
  It shows one of:
  - LOBBY Opening... (spinner)
  - CODE + short code (PRIVATE for a private lobby)
  - LINK Ready to share
  - CODE Connecting... (spinner)
  - OFFLINE + the reason
- **Mixed columns** (`roster::columns_mixed`): this machine's players get the full Local-play roster columns. Other
  machines' seats and open seats get compact cards up to 300 mockup px wide, and the full columns share the rest.
  - The compact card shows:
    - the band with the character and owner (plus ping) and the Near/Far side
    - a preview with a Ready!/Picking... pill
    - a mod note
    - a "Style - Left/Right" pill
    - the five stat bars filled to the character's grades
    - a foot with the person's ONLINE RECORD (W–L and win %, "First match" at 0–0) or COMPUTER and its rank
  - The remote roster isn't shown.
- **Online record** (`hst-base/src/net.rs`):
  - `Record {won, lost}` is kept in online.txt (`won =`/`lost =`) and sent in the Hello.
  - The host keeps `Room::records` per peer.
  - `Room::scored(team)` counts each person once per finished match, from `again()` in netplay.rs.
  - A guest takes the host's counted room; a host-less guest counts its own.
  - The menu stores this machine's entry when it's back in the lobby (`kept_record`).
- **No court behind the Online screens** (user 2026-10-16): `frame::stage` is split into `court` and `sides`. The
  online lobby and the online confirm draw only the team labels and the VS badge.
- Samples: a new `HST_MENU=chars-hosting` shows the host picking while the lobby opens. The online samples now
  carry records.
- Tests:
  - `records_count_each_person_once` (hst-base)
  - `compact_cards_leave_the_rest_to_the_rosters` (roster widths at six aspects, every mix)
  - the Host flow test now expects Chars

Shots: `context/m120/run3/` (chars-online at 16:9, 4:3 and portrait; chars-hosting; lobby; lobby-guest).

## Not verified / not 1:1
- I couldn't fetch the concept mockup (the artifact page is rendered client-side). The layout follows the task text
  and the existing Local-play roster, not the mockup.
- "Rating" isn't done. Only win/loss is shown: there's no rating system to draw one from, and inventing one would be
  a guess.
- I didn't play a real two-machine online match to watch the record count up. Only the unit test and the build cover
  it. Matches that end by a disconnect or a leave aren't counted (only a finished match's result is).
- Portrait: the compact cards are as wide as the full columns (the even share is under 300 px), which leaves empty
  space inside them. Readable, not tuned.
- Menus don't need to be 1:1 (PLAN), so there was nothing to check against the original.
