# P17s1 — the players' light (2026-10-08)

**Question.** In `context/p17s/s5.gs` the characters' lit/unlit ratio reaches 1.19 where the model light's
ambient + light gives 1.13 (the ball's stays ≤ 1.094). Is it the specular term, unnormalised skinned normals, or
something else?

**Answer: something else. The players have light factors of their own, one row per court.**

- It is not specular: every pc05 material has h14 = 0.
- It is not a normal scale: the data normals are unit, and a fit with a ×1.14 normal is worse than a fit with an
  intercept. The intercept fit gave ambient ≈ light ≈ 0.604, which pointed to the factors.
- The player object's setup (the function at 0x3484d0) copies the court light block. It sets its second light
  direction to (0, 0, ±1) by the sign of +0x12b0, the player's court end. It then runs 138410 with factors from the
  GAME overlay table at 0x41dac0 + court × 16. Each row is [ambient, light, third, fog value]. The court index is
  DAT_00422f90, which is 10 for slot 5. The 4th value goes through 1383d0 to +0x10c (fog), not to the light.

| court | ambient | light | third | fog |
|---|---|---|---|---|
| 1 | 0.8 | 0.4 | 0.03 | 244 |
| 2 | 0.75 | 0.68 | 0.05 | 255 |
| 3 | 0.55 | 0.58 | −0.1 | 240 |
| 4 | 0.75 | 0.35 | 0.1 | 255 |
| 5 | 0.6 | 0.5 | 0.1 | 255 |
| 6 | 0.6 | 0.5 | 0.05 | 235 |
| 7 | 0.6 | 0.51 | 0 | 255 |
| 8 | 0.7 | 0.6 | 0.1 | 255 |
| 9 | 0.72 | 1.0 | 0.02 | 245 |
| 10 | 0.7 | 0.7 | 0 | 245 |
| 11 | 0.55 | 0.51 | 0.22 | 240 |

**Checks.**

- **The DMA packets.** Slot 5's player draws in ee05.bin (e.g. 0x4c9610 and 0x4c9640) carry light = ambient =
  RGB(0.8627451, 0.8392157, 0.8392157) × 0.70 = (0.60392, 0.58745, 0.58745). The ball and NPCs keep the 0x423960
  block (light 0.5349, ambient 0.5953), and the ball's dump colours match that.
- **The dump, vertex for vertex.** Unlit vertices are (77,75,75) and the brightest are (154,150,150) at vc 0x80. For
  each bone I fitted a unit light direction with A = L = colour × 0.7:
  - pc05: 1368 of 1413 single-bone vertices are exact, and the rest are within ±1.
  - pc00: 207 of 217 are exact, and the rest are within ±1.
- **The VU1 lit colour** is FTOI0(vc·(109·max(n·L,0) + 110·N'y + 112) + 111·q), with
  q = N'z/(vf11.z + N'z·vf11.y) (Schlick on the H row). VU1 sums the bone-weighted normals and doesn't normalise
  them.
- **What the dump can't check.** Multi-bone vertices: their nodes (2, 54, 55, 58, 59, 3, 35) have no single-bone fit.
  There are 251 two-entry vertices whose normals come pre-weighted (|n_i| = w_i).

**Port.**

- `exe::Game::player_light(court)` reads the row, and `CourtLook.players` holds it.
- `gs::player_light` = the light row's RGB × [ambient, light]. It is not × the weather's scale.
- `shade::light` uses it for the players (`SwingTrail`) and keeps `gs::model_light` for the ball and NPCs.
- Test: `gs.rs model_light_row` (court 10 gives 0.60392/0.58745; ×128 gives 77,75,75 and 154,150,150).

**Gaps (new PLAN tasks).**

- **P17s3.** VU1 lights per vertex with the unnormalised weighted normal sum. gs.wgsl lights per fragment with a
  normalised normal, and Bevy's skinning normalises too. Porting it needs a vertex shader of GsMaterial's own.
- **The players' third light and fog value.** The third light has factor table[2] × RGB and direction (0, 0, ±1)
  from the court end. The fog value is table[3]. Both are added to P17s2.
