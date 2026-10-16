// On-screen controls for phones and tablets, and the page's other mobile bits. The controls are a virtual pad in the
// browser's standard layout added to `navigator.getGamepads()`, which the game polls like any controller (gilrs), so
// menus and matches read them through the same bindings as a real pad (✕ = shot / OK, ○ = slice / back, △ = lob,
// □, L1/R1, Select = swap hand, Start = pause; in a match L1/R1 also emote, B271: the game knows this pad by its id). They show on touch screens (menus and matches) and hide while a real
// controller is connected. In the menus, which also take taps (B221), the stick is a fixed circle at the bottom left so
// taps anywhere else reach the menu; in a match it appears wherever the left thumb lands.
// Also here: audio unlocks on the first tap or key (browsers start every AudioContext suspended), and a touch screen
// goes fullscreen on the first tap, either way up (B112) (where the browser allows it; an installed app starts that way).
// B312: in an online lobby or match a chat button (a speech bubble) sits beside Start, styled as Select/Start; it isn't
// a pad button: a tap on it opens the game's chat box (`chat.rs`, which shows it through `hstTouchChat`) and the phone's
// keyboard at once (B531: one tap); while the box is open it stays up and a tap brings the keyboard back.

// every AudioContext the game makes, resumed on the next gesture
const contexts = [];
const AC = window.AudioContext ?? window.webkitAudioContext;
if (AC) {
  window.AudioContext = class extends AC {
    constructor(...a) {
      super(...a);
      contexts.push(this);
      this.end = 0;
    }
    // cpal's WebAudio output plays two buffers in turn, each started at its own time cursor. After the page stalls
    // (loading, a slow frame, a hidden tab) that cursor lags the context's clock, both buffers start "now" and overlap,
    // and the game's audio (music tempo included) is pulled at twice real time until the cursor catches up: queue every
    // buffer after the one before instead, so audio always plays at real time.
    createBufferSource() {
      const s = super.createBufferSource();
      const start = s.start;
      s.start = (when = 0, ...rest) => {
        const at = Math.max(when, this.currentTime, this.end);
        this.end = at + (s.buffer?.duration ?? 0);
        start.call(s, at, ...rest);
      };
      return s;
    }
  };
}
const coarse = matchMedia('(pointer: coarse)');
// M96: the site (index.html #disc, a scrolling page) is up: no pad, no fullscreen, no edge-swipe guard. A reload that goes
// straight back into the game (the Mods Reload, an update) hides #disc with `html[data-game]` only, not `hidden`
const site = () => !('game' in document.documentElement.dataset) && document.getElementById('disc')?.hidden === false;
const unlock = () => {
  for (const c of contexts) if (c.state !== 'running') c.resume().catch(() => {});
  if (coarse.matches && !site() && !document.fullscreenElement && !matchMedia('(display-mode: fullscreen), (display-mode: standalone)').matches) {
    // B112: either way up (the game follows a turn live)
    document.documentElement.requestFullscreen?.({ navigationUI: 'hide' })?.catch(() => {});
  }
};
for (const e of ['pointerup', 'touchend', 'keydown']) addEventListener(e, unlock, { capture: true });

// the standard mapping's button indices
const B = { cross: 0, circle: 1, square: 2, triangle: 3, l1: 4, r1: 5, select: 8, start: 9 };
const realPads = navigator.getGamepads?.bind(navigator) ?? (() => []);
const anyReal = () => [...realPads()].some(g => g && g.connected);
let shown = false;
// B118: the press buzz (Settings → Mobile)
let buzz = true;
// B329: the buzz. Android's browsers have `navigator.vibrate` (Chrome skips it while the phone's ringer is on Silent, and
// only after the page's first tap); 8 ms was too short for many phones' motors to be felt at all. iOS Safari has no
// vibration API, and iOS 26.5 closed the switch-checkbox haptic trick, so an iPhone browser gets nothing.
const vibrates = typeof navigator.vibrate === 'function';
const buzzOnce = () => vibrates && navigator.vibrate(15);
// what the Vibration setting's text says: 'vibrate' or 'none'
window.hstVibration = () => (vibrates ? 'vibrate' : 'none');
// B165: the game's polls, counted in animation frames. A tap shorter than a frame (or one that lands while the page is
// busy, its down and up queued together) would come and go between two polls and never be seen: a button stays down
// until a poll has seen it and a later frame has started, so every tap counts once.
let tick = 0;
// the game's menus take taps themselves (B221): there the stick keeps to its own circle (`.menu`), drawn at rest
let menu = false;

