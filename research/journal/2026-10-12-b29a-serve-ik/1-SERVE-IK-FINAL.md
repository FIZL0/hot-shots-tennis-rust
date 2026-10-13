# 1 — serve IK ported and verified (FINAL)

See README. Done: solve + tables (oracle, 12000 inputs, 0 differ), app wiring (capture D bit-exact through
`serve_turn`), capture test over A/C/D + two bot serves.

## Not verified / not 1:1

- **The jump itself**: the jump scale is computed bit-exact, but the port keeps players on the ground (player y is
  not modelled; the original scales y by the root path during the swing, D: y −0.33 at contact). Presentation only:
  nothing in the sim reads the server's y.
- **Smash IK**: the original runs the same overhead solve (smash table, idx 0) for smashes; the table and solve are
  ported and oracle-checked, but `advance_stroke` still skips IK for motions ≥ 0x1c. Not in B29a's scope; would need
  its own capture to check the timing.
- **Left-handers**: no left-handed serve captured (the app test sets the save's right-handed Carol); a lefty goes
  through the same side scale as the strokes' solve, unchecked.
- **Order vs the hit in the contact tick**: the IK tick runs after the serve's hit logic (the press tick's sample
  already shows the tick after the solve). The hit's target reads the position from before that tick (11/12 of the
  step). The launch can't be checked bit-exact here anyway (B29: the RNG state isn't the capture's).
- **Swing speed 1 at countdown 1**: `ArmIk::tick`'s "speed back to 1" flag is ignored in the serve (the serve's motion
  speed is set in `motions`); as B29, presentation only.
- **Second size factor**: passed as 1, as the strokes' solve does in normal play; modes that change it weren't looked at.
