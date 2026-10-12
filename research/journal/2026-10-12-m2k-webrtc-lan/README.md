# M2k — LAN in the browser build (WebRTC, codes by QR)

A browser can't send UDP, so M2j's LAN (iroh with the relay off, mDNS) isn't open to the browser build. Native mobile
(M9b) was dropped (user 2026-10-12), so this task applies: two browsers on one network connect over WebRTC with no
server, passing the offer and answer as QR codes.

## How it works

- **Mode**: in the browser, relay Off now means WebRTC LAN (`net.rs bind`: no iroh endpoint, `Ctx.ep = None`). A
  guest with an RTC code joins whatever its relay setting (the menu forces Off for it).
- **Transport** (`net/rtc.rs`):
  - An `RtcPeerConnection` with no ICE servers, so only host candidates.
  - Two pre-negotiated data channels: id 0 is reliable and ordered (the lobby's JSON `Msg`s); id 1 is unordered with
    no retransmits (ggrs datagrams).
  - `net.rs` now talks to a transport-neutral `Conn`, `Link` and `Inbox`; iroh's QUIC is the other variant. The lobby
    protocol and host relay are unchanged.
- **Codes**:
  - The SDP is boiled down to ufrag, pwd, the DTLS fingerprint and the host candidates (mDNS name, IPv4 or IPv6, plus
    the port), base64url-encoded behind `rtco` (offer) or `rtca` (answer). The answer also carries the offer's ufrag,
    so a stale answer is caught.
  - Codes are ~110–120 characters, and the QR is a URL (`<page>?code=…`).
  - `Sig::sdp` rebuilds a minimal SDP from a code.
  - ICE gathering is awaited (≤ 3 s) instead of trickled, since the code is passed once.
- **Flow**:
  1. Host: the Lobby shows the offer QR and the status "Players scan this code, then you scan the one their screen
     shows (or paste it here)."
  2. The guest's phone camera opens the URL, so the existing `?code=` path fills Join by Code.
  3. The guest joins and its lobby shows the answer QR ("Now the host scans this code…").
  4. The host's camera opens that URL in a new tab. `web/index.html` sees an `rtca` code, posts it on
     `BroadcastChannel('hst-rtc')`, tells the user to switch back, and doesn't start the game.
  5. The game tab's `paste()` → `rtc::deliver` hands it to the host loop. Pasting works too.
  6. After each answer the host makes a fresh offer for the next player.
- The RTC peer's name is its answer's ufrag.
- Find shows no lobbies in this mode, and Quick play hosts (the same as before).

## Verified

- **Two browsers, one headless Chromium** (`tools/web_lan.mjs`, `context/m2k/run1`):
  - The host went Online → Relay Off → Host → singles → lobby. The guest page was opened with the host's logged offer
    URL and went Join by Code → lobby.
  - The guest's answer URL was opened in a third page, which said "Sent to the game in your other tab".
  - Both pages logged `starting the online match`.
  - On the real GPU (`--use-angle=vulkan`), rollback (ggrs) ran 9000+ ticks with identical state hashes on both sides
    at ticks 0, 1800, 3600, 5400, 7200 and 9000. There was no desync and no disconnect.
  - RTC stats showed both channels carrying traffic (datagrams ~130/s each way once the match ran).
  - Screenshots of both matches: `context/m2k/run1/{host,guest}-match-*.png`.
- **The QR codes on screen** (`host-lobby.png`, `guest-lobby.png`) decode with `zbarimg` to exactly the offer and
  answer URLs.
- **Code round trip** (`net::rtc::tests`): Chrome SDP → code → SDP, links accepted, bad codes rejected, and answers
  reach the host only when they are answers.
- **`tools/check.sh`** passes.

## Findings on the way

- In one browser window, a page behind another draws no frames, so the driver gives each page its own window (as on a
  phone, where the game tab is in front again once the answer tab hands the code over).
- Another agent's virtual pad reached the headless pages through the Gamepad API and steered the menus. The driver
  hides gamepads from its pages.
- On SwiftShader, two matches at once run at ~5 fps. That's still connected and correct, just slow.
- M3's browser test used lockstep (the desktop host had `HST_NET_LOCKSTEP`). This is the first browser rollback
  match, and it works.

## Not verified / not 1:1

- **Two devices**: both browsers ran on one machine (mDNS/loopback host candidates). Two phones on a Wi-Fi network
  weren't available. Some networks block peer-to-peer traffic between clients (AP/client isolation), and nothing gets
  through those.
- **Phone cameras**: the scan → open URL → BroadcastChannel hand-off was simulated by opening the URL in another page
  of the same browser. A camera app may open its own in-app browser instead. BroadcastChannel then can't reach the
  game, and the codes must be copied and pasted. There's no in-page scanner (BarcodeDetector isn't in Firefox or
  Safari).
- **Background tabs on phones**: while the camera's tab is in front, the host's game tab may be throttled. The lobby
  (Pings) should hold for that long, but this wasn't tried.
- **Firefox and Safari**: only Chromium was available. The minimal rebuilt SDP (no trickle, `a=setup` actpass/active,
  mDNS candidates) is untested in their WebRTC stacks.
- **Desktop builds can't join browser LAN lobbies** (no WebRTC in the native build). This is by design: desktops have
  M2j's LAN.
- **Settings**: the browser doesn't keep `online.txt` (M3), so the host sets the relay to Off on each visit.
- **Not 1:1**: the original has no online play. Menus and status texts are ours (PLAN: front-end is ours to design).
