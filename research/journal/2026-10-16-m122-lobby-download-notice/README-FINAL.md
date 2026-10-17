# M122 — online lobby: everyone sees who is still downloading a mod

Builds on B537 (per-pack progress events on the downloading machine).

## What
- `Room::fetching` (peer → percent, all its wanted packs averaged, a pack not started counts 0) in `net.rs`;
  `Cmd::Fetching(Option<u8>)` / `Msg::Fetching`. A guest sends it to the host, the host (own `Cmd` too) puts it in
  the room and rebroadcasts on change only — shown only, no relisting, no ready reset (like M95's `Device`). Cleared
  when the peer leaves or is dropped.
- Menu (`online.rs`): `Menu::fetching()` turns `wanted` + `progress` into 5 % steps; `poll` sends it when it
  changes (`View::fetching`, reset with a new lobby).
- Shown to every player: a remote seat's card pill becomes a yellow "Downloading 45%" (`roster.rs`); the chat box's
  status line "Still downloading a mod: Sunny Day 45%, you 10%"; the Start / Ready small line
  "n of N ready · downloading: …" (after "still picking"). The host's Start already waits for these peers to settle
  (the gate predates this); now the reason is on screen.
- `HST_MENU=lobby-fetching` shot scene; shot in `context/M122/lobby-fetching.png` looks right.

## Tests
- `net::tests::mod_looks_peer_to_peer_and_the_ready_gate`: guest's and host's progress reach the other side over
  loopback, `None` clears them.
- `online::tests::remote_mod_asked_on_the_select`: 5 % steps over several packs, `downloading` / `status_line`.

## Not verified / not 1:1
- Not 1:1: the original has no online lobby; this is our front-end (menus are ours to design).
- Not verified over two real machines: covered by the loopback test and a `--shot` with a sample room only.
- The browser's LAN (rtc) gets no packs at all (B537's ponytail), so it never shows a download there.
