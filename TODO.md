# TODO — Hot Shots Tennis remaster

When asked to "continue", start at the top of **Next up**. Details, evidence and memory maps for each item are in
`context/artifacts/` (git-ignored research journal); this file is only the plan.

## Play it now

```
cargo run -p hst -- "Hot Shots Tennis (USA).iso" --stage 1 --play
```
WASD move · J topspin · K slice · I flat · L lob · U drive (shot kind 4) · J/Space serves · mouse drag orbits camera.
Gamepad: left stick, A/B/X/Y, RB. `HST_AUTOPLAY=1` lets a bot play your side (unattended tests).
`--stage 01..11` picks the court; `--court 0..11` the bounce surface table.

## Done (ported and verified against the original)
- Disc/XB/TIM2/MDL/MTL readers; court layout placement; Bevy renderer with 60 Hz fixed sim + interpolation.
- Ball flight (drag, Magnus, gravity, curve/bend), ground bounces, rolling — matches 4524 recorded frames.
- Shot tables (TRAJ): lookup + launch speed/elevation/frames — matches 11 recorded strokes.
- Net contact (flat net from the game's predictor), material-based bounce response.

## Next up
1. **Shot spin/param records** — decision pending: read the table from the user's GAME.BIN at runtime (data-only
   offset exception to AGENT.md) or keep placeholders. Currently `KIND_SPIN` in `play.rs` (recorded typical values).
   Journal: `ball-physics/5-SHOT-PARAMS-READY.md`.
2. **Shot bearing / aim error** — the game launches toward target + offset (likely AI aim error from AIParam.csv).
3. **Real player movement** — speeds, acceleration, reach and swing timing per character (TParam.csv), replacing
   `RUN`, `REACH`, `SWING_WINDOW` in `play.rs`. Capture player objects over PINE from save states 3/5.
4. **Shot selection by input** — how the original maps buttons + timing + stick to class/kind/charge and to the
   normal↔charged table blend (37e420 param_1), the `_dw/_up` table variants, volleys (`voly*`), smashes (`smsh*`).
5. **Serve** — real toss (ported: straight up, spin 0.1) + serve tables (`serv0..3`), faults/lets, service boxes.
6. **AI** — target choice, shot type, positioning, reaction/error frames from AIParam.csv; uses the stored path
   (path recorder steps the ball 15 frames per frame against the court plane only).
7. **Court collision mesh** — net cord/posts, walls, fences (live ball uses the mesh query, not the flat net).
8. **Rules/scoring** — games/sets/tiebreak, doubles, side switching (current: point counter only).
9. **Characters** — player models/animation (MDL skinning, ANI/ANI2) loaded from the user's disc at runtime;
   stand-in capsule figures until then. Umpire/NPCs likewise.
10. **Rendering polish** — material blend modes (clouds, decals), cloud placement (category 14 records), lighting,
    upscaled texture pack from `replacements/` (PCSX2 hash names → disc textures), widescreen/HUD.
11. **Audio** — HD/BD banks (Sony VAG) + MIDI BGM.
12. **Menus/modes** — title, character select, exhibition, tournament.

## Known gaps / caveats
- Table lookups at an axis maximum read one cell past the table in the original; we clamp (never seen in captures).
- Stored-path fixtures can't verify net hits (the game records paths against the court plane only).
- Disc court folder ↔ physics court index mapping is unverified (slot 5 = court index 10).