const css = `
/* B333: over the canvas, sized like it from the screen's current size (index.html) */
.touch { position: fixed; left: 0; top: 0; width: var(--app-w, 100vw); height: var(--app-h, 100dvh); z-index: 10; pointer-events: none; user-select: none; -webkit-user-select: none;
  --u: min(7vw, 12.5vh); font: 600 calc(var(--u) * .42) system-ui, sans-serif; color: #fff; }
.touch[hidden] { display: none; }
/* B314: the inset the layer's bottom keeps clear (the home indicator); its top keeps the screen's top clear (the
   shoulders, Select, Start) */
.touch { --safe-b: env(safe-area-inset-bottom); }
.touch .zone { position: absolute; left: 0; top: 18%; bottom: 0; width: 45%; pointer-events: auto; touch-action: none; }
.touch .base, .touch .knob { position: absolute; border-radius: 50%; transform: translate(-50%, -50%); }
.touch.menu .zone { top: auto; width: calc(var(--u) * 2.6); height: calc(var(--u) * 2.6); border-radius: 50%;
  left: calc(env(safe-area-inset-left) + var(--u) * .4); bottom: calc(var(--safe-b) + var(--u) * .4); }
.touch .base { width: calc(var(--u) * 2.6); height: calc(var(--u) * 2.6); background: #fff2; border: 2px solid #fff5; }
.touch .knob { width: calc(var(--u) * 1.2); height: calc(var(--u) * 1.2); background: #fff6; }
.touch button { position: absolute; pointer-events: auto; touch-action: none; border: 2px solid #fff5; background: #0004;
  color: inherit; font: inherit; border-radius: 50%; width: calc(var(--u) * 1.25); height: calc(var(--u) * 1.25); padding: 0; }
.touch button.on { background: #fff6; }
/* B165: a touch just past a button's edge still hits it (the face buttons' extended circles just don't overlap) */
.touch button::after { content: ''; position: absolute; inset: calc(var(--u) * -.2); border-radius: inherit; }
.touch button.small { border-radius: calc(var(--u) * .3); width: calc(var(--u) * 1.5); height: calc(var(--u) * .7);
  font-size: calc(var(--u) * .3); opacity: .7; }
/* B312: chat, an icon on Select/Start's small button: right of Start; upright there's no room on that row (L1/R1 all
   but touch Select/Start), so it sits centred under the two */
.touch button.chat { left: calc(50% + var(--u) * 2.4); top: calc(env(safe-area-inset-top) + var(--u) * .3); }
@media (orientation: portrait) { .touch button.chat { left: calc(50% - var(--u) * .75); top: calc(env(safe-area-inset-top) + var(--u) * 1.2); } }
.touch button.chat svg { width: calc(var(--u) * .42); height: calc(var(--u) * .42); vertical-align: middle; fill: currentColor; }
/* B112: upright, the pad sits under the picture (the 4:3 screen is letterboxed mid-height), sized by the width */
@media (orientation: portrait) { .touch { --u: min(12vw, 7vh); } .touch:not(.menu) .zone { top: 50%; } }
/* B313: upright, the shoulders and Select/Start sit over the face buttons, not along the top edge:
   there the status bar and the notch (viewport-fit=cover) covered them, and the thumbs are at the bottom anyway */
@media (orientation: portrait) { .touch button.small { top: auto !important; bottom: calc(env(safe-area-inset-bottom) + var(--u) * 4.1) !important; } }
/* B531: chat's spot upright, centred under Select/Start (the rule above had put it on top of them) */
@media (orientation: portrait) { .touch button.small.chat { bottom: calc(env(safe-area-inset-bottom) + var(--u) * 3.2) !important; } }
`;
// face buttons in a diamond for the right thumb, shoulders and Select/Start along the top
const right = (x, y) => `right: calc(env(safe-area-inset-right) + var(--u) * ${x}); bottom: calc(var(--safe-b) + var(--u) * ${y})`;
const layout = [
  ['cross', '✕', right(1.4, 0.3)],
  ['circle', '○', right(0.2, 1.5)],
  ['square', '□', right(2.6, 1.5)],
  ['triangle', '△', right(1.4, 2.7)],
  ['l1', 'L1', 'left: calc(env(safe-area-inset-left) + var(--u) * .4); top: calc(env(safe-area-inset-top) + var(--u) * .3)', 'small'],
  ['r1', 'R1', 'right: calc(env(safe-area-inset-right) + var(--u) * .4); top: calc(env(safe-area-inset-top) + var(--u) * .3)', 'small'],
  ['select', 'SELECT', 'left: calc(50% - var(--u) * 1.8); top: calc(env(safe-area-inset-top) + var(--u) * .3)', 'small'],
  ['start', 'START', 'left: calc(50% + var(--u) * .3); top: calc(env(safe-area-inset-top) + var(--u) * .3)', 'small'],
];
const bubble = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 3h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H10l-5 4v-4H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/></svg>';
// B312: the chat button, right of Start (the near player's layer only: a 1v1 on one phone is never online)
const chatButton = `<button data-chat class="small chat" aria-label="Chat" hidden>${bubble}</button>`;
let chatTaps = 0;
let chatShown = false;
const style = document.createElement('style');
style.textContent = css;
document.head.append(style);

