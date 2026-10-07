# P12b4 — finish banners and Set / Match Point

Scoreboard kinds drawn by the quad helper `3a6e70(u, v, uw, vh, x, y, w, h, sprite)`: w = −1 is native size,
otherwise scaled about the centre (x' = x + w/2 − w·s/2). Sprites: 40 `inpane_finish` (white sheet), 41–43
`inpane_finish00–02` (Service / Return / Smash Ace), 44 `inpane_Counter`, 45 `inpane_finish03` (On the Line),
46 `inpane_match`, 47 `inpane_set` — all in `AZUMA/INPANE/INPANE.XB0`.

## Finish banner

- Start (point decided, message 0x17): stage 1, t = 5, x = 208, y = 192 with On the Line else 212. Sound slot 9
  prog 0 key 9 vol 0x60 (slot 9 isn't loaded in the port: ponytail).
- Tick: t −1; t < 0: 1→2 (t 45), 2→3 (t 5), 3→off.
- Draw: word UV (0,24,224,40) from sprite 41+kind at (x,y) — the "Untouchable" row (v 0–24) is never drawn;
  On the Line UV (0,0,224,24) at (x+24, y+40). Stage 1 adds the white flash from sprite 40, UV (0, kind·64+24,
  224, 40) and (0,232,224,24) for On the Line, alpha 128·t/15 (multiply 0x88888889 → /15, truncating). Stage 3
  fades word and line at 128·t/15. Order: word, line, flash.
- Kind (only verdict call 0, not in replay): shots 1 → Service, 2 → Return, ≥3 → Smash if the last shot's
  branch is 4, Counter if the last shot was a counter (overrides); none after a net cord in. Each also needs the
  ball untouched by the winner's team's body (no body hits in the port: ponytail).
- On the Line (no body hit or contacts > 1): mark A = the last flight's first court bounce (y 0); B = A + fwd ·
  0.8·|v_xz| after the bounce. Armed at new point (0xe), disarmed at 0x17. Distances: serve
  dx = |x| or |x|−4.115 (nearer), dz = |z|−6.4; rally dx = |x|−4.115 (5.485 with 3+ players), dz = |z|−11.885.
  On if any |d| ≤ 0.1, or A inside and B outside the side line; else off if A beyond or B short of the end line;
  else on.

## Counter (set at the strike, shot record +0x3f06..)

- Hitter's kind topspin or flat, |offset| < 2, not framed; the previous shot (shared record, read before this
  strike overwrites it) a topspin/flat ground stroke, volley or dive.
- Volley (branch 2): TParam Voley POW (col 14) < the previous hitter's. Ground (branch 1): only with ≥ 2
  players, Strk POW (col 13); skipped when kind < 2, |z at swing start| < 6.4 and contact ≥ 0.6 (the port uses
  the player's z at the strike: ponytail).
- Gap = |difference|, the hit's `power_gap` (gap > 3 doubles key 8 in `hit_sounds`; was unmodelled before).

## Set / Match Point (message 0xe, not in replay)

- r per team 0 then 1, first ≠ −1: −1 no game point; 0 if not tiebreak and games < rules.games − (leading ? 1 :
  0); 1 if sets < rules.sets − 1; else 2 (3 when only the other team has a controller — jingle pitch only).
- r ∈ {−1, 0} arms; r > 0 when armed: jingle (slot 9 key 13 vol 0x40, pitch 1.0 / 1.1: not loaded), banner,
  disarm. Armed starts false; the port also runs it for the first point.
- Banner: stage 1 t 60, then stage 2 t 10, then off. UV (0,0,256,48) at (192,176,256,48) scaled by s:
  stage 1 s = 1 until t < 11 then 1 − 0.02·(10−t) (alpha 128); stage 2 s = 0.8 + 0.07·(10−t), alpha 128·t/10.

## Port

`popups::Finish` (in `Game`), hooks in `play.rs`: strike (counter + power gap), first court bounce, point
decided, new point. Banners draw on their own UI layer (`GlobalZIndex(1)`). Forced-banner check:
`context/shots_p12b4/port_counter.png`. Not compared frame by frame against a PCSX2 capture.
