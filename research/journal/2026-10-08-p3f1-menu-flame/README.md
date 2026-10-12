# P3f1 — the menus' rand() draws

Every `rand()` call before the match seed in P3f's menu logs (`context/p3f/menu_c1.bin`, `menu_c2.bin`, from
research/p3f_menu_log.py) comes from one menu effect, ported as `hst_sim::rng::MenuFlame`.

- `01-MENU-FLAME-FINAL.md`: what the effect is, its draws, the test.
- `02-FLAME-DRAW-FINAL.md`: P3f1a — the effect drawn (motion, spin, alpha, `f_s` sprites) behind the confirmation VS, run from P1's pick; match seed from its draws.
