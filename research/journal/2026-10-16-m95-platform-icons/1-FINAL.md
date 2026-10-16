# 1 — FINAL

## What changed
- `hst-base/src/device.rs` (new): `Device { platform, input }`, the build's `PLATFORM` (wasm32 → Browser, android,
  ios, else Desktop), and `watch` (PreUpdate): the input last used, from keyboard/mouse/gamepad/touch events (touch
  wins over the emulated mouse click a tap makes; a pad needs a button or stick past 0.5, so a resting stick's noise or
  a connection doesn't count). A change, or a new `Net`, is sent as `Cmd::Device`. Registered in `main.rs`.
- `net.rs`: `Msg::Hello` carries `device`; `Room::devices` (peer → Device, kept on rejoin/leave/back-to-lobby);
  `Msg::Device` / `Cmd::Device` for a switch mid-lobby (the host updates and rebroadcasts the Room; no relisting, no
  ready reset, ignored once playing). `Net` shares its device with the network task (`Ctx::device`), so the Hello has
  the value even when the network starts the same frame as Host/Join.
- Not in `Handshake` (M2e) or anything the match reads: no effect on the sim.
- `frame.rs`: `Plate::device` + `device_icons` (drawn from rounded quads: monitor, browser window, phone with notch,
  Android head; keyboard, pad, tap ripple); `lobby.rs` and `confirm.rs` fill it in.
- `online::sample`: devices for the shot rooms; `HST_MENU=lobby-ios` shows the iPhone icon.

## Checked
- `net::tests::devices_in_the_lobby`: host (desktop+pad) and guest (Android+touch) over a testnet see each other's
  devices from the Hello; the guest switching to a pad and the host to the keyboard reach the other side.
- `device::tests` (input priority, wire form). `tools/check.sh -p hst main_menu`: 157 passed; `upload_all` failed once
  under load and passes alone (unrelated: mods store packing threads).
- Shots `context/m95/run2/` (lobby, lobby-ios, lobby-guest, 600×1000 tall, 1600×600 wide): icons legible, scale
  with the plates, header names/pings untouched. (run1 had them in the header, which cut off names and pings.)

## Not verified / not 1:1
- Not tried with three real devices (browser, desktop with a pad, phone) at once: an unattended run has no phone or
  browser peer; the protocol is covered by the two-peer test and the drawing by shots.
- The server browser's list shows no player names (`Listing` has none), so it has no icons.
- A browser on a phone shows as Browser (the platform is the build, not the device). Touch on a phone browser shows
  the touch icon.
- An input switch during a match isn't sent (the lobby after the match shows the input from before it until the next
  switch).
- Icons are ours (front-end, not 1:1 by PLAN.md).
