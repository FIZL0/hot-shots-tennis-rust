# R1 → B54 player teeth: blocked (needs a close-up of the original's face)

Data (Ashley, pc00 c00; same layout on the others):
- The mouth is three untextured batches (PRIM 0x20, VU1 flags 0x0c = lit): S_HA teeth (16 tris, two-sided, vertex colour
  0x5f, material colour 255/128, +0x10 0.95), S_KUTINAKA mouth interior (vc 0x6f/0x80, colour (0.92, 0.20, 0.16)),
  S_BERO tongue (vc 0x80, colour (1.19, 0.48, 0.48)). The teeth normals point up and into the head.
- The port draws them on the untextured path: 8-bit colour = vc · material · (ambient + light · N·L), shown as n/255
  with no MODULATE ×2. Under the viewer's light (`gs::DEFAULT_LIGHT`, 0.5 ambient + 0.49) the teeth come out
  grey (≈125/255). Under a court light they get brighter in proportion to that light.
- The face morphs work: weights 1 on joy/anger/sorrow/doki_mouth (channels 2/4/6/8, forced through a temporary
  env hook that has since been removed) open the mouth. The upper teeth band, the dark red interior and the tongue
  all show. The interior's corners stick out past the lips as wedges (joy_mouth most). That is the target geometry
  at full weight.

Original: the only face close-up we captured is the "game won" celebration on court 0 (context/shots/b54/run2/s03.png,
crop_orig_s03.png). There the open smile shows a bright white teeth band over a dark interior, but at
640×480 the mouth is about 4 px tall. Nothing in that shot tells a lighting difference apart from a shape or depth
difference. The port's own run on the same match (context/shots/b54/portrun/) never put a face in close-up: the
cutaways showed the backs of the heads.

## Not verified / not 1:1
- Not verified what's "weird". Grey teeth under some lights, the interior wedges at full mouth weight and the white
  sliver between closed lips (neutral face, context/shots/b54/port_mouth.png) are all candidates. None was confirmed
  against the original, so nothing was changed.
- The original's VU1 colour for untextured lit batches (vc · material · light, n/255) is the port's model from
  B-series court work. It wasn't re-checked on a character mouth.

Unblock: a close-up of the original's open mouth, so the port can be matched with `--character N --motion 46`
plus `HST_EYE`. Either the character-select or win-screen 3D model, which draws under the same default light as the
port's viewer, or a PCSX2 screenshot with the camera poked onto a face. Alternatively, the user names what looks
weird (colour, wedges, or the sliver).
