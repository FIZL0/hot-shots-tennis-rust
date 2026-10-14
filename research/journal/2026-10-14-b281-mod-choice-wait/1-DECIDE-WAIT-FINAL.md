# B281 — Start waits 10 s for a player choosing about a mod, then stand-ins

Why the match didn't start: the host's start waits until every peer has said it's settled (`Msg::Ready(ver)`: every
mod's looks here, fetched, or a stand-in). A peer with the download prompt open never settles, and nothing said so.
Also, the menu only settles on the lobby screen, so a peer elsewhere (e.g. the select) never even got the prompt.

What changed
- `net.rs`: `Room.deciding` (peers with a mod not yet chosen to download or not; from the new `Msg/Cmd::Deciding`),
  `Room.wait` (seconds left, counted by the host like the start countdown), `Room.stand_in` (the wait ran out).
  The wait starts once the countdown has ended (`go`) and someone is deciding; it stops if nobody is deciding any
  more or the start is called off. `DECIDE_MS` = 10 000 (tests shorten it).
- `online.rs`: each frame the menu tells the host whether it still has an undecided mod (asked, queued, or not
  asked because it's off the lobby screen). With `room.stand_in`, `settle` closes the prompt and plays every
  undecided mod as its stand-in (same as "Stand-in this time", the remembered setting untouched), on any screen,
  then sends Ready as before → the match starts.
- Lobby: a "Deciding" chip on that player's seat; the help line under the seats says
  "Waiting for NAME to choose whether to download a mod: N s, then a stand-in plays it."; the deciding player's
  prompt says "The match is waiting: choose in N s, …". `HST_MENU=lobby-wait` shows the host's view.
- A player who already chose to download keeps downloading (not deciding): the host waits for the download, as before.

Checks
- `net::tests::start_waits_for_a_mod_choice_then_gives_stand_ins` (loopback, wait shortened to 2 s): deciding shows on
  the host, Start → wait counting on the guest, no start before `stand_in`, then Ready → Start.
- Shot: `HST_MENU=lobby-wait` (context/b281/lobby-wait.png).

## Not verified / not 1:1
- Front-end only (ours to design, PLAN): no original to match.
- Not tried on two real machines / the browser build: covered by the loopback test; the browser's guest path uses
  the same `join` loop (`Cmd::Deciding` forwarded there).
- A download that stalls still holds the start (not part of this task: the player chose).
