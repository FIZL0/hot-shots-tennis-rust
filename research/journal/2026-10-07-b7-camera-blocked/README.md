# B7: court 1's camera looked blocked

The first court is `--stage 1` (North Academy Grounds, clay). `--court 0` loads no scenery. On that court the near school-wall fence (`sclwall-*` plant props) drew across the whole match view. The original shows no fence there, and from the camera side it also hides the far one.

The cause was culling. The fence materials are one-sided (MDL material header +0x24 = 0), and VU1 drops their back faces. We drew every material two-sided. See [1-ONE-SIDED-CULL-FINAL.md](1-ONE-SIDED-CULL-FINAL.md).
