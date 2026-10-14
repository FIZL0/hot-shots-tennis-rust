# B207 — phone keyboard name / code entry

- Return / Go on the phone keyboard: `index.html`'s `#codekeys` input (now `enterkeyhint="done"`) sends `\n` through
  `typed_code` and blurs (closes the keyboard). `online::typing` turns `\n` into a tap on Save / Join
  (`pointer::Pending`, `Act::Row(keys+2, OK)`), so it saves / joins exactly like ✕ on that button and leaves the entry.
- Touch page without a real controller (`web::touched() && !web::real_pad()`, `touch.js` `hstRealPad`): the letter grid
  and its Delete/Clear/Save/Back row aren't drawn (`online::touch_typing`), the legend keeps only OK (Save/Join) and
  Back, the cursor is held on Save / Join so the touch pad's ✕ saves, and the info line says "Tap to type …; Return
  saves it." A real pad connected brings the grid back (touch.js hides its own pad then too).
- Start on any pad: `typing` puts the cursor on Save / Join; the same Start is the pad's OK in `step`, so it saves from
  any hovered key (test `start_finishes_entry`, beside B159's `enter_finishes_entry`).
- The keyboard input now shows when the page is touched (`hstTouched`), not only on `(pointer: coarse)`, so a touch
  laptop that hides the grid can still open the keyboard.
- Covers every text entry: the player/host name and Join by Code are both `Screen::Code`.

## Not verified / not 1:1
- Not run on a real phone (none here): Return on iOS/Android keyboards, keyboard closing on blur, and the grid
  coming back on pad connect are checked by reading the code and a wasm `cargo check`, not on a device. Some Android
  keyboards send keyCode 229 for Go instead of Enter; if Return doesn't save there, that's why.
- Front-end only: no original behaviour to match (PLAN: menus are ours).
