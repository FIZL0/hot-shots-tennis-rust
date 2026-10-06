# TODO — Hot Shots Tennis remaster

When asked to "continue", start at the top of **Next up**. Details, evidence and memory maps for each item are in
`context/artifacts/` (git-ignored research journal); this file is only the plan.

## Play it now

```
cargo run -p hst -- "Hot Shots Tennis (USA).iso" --stage 1 --play
```
WASD/left stick move (and aim at contact) · Shift/LB sprint · J/A topspin · K/B slice · I/X flat · L/Y lob ·
U/RB drive · J/Space/A/Start serve · C/Select camera (follow/broadcast/free) · arrows/right stick turn camera.
`HST_AUTOPLAY=1` lets a bot play your side (unattended tests). Timing pop-ups: SWEET SPOT / QUICK / SLOW;
red dot = where the ball will bounce.
`--stage 01..11` picks the court; `--court 0..11` the bounce surface table.

## Done (ported and verified against the original)
- Disc/XB/TIM2/MDL/MTL readers; court layout placement; Bevy renderer with 60 Hz fixed sim + interpolation.
- Ball flight (drag, Magnus, gravity, curve/bend), ground bounces, rolling — matches 4524 recorded frames.
- Shot tables (TRAJ): lookup + launch speed/elevation/frames — matches 11 recorded strokes.
- Net contact (flat net from the game's predictor), material-based bounce response.
- Stroke timing (original): press → search the predicted path (28 frames) for a contact the per-player
  timing table allows (0–1.625 m high, within reach of a point 0.484 m in front, ball ≥ 0.5 m from the net,
  nearest in depth); grade = table[frame], offset = frame − 8 → SWEET SPOT / QUICK / SLOW. Pending presses
  re-check every frame. Swing animation is timed so contact lands on that frame.

## Next up
1. **Shot spin/param records** — decision pending: read the table from the user's GAME.BIN at runtime (data-only
   offset exception to AGENT.md) or keep placeholders. Currently `KIND_SPIN` in `play.rs` (recorded typical values).
   Journal: `ball-physics/5-SHOT-PARAMS-READY.md`.
2. **Shot bearing / aim error** — the game launches toward target + offset (likely AI aim error from AIParam.csv).
3. **Timing effects + other contact branches** — what the grade changes in the shot (table variant
   `_dw/_up`, power param of the hit entry), and the volley / smash (1.85–2.65 m, reach 1.1) / diving branches
   of the contact search (decomp 0x34d8a0). Reach growth while winding up is approximate (`find_contact`).
4. **Real player movement** — speeds, acceleration, reach and swing timing per character (TParam.csv), replacing
   `RUN`, `REACH`, `SWING_WINDOW` in `play.rs`. Capture player objects over PINE from save states 3/5.
5. **Shot selection by input** — how the original maps buttons + timing + stick to class/kind/charge and to the
   normal↔charged table blend (37e420 param_1), the `_dw/_up` table variants, volleys (`voly*`), smashes (`smsh*`).
6. **Serve** — real toss (ported: straight up, spin 0.1) + serve tables (`serv0..3`), faults/lets, service boxes.
7. **AI** — target choice, shot type, positioning, reaction/error frames from AIParam.csv; uses the stored path
   (path recorder steps the ball 15 frames per frame against the court plane only).
8. **Court collision mesh** — net cord/posts, walls, fences (live ball uses the mesh query, not the flat net).
9. **Rules/scoring** — games/sets/tiebreak, doubles, side switching (current: point counter only).
10. **Characters** — players are original stand-in athletes (`figure.rs`, procedural idle/run/swing/serve).
   Importing the original character models/animations is not being done by the assistant.
11. **Rendering polish** — material blend modes (clouds, decals), cloud placement (category 14 records), lighting,
    upscaled texture pack from `replacements/` (PCSX2 hash names → disc textures), widescreen/HUD.
12. **Audio** — HD/BD banks (Sony VAG) + MIDI BGM.
13. **Menus/modes** — title, character select, exhibition, tournament.

## Known gaps / caveats
- Table lookups at an axis maximum read one cell past the table in the original; we clamp (never seen in captures).
- Stored-path fixtures can't verify net hits (the game records paths against the court plane only).
- Disc court folder ↔ physics court index mapping is unverified (slot 5 = court index 10).
