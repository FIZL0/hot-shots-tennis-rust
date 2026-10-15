# P29 — iOS: 120 Hz on ProMotion iPhones

## Why it ran at 60
iOS caps an iPhone app's display refresh at 60 Hz unless its Info.plist sets `CADisableMinimumFrameDurationOnPhone`
to true (iPads don't need it). On iOS, wgpu's Metal surface only has FIFO presentation (no `displaySyncEnabled`
there), so `AutoNoVsync` falls back to vsync: every present waits for the display, and the display was held at 60.
So `--fps 120` / uncapped could never go past 16.67 ms frames, as seen on the 15 Pro.

## Change
`tools/ios-ipa.sh`: `<key>CADisableMinimumFrameDurationOnPhone</key><true/>` in the generated Info.plist (checked it
parses with plistlib and the key reads true). No code change: the frame limit (`graphics.rs` `limit`) is our own
sleep, independent of the display, so:
- 60 (Phone preset, `Graphics::PHONE.fps == 60`, pinned by `text_and_flags_round_trip`): still sleeps to 16.67 ms frames.
- 120: sleeps to 8.33 ms, presents now land on the 120 Hz vsyncs.
- uncapped: paced by the display, now 120 Hz.

## Not verified / not 1:1
- On-device behaviour (the task's actual check): no iPhone here. Added to `plan/HUMAN.md`: build the IPA, on the
  15 Pro with Show FPS → Detailed / `hst-log.txt` read frame times at frame limit 60, 120 and uncapped.
- 60 cap on a 120 Hz display: our sleep isn't aligned to the 8.33 ms vsyncs, so a 60 fps run may pace unevenly
  (some frames 8.3, some 25 ms) where it was perfectly paced at a fixed 60 Hz before. If the log shows that, the fix
  is a `CADisplayLink`/`preferredFrameRateRange` of 60 on the Metal layer for the 60 cap (winit/wgpu don't expose it).
  Not done blind.
- Heat/GPU at 120: the Phone preset keeps 60, so only users who pick 120/uncapped get the doubled GPU load.