// The on-screen pad: its buttons, stick and fingers, listed as gamepad `index`.
const side = index => {
  const buttons = Array.from({ length: 17 }, () => ({ pressed: false, touched: false, value: 0 }));
  const pad = {
    id: 'Touch controls (STANDARD GAMEPAD)',
    index, // past the browser's own slots (0–3)
    connected: true,
    mapping: 'standard',
    timestamp: 0,
    axes: [0, 0, 0, 0],
    buttons,
    vibrationActuator: null,
  };
  const seenAt = {}; // button -> the frame a poll first saw it down
  const held = Object.fromEntries(Object.keys(B).map(k => [k, new Set()])); // button -> the pointers on it
  const root = document.createElement('div');
  root.className = 'touch';
  root.id = 'touch';
  root.hidden = true;
  root.innerHTML = `<div class="zone"><div class="base" hidden></div><div class="knob" hidden></div></div>`
    + layout.map(([k, label, pos, cls = '']) => `<button data-b="${k}" class="${cls}" style="${pos}">${label}</button>`).join('')
    + chatButton;
  document.body.append(root);

  const stamp = () => (pad.timestamp = performance.now());
  const set = (k, on) => {
    const b = buttons[B[k]];
    b.pressed = b.touched = on;
    b.value = on ? 1 : 0;
    root.querySelector(`[data-b="${k}"]`).classList.toggle('on', on);
    if (on && buzz) buzzOnce();
    else if (!on) delete seenAt[k];
    stamp();
  };
  // up once no pointer holds it and the game has had it down for a frame
  const settle = k => {
    if (!held[k].size && buttons[B[k]].pressed && seenAt[k] !== undefined && tick > seenAt[k]) set(k, false);
  };
  // each finger tracked on its own (B165): a second finger on a button neither re-presses it nor lets it go when it lifts
  const press = (k, id) => {
    held[k].add(id);
    // ponytail: a tap landing while the last one still waits on its frame merges with it (taps < 2 frames apart)
    if (!buttons[B[k]].pressed) set(k, true);
    else root.querySelector(`[data-b="${k}"]`).classList.add('on');
  };
  const lift = (k, id) => {
    if (!held[k].delete(id)) return;
    if (!held[k].size) root.querySelector(`[data-b="${k}"]`).classList.remove('on');
    settle(k);
  };
  // a mouse or pen (`?touch` on a desktop); fingers come through the touch events below
  for (const el of root.querySelectorAll('button[data-b]')) {
    const k = el.dataset.b;
    el.addEventListener('pointerdown', e => {
      if (e.pointerType === 'touch') return;
      e.preventDefault();
      el.setPointerCapture(e.pointerId);
      press(k, `p${e.pointerId}`);
    });
    for (const t of ['pointerup', 'pointercancel', 'lostpointercapture']) el.addEventListener(t, e => lift(k, `p${e.pointerId}`));
    el.addEventListener('contextmenu', e => e.preventDefault());
  }
  // B165: no browser gesture (pinch zoom, double-tap zoom, scroll) out of two fingers on the controls; iOS Safari can
  // start one despite `touch-action: none` and cancel both pointers
  for (const t of ['touchstart', 'touchmove']) root.addEventListener(t, e => e.cancelable && e.preventDefault(), { passive: false });
  // B312: chat counts its taps for the game (`hstChatTaps`); a mouse or pen here, fingers in `touchstart` below
  const chat = root.querySelector('[data-chat]');
  const chatTap = () => {
    chatTaps++;
    if (buzz) buzzOnce();
  };
  // the phone's keyboard comes up from the finger's lift (index.html). B531: from any touch that landed on the button,
  // whatever the game has done since: it often opens the box between the finger's landing and its lift, and the
  // keyboard comes only for a focus inside the lift's own event
  let chatTouch = false;
  chat?.addEventListener('touchend', () => {
    if (chatTouch) window.hstChatKeys?.();
    chatTouch = false;
  });
  chat?.addEventListener('pointerdown', e => {
    if (e.pointerType === 'touch') return;
    e.preventDefault();
    chatTap();
  });

  // the stick appears where the left thumb lands; full tilt at the base's radius
  const zone = root.querySelector('.zone');
  const base = zone.querySelector('.base');
  const knob = zone.querySelector('.knob');
  let stick = null;
  // a point on the screen → the zone's own frame
  const local = (x, y) => {
    const r = zone.getBoundingClientRect();
    return { x: x - r.left, y: y - r.top };
  };
  const place = (el, x, y) => { el.style.left = `${x}px`; el.style.top = `${y}px`; };
  const centre = el => { el.style.left = el.style.top = '50%'; };
  const stickDown = (id, x, y) => {
    if (stick) return;
    // in the menus the stick sits still: its centre is the zone's
    stick = menu ? { id, x: zone.offsetWidth / 2, y: zone.offsetHeight / 2 } : { id, ...local(x, y) };
    place(base, stick.x, stick.y);
    place(knob, stick.x, stick.y);
    base.hidden = knob.hidden = false;
  };
  const stickMove = (id, x, y) => {
    if (stick?.id !== id) return;
    const p = local(x, y);
    const R = base.offsetWidth / 2;
    let dx = p.x - stick.x, dy = p.y - stick.y;
    const d = Math.hypot(dx, dy);
    if (d > R) (dx *= R / d), (dy *= R / d);
    place(knob, stick.x + dx, stick.y + dy);
    pad.axes[0] = dx / R;
    pad.axes[1] = dy / R;
    stamp();
  };
  const stickUp = id => {
    if (stick?.id !== id) return;
    stick = null;
    base.hidden = knob.hidden = !menu;
    if (menu) centre(knob);
    pad.axes[0] = pad.axes[1] = 0;
    stamp();
  };
  zone.addEventListener('pointerdown', e => {
    if (e.pointerType === 'touch') return;
    e.preventDefault();
    zone.setPointerCapture(e.pointerId);
    stickDown(`p${e.pointerId}`, e.clientX, e.clientY);
  });
  zone.addEventListener('pointermove', e => stickMove(`p${e.pointerId}`, e.clientX, e.clientY));
  for (const t of ['pointerup', 'pointercancel', 'lostpointercapture']) zone.addEventListener(t, e => stickUp(`p${e.pointerId}`));

  // B274: fingers by their touch events, each by its own `identifier` and only from `changedTouches`. Phones' pointer
  // events for a second finger were where two controls at once still blocked (a lift or cancel lost, or a pointer
  // re-targeted); touch events are what iOS has always delivered per finger. A touch keeps the control it landed on
  // (its `target`) until it lifts, wherever it slides.
  const control = new Map(); // touch identifier -> button key, or 'stick'
  root.addEventListener('touchstart', e => {
    for (const t of e.changedTouches) {
      const el = t.target instanceof Element ? t.target.closest('button, .zone') : null;
      if (!el || !root.contains(el) || control.has(t.identifier)) continue;
      if (el === chat) {
        chatTap();
        chatTouch = true;
      } else if (el === zone) {
        if (stick) continue;
        control.set(t.identifier, 'stick');
        stickDown(t.identifier, t.clientX, t.clientY);
      } else {
        control.set(t.identifier, el.dataset.b);
        press(el.dataset.b, t.identifier);
      }
    }
  }, { passive: false });
  root.addEventListener('touchmove', e => {
    for (const t of e.changedTouches) if (control.get(t.identifier) === 'stick') stickMove(t.identifier, t.clientX, t.clientY);
  }, { passive: false });
  const letGo = id => {
    const k = control.get(id);
    control.delete(id);
    if (k === 'stick') stickUp(id);
    else if (k) lift(k, id);
  };
  return {
    pad, root,
    poll: () => {
      for (const k in B) settle(k);
      for (const k in B) if (buttons[B[k]].pressed) seenAt[k] ??= tick;
    },
    letGo,
    // every touch event lists the fingers still down (`touches`): one that's gone without its own end (a cancel sent
    // elsewhere, a system gesture) lets its control go too, so nothing stays held or taken
    keep: down => { for (const id of [...control.keys()]) if (!down.has(id)) letGo(id); },
    menu: on => {
      root.classList.toggle('menu', on);
      if (stick) stickUp(stick.id);
      base.hidden = knob.hidden = !on;
      if (on) [base, knob].forEach(centre);
    },
    // hidden: every finger and button let go
    reset: () => {
      control.clear();
      if (stick) stickUp(stick.id);
      for (const k in B) held[k].clear(), buttons[B[k]].pressed && set(k, false);
    },
  };
};
const touch = side(16);
// B247: never listed next to a real pad, even before `update` hides the controls: a controller picked up between two
// frames would otherwise take 2P's slot while the touch pad holds 1P's, and keep it after the touch pad goes (1P left
// with none)
navigator.getGamepads = () => {
  touch.poll();
  const list = [...realPads()];
  if (shown && !list.some(g => g && g.connected)) list.push(touch.pad);
  return list;
};
addEventListener('gesturestart', e => e.preventDefault());
addEventListener('touchend', e => { for (const t of e.changedTouches) touch.letGo(t.identifier); }, true);
addEventListener('touchcancel', e => { for (const t of e.changedTouches) touch.letGo(t.identifier); }, true);
for (const t of ['touchstart', 'touchend', 'touchcancel']) addEventListener(t, e => {
  touch.keep(new Set([...e.touches].map(t => t.identifier)));
}, true);

