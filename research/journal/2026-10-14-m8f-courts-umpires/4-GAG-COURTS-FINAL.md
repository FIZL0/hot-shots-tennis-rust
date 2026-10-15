# 4 — Get a Grip courts → court mods (M8f5)

## What landed (all in ../opentennis-mods, commit e465bb8; nothing in this repo but this entry)
- `tools/psp2gltf/export_getagrip_courts.py`: the 16 courts (01–09, 13–18, 20) → `out/mods/gag_cNN_<stem>/`:
  `court.glb` (ground model less its sky + every haichi-placed object), `sky.glb` (dome + drifting clouds with their
  motion, unlit), `gimmick.glb` (csv-placed set pieces at their route's first point looping a resting motion, plus
  placed models that ship with motions: flags, c07's merry-go-round), `sound/bgm.wav` + `sound/ambience_0.wav` with
  loop points, `lensflare.png` (outdoor), `mod.json` (name from the US text, layout, surface, light/fill/fog from
  `1_courtsetting`, `indoor`, `gimmicks`, `todo`).
- Standard §2 additions: `__singles` / `__doubles` node variants (GaG ships the net and chair in both), `fill` /
  `fill_colour`, fog `amount`, `ambience_loop`, `lens_flare`, gimmick animations loop independently, `indoor` and
  `gimmicks` informational. M8f6's loader has to honour the variants and may ignore the rest at first.
- `notes/getagrip.md` "Courts": formats (haichi tracks, route.dat, the csvs), the name table, what's not exported.

## Layouts picked (same surface, then the nearest flat run-off; numbers in each `layout_why`)
01 Nagisa Hill → 5 Kar · 02 Sakuragaoka → 3 Park · 03 Sunflower → 0 Alps · 04 Laguna → 10 Wafu · 05 Fighill →
0 Alps · 06 Fields → 5 Kar · 07 Wonderland → 6 Extra · 08 Victor Mansion → 5 Kar · 09 Central → 0 Alps ·
13 Nakagawa → 2 Western · 14 University → 5 Kar · 15 Shiranui → 0 Alps · 16 Studio A → 3 Park · 17 Love Tennis →
3 Park · 18 Tianlongshan → 5 Kar · 20 Dubno → 3 Park.

## Checks
- Lines: on every court the ground's painted-line vertex rows sit exactly on HST's 5.485 / 4.115 / 11.885 / 6.40 m
  after the (0.01x, −0.01y, −0.01z) map; the GaG umpire seat locator (650, 172.8, 0) cm matches HST's chair
  (6.52, −1.77, 0) m, so the chair side agrees.
- Khronos glTF validator: 0 errors / 0 warnings on all 47 glbs (after fixing two found by it: c04's sheared
  net-shadow nodes are baked into their mesh, c17's zero normals point up).
- Cropped top/side renders of all 16 (`context/m8f5/tops.png`, `c01_*.png`): ground and objects line up, objects
  stand on the ground, net and chair on the court's +X side, indoor courts show their roofs from above. The first
  render caught `gag_cm` flipped upside down (rotation [0,1,0,0] → [1,0,0,0]) and a name-parsing bug that dropped
  `<model>__NN__singles` tracks (nets on 9 courts).

## Not verified / not 1:1
- Not seen in HST: there is no court mod loader until M8f6, so none of this has been drawn in a match or checked
  against the disc layout's collision in-engine; only the line rows and the chair locator were compared numerically.
- Not compared with Get a Grip running: no PSP emulator capture here, so lighting, fog, sky and gimmick placement
  are judged only from the data and offline renders, not side by side with the original.
- Light/fog values are copied raw from the csvs: GaG's ambient is 1,1,1 on most courts (its material model
  differs from HST's), and fog distances are assumed to be metres (the route data is in metres). How they look in
  HST is for M8f6 to tune by eye.
- Gimmick placement: route first point and the csv's default angle (sign of the yaw assumed); their reactions
  (escape along the route, break into `_b01`) live in the encrypted PSP program and aren't reproduced. Folders with
  no csv row (spawned by code) aren't placed; each court's `todo` lists them.
- `cNN_se.csv` one-shot ambience (birds, bells, buzzers): their cue names exist in no bank on the disc, so only the
  `env_coNN` loop ships; listed in `todo`.
- Layout choice: the run-off metric (flat ground within 5 cm, walked out from the line centres, cap 40 m) is crude
  and measures the ground model only (fences and walls are separate objects).
- Sky following the camera is the standard's assumption; whether GaG moves its dome with the camera isn't known.
- `201_court` (lower-detail copies of the same courts) not exported; `courtNN_gallery` is M8f7; no
  `previews/court.png` made (optional; needs the loader's renderer to look right).
