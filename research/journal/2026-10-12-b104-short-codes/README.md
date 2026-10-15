# B104 — short lobby codes, code entry, Copy Link (FINAL)

## What landed

- `net.rs`: a host rolls a 4-character code from `SYMBOLS` (`23456789ABCDEFGHJKLMNPQRSTUVWXYZ`: 32 symbols, no
  0/O/1/I; ~1M codes), re-rolling if the code's record was written less than `STALE` (90 s) ago. The code's ed25519
  key is seeded from `hst-lobby-code/1/<code>` (public: anyone can derive it); the host puts its ticket (a DNS packet
  with one `_hst` TXT record, so it is also a valid pkarr record) as a BEP 44 mutable item (seq = unix µs), and
  republishes every 30 s with its lobby listing. When the host starts the match or leaves, it stops and the code goes
  stale (free again) within 90 s. `Event::Short(code)` tells the menu; `connect()` takes a short code like a ticket
  ("Looking up …" → join, or "No open lobby has the code …").
- Where the record lives: native hosts put it on the mainline DHT **and** on the pkarr relay (`relay.pkarr.org`, HTTP
  PUT); the browser (no UDP) reads and writes only through the relay. The relay's CORS allows any origin.
  Found in testing: a DHT-only put was found by the relay in run1 but not in run2 (relay GET 404 for minutes while a
  native DHT lookup found it at once), so native hosts now also PUT to the relay. Native reqwest needs a rustls
  crypto provider: we install ring's (`rustls` dep, ring only, which iroh already builds).
- Menu (`online.rs`): Join by Code opens a code-entry screen: 4 pills plus an 8×4 symbol grid and Delete/Paste/Join/
  Back (pad); keys type directly (keyboard; typing doesn't fire menu bindings); on a touch screen `web/index.html`
  puts a transparent input over the canvas so a tap opens the OS keyboard (`web.rs` `code_entry`/`typed_code`).
  Paste still takes full tickets, `rtc…` codes, short codes or a whole link (`code_in` pulls `code=` out of it).
- Host lobby: the short code big beside the QR (QR now encodes the link), "OK: Copy Link" copies
  `https://fizl0.github.io/opentennis/?code=<code>`.
- Opening `?code=<code>` (or `HST_CODE=<code> hst` on desktop) skips the title and goes straight to character pick,
  joining that lobby after the pick; on a first visit the disc import runs first (same as before).

## Verified

- Tests: `net::short_code_parts` (symbols, `code_in` on links/lowercase, key derivation), `net::short_code_on_a_testnet`
  (local DHT testnet: host gets `Event::Short`, unknown code fails, a second endpoint joins by the code, both reach
  `Event::Start`), `online::join_by_typing_a_short_code` (menu: type the 4 characters on the grid, Join → connect with
  that code).
- Real DHT, two native instances: `hst net host` → `Short("QW6H")`, `hst net join QW6H` → both Room + Start. Later
  `hst net join PXVP` found a lobby the relay couldn't (that's the run2 finding above).
- Browser, link → lobby (headless Chromium, WebGL2, `--use-angle=vulkan`, profile with an imported pack; captures in
  `context/b104/run3/`): native `hst net host` → code KKAV, relay GET returns it 13 s after start; page
  `?webgl2&code=KKAV` lands on character pick (25 s shot), Enter picks, host log "f5909754 joined." → Start, browser
  shows the match's "Waiting for 1P..." (the CLI host doesn't play). run1 (code A7WA) did the same, run2 (PXVP) failed
  the lookup before the relay PUT existed.
- Screens: `context/b104/menu-code.png` (code entry), `menu-lobby.png` (host lobby with code + QR); wasm build checks.

## Not verified / not 1:1

- Phone OS keyboard input (`#codekeys` overlay) not tried on a real phone or touch emulation: no phone in an
  unattended run.
- A browser *hosting* (its relay PUT then a native or browser join by code) not run end to end: only the native host
  side was exercised; the browser's PUT is the same request as the native one that works.
- "Copy Link" in the browser (clipboard permission in a real tab) not checked; desktop copy uses the same clipboard
  path as the existing Copy Code.
- First visit with `?code=` (import, then character pick) only partly seen: run1's import visit ran on WebGPU, which
  loses its device in headless Chromium, so no screenshot of it; the visit after import (run3) is verified.
- Re-roll on a taken code is only checked at unit level (no forced collision on the real DHT).
- Anyone can overwrite a code's record (public key): a griefer could point a code elsewhere; the joiner still
  authenticates the endpoint it dials, and the host republishes every 30 s.
- Depends on `relay.pkarr.org` (third party) for browsers and as the fast path for native hosts; a down relay means
  browsers can't use short codes (full codes/links still work), native falls back to the DHT.
- The task said "~31 symbols"; we use 32 (power of two, no look-alikes).
- Headless WebGL2 logs bevy `slab_allocator` "Use-after-free" errors during the match load (run3 console); not from
  this task, the match still loaded.
- The link's character pick shows the generic (Doubles) layout until the lobby is joined after the pick, as the
  existing paste-a-code flow does.