// shown on touch screens (or after any touch) while no real controller is connected; `?touch` forces them on
let touched = coarse.matches || new URLSearchParams(location.search).has('touch');
addEventListener('pointerdown', e => { if (e.pointerType === 'touch') touched = true; }, { capture: true });
// B247: the game asks, so a real pad's first press takes over the screen's 1P seat
window.hstTouched = () => touched;
// B207: the name and code entries show their letter grid again for one
window.hstRealPad = anyReal;
window.hstTouchMenu = on => {
  menu = on;
  touch.menu(on);
};
// B312: the chat button shown (online, chat on, its box closed), and the taps on it since the last call
window.hstTouchChat = on => {
  chatShown = on;
  for (const b of document.querySelectorAll('.touch [data-chat]')) b.hidden = !on;
};
window.hstChatTaps = () => {
  const n = chatTaps;
  chatTaps = 0;
  return chatShown ? n : 0;
};
// B118: the Mobile settings' vibration
window.hstTouchBuzz = vibrate => {
  buzz = vibrate;
};
const update = () => {
  // gone when the game stops (index.html `fail`): its Reload must take the taps
  const want = touched && !anyReal() && !site() && !document.getElementById('boot')?.classList.contains('err');
  if (want !== shown) {
    shown = want;
    touch.root.hidden = !want;
    if (!want) touch.reset();
  }
};
const frame = () => {
  tick++;
  update();
  requestAnimationFrame(frame);
};
frame();
// B247: a controller connected after the page loaded hides them at once; listening for connections also wakes the
// engines that don't pick up a hotplugged pad from `getGamepads()` polls alone
for (const e of ['gamepadconnected', 'gamepaddisconnected']) addEventListener(e, update);

