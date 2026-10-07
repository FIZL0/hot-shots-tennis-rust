# Done prompts (full text as they were in TODO.md)

- [x] **Player 1 is Carol** (character 6, left-handed) by default; `--chars` still overrides. Note: the contact
  search and serve still use character 0's reach/timing data for everyone (P7/P3).

- [x] **P0 — Input replay harness.** Fixture reader `hst_sim::replay` (pad, globals, match, ball, player
  position = model matrix translation +0x3d70/+0x3d78), CLI `cargo run -p hst-sim --bin replay -- <fixture>
  [from] [to]` (CSV per frame), recorder `tools/record_p2m2.py`. round1 (`context/fixtures/round1.bin`, doubles,
  20791 frames, vsync 10018–30808, frame-exact, complete) replays score (29 points), verdicts (30) and line
  calls (158, bit-exact); `tests/replay.rs` checks the fixture itself. Input-driven player diffs land with P7
  (the port has no ported movement yet; P7 builds them on `Frame::{pad, player_pos}`). Next recording: add the
  live ball `*(gm+0x88)` and the rally block 0x3165f0. Journal: `research/journal/2026-10-06-p0-input-replay/`.
  - [x] **Capture `context/recordings/new_recording.p2m2`** (user-recorded, with its save state) →
    `context/fixtures/new_recording.bin` (`frames_live` layout, doubles, 5454 frames, vsync 5863–11316, no gaps,
    captured at NominalScalar 0.5). Contains player–player collision, a player hit by the ball, dives and
    smashes: the source for P4's dive branch, N5/smash contacts and player collision / ball-hits-player
    verification. The `replay` CLI picks the layout by file size.

- [x] **P0a — Bit-exact bounce.** Port the contact path with the original instruction order so
  `crates/hst-sim/tests/flights.rs` can require bit-exact position *and* velocity on every frame (today: airborne
  frames exact, first miss at the first bounce). Pieces: plane sweep `0x328ed0` (contact skin 1.005×r, eps 0.005,
  0.98 factor, VU0 dot products), contact point `0x12fc30`, VU0 helpers (normalize `0x125b10` with Q sqrt/div,
  cross `0x125ac8`, mat×vec `0x125a50`/`0x125a80`, inverse `0x32d190`), quaternion slerp (`0x12d620`, `0x12d860`,
  `0x12d340`) and the game's own sin/cos/acos (`0x115fb8`, `0x115d18`, …). Needs a `vu0` arithmetic model
  (PCSX2 VU path: chop, no add alignment — verify like `ps2`). Asm: `research/ghidra_scripts/DumpAsm.java` →
  `context/notes/asm_*.txt`.

- [x] **P0c — Live net hit, bit-exact.** Record a natural net contact (bot match, slot 5) per frame with
  `tools/trace_live.py`-style capture and replay it through `Flight` bit-exact. Needs the live collision path:
  world mesh query instead of the flat net (court object + grid objects, per-model triangle sweep) and the
  after-first-bounce redirect `+0x1b0`. Findings so far: `research/journal/2026-10-06-net-hit/FINDINGS.md`.
  Split (journal `research/journal/2026-10-06-net-hit/`):
  - [x] **P0c1 — Live-ball recording + per-frame replay.** `tools/record_live.py` (slot 5, every frame at
    0.25 speed: live ball `*(gm+0x88)`, predictor `*(gm+0x98)`, rally block) → `context/live/net_s05.bin`;
    `crates/hst-sim/tests/live.rs` steps each recorded frame through `Flight`: 12309 airborne frames bit-exact;
    189 contacts listed (court 144, fence 19, outside ground 12, net 5, net cord 3, 6 uncounted pushes) — court
    bounce velocity already exact, position off ~1e-4·r. Journal `1-LIVE-RECORDING-PART.md`.
  - [x] **P0c2 — Collision world data.** The live ball queries the court object and the world grid's props (net =
    `znet_s1000.mdl`, umpire chair, seats, hut, gate, trees); each is its `.mdl` loaded in place. `mdl::Model::collision`
    / `colliding(&mtl)` (static batches of materials with an attribute map; order node → material → batch → strip slot,
    restarts skipped), `node_inverse`; `mtl::Material::{two_sided, attributes}`, `Mtl::attributes` (embedded textures +
    16-byte material table). `crates/hst-data/tests/collision.rs`: slot 5 RAM vs disc, 15 models / 6368 triangles,
    node matrices, flags and maps bit-exact. Journal `2-COLLISION-DATA-FINAL.md`.
  - [x] **P0c2b — Collision world placement.** `hst_sim::world`: plant records (categories 15, 17–20, model, scale ≠ 0)
    → `place` (yaw, code-byte tilt/turn, net post at the origin) → `instance` (scaled node matrices, world→model,
    bounding sphere) in the game's list order (`list_order`), and the 20 m `grid` of colliding props.
    `crates/hst-sim/tests/world.rs`: 117 props / 66 colliding, matrices, spheres, grid box and all 704 cell lists
    bit-exact vs slot 5. Also fixed `vu0::add` (exact round-toward-zero for far smaller addends). Journal
    `3-PLACEMENT-FINAL.md`.
  - [x] **P0c3 — Triangle sweep bit-exact.** `hst_sim::mesh`: swept sphere vs triangle (face, edges, corners), the
    attribute-map ground material (texel at the hit's UV; `mdl::Model::wrap` = GS CLAMP modes), the per-object sweep
    (world→model, VU0 strict box test, two-sided passes, box shrink after a hit) and the world query (court, then grid
    props with the sphere pre-test). `Flight::step_world` + `ball::Material` = rows of the game's material table
    (`exe::Game::surfaces`); ghost material 32 (passes through, uncounted). `tests/live.rs` steps **all 12498 frames
    bit-exact** against court 10 built from the disc, contacts included (court 144, outside 12, walls 19, net 5,
    cord 3). Fix on the way: off-court spin loss also resets the kick's "spin before". Journal `4-TRIANGLE-SWEEP-FINAL.md`.
  - [x] **P0c4 — Net through the mesh in live play.** First-bounce turn `+0x1b0` (`0x379bd0`; per-character serve
    table, game's atan2f/atanf in `libm`) on `Shot::bounce_turn`, gated as the original (rally on, first sub-step,
    first bounce): round1's two turned serves now bit-exact, `line_calls.rs` exemption removed. `hst_sim::court`
    builds any court's collision world from the disc (multi-node props: local·parent); `--stage N` play steps the
    live ball through `step_world` (flat net only without a stage). `0x379ae0` (random bounce option) left for P21.
    Journal `5-LIVE-PLAY-FINAL.md`.

- [x] **N1d1 — Exact ANI sampler.** `hst_sim::pose::Clip`: clip bind (`0x13d950`: keyed tracks with a node, length,
  bone-length position scale), squad rotations with load-time control points (`0x175c00`/`0x1759f0`, VU0 slerp
  micro 0x288), Hermite positions with interval-scaled Catmull-Rom tangents (`0x17b160`/`0x17b010`), quat → rows
  (`0x175eb0`), time wrap/clamp (`0x1404c0`). App `character::animate` uses it. t: motion.rs `clip_sampler_ram`
  (1061 tracks, 20 players, 5 RAM dumps, bit-exact). j: 2026-10-06-n1-animations/4-SAMPLER-FINAL.md
