# P12b3 call models — WIP (scale still open)

- Show logic (start 384280, update 388190, hold/fade 383fa0, msg 0xc → Change Sides at change ends)
  ported in hst-sim flow (`CallShow`, settle waits for ANI end; test `crates/hst-sim/tests/calls.rs`).
- Bevy: `play/popups.rs` `setup_calls`/`tick_calls`/`draw_calls`, overlay Camera3d layer 7;
  main-camera queries narrowed to `With<Orbit>`.
- Camera +0x680 dump: identity view, 640x224 field, 688.39 px/unit x at depth 1 → **horizontal**
  half-fov 25° (tan 0.4649); y 320.25 lines/unit → half-height tan 0.3497 (4:3).
  Our `CALL_FOV_DEG` treats 25° as vertical: fix to horizontal.
- Wrapper matrix +0x10 confirmed in RAM: rotY(π), translation z = 10 (wrapper.bin).
- **Open:** original on-screen sizes (alpha bbox of textures vs captures c3_20 Net, c1_40 Change
  Sides) give effective depth ≈ 1.72–1.79 for both models, i.e. ×~5.7 vs z = 10. Source unknown:
  check the MDL node/bind matrices our loader may drop (Net's single "" node, coatchange root),
  and the deferred draw path in 14df60 → 14de10 (per-model scale at model +0x124?).
- Temporary: `HST_CALL_SHOT` hack in popups.rs (TEMP-CALLSHOT) and `tmp_nodes` test in calls.rs — remove.
