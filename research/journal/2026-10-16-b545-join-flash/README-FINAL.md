# B545 — spurious "can't join" while joining a lobby

## What showed it
Since M120 a join lands straight on the lobby's select before the host's line-up comes. A guest's open seat card
(2P) took its line from `reach()`: `Connecting` fell through to the host's B519 text "Not open yet: friends can
join once the relay is up", so a guest saw a "can't join" message for the ~1 s until the host's `Room` arrived,
then the lobby filled in normally. The network side is clean: `hst net host --relay off` + `hst net join <ticket>`
locally gives Status → Me → Connected → Joined → Room, with no `Failed` before the room.

## Fix
- `roster::open_line`: a guest still connecting gets "Joining the lobby..." on the open seat ("Finding a lobby..." for
  Quickplay); the relay text stays the host's.
- Two related races in `heard` that could flash a real failure screen during a join that then works:
  - a Find cut short by the join can still deliver its last `Found`/`Failed` ("Couldn't read the lobby list", now
    `net::LIST_UNREAD`): ignored once a lobby was chosen (`pick_next`/room, `finding == 0`).
  - Quickplay failing to join one stale listed lobby goes on to the next or hosts: that `Failed` (before `Hosting`)
    is now a status line, not "Couldn't find or create a lobby" + the Lobby screen.
- Test `joining_shows_no_failure_before_it_fails`; menu shot state `chars-joining` (screenshot in `context/b545/`).

## Not verified / not 1:1
- Not reproduced with two GUI instances over a real network: the flash's cause was found from the code and the
  select's joining state (shot), not watched live. The Find/Quickplay races are covered by the unit test only.
- Front-end only, no original to compare against (menus aren't 1:1 per PLAN).
