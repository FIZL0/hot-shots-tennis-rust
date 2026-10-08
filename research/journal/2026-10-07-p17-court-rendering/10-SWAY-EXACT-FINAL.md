# P17m — costume sway, exact (P17k's gaps)

## Port (`crates/hst/src/noise.rs`, `character.rs`)

- Restart: `Motion::sets` counts every `Motion::set`, including a restart of the same motion. The deformers reset
  whenever it changes. Before, they reset only on a change of motion id or serial.
- Bone space: the game moves each position entry p_e (stored pre-weighted in its bone's space) by d_e before
  skinning, so the drawn vertex moves by Σ R_e·d_e, where R_e is the rotation of bone e as posed.
  - Bevy skins the bind-pose mesh, so `draw` (Update, after `animate`) writes the bind-space offset
    (Σ_k w_k·R_k·bind_k⁻¹)⁻¹ · Σ_e R_e·d_e, using the mesh's own skin weights.
  - In the rest pose this equals the old offset (rotated into the bind pose). The two differ on multi-bone
    vertices, such as pc00's skirt (bones 4 + 61/65).
- `step` (FixedUpdate, after `character::tick`) advances the phases, and `draw` redraws the moved vertices every
  drawn frame.
- Not capture-checked: Bevy's GPU skinning isn't bit-exact anyway. The viewer (`--character 0`) draws the skirt
  whole.

## VU half against a GS dump: matches

- `research/p17m_noise_dump.py` (copy 4, slot 5) finds the deformer objects (vtable 0x1d0cf8; +0x1c phase,
  +0x20 rate, +0x30 freq, +0x34 prev, +0x40 amp; `**(obj+0x14)` = the NOI entry, re-read every frame by the step at
  0x150700).
  - It sets each entry's rate to 0 and its amp ×30, then takes a GS dump of a single frame. The dump goes to
    `context/p17m/k30.gs`.
  - The GS-dump hotkey with a modifier (Shift+F8) doesn't arrive through `hyprctl send_shortcut`. Copy 4's
    `GSDumpSingleFrame` is now bound to F11.
  - Player 0 is pc00 (deformers s_body on node 1, s_maekami_00 on node 14, pony on node 69). All three had prev =
    phase = 0 (a motion set, with rate 0) before and after the dump.
- `research/p17m_noise_gs.py` checks the dump against `noidump`'s packets (`hst-data` bin):
  - It finds each packet's run in the last frame by length and kick (ADC) pattern, using the textured pass only:
    the untextured pass is the shadow, and the blended pass repeats the textured one.
  - 93 of pc00's 151 packets are found exactly once.
  - It fits every bone's 3×4 model→screen matrix to the noise-free vertices: 1644 vertices, mean residual 0.026 px.
  - Then it refits with the noise vertices moved under each hypothesis:

| hypothesis | skirt (node 1) mean / max | fringe (node 14) mean / max | fit vertices |
|---|---|---|---|
| port (lane x reads z, y x, z y) | 0.039 / 0.255 px | 0.023 / 0.045 px | 0.028 |
| unmoved | 2.29 / 20.2 | 10.9 / 56.9 | 0.78 |
| lanes x,y,z | 4.63 / 17.2 | 5.00 / 22.7 | 0.73 |
| lanes y,z,x | 5.73 / 28.7 | 10.0 / 82.7 | 1.05 |

- Reprojecting through the noise-free fit alone leaves the fringe (one bone) at 0.038 px. The skirt sits at
  0.31 px, max 2 px, on its bone-61 side, because the fit extrapolates bone 61's matrix out to the skirt.
- Test: `hst-sim/tests/noise_gs.rs` projects `hst_sim::noise::deform`'s entries through the fitted matrices
  (`context/fixtures/noise_gs_s05.txt`, written with `FIXTURE=`). Results: 101 vertices, mean 0.034 px,
  max 0.255 px; unmoved entries average 0.9 px.
- What this doesn't test: prev ≠ phase (rate 0 freezes both). P17k already checked which phase the VU reads
  against the EE capture.
- The pony packets (node 69, bones 19/20) aren't checked: no noise-free vertices of theirs were matched, so those
  bones can't be fitted.
