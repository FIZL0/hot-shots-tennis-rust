# M2f — online lobbies (port onto M2c/M13a)

The design, protocol and the session's own tests are in `HANDOFF-M2f.md`, beside this file. This entry covers bringing
that WIP onto task/M2c HEAD (M2c + M13a) and finishing what can be finished on one machine.

## What's in
- `net.rs`, unchanged in substance:
  - iroh 1.3 endpoint with mainline DHT address lookup.
  - Lobby board as BEP44 signed items.
  - Join codes as `EndpointTicket` or a bare ID.
  - Host, find, quick and join over a tokio thread behind the `Net` resource.
  - `hst net …` CLI.
- Menus (`play/main_menu/online.rs`): Online, Lobbies and Lobby screens, as in the handoff.
- Port fixes:
  - Conflicts in `Cargo.toml`, `main.rs` and `Cargo.lock` resolved. M13a's `fn start(argv)` and wasm gating are kept.
  - `hst net` is checked before `disc::resolve`.
- Browser build:
  - iroh/n0-mainline/tokio are native-only deps; the transport code is `cfg(not(wasm32))`.
  - The wasm `Net` stub answers every command with "Online play isn't in the browser build yet."
  - `net::is_code` is false on wasm.
  - `qrcode` builds on all targets.
- `online.txt` is now read when Online opens (`Fx::Load`). Before, the Relay row showed the default until the first
  network press (the handoff's caveat).
- QR:
  - It stays square at any aspect: the largest square in its 640-space rect, centred, so it scans in portrait and
    ultra-wide.
  - It is remade whenever the code changes (`QrImage` remembers its code).
- Shots: `HST_MENU=lobby|lobbies` fill the screen with a sample room and list (`online::sample`).
- `tools/shot.sh` takes `HST_SIZE='W H'` for other window sizes.
- README *Online play* covers `online.txt`, running your own `iroh-relay` and the CLI.
- PLAN *Needs the human* has the two-network test.

## Verified
- Re-run by s2 on task/M2c after the port (`context/m2f/net1/`): `hst net host singles 1 --relay off`, then
  `hst net join <code> 1 --relay off` from a second process reached Room and Start on both sides; `hst net find` on
  the public mainline DHT listed the host's lobby.
- `cargo test -p hst --bin hst -- main_menu net::` gives 16 passed. This includes the handoff's `find_on_a_testnet`,
  which runs offline.
- `tools/check.sh`: everything passes except `hst-sim`'s `motion::match_serve_walk`, which panics because its
  fixtures (`match_s05.bin`, `1p3goodcpus.bin`) aren't in this worktree's `context/`. It's fixture-only and untouched
  by this port.
- Native and `wasm32-unknown-unknown` release builds pass. The only warning is the existing `human_mate` one.
- Shots of Online, Lobbies, Lobby and Main at 1280×720, 960×720, 1680×720 and 600×900 are in `context/m2f/run1/`.
  - Everything lays out at every size.
  - The ticker scrolls.
  - The QR is square in portrait.
- From the handoff, not re-run here (it was on the same code):
  - Public DHT find in about 13 s.
  - Join by ID and by ticket.
  - Quick play.
  - `--relay off`.
  - Relay-only through a local `iroh-relay --dev`.

## Not verified / not 1:1
- **Two networks**: nothing has run across real NATs or two machines yet. This is in PLAN *Needs the human*.
- **Start doesn't start a match.** The lobby reaches Start, then nothing happens. Lockstep play is M2g.
- Seats fill in the order players join. There's no team choice in the lobby.
- Lobby numbers are 16-bit, taken from the endpoint ID. Two hosts on the same number overwrite each other's listing
  (about 1% odds with 36 hosts at once; `lobby_number`'s ponytail note gives the fix).
- Each process runs two DHT nodes: iroh's address lookup and the lobby board.
- There is no online play in the browser. Find needs UDP for the DHT. iroh itself would work over relays, but a lobby
  list the browser can reach is a follow-up.
- The original has no online mode, so these menus are new:
  - The titles reuse the disc's "Main Menu" sheet on Online/Lobbies and "Confirm Settings" on Lobby.
  - The BGM and cursor/OK sounds are picks from the main menu's, not taken from any original screen. This is
    "similar, not 1:1" per the menus rule.
- In portrait windows, menus stretch vertically. All menus already did this; only the QR is kept square.
- Paste reads the clipboard synchronously only (`system_clipboard`). On Wayland, a clipboard owner that answers
  slowly reads as empty.
