# B240 — the court-object pass's "eye buried" test

**Result:** the original hides props near the camera. The per-frame court-object pass (the one whose net test B74d
ported) runs a second test on every other court object of plant categories 17–20: if the eye is buried in the
object, the object isn't drawn that frame. The port now runs the same test on the drawn camera in every mode
(match, cut-away, replay, own camera, `HST_EYE`). It is presentation only: the sim's collision world is only read,
and the replay tests are untouched.

## The original
- **Which objects.** These are records without the near-camera (net) byte, with two more bytes clear. The setup sets
  one of them when the plant code's third character is `'0'`. The other is initialised to 0, and no setter was
  found. So `tested(category, code)` is (17..=20) and code[2] ≠ '0'. Alpine Heights' umpire chair (`w-shpn`, code
  `"\0\00\0"`) is excluded this way.
- **The test, in game space.** r = clamp(tan 20° / tan(hfov/2), 1, 15).
  - The sphere's centre is c = view→world(0, 0, r). A RAM read of the view block in slot 5 showed the matrix's rows
    are right/up/forward/eye, so c = eye + r·forward.
  - If |c − object centre| ≤ r + scale·model radius, three sphere sweeps of radius r onto c are run against the
    object's collision mesh: from 100 m above, and from ±(3·scale·radius + 2r) along x. If all three hit, the object
    isn't drawn.
  - The mesh sweep is the any-hit sibling of the ball's triangle-tree sweep. It transforms into model space and
    divides the radius by the scale.
- Objects without a collision mesh, such as Alpine Heights' grass clumps and big trees, can never be hidden by this.

## Port
- `net_hide::buried(view, models, object)` runs the test with `hst_sim::mesh::Object::sweep` against the court
  world `Game::world` built by `court::world`.
- `hide_around_eye` (PostUpdate, before transform propagation) builds the view from the drawn camera. Its eye and
  forward are taken from the camera transform, and its horizontal half-fov is inverted from the projection the way
  `fixed_eye` sets it.
- Each plant entity marked `HideAroundEye` in `main.rs` is matched once to its collision prop by placement, and
  hidden or shown each frame.

## Verification
- **Logging runs.** Alpine Heights: two singles and two doubles bot matches of 10 minutes each. Wild Green: one
  7-minute singles run. Each logged the drawn camera and the props hidden.
  - Alpine Heights hid props 91, 77/11, 89/88, 87 and 92/91/78 in some cut-aways. All of them were outside the
    telephoto frames, so on and off look alike (`context/b240/c6_a*`, `c6_b*`, `c6_p91*`).
  - Wild Green hid props 11, 41, 44 and 45 in cut-aways.
- **Before/after `--shot`s** (`HST_EYE` at the logged views, in `context/b240/`, `*_off` = test disabled):
  - `c8_p44`: the cut-away camera stood in a red flower bush (`c8hana-red`) that filled the whole screen. With the
    test, the view of the court is clear.
  - `c6_bed`: an Alpine Heights cut-away-style view from just outside a side flower bed (`kadan1`, x −9.7), looking
    across the court. The bed filled the lower two-thirds of the frame. With the test, the court is clear.
    This view is synthetic (see below).
  - `c8_match`: Wild Green's match camera stands in the forked tree behind the baseline (`c8futamata`, prop 54). The
    test hides that tree, and the frame looks the same, since the tree is behind and above the lens.
- **Unit test** `net_hide::tests::buried_props` (needs the ISO):
  - Alpine Heights' prop 91 and Wild Green's prop 44 are hidden for the logged views.
  - The match camera hides nothing on court 6, and only prop 54 on court 8.
  - It also checks `tested`. `tools/check.sh -p hst`: 251 passed.

## Not verified / not 1:1
- **No side-by-side with the original.** Getting the original onto Alpine Heights or Wild Green in a matching
  cut-away needs menu driving and a save of that moment, and slot 5 (the only bot-only save) is court 10. The test
  is ported from the decompile. The view-matrix layout was checked against RAM.
- **The offending Alpine Heights shot itself wasn't reproduced.** In 40 minutes of bot play on Alpine Heights no
  cut-away put a prop in frame in front of the lens; bots don't run out to the side flower beds. The Alpine
  Heights before/after pair is a synthetic `HST_EYE` view at a flower bed.
- **Props without collision meshes are never hidden.** Grass clumps and the large trees are drawn even when the
  camera is in them, as the original's test can't touch them. Wild Green's `c8_p41` view keeps a grass clump in
  the foreground. If the user still sees such a prop blocking a shot, that is outside the original's mechanism.
- **The umpire chair** (code '0') is excluded as the original. Alpine Heights' doubles umpire-side cut-away shows
  one corner of it at the bottom of the frame (`context/b240/c6_ump_on.png`).
- **Gates not ported.** The pass's outer gates (game mode, player count, replay flag, as the net test) and two more
  globals that gate the category 17–20 loop aren't ported. The test runs in every mode.
- **One byte assumed 0.** The second byte that has to be clear for the test is assumed 0, since no setter was
  found.
- **The sweep is the ball's sweep.** It drops texel-0 material hits, and it uses plain f32, not the game's
  any-hit walk and `ps2` arithmetic. This is presentation only.
- **The fov comes from the drawn projection**, not the game's own view block. In the match and cut-away views it is
  the same value, `fixed_eye`'s inverse.
- **Season-0 collision.** The collision world uses season-0 stems, as `court::world` does, while the drawn models
  are the seasoned ones.
- B185's `HideInside` hack for Grand Palais's w-house was left alone. The ported test may cover it, but that wasn't
  checked.
