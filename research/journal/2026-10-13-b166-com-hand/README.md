# B166: hand tag on COM cards (FINAL)

The character select drew the hand tag (B40e4/B79) only in the human branch of the card. It is now drawn on every
card that is picking or ready, COM seats included, in the same place (top right). `play/main_menu.rs chars`.

## The original
PCSX2 copy 3, slot 2 (doubles select), ✕ readies 1P, then the d-pad walks 2P (COM) over the grid
(`context/b166/h*.png`). With 2P's cursor on Carol, the COM card shows the "Left hander" tag top right, the same
place as a human's (h0 shows 1P on Carol with it too). So the original does draw it for COM players.

## Remaster
`HST_MENU=coms` shot (`context/b166/ours.png`): 2P COM Ashley shows "Right hander" top right, clear of the COM
label, costume shirt and rank pill.

## Not verified / not 1:1
- In the original the tag showed only on left-handers (COM Jun, Kaito and Suzuki, and the readied 1P Ashley, have
  none). The remaster shows "Right hander" too, as it already did for humans: menus are ours to design (PLAN
  2026-10-12) and the task asks for the same tag as the humans'.
- Mod characters show the tag from their own hand value, not checked against anything (no original for them).
