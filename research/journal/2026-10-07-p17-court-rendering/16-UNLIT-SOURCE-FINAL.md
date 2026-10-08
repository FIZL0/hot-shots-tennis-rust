# P17s6: where the sky/bg unlit flag comes from

**Question.** Slot 4's sky, bg and cloud draw records have object flag bit 1 (0x10e/0x30e), but their MDL batches
have 0x0c (P17s5 gap). Who sets it, and does a sky/bg draw check out against the GS dump?

**The setter.**
- The model constructor 15be80 (via 15bda0/15bde0) builds the draw records, then calls its own vtable +0xc
  (15c0a0) with `ctor flags | model data +0x44`. 15c0a0 ORs them into the model's +0x58 and into every record's
  +0x10 (15dce0 → 15df30).
- The basic model class 14c2e0 passes its last argument through as the ctor flags.
- The court load 32d4e0 builds the bg-dir categories with flags 2:
  - category 2 `_bg` (env +0x13c), 3 `_acc` (+0x140), 4 `_clo` (+0x144): `14c2e0(…, 2)`;
  - category 5 `_sky` (+0x148): `14c2e0(…, 2)`;
  - category 1 (holeend) passes the caller's flags (0).
  The category → dir table at 0x3fbf50 is hole, holeend, bg, bg, bg, bg, cloud, …; 3338f0 picks category 2–5 by
  the `_bg`/`_acc`/`_clo`/`_sky` in the name.
- In slot 4's RAM the sky, bg and clo model objects all have +0x58 = 2 and model data +0x44 = 0, so the 2 is the
  ctor's.
- The moving clouds (`/cloud/` dir, category 6, singles) are built by 32c4f0 → 32c780 → `15bda0(…, 3)`: alpha
  (bit 0) and unlit (bit 1).

**GS dump check** (`research/p17s6_unlit_gs.py context/p17s2/s4.gs 1.2 <noidump txt>…`).
- Unlit colour = min(⌊vc · mat · (A + L)⌋, 255). Court 4's sky/bg/clo light blocks are (0.7, 0.5, 0.02), so K = 1.2.
- park00_sky / park01_sky: all 16 packets (8–28 vertices, 352 of 792 channels not saturated; mat 1.99) have an
  exact GS run. K = 1.19 or 1.21 gives 0/16.
- park_bg: 16/28 exact. The other 12 aren't in the frame: no run has their ⌊(84, 90, 101)·1.2⌋ colour.
- park_clo: 20/20 exact (only 3–4 vertices each, so this one is weak).
- The P17s5 matcher found no sky match because it ignored the 255 clamp (the sky's mat is 1.99).

**Port.**
- `weather::apply` already marked the whole bg dir (`look.skies` + `look.bg`) unlit. That's exactly categories 2–5;
  only the comment changed.
- New: the moving clouds' materials are unlit too (main.rs `clouds`).

**Gap.** The moving clouds' light block isn't checked: there's no singles GS dump. The port gives them the court
light.
