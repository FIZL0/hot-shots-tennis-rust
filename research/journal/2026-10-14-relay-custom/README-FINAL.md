# B323 — Custom relay entry in the Online menu

## What changed
- Online → Relay now cycles n0 → Off → own (only once a `relay_url` is set) → **Custom...** → n0 (`Online::next_relay`
  in `net.rs` plus the Custom... step in `play/main_menu/online.rs`). Custom... is last because it's for advanced
  players. Landing on it changes nothing yet. The row and the card show "Custom...", and the card's hint says it's
  for self-hosted relays. Moving off the row shows the current relay again.
- OK on Custom... opens the code screen as a URL field (`View::url`): lowercase/digit/`.-:/` keys, keyboard typing
  (any printable ASCII, no spaces, up to 120 chars), Delete / Paste / Save / Back. It starts from the saved URL, or
  `https://` if none is saved. Save runs `net::relay_url` (iroh's `RelayUrl` parse + http(s) + a host). A bad URL
  stays in the field with a status line. A good one goes to `Online::set_own` → `relay = own` + `relay_url`,
  written to online.txt, and the network restarts (`Fx::Off`), so the next Host/Find/Join uses it. Back moves on
  round the cycle to n0.
- Shortened name as before (`relay_name`, 18 chars without the scheme).
- Browser: `web::code_entry(on, url)` → the phone keyboard input gets `autocapitalize=none`, `inputmode=url` for
  the URL (`web/index.html`).
- `HST_MENU=online-custom | relay-url` for shots. README *Online play* describes it.

## Verified
- `relay_custom_url` (online.rs) walks the whole menu flow: cycle, Custom..., a bad URL refused, Paste,
  Save → `Relay::Own`, SaveCfg + Off, online.txt round trip, Back → n0. `relay_cycle_and_urls` (net.rs) checks the
  cycle and which URLs are accepted or refused.
- End to end on this machine: `iroh-relay --dev` (1.3.0, on :3340). Then
  `HST_RELAY_ONLY=1 hst net host --relay http://127.0.0.1:3340/` (the exact form `relay_url` saves, trailing slash
  included) and `hst net join <short> --relay http://127.0.0.1:3340/`. The guest joined and the lobby started,
  round trip 3 ms, with direct paths disabled. All traffic went through the typed relay.
- Shots: `context/B323/online-custom.png`, `context/B323/relay-url.png` (look right; the URL pill shrinks long URLs
  to fit; the screen is the existing code screen, which already scales to any aspect).

## Not verified / not 1:1
- Not 1:1 with the original: the original has no online mode, so this is our own front end (PLAN: menus are ours).
- The browser build compiles (`cargo check --target wasm32-unknown-unknown`) but wasn't run in a browser. The
  phone keyboard's `autocapitalize`/`inputmode` change hasn't been tried on a phone.
- The end-to-end ran through `hst net` with `--relay`, not by clicking through the window. The menu → cfg →
  `Net::start(cfg)` link is covered by the unit test plus `apply` reading `menu.net.cfg` (code unchanged).
- Desktop Ctrl+V in the field types `v` (same as the existing code entry); use the Paste button.