// iOS Safari / Android Chrome edge swipes go back a page (and leave the game) despite `overscroll-behavior: none`:
// swallow touches that start at the left/right edge (pointer events, so the touch stick, still fire), and keep a
// spare history entry that a back gesture getting through only pops. Not on the ISO import page.
const inGame = () => !site();
addEventListener('touchstart', e => {
  // the touch that just landed (B165: `touches[0]` is the first one still held, so a stick thumb near the edge used to
  // swallow every other finger's touches too)
  const x = e.changedTouches[0]?.clientX;
  if (inGame() && (x < 20 || x > innerWidth - 20)) e.preventDefault();
}, { passive: false, capture: true });
// B113: iOS starts a text selection, the magnifier or the long-press callout from a drag on the stick or the canvas,
// and zooms on a double tap or pinch: pointer events' preventDefault doesn't stop those, the touch events' does. Only
// the canvas and the pad: the page's own inputs and links (file pick, code entry, Reload) keep their taps.
const ours = t => t instanceof Element && (t.tagName === 'CANVAS' || touch.root.contains(t));
for (const t of ['touchstart', 'touchmove']) addEventListener(t, e => { if (ours(e.target)) e.preventDefault(); }, { passive: false });
// B256: a selection, select-all or callout starts nowhere on the page but in a text input (the boot text, the ISO
// page and anything outside the canvas took them before), and one that got through anyway (iOS select-all) is dropped
const typing = t => t instanceof Element && t.matches('input, textarea, [contenteditable]');
for (const t of ['gesturestart', 'gesturechange', 'dblclick', 'contextmenu', 'selectstart']) addEventListener(t, e => { if (!typing(e.target) && !site()) e.preventDefault(); }, { passive: false, capture: true });
document.addEventListener('selectionchange', () => {
  const s = getSelection();
  if (s && !s.isCollapsed && !typing(document.activeElement) && !site()) s.removeAllRanges();
});
const guard = () => inGame() && history.pushState({ hst: 1 }, '', location.href);
// pushed on a tap: Chrome skips entries a page added before any user activation
addEventListener('pointerdown', function once() {
  if (!inGame()) return;
  removeEventListener('pointerdown', once, true);
  guard();
  addEventListener('popstate', guard);
}, true);
