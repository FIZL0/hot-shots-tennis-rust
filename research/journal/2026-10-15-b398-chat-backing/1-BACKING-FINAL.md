# B398 — chat lines on a slight backing

The chat lines and the open box/phrase drew straight over the menus and the HUD. Now each row
(`crates/hst/src/chat.rs`, `draw`) is its own node with a translucent dark (`BACKING`, black at 55 %), rounded
backing that hugs its text (`align_self: FlexStart`), with padding, corner radius and a small gap all sized on the
4:3 screen's height (or width on a tall window) like the font, so it scales uniformly. The backing's alpha follows the
line's fade, and an empty or faded-out row is `Display::None` so it takes no room. The tap "Chat" label, the box and
the pads' phrase get the same backing.

Also: the phrase prompt's "·" was a missing glyph (tofu box) in the chat font; now "R2 next, L2 say: …".

Shot hook: `HST_CHAT=lines|open|phrase` shows chat outside a room with five lines held at fixed ages (one fading),
and the box open on a draft / the phrase up. Checked with `tools/shot.sh` (through an `HST_BIN` wrapper for the env)
over `HST_MENU=lobby` (16:9 and a 600×900 tall window), `confirm` (box open), `roster2` (phrase) and a `--play`
match; shots in `context/b398/`. Each line reads cleanly, the faded line's backing fades with it.

## Not verified / not 1:1
- Not compared with the original: chat is ours (M20), there's nothing in the original to match.
- In the match shot, the "A Team" intro banner draws over the chat's middle line. That banner is drawn above the chat
  root, so the chat (and its backing) sits under it while the banner shows. Left alone, since it's only for the
  intro moment and moving chat's layer is outside this task.
