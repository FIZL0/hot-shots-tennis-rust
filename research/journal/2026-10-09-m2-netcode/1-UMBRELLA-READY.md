# M2 umbrella — status check (2026-10-14)

M2 is the umbrella over M2a–M2l plus the lobby flow (B195, B201, B201a). Nothing to port in it directly; this
pass checks what's left.

## Done
M2a (same floats), M2b (tick input), M2c (hashable/copyable state), M2d (sounds once), M2e (seed + handshake),
M2g (lockstep), M2h (rollback, ggrs), M2i (mods online), M2k (browser LAN over WebRTC), B195 (lobby browser first),
B201 (shared lobby, live picks), B201a (lobby voting).

Re-run on this branch: `tools/check.sh -p hst three_peers_see_the_hosts_court_and_start_on_it` → passed (three
peers in one lobby see the host's court and Start into the match on it).

## Left
- **M2f** blocked: acceptance needs two machines on different networks (Find, code, own relay with n0's off).
  Steps are in PLAN.md *Needs the human* (M2f).
- **M2j** blocked: needs Android phones (M9b, dropped by the user 2026-10-12) or a second machine.
- **M2l** (stretch, practice while a hosted lobby waits): written for the 2026-10-09 flow (wait for slots to
  fill with humans). B201's lobby replaced that: the host starts any time with bots in empty seats, so there is
  no wait to fill. Likely obsolete; the human decides whether to drop it.

## Not verified / not 1:1
- Cross-network play (M2f) and native/browser GUI crossplay (B195/B201): need two machines; listed under *Needs the
  human*.
- Phones over LAN (M2j): native mobile was dropped.
- Not 1:1 by design: the original has no online play; the lobby UI is ours (PLAN).
