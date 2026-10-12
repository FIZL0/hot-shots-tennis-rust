# M2f handoff (slot 4 → slot 2, 2026-10-12)

M2f was stopped mid-task by the user so slot 2 can finish it after M2c (the Game/play state split). Branch
`handoff/M2f`: commit `6b77672` "M2f WIP handoff" holds all the code; this note is the commit after it. It is based on
main at `7d1c6ad`. `task/M2f` was reset to main, so nothing was ticked, blocked or merged.

**Status:** `cargo build -p hst` is clean (only the existing `human_mate` warning in play.rs).
`cargo test -p hst --bin hst -- main_menu net::` gives 15 passed, which covers the 5 `net` tests and the new
`online_flow_host_and_code`. `tools/check.sh` was **not** run. The menu screens were **not** looked at yet: no
`--shot` at any aspect ratio. There is no journal README yet, only this note.

## Files

| File | State |
|---|---|
| `crates/hst/Cargo.toml` | + `serde` (derive), `iroh = "1.3"`, `iroh-mainline-address-lookup = "0.6"`, `n0-mainline = "0.7"`, `iroh-tickets = "1"`, `n0-future = "0.3"`, `tokio` (rt-multi-thread, macros, time, sync), `qrcode = 0.14` (no default features); bevy gets the `system_clipboard` feature. `Cargo.lock` changed accordingly. |
| `crates/hst/src/net.rs` (new, ~560 lines) | Transport and lobbies, done and tested over the real internet (below). No Bevy apart from `#[derive(Resource)]` on `Net`. |
| `crates/hst/src/main.rs` | `mod net;`, plus `hst net …` as the first thing in `main()` (the CLI, `net::cli`). |
| `crates/hst/src/play/main_menu/online.rs` (new) | Online screens: state (`View`), presses, layout, QR texture, network event polling, effects. **Written, compiles, unit-tested, never seen on screen.** |
| `crates/hst/src/play/main_menu.rs` | Small additive hooks only (listed below). |

### `net.rs`, what's in it
- **Endpoint** (`endpoint(relay)`):
  - `Endpoint::builder(presets::Minimal)`, so no n0 DNS. Address lookup is `DhtAddressLookup` with
    `AddrFilter::unfiltered()`, which publishes pkarr records on the mainline DHT.
  - `RelayMode::Default` for n0, `Custom(url)` for your own relay, `Disabled` for Off.
  - `HST_RELAY_ONLY=1` → `clear_ip_transports()`. It's for testing only: it forces traffic through the relay.
  - Waits up to 10 s for `ep.online()` unless the relay is Off.
- **Settings:** `Online { name, relay: Relay::{N0, Own(url), Off}, relay_url }` lives in `online.txt` beside the
  disc (`Online::path/load/parse/text/next_relay`). Keys: `name =`, `relay = n0|own|off`, `relay_url =`.
- **Lobby list** (`find`, `announce`):
  - Hosts announce on the fixed infohash `TOPIC = b"hst-remaster-lobby/1"`. Each uses its *lobby number* as the
    port: a u16 from the endpoint ID bytes, `lobby_number`.
  - The details are a BEP44 mutable item under one public shared key,
    `SigningKey::from_bytes(BOARD = b"hot-shots-tennis-remaster-lobby1")`, salted `hst1/<n>`.
  - The shared key means anyone can write, so the value is the **endpoint's own signature** over (salt + JSON)
    followed by the JSON (`seal`/`unseal`). A forged or replayed listing for another lobby number fails `unseal`.
  - seq = now in µs. Republish every `REPUBLISH` = 30 s and on every change; Find drops listings older than
    `STALE` = 90 s, the Halo journal's 30 s / 90 s.
  - Find = `get_peers(TOPIC)` (30 s cap), then the mutable item for each port in a `JoinSet`. It drops stale or
    full lobbies and dedups by id (newest wins).
  - BEP signed peers were rejected: it's an unstable extension the real DHT doesn't support.
- **Quickplay** (`quick_pick`): mode matches, open seats ≥ my players, busiest first, then newest. It tries each
  in turn and otherwise hosts.
