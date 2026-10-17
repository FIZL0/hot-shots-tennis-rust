# B565: a lobby link's start leaves 1P open

**Cause.** `main_menu.rs` `setup`: a fresh start with no `HST_MENU` opens on the splash, `online::linked` turns it
into the link's `Screen::Assign`, and then the `HST_MENU` shot block (`!lobby && menu.screen != Screen::Main`) took
it for a shot setup: the keyboard on 1P, 2P/3P computers, 1P's cursor moved. `Menu::step`'s B392 rule (the first
device to press anything on an online Assign with 1P empty takes 1P and goes into the lobby) never fired, because 1P
was already the keyboard.

**Fix.** `setup` notes whether `linked` took the start (`linked`) and skips the shot block for it. The rejoin start
(M126, the same path) gets the fix too.

**Checked.** `HST_CODE=VTPP tools/shot.sh` (`context/b565/link.png`): the controllers screen with 1P–3P Open, no
keyboard, no computers. With `HST_KEYS='4:Enter …'` (`context/b565/after.png`): Enter puts the keyboard on 1P and
it goes straight into the join ("Looking up VTPP..."). The pad side is `online.rs`'s existing B392 unit test (any
device's press takes 1P).

## Not verified / not 1:1
- A real gamepad's first press in the browser: not tried. `--shot` runs read no pad (P62), and there's no browser
  pad here. The browser only reports a pad after its first button press, so that press may only connect the pad,
  and taking 1P might need a second press. The code path is the same as a key's.
- Menu flow, not gameplay: nothing to compare with the original.

## Wrap-up
`tools/done.sh` failed on one test that has nothing to do with this change: hst-base's
`mods::tests::rerigged_mod_plays_forehand_and_run` ("rest·inverse bind off by 0.0037"). It reads the local
`context/mods/test_pc00`, and hst-base doesn't depend on `hst`. It fails the same way when run on its own. I logged
it as a new B task and did done.sh's steps by hand: audit, sim clippy, `tools/check.sh -p hst` (228 passed), tick,
commit.
