# 1 — draw order (FINAL)

The first pass at this task (slot s3) was never committed: the slot was reset to main before `done.sh` finished.
This pass (slot s2) rebuilt the same change from that session's transcript and checked it again.

## What was off
Port `--play --stage 10 --shot` frames (`context/shots/b91/g_*`, `app_*`, `r_*`) against the original, slot 5,
court 10 (`tools/screenshot.sh`, `context/shots/b91/o_00..39`; the red marker is in o_08 and o_09).

- Placement, timing, which shots show it: no change since N4/N4a/B4, and these shots show no differences.
- Depth against the ground: not the cause. The marker's layers sit at y −0.001 to −0.007 in the model. With the root
  at y 0 the marker shows in full on courts 1, 4 and 10 (court 10's surface is at −0.0055). Checked by lifting the
  root to 0 and to 0.02 (`fix_*`).
- **Order against the court lines**: in the original the marker always draws over the lines (a line under the blob
  is tinted red). In the port a white line sometimes cut across the marker (g_9). Cause: Bevy sorts blended draws
  by mesh centre. The court-line mesh is huge, writes no Z, and has its centre near the middle of the court, so it
  won or lost against the marker depending on the camera, and the result flickered.
- **Order inside the marker**: the blob (`droppoint`) and the rings (`00@add`..`03@add`; on the smash marker
  `wa00`..`wa05`) are separate meshes with the same centre and the same bias. That is an unstable tie, so their
  order could swap from frame to frame.

## Fix
`effects.rs` `layer()`: each landing-marker material gets `depth_bias = 1e3 + rank`, where rank comes from the
material's mean height in the model, bottom layer first. That is the order the PS2's Z gives layers stacked a
millimetre apart. The marker then draws after every court mesh (the court's biases are 0 for its material order,
and −1e6 for B80's hole) and before the head markers (`LAST` = 1e4).

Check: with the marker forced onto the T and the sideline (a temporary override, not committed:
`context/shots/b91/v_*`), every frame shows the marker over the line, as the original does. After the rebuild,
`r_9..12` (stage 10, autoplay): no marker on the serve, and the red marker at the aim during the rally, like o_08.

## Not verified / not 1:1
- The original's own draw order (the marker's bucket against the court lines, the blob against the rings) wasn't
  read from the decompile. The order here comes from the screenshots (marker over the lines) and the layer heights.
- Only court 10 was checked in the original (slot 5 is the only bot-only state). Courts 1 and 4 were checked only in
  the port.
- The flicker was worked out from the sort tie. It wasn't captured as a sequence of frames.
- The marker now also draws after any other blended mesh within about 1 km, for example a ball or player shadow decal
  it overlaps. Whether the original draws it over or under those wasn't checked. (In o_08 the blob looks drawn over
  the near player's feet; it wasn't confirmed against the port.)
