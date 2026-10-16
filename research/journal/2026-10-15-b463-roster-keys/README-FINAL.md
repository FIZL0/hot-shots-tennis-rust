# B463 — roster keys: 1–4 emotes, Q / E sections

On the character select (`Screen::Chars`):

- **Emotes**: the keyboard's emote keys (1–4 by default, the match's bound `EmoteUp..Right`) play that emote on the
  keyboard's seat's 3D preview, through B460's lobby emote path (`play/main_menu/plate_emote.rs`: same queue/rate
  limit, same motions and voice lines, back to the pose after). Pad equivalent: the right stick's **left / right**
  (emotes 3 / 4); its up / down already are B422's Win / Loss (`previews::buttons`), which are the same two motions
  (0x2e / 0x2f) as emotes 1 / 2, so a pad reaches all four. The touch pad's L1 / R1 stay the costume here (they emote
  only in the lobby / Ready screen). Not for Random (no figure) or a seat naming itself. Online select: sent to and
  heard from the peers like the lobby's.
- **Sections**: keyboard Q / E (and PageUp / PageDown) are now the sections, as Settings' tabs; the keyboard costume
  moved to **Z / X** (Q / E were the costume before; 1 / 3 were the sections). Pads unchanged (L1/R1 costume, L2/R2
  sections). The Mods page's kind tabs keep 1 / 3.
- **Legend**: `Act::Emote` "Emote" (keyboard `1-4`, pad a right-stick glyph with left / right arrows,
  `Glyph::Stick(false)`); Section shows Q E, Costume Z X (also on the costume pill). `menu_keys` takes a `select` flag.

Shots (context/b463/): keys.png, xbox.png (legend), emote.png (`HST_EMOTE=2 HST_MENU=chars`, both previews mid-emote).

Tests: `plate_emote::the_selects_emote_keys`, `legend::select_keys`; the legend probe test still passes.

## Not verified / not 1:1
- Front-end only (ours to design, PLAN.md); nothing to compare against the original.
- Online select emotes: wired through the lobby's `Cmd::Emote` but not run between two peers (no second machine in
  this run); covered only by the shared code path.
- The new keyboard costume keys (Z / X) are my choice: Q / E had to go to the sections and 1–4 to the emotes.
