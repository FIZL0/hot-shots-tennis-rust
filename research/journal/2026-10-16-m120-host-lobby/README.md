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

Iteration 2 (`2-FINAL.md`) replaced the record, the compact card and the corner chip; see there.

## Not verified / not 1:1
- The mockup (`context/m120/mockup/project/Main.dc.html`) puts the court/umpire/sets strip and Start under the
  select on one screen. We keep two screens (the select, then the Lobby with the strip and Start), so the strip
  stays as it was. The mockup's "Fill with COM" button on open cards isn't there.
- No real two-machine run against a live relay: `Event::Joinable` timing is covered by the unit test and the code
  reading only. The browser LAN host says Joinable once its first offer is out.
- The open-seat and notice wording is ours (menus don't need to be 1:1, PLAN).
- The glyph advance used to wrap the notice band and the open-seat line is an estimate (0.55 em); each line is
  squeezed to fit, so nothing is cut, but a very long word could look narrow.
