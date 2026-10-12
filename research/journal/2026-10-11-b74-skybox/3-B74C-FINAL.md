# B74c — leaning beach palm

## Cause
The plant record's tilt. The palm right of centre in `c02_1` is court 2's tree record at (15.15, 0, −9.42), yaw 2.78,
scale 0.445, code `"\0" "4" "\0" "\0"`: code byte 1 = 4 → a 4·π/36 = 20° tilt about X. `hst_sim::world::place` already
built that matrix (tilt from byte 1, extra turn from byte 0) for the collision world and the shade map, but `main.rs`
drew props with only translation · rotY(yaw) · scale. The model and the sway are not involved.

Court 2 has six records with a code tilt/turn (four trees, two structures, one with 'I' = 18 → π/2 folded to −π/2 + …).

## Fix
`main.rs` plant placement draws each prop with `world::place`'s matrix (row vectors → `Mat4::from_cols_array_2d`)
times the plant's scale. The net post (a tree within 1 m of the origin, which `place` returns as identity + flag) keeps
the old transform.

## Check
Same eye as B74b's `c02_1` (slot 10, vsync 33316, held view; eye from the hunt's printout), `HST_EYE` through a
wrapper `HST_BIN`, disc textures via an `HST_ISO` symlink: `context/b74c/port_c02_1_tilt.png`, blend with the
original `context/b74c/blend_c02_1_tilt.png`. Trunk, fronds and everything else coincide (before: trunk upright,
a second ghost trunk in the blend).

## Not verified / not 1:1
- Only the one palm was compared on screen. The other tilted/turned records (court 2 and other courts) follow the
  same matrix but weren't screenshotted.
- New PCSX2 screenshots weren't possible: Hyprland was in a recovery-config instance (no client list, F8 to the
  PCSX2 window never saved a PNG), so the comparison uses B74b's existing `c02_1.png`.
- The net post keeps its old yaw-only transform; the game uses identity for it in the collision world, and its
  drawing wasn't checked against this.
