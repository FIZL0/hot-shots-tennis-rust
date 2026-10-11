# Halo CE port vs our matchmaking outline (M2f) — FINAL

User 2026-10-11: "look more into the Halo CE port I linked and see if they use our proposed outline for
matchmaking". The link is <https://hcemobile.com/> (M9). HCEMobile (by "Trevor") is built on
[cybersecurity/halo-ce-universal](https://github.com/cybersecurity/halo-ce-universal) (decomp-based port: Linux,
Windows, Android, web/wasm). Our outline: `plan/M2.md` → *Hosting and finding games*.

## What they do
- **No server of theirs**, peer-hosted (same goal as ours). Host = authority; it is Xbox system link tunnelled
  over the internet.
- **Discovery: public MQTT brokers** (EMQX, HiveMQ, Mosquitto; list in a brokers file), not a DHT. A public host
  publishes a *retained* listing (invite, name, map, mode, player counts) to its own topic
  `…/lobby/s/<hash of host key>`, republished every 30 s or on change; browsers drop a game unheard for 90 s.
  Rooms: topic = hash of a secret, messages sealed with a key derived from it, so brokers see nothing.
- **Invite code/link**: `halo://join/<code>` / `?join=<code>` = 16-byte hash of the host's public key + random
  16-byte token; password games encrypt the token (Argon2id → XChaCha20-Poly1305).
- **Quick Match** (HCEMobile): "joins the busiest public lobby, or opens your own if none are available".
  Also playlists/ranks/parties (HCEMobile extras).
- **NAT traversal**: STUN (public servers) + UDP hole punching, UPnP asked as a fallback (host on invite use,
  joiner after 5 s). **No relay by default**; HCEMobile adds a user TURN setting (server/user/password) for
  networks like some mobile carriers. Networks that block WebRTC keep browsers out.
- **Transport**: native = UDP (+ KCP for reliable streams); browser = WebRTC data channel, the native build acting
  as an ICE-lite / DTLS / SCTP peer so browsers and natives play together. LAN = broadcast (native only; a browser
  has no UDP, so browsers join natives over the internet path).
- **Netcode**: host-authoritative with client prediction and corrections, 30 Hz, inputs sent every tick with the
  3 previous ticks' buttons (a press is lost only with 4 datagrams lost in a row), version mismatch refused.
  Not lockstep/rollback.

## Compared with ours
Matches: no server of ours; host + open slots; browse list + quick match + join by code/QR carrying the host's
key; bring-your-own relay setting; LAN discovery separate from internet; version check on join (our M2e).

Differs: they use public MQTT brokers (WebSocket, works in the browser) where we planned the mainline DHT, which
needs UDP and so cannot run in the browser build (M3) at all. They default to no relay (STUN + UPnP, TURN opt-in);
we default to n0's relays. Their netcode is host-authoritative prediction; ours stays lockstep → rollback, which
fits a deterministic, bit-exact sim better (no state to correct).

## Worth adopting
1. Quick Match rule: join the *busiest* open lobby matching the filter, else host.
2. Lobby listings: per-host key, signed, retained, republish 30 s / on change, expire after 90 s unheard.
3. Browser-reachable discovery: MQTT-over-WSS on public brokers (or iroh's relay/pkarr path) as the browser's
   (and fallback) lobby list beside the DHT; listings E2E-sealed for private rooms.
4. Invite = host key hash + random token, optional password (Argon2id-sealed token).
5. UPnP as an extra traversal step before the relay; redundant input sending (last 3–4 ticks per packet) in M2g/M2h.

Sources: <https://hcemobile.com/>,
<https://github.com/cybersecurity/halo-ce-universal> (`port/linux/README.md`, `port/linux/NETCODE.md`,
`port/web/README.md`).
