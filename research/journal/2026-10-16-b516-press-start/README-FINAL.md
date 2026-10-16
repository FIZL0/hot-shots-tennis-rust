# B516 — press-start splash, 1P claim

## The bug
Online on the keyboard, an idle connected gamepad took over the controls. Cause: `controls::PadSlots::update` let a
keyboard seat take any pad no other seat wanted whenever `cfg!(wasm32) || touch_pad::on()` (B244, meant for the
on-screen touch pad, which is a virtual gamepad). In the browser that is every pad, so a real pad sitting there joined
the keyboard's slot and its stick and buttons drove the keyboard player's character too.

Fix: `PadSlots::update_with(present, touch)`: the keyboard seat only ever takes an on-screen pad (`emote::is_touch`
by gamepad `Name`: web/touch.js's "Touch controls (STANDARD GAMEPAD)", the apps' "Touch controls (iPhone/Android)").
`play::read_input` passes them (a 3-line, local change). Test `controls::seated_pads` gained the case.

## The splash (`main_menu/splash.rs`)
- `Screen::Press`, a fresh start's first screen (`HST_MENU` unset; `press` / `press-claimed` for shots; anything else,
  e.g. `main`, skips it). First ✕ / Start (pad), Enter / Space (keyboard; the bound ✕ key too) or a tap/click (the
  keyboard's, as `pointer::device` has no 1P yet) sets `Menu::p1`; a "Player 1: <device>" card shows 1.2 s, then Main.
- `p1` seats the match: Local / Online put `p1` on 1P whoever chose the row (`step`), so a pad holding 1P is 1P on the
  roster and the keyboard isn't seated (it can still join as another player with ✕ like any device).
- The keyboard still works the menus: `Menu::hosts` lets it drive Mode and Confirm (screens with no seat of its own)
  whoever has 1P; the match's pause (Esc) was always open to it.
- 1P's pad unplugged/flat: the next device to press anything takes `p1` (`takes_lost_p1`, like B388's
  `takes_lost_1p`). On a touch screen a real pad's first press takes `p1` along with the seat (B247's rule).
- A lobby link's start (`online::linked`) skips the splash (its own "first device to press is the player"), and sets
  `p1` there.
- Backdrop: the confirm screen's court pictures (`court_pic`, already loaded with the menus), one Span quad per tap
  filling the window, the source rect cropped to the window's shape, zoomed 1.12× and panned across the slack over the
  8 s a court shows (+1.6 s crossfade, smoothstep), alternate courts panning the other way. Court mods with a picture
  loaded join the cycle. Dimmed (rgb 52/128) and softened by drawing 5 taps ±1.4 source px apart, each at alpha
  (w/n)/(1−w+j·w/n) so the stack averages them exactly, also under the crossfade's weight w.
- Logo: `web/logo.svg` rasterized once to `crates/hst/assets/logo.png` (`rsvg-convert -w 640`), `include_bytes!`,
  `Tex::Logo`. Text in the menu font, Centre anchored like every other screen: lays out at any aspect (checked
  16:9 and a 600×1000 portrait phone window).

Shots (context/b516/run1: press, fade, claimed, phone; run2: HST_KEYS Enter → claimed → Main → Enter → roster with the
keyboard as 1P).

## Split
The sharp baked backdrops (off-screen frame per court during a match, PNG beside settings, re-bake on size change,
modded courts) are **B524**: a match-side render job of its own. Until it lands the splash uses the fallback the task
names (disc photo dimmed + blurred).

## Not verified / not 1:1
- Menus/front-end are ours (PLAN): nothing to match against the original; the concept artifact
  (claude.ai/artifact/…) couldn't be opened from an unattended run, so the three screens follow the task's words
  (waiting, P1 claimed, phone) rather than the mock-up's exact look.
- Not run with a physical/virtual pad pressing on the splash: `--shot` runs read no pads (P62) and plain runs would
  grab other agents' vpads; the pad path is covered by the unit tests (`splash::tests`, `controls::seated_pads`) only.
- Not checked in the browser or on a phone build (touch prompt checked natively with `HST_TOUCH=1`).
- B388's stand-in stays: with 1P's pad gone mid-match the keyboard drives 1P until a pad fills it (a user decision
  predating this task; it only applies when the pad is out, never beside an idle one).
- Baked backdrops: not done here, B524.
