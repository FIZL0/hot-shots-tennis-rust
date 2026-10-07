# P12b3 — call pop-ups (Let / Out / Net / Fault / Double Fault, Change Sides)

Ported to `crates/hst/src/play/popups.rs` (`setup_calls`, `tick_calls`, `draw_calls`), driven by
`hst_sim::flow`'s call show (`CallShow`, `PostPoint::call` / `set_call_anim`). Test: `crates/hst-sim/tests/calls.rs`.

## Models (scoreboard load `3d7250`-area loop over `i_let_00`… at `+0xca0`)

- Six `azuma/inpane/mdl` models in order let, out, net, fault, doublefault, coatchange; each played by the
  ANI/MOR/MTA player (`38e870` load, `38eeb0`), `hst_sim::effect::Effect` (sample, then advance, clamped).
  Judge call → model: 1 out, 2 fault, 3 double fault, 4 let, 5 out-after-net → Net.
- ANI lengths (frames): let 0, out 0, net 0, fault 15, doublefault 15, coatchange 70. Let/Out/Fault/Double Fault
  have no MOR. Every MTA track binds to its material name, MOR tracks to morph names. Net's MDL has one node `""`
  so its ANI tracks (Camera01, Plane01..04) bind nothing.
- The scenes are authored around a 3ds Max `Camera01` at z = 2; the game ignores it.
- **Every call model gets a uniform scale 5.7** (`14c5e0(5.7, model)` → model `+0x120`, applied in model space).
  This was the missing factor (RAM: model `+0xe0` world matrix, `+0x120` = 5.7, `+0x124` alpha 1.0).

## Draw (`38efd0`, from the scoreboard draw for kinds 5/7)

- World matrix at wrapper `+0x10`, set on first draw: identity · rotY(π), translation z = 10.
- Camera `DAT_0043b1d8 + 0x680` (dump `context/shots_p12b3/camera.bin`): eye 0, target +z, up +y, fov 50
  (horizontal), near 0.01, far 3072; screen matrix `+0x1e0` 688.39 px/unit x, 320.25 lines/unit y (640×224
  field) → horizontal half-angle 25°, 4:3. The −108 px projection shift (`+0x180` −0.2523·428) cancels the
  +108 viewport offset (`+0x3d0` 2156).
- RAM pokes (`context/shots_p12b3/probe/poke_*.png`): scaling `+0x1e0` ×1.5 stretches the text; fov `+0x40`,
  `+0x160`, `+0x3a0` change nothing; wrapper z 10 → 5 doubles the size (`probe/z10.png`, `z5.png`).
- Ours: a second Camera3d (order 1, no clear, layer 7, no tonemapping) with the match camera's fov rule
  (vertical = 2·atan(tan 25° · max(0.75, 1/aspect))); model root at z 10, rotY π, scale 5.7.
  Check: Net width 0.149 (orig) vs 0.144 (ours) of the screen, centre (0.502, 0.486) vs (0.501, 0.490).

## Show logic (start `384280`, update `388190`, hold/fade `383fa0`)

- Start (kinds 5/7): current model at `+0x58`, all six reset (frame 0, alpha 1), fade stage 1; kind 5 loads the
  countdown `0x42d6e8` from the table at `0x410f0c` (`call_wait`).
- Message 0xc on entering change ends (`323990`): `+0x151 = 0`, `+0x427 = 5`, kind 7 — Change Sides starts
  at change ends and never reaches the fade (80 change-ends ticks < 70 + 30).
- Each tick: players evaluate then advance; countdown −1; settled (`+0x158`) once the ANI has ended (time ≥ end;
  no ANI counts as ended) and the voice is idle or countdown < 1. Settled → `+0x14c` +1; past 0x1d → fade
  stage 2, t = 5: alpha = (128·t/5)/128, t −1; t < 0 ends the show. Captured trace: `context/shots_p12b3/calls.txt`
  (script `research/call_shots.py`).
- Settle now waits for the model's animation (`set_call_anim` from the model's ANI length).

## Side changes

- `effects::model` / `pose` / `Shown` are `pub(crate)`; a model without MOR loads with no morphs.
- With two Camera3d entities, the match-camera queries (main.rs, effects.rs, panel.rs, play.rs one line,
  popups.rs) now filter `With<Orbit>` / `Without<Orbit>`.