- **Join codes:** the code is an `iroh_tickets::endpoint::EndpointTicket` (the host's address). A bare
  `EndpointId` works too, resolved through the DHT's pkarr record. That's what Find's rows use.
- **Protocol:** ALPN `hst/lobby/1`, one bi stream, JSON messages with a u32 LE length prefix and a 64 KiB cap.
  `Msg::{Hello{build, picks}, Room(Room), Refused(String), Start(Room)}`.
  - The host checks `build == BUILD` (`"hst <pkg version>"`) and seats the guest all-or-nothing in the first open
    seats (`Room::take`). Otherwise it sends Refused and drops the guest after a short delay.
  - It broadcasts Room on every change, Start once every seat is full, and frees the seats when a guest's stream
    ends (`Room::leave`).
- **App side:** `Net` resource. `Net::start(cfg)` spawns a thread running a 2-worker tokio runtime. Commands go in
  through a tokio unbounded channel (`send(Cmd)`); events come out through std mpsc (`poll() -> Vec<Event>`).
  Dropping `Net` ends the thread.
  - `Cmd::{Host(Room), Find, Quick(Room), Join(code, picks), Leave}`.
  - `Event::{Status, Code, Found, Room, Start, Failed}`.
- **CLI:** `hst net host|find|quick [doubles] [players] | join <code> [players] [--relay n0|off|<url>]` prints the
  events. This is how everything was tested.
- **Tests:** `seats_fill_and_free`, `quickplay_takes_the_busiest_that_fits`,
  `listing_signature_binds_the_lobby_number`, `online_txt_round_trips`, `find_on_a_testnet`
  (`n0_mainline::Testnet`, offline).

### Verified (one machine, real internet, public mainline DHT)
- Find: a host from `hst net host doubles 1` was found by `hst net find` in about 13 s.
- Join by bare endpoint ID (address from the DHT pkarr record): worked.
- `hst net quick doubles 2` joined the busiest lobby and filled it; Start reached the host and both joiners.
- Join by ticket with the joiner on `--relay off`: worked.
- Self-hosted relay:
  - Installed with `cargo install iroh-relay --version 1.3.0 --features server --root <dir>` and run as
    `iroh-relay --dev` (plain HTTP on :3340, metrics on :9090).
  - Both sides ran `HST_RELAY_ONLY=1 … --relay http://localhost:3340`. The ticket held only the relay URL, the
    join reached Room and Start, and the relay log showed both endpoints connected.
  - Logs were in my scratchpad; they're gone, so re-run if needed.
- **Not verified:** two machines on *different networks* (the task's acceptance). That needs the human or a
  second machine. Add it under PLAN.md *Needs the human* with exact commands (`hst net host` on one,
  `hst net find`/`join <code>` on the other, and once with both on `relay = own` pointing at a public
  `iroh-relay`).

### `main_menu.rs` hooks (all additive; keep them minimal on rebase over M2c)
- `mod online;`. `Screen::{Online, Lobbies, Lobby}`. `Menu.net: online::View` (+ Default).
  `Effect::Online(online::Fx)`. `Tex::Qr`.
- Main row 1 (Online): `seats = [Some(dev), None…]`, then go to `Screen::Online`. This replaced the
  "ponytail: Online is M2's" line.
- Mode: Back goes to Online when `menu.online()` (`net.go.is_some()`).
- Assign: online, other devices may only take seats `< local_max()` (1 in singles, 3 in doubles). The seats left
  over are for peers, not the computer.
- `driven()`: online, a device drives only its own player (no computer players to pick).
- Chars: online, "all ready" means every *seated* player is ready, then `online_ready()`. That goes to Confirm
  (Host), Lobbies + `Cmd::Find` (Find), Lobby + `Cmd::Quick` (Quick), or Lobby + `Cmd::Join(paste)` (Code).
- Confirm row 0 online calls `online_host()`: Lobby + `Cmd::Host(room)`.
- `step()` match: Online/Lobbies/Lobby → `online_step`. `layout()` arm → `online::layout`. `bgm()`: these screens
  play track 0, a guess (see below). `HST_MENU=online|lobbies|lobby`.
- The `step` system takes one more param, a tuple `(Option<Res<Net>>, ResMut<Clipboard>)`; it's a tuple because
  the system was at the param limit. It handles `Effect::Online` via `online::apply`, which returns the sounds.
- `draw` takes `Option<Res<online::QrImage>>`. `Tex::Qr` gets `rect = None`, and the quad is hidden until a code
  exists. Plugin: `online::poll.before(draw)`.
- `Cmd`, `Fx` and `Effect` derive PartialEq (the existing tests compare `Vec<Effect>`).

### `online.rs` flow
- **Online screen** rows: Host, Find, Quickplay, Join by Code, Relay: <n0 | url | Off (LAN)>, Back.
  - Relay cycles with ok/←/→, saves `online.txt` and stops `Net` (`Fx::SaveCfg`, `Fx::Off`).
  - Join by Code reads the clipboard (`Fx::Paste` → `Menu::pasted`). It goes on only if the text parses as a ticket
    or an endpoint ID; otherwise the status line explains.
- **Lobbies** (Find): rows "name  Singles/Doubles  taken/seats", then Refresh and Back. One page of 7, scrolling by
  selection (marked ponytail). The info bar shows the court and open seats.
- **Lobby:** title, court, a seat pill per seat (name from the select's NAMES sheet, the host or the peer ID's first
  6 characters, or "open"), the QR at [400, 120, 200, 200] and "Code: X / Enter copies" (`Fx::Copy` →
  `Clipboard::set_text`).
  - Back sends `Cmd::Leave` and returns to Online.
  - `Event::Start` sets `started` and the status "Everyone's in. (The online match itself comes with lockstep
    netplay, M2g.)": **Start doesn't launch a match.** M2g (or M2c's split) has to turn `Room` into a match.
- `Net` is created lazily on the first `Fx::Net` (`apply`), and removed by `Fx::Off` (back to Main, or a relay
  change). The config comes from `menu.net.cfg`, else `Online::load(iso)`.
  - Caveat: `menu.net.cfg` stays `None` until the Online screen's first press (`get_or_insert_with(default)`), so the
    Relay row shows the default rather than what's in `online.txt`. Load it when Main → Online instead. That needs
    the iso path in `Menu::step`, or a load in `setup()`.
- QR: `qrcode` → white RGBA `Image` with a 4-module quiet zone and a nearest sampler, inserted as the `QrImage`
  resource when `Event::Code` arrives.

## What's left (in order)
1. Rebase or redo the main_menu.rs hooks on top of M2c's split. `net.rs` and `online.rs` shouldn't care.
2. Load `online.txt` when Online opens (caveat above). Let the player edit `name` in the menu, or settle on
   online.txt only plus a default name (`Online::default()`, check what it uses).
3. Look at it: `HST_MENU=online|lobbies|lobby` with `tools/shot.sh` at 4:3, 16:9, 21:9 and a tall window
   (PLAN.md: any aspect ratio). The layout coordinates are guesses in the 640×448 space with the default anchor;
   the QR and seats may need `Anchor` choices like the other screens. The QR in particular must stay square on
   wide windows, so check `widescreen::place`.
4. On Start, hand `Room` over to a match. That's M2g's lockstep, or at least a placeholder flow; decide with the
   M2 plan.
5. README *Online play*:
   - `online.txt` (name, relay = n0|own|off, relay_url).
   - Running your own relay: `cargo install iroh-relay --features server`. It needs a public IP and domain, and a
     TLS config (`--config-path`, see iroh-relay's docs). `--dev` is plain HTTP for local tests. Set
     `relay = own` and `relay_url = https://your.domain`.
   - The `hst net` CLI.
6. Journal README for `2026-10-12-m2f-lobbies` with a *Not verified / not 1:1* list:
   - Cross-network test not done (needs the human).
   - Seats fill first-open, so two players from one machine can land on opposite teams in doubles; there's no team
     choice.
   - 16-bit lobby numbers can collide (two hosts then overwrite each other's listing, and each signature check
     drops the other's). Upgrade path: salt with the full ID, keeping the port only as a hint.
   - Two DHT nodes per process (address lookup builds its own, plus our `Dht::client()` for the lobby board).
   - No DHT in the browser, so the wasm build (M3/M9, the mobile path since M9b was dropped) can't Find. A
     browser-reachable lobby list (an HTTP mirror or the relay) is a follow-up TODO.
   - Menu sounds and BGM for Online are guesses (remaster-only screens; acceptable per "menus not 1:1").
   - Start doesn't launch the match (M2g).
7. `tools/check.sh`, then `tools/done.sh M2f …`. Probably `--blocked` on the two-machine test, or tick it with the
   cross-network check as a *Needs the human* item, as the user prefers.

## Learned / pitfalls
- `n0_mainline::GetStream`'s field is private: use `n0_future::StreamExt::next`.
- iroh 1.3's Minimal preset has no address lookup; add `DhtAddressLookup` yourself. Without `unfiltered()` it
  publishes only relay URLs, not direct addresses.
- A leftover `iroh-relay --dev` keeps :9090 busy ("Error starting metrics server: Address already in use"); kill
  the old one by PID.
- Bevy 0.19 clipboard: feature `system_clipboard` (wayland comes with bevy's default `wayland` feature). The
  `Clipboard` resource comes with DefaultPlugins. `fetch_text()` → `ClipboardRead`, then `.poll_result()` is
  `Some(..)` at once on desktop and async on web. `set_text(..)`.
- iroh works in the browser (relay only), but n0-mainline doesn't (UDP).

## Open questions for the user (none blocking)
- Should two players from one machine be kept on the same team in doubles?
- A player-editable name in the menu, or `online.txt` only?
