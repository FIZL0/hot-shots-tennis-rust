# B85 — the net's sound words (THIP / THUP / GONG)

The original's "thwip" is one of its material bounce words, which B39 had already ported (`hst_sim::bodyhit::BounceWords`,
`play/bodyhit.rs`). B85 checked the net cases against the game, fixed two differences and added a recording test.

## When it fires
- It fires on each new ball contact (the ball's bounce count +0x224 goes up to n, with n < 6) once the rally flag is set. Two
  gates apply: the shared cooldown (4 frames) and a cap of 10 words. Serve and rally behave the same.
- Which word depends on the material's effect kind (material table, 0x10 per entry, kind at +0xc), with texture 12 + kind:
  | Material on court 10 | Effect kind | Word |
  |---|---|---|
  | 2 (net cord) | 2 | THIP (`cf00-netup`) |
  | 26 (net body) | 3 | THUP (`cf00-netdown`) |
  | 22 (net post) | 1 | GONG (`cf00-netpole`) |
- After a soft-net contact, a second THIP is skipped. Kind 5 never pops.
- **Fix 1, the cap:** the cap of 10 counts live words. The game decrements +0x844 when a row-2 entry finishes fading out.
  B39 had read it as 10 per point. `BounceWords` now caps on `pops.len() < 10`.
- **Fix 2, the position:** the word sits at the contact record's point (record (n−1)·0x50 + 0x10). That point is the
  world-space hit centre − radius × normal. `Flight::step_world` used the model-space point carried through the node
  instead, which differs in the last bits off the net's tilted faces (z −1.4058e-7 vs the game's −1.5157e-7).
  `step_world` now returns the world-space form. Only the bounce words read `Flight::landing`.

## Where and how it draws
- The word stays on screen 26 frames: alpha 128 for 21 frames, then a 5-frame fade.
- It is drawn the same way as the body-hit words: anchor 0.25 m up and 0.5 m toward the camera, size from balloon row 2
  (0.3/0.18/0.3), no rise or swell. P25/B39's draw path (`draw_words`) already does this.

## Verification
- **Recordings:** `research/b85_net_rec.py` loads slot 5 and overwrites the live ball's velocity once. It then records
  the ball, its contact records and the game's pop-up object every frame, and takes screenshots.
  `research/b85_parse.py` prints the result. Commands are in `context/b85/run.sh`:
  | Recording | Velocity overwrite | Word |
  |---|---|---|
  | `cord_over3` | vsync 7864, (-0.056, -0.1, -0.55) | THIP |
  | `serve_let` | vsync 7785, (-0.148, 0, 0.58), a serve off the cord | THIP |
  | `net_low` | vsync 7864, (-0.056, -0.06, -0.55) | THUP |
  | `post_back` | vsync 7864, (-0.092, -0.092, -0.58) | GONG, then a material 48 word |
- **Test:** `live.rs net_touch_words_match_the_game` replays each recording from the frame after the overwrite with
  `step_world` on court 10. Every frame, the ball and the bounce words must equal the game's row-2 entries: texture,
  position bit for bit, and alpha. The runs cover 144, 154, 71 and 94 frames, so every word's whole lifecycle.
- **Screenshots:** `HST_BALL=x,y,z,vx,vy,vz` (new, `play/bodyhit.rs`) puts the ball on the original's overwritten path
  30 frames into each rally. Each word's spawn is logged with its time.
  - Original: `context/b85/cord_over2/orig_7891.png`.
  - Port, `--play --stage 10`, 2048×1152: `context/b85/port/wide_5.0.png` (crop `crop_5.0.png`).
  - Port, 1536×1152 (4:3): `context/b85/port/four3_word.png`.
  - Result: THIP above the left net cord at the same spot. The word is ~4.6% of the frame width in the port vs ~4.4% in
    the original.

## Not verified / not 1:1
- **The soft-net flag is also cleared when a timing balloon (rows 0/1/4/6) activates.** This is not ported. The port
  clears it only on a court-material contact, so a THIP straight after a timing balloon could differ. The recordings
  never hit this case.
- **The cap of 10 also gates the contact sounds in the game.** The port's sound path wasn't touched; only the words were
  checked.
- **The port's screenshots are not frame-matched to the original.** `--play` runs differ from launch to launch (serve
  speed and timing), so each word frame was picked from the log. The port's ball state also differs from slot 5's
  (spin, players), so the hit point is ~6 cm off the original's. The test is the exact check; the screenshots only
  compare by eye.
- **The 16:9 view is not identical.** The original ran widescreen-patched in PCSX2, and the port keeps 4:3's height.
  The two match by eye.
