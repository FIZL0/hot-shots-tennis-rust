# B62 — music in matches started from the main menu

Bug (user 2026-10-08): `Settings::default()` was all on, so `music` defaulted on and the menu passed `--music`.

Already fixed by B123 (51d6631, user 2026-10-12): Settings splits Menu Music (`menu_music`, default on) from
Match Music (`music`, default off; `SETTING_DEFAULTS`). `Settings::flags` passes `--music` only when Match Music
is on; `main.rs` defaults `music = false`, so the court BGM (`bgmg_NN`) only loads with `--music`. The menu's own
BGM drops to volume 0 on screens with no menu track (`main_menu.rs` BGM system), and `mode.rs` stops it before an
in-process match.

Checked: the user's `settings.txt` files (repo root, `~/.local/share/hst/`) both hold `music = off`, so no stale
`music = on` from the old all-on default survives. Existing test `main_menu::tests` (settings parse) asserts
`Settings::default().flags()` has no `--music`. No code change.

## Not verified / not 1:1
- The original plays court BGM in its matches; match music off by default is the user's choice (B123), not the
  original's behaviour. Front-end settings are ours to design (PLAN.md), so no PCSX2 check.
