# Ball physics — locating the model (PART: integrator found, not yet ported/verified)

Savestates from the user (~/Emulation/saves/ps2/states/SCUS-97610 (72326E67).NN.p2s):
3 = match start (P1 + 3 bots, doubles), 4 = mid-rally just after serve, 5 = all-bot game. Never overwrite 1-6/10.
Agent snapshots: 8, 9 (load 4, +1.0 s save 8, +0.25 s save 9). RAM = `7z e -so <p2s> eeMemory.bin` → context/ram/.
tools/pine.py has save_state/load_state (PINE opcodes 0x09/0x0A).

## Memory map (US 1.00, verified in RAM)
- game manager ptr: *(0x422f80). +0xa4 → ball object, +0x88 → (rally/serve state), +0x58 → shot serial.
- ball object (s08: 0x6d3350): +0x50 path array ptr, +0x54 path length, +0x58 current index.
  path entry 0x30 bytes: pos xyz 1 | vel xyz 0 | extra (bounce flags?). Values quantised to 1e-4.
  Playback rule exact: pos[n+1] = pos[n] + vel[n+1]  (semi-implicit Euler).
- AI copy of upcoming path: 0x424e90, 180 (0xb4) entries; filled by the "copy future path" routine.
- Second MT19937 at 0x427130 seeded with constant 1 at overlay init.

## Integrator (ball step, GAME overlay function at size 8692 using 9.8/3600)
per frame, v = ball+0x130:
1. drag:   v -= v̂ · |v|² · k            (k = +0x8c4 of shot/ball params)
2. magnus: v += normalize(v × axis) · |v| · spin(+0x1a4) · C   (axis = const vec, C = const in overlay data)
3. gravity: v.y += dt · gmul(+0x8c0) · 0.0027222224 (9.8/60²)   — game Y is DOWN here
4. pos += v (+ wind +0x240·dt), + sinusoidal side curve amplitude +0x254 over the flight (cos(π·t/T) diff)
Rounding to 1e-4 happens when the path is stored (not yet located).

## Next
- Read the rest of the step (bounce/net/court collision) + where path entries are written & rounded.
- Port to crates/hst-sim, regenerate the path from the shot params in s08 and diff against RAM (must match to 1e-4).

## Update — flight model VERIFIED (crates/hst-sim, commit "Shot capture over PINE")
- Live ball sim object = *(gm+0x98) (0x290 bytes): +0xac frame in path, +0xe0 pos, +0x130 vel, +0x1a4 spin,
  +0x1a8 spin applied at first bounce, +0x1ac first-bounce restitution scale, +0x58 shot class, +0x5c shot kind,
  +0x160 spin-orientation 3x4, +0x1c0 contact basis, +0x200 |vt|/r, +0x210 contact normal, +0x220 material,
  +0x224 bounce count, +0x54 → params (+0x8c0 gravity mul 0.9, +0x8c4 drag 0.04, +0x8d0 ball size 0.064).
- Stored path (gm+0xa4) is Y-UP and is filled progressively over the first frames of a shot.
- tools/trace_shots.py <slot> <secs> <dir>: frame-0 ball+params and final path per shot. 42 shots from slot 5
  in context/shots_s05. research/tools/check_shots.py / fixture_shots.py → context/fixtures/shots_s05.csv.
- Every flight segment has ONE constant spin; model exact (1e-8) on all consistent shots.
- PINE batching: one header, many ops (separate concatenated messages hang PCSX2's PINE).

## Bounce — next (2-BOUNCE)
Constants (court idx *(0x422f90)=4 in slot 4): friction mu 403c6c[court]=0.3, spin relax 403cdc[court]=0.4,
spin→vel 403d14[court]=0.2, material table 0x410b70 (16 B/row: b0 court-flag, b1 special, +4 restitution, +8 spin loss).
Still to read: 403c34[court] restitution, 403ca4[court] spin→normal coupling, 403d54..d70 shot multipliers.
Dataset: 40 first bounces with pre/post velocity + per-segment spins in shots_s05.
