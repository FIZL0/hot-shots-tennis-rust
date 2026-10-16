# B505 — F10 hides the HUD only with `--hud-key`

User hid the HUD with F10 by accident and couldn't get it back. Now `play::no_hud::KEY` (off by default) gates the
F10 toggle; `--hud-key` turns it on. `--no-hud` still starts with the HUD off (no toggle unless `--hud-key` too).
A scripted `F10` in `HST_KEYS` (auto_shot / promo runs) sets `KEY` before pressing, so unattended shots keep working.
Test: `no_hud::tests::f10_does_nothing_without_the_hud_key`. REFERENCE.md promo-shot line updated.

## Not verified / not 1:1
- none (our own feature, not in the original; no game check needed). A run with `HST_KEYS="…:F10"` wasn't
  re-shot: the scripted path is a one-line store before the same key press.
