# B235 — volume settings

Settings → **Sound** (new row between Graphics and Controls; the Settings screen now has nine rows at the
Graphics screen's top/pitch): sliders Master, Music, Sound Effects, Voices, 0..=10 each, ←/→ (pad, keys) or a tap on
a slider's left/right half (touch/mouse, as the Graphics values). Saved in settings.txt as `master_volume`,
`music_volume`, `sfx_volume`, `voice_volume` (`audio::Volumes`, third field of `Settings`), so the pause menu's
Settings has them too.

Mixer (`audio.rs`): every play id is tagged with a `Group` when it starts — by bank slot for the `play_at` /
`play_centre` / `play_toward` calls (1..=5 and 0x16 voices, 8 jingles = music, the rest SFX), the BGM's 16 channel
ids as music, plain `play` as SFX. `render` sums voices per group and scales each group by master × group at the
output, so a change applies at once to sounds already playing. No play's level, timing or `centre_frames` changes;
at full volume the output equals the old one (integer sums, gain 1.0). `audio::start` reads the saved volumes, so
command-line matches use them too.

Test: `audio::tests::each_volume_silences_its_group` (BGM, a system sound and the umpire alone; each slider at 0
silences only its group, master all, others leave the peak unchanged; half master ≈ half peak),
`main_menu::tests::sound_sliders` (screen flow, clamping, settings.txt round trip). Shots in `context/b235/`.

## Not verified / not 1:1
- Not compared with the original: it has no volume settings (front-end and presentation are ours, PLAN.md).
- Character 9's lob shout is on the system bank (slot 9 key 7), so it follows Sound Effects, not Voices.
- No drag on the sliders: touch steps one tenth per tap on either half, like the Graphics screen.
- On a portrait window the whole menu frame is squeezed horizontally (existing behaviour of every menu screen,
  not introduced here).
- Not heard live by ear in this unattended run; checked by the rendered-output test only.
