# B474 — mute emote voice lines (Settings → Sound → Emote Voices)

- New switch **Emote Voices** (on by default) on the Sound page, after Match Music; saved as `emote_voices = on|off`
  in settings.txt (`Settings.0[EMOTE_VOICES]`, the ninth flag). The in-match pause screen embeds the same page.
- It drives `hst_base::audio::EMOTE_VOICES` (an AtomicBool, like `chat::HIDDEN`): set on parse, on each toggle and
  when ○ puts a picked row back.
- `play/emote.rs` `voice`: the line is still picked (`line`, which turns the emote key memory in `sound::Voice`,
  match state carried by snapshots) and only then dropped instead of pushed to `g.whooshes`. So the motion plays,
  every peer's key memory moves alike, and nothing touches the sim RNG. `whooshes` isn't in the snapshot.
- Tests: `settings_round_trip` (default on, off round-trips), `every_page` (the new row toggles, saves, ○ restores),
  `emote` tests in hst-play.

## Not verified / not 1:1
- Not checked against the original: an addition (the original has no emotes).
- Not heard in a running match by ear: the gate sits on the one push of the emote line, covered by reading; no
  test drives `voice` with the switch off (the switch is a process-wide static, and a test flipping it would race
  other tests in the same binary).
