# B40e2 — slide between menu screens (FINAL)

Menus no longer have to be 1:1 (PLAN, user 2026-10-12), so this is our own transition, not a port.

- `play/main_menu/slide.rs`: on a screen change the last screen's panels slide 160 units and fade out over 8 frames
  (60 Hz, smoothstep), then the new screen's slide in from the other side and fade in over 8 more. Forward (a later
  `Screen` variant) moves left, back moves right. The wall and title bar (`Anchor::Span`) don't move; they switch at
  the midpoint. Travel + fade instead of a full off-screen push so it works at any aspect ratio (a Centre panel
  pushed past 640 would still be visible on a wide window).
- No slide into or out of `Playing`, and none on the first frame (so `--shot` / `--menu` start screens are unchanged).
- The 3D character cards (`previews.rs`) follow the select's slide-in shift (no fade: they're render targets) and are
  hidden while the previous screen leaves.
- Test: `slide::tests::slides_out_then_in`.

## Not verified / not 1:1
- Not compared with the original's screen transitions: by design (menus are ours, user 2026-10-12).
- Not looked at on screen mid-slide: `--shot` starts on a screen without a transition, so there's no frame to grab.
- The description ticker and the 3D cards when *leaving* the select don't slide (the ticker changes text at once, the
  cards vanish at the start of the slide-out).
