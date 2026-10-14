# B322 — guests see the lobby's code and QR

Only the host's `host()` sent `Event::Code`/`Event::Short`, so a guest's lobby card had a spinner and "....".

- `net::Room` gained `short` (serde default, skipped when empty): the host's short code once `roll` has put it
  online (empty with none, e.g. no internet). `host()` sets it when the rolled code arrives, broadcasts the room to
  the guests and says it locally. Guests who join later get it in the room they're sent on join. The candidate shown
  at once (before publishing / re-rolls) isn't sent, because other people can't join with it yet.
- `online.rs` `Event::Room`: a guest (not hosting) with a non-empty `r.short` takes it as `v.short`, so `shared()`
  gives the same link the host has, the same QR (`QrImage`), Copy Link and the big code on the side card.
  The long ticket is never put in the room (B224).

Tests: the menu's code-join test checks that a guest's Room with `short` fills the code and QR link;
`short_code_on_a_testnet` (ignored, run) checks that the guest's Room carries the host's rolled code over a real
DHT testnet. Ran `tools/check.sh -p hst` (342 tests passed), the ignored test (passed) and `cargo check --target wasm32` (builds).

## Not verified / not 1:1
- Not checked by eye with two running instances (native or browser). The browser build uses the same `net.rs`
  host/join code, and the wasm build compiles, but I didn't run web_duo.
- A browser-LAN (WebRTC) host has no short code: each guest gets its own one-use offer. Guests of that host still
  show no code, because there's nothing reusable to pass on.
- This is front-end (menus) work, so there's no original to match against.
