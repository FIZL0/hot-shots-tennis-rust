# B301 — in-match key hints (FINAL)

- `play/key_hints.rs`: in a match, when nothing else fills the M14 legend strip (pause menu, its Settings, result
  screen take it), fills it with the match's keys from `controls.txt`: Move (WASD), Topspin, Slice, Lob, Serve,
  Emotes 1–4, Pause (Esc, fixed). Shown only when the keyboard was the last device (`legend::Last::Keys`), it drives a
  human seat (`PadSlots::keys` + `Pads::seats`) and not autoplay; a pad or touch hides it live (legend's own tracking).
  Rebound keys show as bound; an action with no key drops out.
- Setting: Settings → Display → Key Hints (On default), `key_hints = on|off` in settings.txt (`Settings` bools now 5).
  Applied through `key_hints::ON`, stored by the main menu every frame (with the aspect) and by the pause menu's
  Settings on leaving.
- `legend.rs`: `Act::Key` (keycaps only), `draw` made `pub(crate)` so the fill runs before it.
- Layout: the legend strip (640×448 y 436..448, spans the window, chips right-aligned, shrinks to fit), below the
  bottom Sets/Games strip. Checked with `--play` shots at 1280×720 and 1280×1600 (context/b301/).
- No sim effect: reads input resources only, writes the legend resource. Tests: `hints_follow_the_bindings_and_the_keyboard`,
  options `presets_and_switches` (Key Hints round-trips settings.txt).

## Not verified / not 1:1
- Addition, nothing in the original to compare.
- A match started straight from the command line (`--play`, no menu first) doesn't read `key_hints` from settings.txt:
  it shows the hints (default on) until the pause menu's Settings are opened.
- Not looked at with a pad switching live in a running match (no pad here); the hide uses the same `Last` tracking the
  menus' legend already switches on.
