// On-screen controls for phones and tablets, and the page's other mobile bits. The controls are a virtual pad in the
// browser's standard layout added to `navigator.getGamepads()`, which the game polls like any controller (gilrs), so
// menus and matches read them through the same bindings as a real pad (✕ = shot / OK, ○ = slice / back, △ = lob,
// □, L1/R1, Select = swap hand, Start = pause). They show on touch screens (menus and matches) and hide while a real
// controller is connected. In the menus, which also take taps (B221), the stick is a fixed circle at the bottom left so
// taps anywhere else reach the menu; in a match it appears wherever the left thumb lands.
// Also here: audio unlocks on the first tap or key (browsers start every AudioContext suspended), and a touch screen
// goes fullscreen in landscape on the first tap (where the browser allows it; an installed app starts that way).

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
const unlock = () => {
  for (const c of contexts) if (c.state !== 'running') c.resume().catch(() => {});
  if (coarse.matches && !document.fullscreenElement && !matchMedia('(display-mode: fullscreen), (display-mode: standalone)').matches) {
    document.documentElement.requestFullscreen?.({ navigationUI: 'hide' })
      .then(() => screen.orientation?.lock?.(duo ? 'portrait' : 'landscape'))
      .catch(() => {});
  }
};
for (const e of ['pointerup', 'touchend', 'keydown']) addEventListener(e, unlock, { capture: true });

// the standard mapping's button indices
const B = { cross: 0, circle: 1, square: 2, triangle: 3, l1: 4, r1: 5, select: 8, start: 9 };
const realPads = navigator.getGamepads?.bind(navigator) ?? (() => []);
const anyReal = () => [...realPads()].some(g => g && g.connected);
let shown = false;
// B118: two players on one phone (the main menu's 1v1 on this phone): a second pad for the far end, turned to face
// its player; and the press buzz (Settings → Mobile)
let duo = false;
let buzz = true;
// B165: the game's polls, counted in animation frames. A tap shorter than a frame (or one that lands while the page is
// busy, its down and up queued together) would come and go between two polls and never be seen: a button stays down
// until a poll has seen it and a later frame has started, so every tap counts once.
let tick = 0;
// the game's menus take taps themselves (B221): there the stick keeps to its own circle (`.menu`), drawn at rest
let menu = false;

const css = `
.touch { position: fixed; inset: 0; z-index: 10; pointer-events: none; user-select: none; -webkit-user-select: none;
  --u: min(7vw, 12.5vh); font: 600 calc(var(--u) * .42) system-ui, sans-serif; color: #fff; }
.touch[hidden] { display: none; }
/* B118: the far player's controls, the whole layer turned a half turn: its bottom is the screen's top */
.touch.far { transform: rotate(180deg); }
.touch .zone { position: absolute; left: 0; top: 18%; bottom: 0; width: 45%; pointer-events: auto; touch-action: none; }
.touch .base, .touch .knob { position: absolute; border-radius: 50%; transform: translate(-50%, -50%); }
.touch.menu .zone { top: auto; width: calc(var(--u) * 2.6); height: calc(var(--u) * 2.6); border-radius: 50%;
  left: calc(env(safe-area-inset-left) + var(--u) * .4); bottom: calc(env(safe-area-inset-bottom) + var(--u) * .4); }
.touch .base { width: calc(var(--u) * 2.6); height: calc(var(--u) * 2.6); background: #fff2; border: 2px solid #fff5; }
.touch .knob { width: calc(var(--u) * 1.2); height: calc(var(--u) * 1.2); background: #fff6; }
.touch button { position: absolute; pointer-events: auto; touch-action: none; border: 2px solid #fff5; background: #0004;
  color: inherit; font: inherit; border-radius: 50%; width: calc(var(--u) * 1.25); height: calc(var(--u) * 1.25); padding: 0; }
.touch button.on { background: #fff6; }
/* B165: a touch just past a button's edge still hits it (the face buttons' extended circles just don't overlap) */
.touch button::after { content: ''; position: absolute; inset: calc(var(--u) * -.2); border-radius: inherit; }
.touch button.small { border-radius: calc(var(--u) * .3); width: calc(var(--u) * 1.5); height: calc(var(--u) * .7);
  font-size: calc(var(--u) * .3); opacity: .7; }
/* B118: each player's set at their own end (a phone upright: the bottom and top 45%), the shoulders and Select/Start
   over the face buttons instead of along the screen's far edge */
.touch.duo { --u: min(11vw, 6.5vh); }
.touch.duo:not(.menu) .zone { top: calc(100% - env(safe-area-inset-bottom) - var(--u) * 4); }
.touch.duo button.small { top: auto !important; bottom: calc(env(safe-area-inset-bottom) + var(--u) * 4.1) !important; }
#rotate { display: none; position: fixed; inset: 0; z-index: 20; background: #000; color: #eee; align-items: center;
  justify-content: center; text-align: center; font: 20px system-ui, sans-serif; padding: 16px; }
@media (orientation: portrait) and (pointer: coarse) { #rotate:not(.duo) { display: flex; } }
`;
// face buttons in a diamond for the right thumb, shoulders and Select/Start along the top
const right = (x, y) => `right: calc(env(safe-area-inset-right) + var(--u) * ${x}); bottom: calc(env(safe-area-inset-bottom) + var(--u) * ${y})`;
const layout = [
  ['cross', '✕', right(1.4, 0.3)],
  ['circle', '○', right(0.2, 1.5)],
  ['square', '□', right(2.6, 1.5)],
  ['triangle', '△', right(1.4, 2.7)],
  ['l1', 'L1', 'left: calc(env(safe-area-inset-left) + var(--u) * .4); top: calc(var(--u) * .3)', 'small'],
  ['r1', 'R1', 'right: calc(env(safe-area-inset-right) + var(--u) * .4); top: calc(var(--u) * .3)', 'small'],
  ['select', 'SELECT', 'left: calc(50% - var(--u) * 1.8); top: calc(var(--u) * .3)', 'small'],
  ['start', 'START', 'left: calc(50% + var(--u) * .3); top: calc(var(--u) * .3)', 'small'],
];
const style = document.createElement('style');
style.textContent = css;
document.head.append(style);
const rotate = document.createElement('div');
rotate.id = 'rotate';
rotate.textContent = 'Turn your device sideways to play.';

// One player's on-screen pad: its own buttons, stick and fingers, listed as gamepad `index`. `far` is turned a half
// turn (B118): it lays out the same in its own frame, and its stick reads in the screen's directions (the match's
// sticks are screen-relative), so a push away from its player goes down the screen.
const side = (index, far) => {
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
  root.className = far ? 'touch far' : 'touch';
  if (!far) root.id = 'touch';
  root.hidden = true;
  root.innerHTML = `<div class="zone"><div class="base" hidden></div><div class="knob" hidden></div></div>`
    + layout.map(([k, label, pos, cls = '']) => `<button data-b="${k}" class="${cls}" style="${pos}">${label}</button>`).join('');
  document.body.append(root);

  const stamp = () => (pad.timestamp = performance.now());
  const set = (k, on) => {
    const b = buttons[B[k]];
    b.pressed = b.touched = on;
    b.value = on ? 1 : 0;
    root.querySelector(`[data-b="${k}"]`).classList.toggle('on', on);
    if (on && buzz) navigator.vibrate?.(8);
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
  for (const el of root.querySelectorAll('button')) {
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

  // the stick appears where the left thumb lands; full tilt at the base's radius
  const zone = root.querySelector('.zone');
  const base = zone.querySelector('.base');
  const knob = zone.querySelector('.knob');
  let stick = null;
  // a point on the screen → the zone's own frame (the far side's is turned)
  const local = (x, y) => {
    const r = zone.getBoundingClientRect();
    return far ? { x: r.right - x, y: r.bottom - y } : { x: x - r.left, y: y - r.top };
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
    pad.axes[0] = far ? -dx / R : dx / R;
    pad.axes[1] = far ? -dy / R : dy / R;
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
      if (el === zone) {
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
const sides = [side(16, false), side(17, true)];
document.body.append(rotate);
// B247: never listed next to a real pad, even before `update` hides the controls: a controller picked up between two
// frames would otherwise take 2P's slot while the touch pad holds 1P's, and keep it after the touch pad goes (1P left
// with none)
navigator.getGamepads = () => {
  for (const s of sides) s.poll();
  const list = [...realPads()];
  if (shown && !list.some(g => g && g.connected)) list.push(...sides.slice(0, duo ? 2 : 1).map(s => s.pad));
  return list;
};
addEventListener('gesturestart', e => e.preventDefault());
addEventListener('touchend', e => { for (const t of e.changedTouches) for (const s of sides) s.letGo(t.identifier); }, true);
addEventListener('touchcancel', e => { for (const t of e.changedTouches) for (const s of sides) s.letGo(t.identifier); }, true);
for (const t of ['touchstart', 'touchend', 'touchcancel']) addEventListener(t, e => {
  const down = new Set([...e.touches].map(t => t.identifier));
  for (const s of sides) s.keep(down);
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
  for (const s of sides) s.menu(on);
};
// B118: the game's 1v1 on this phone (both pads, held upright) and the Mobile settings' vibration
window.hstTouchPads = (two, vibrate) => {
  buzz = vibrate;
  if (two === duo) return;
  duo = two;
  for (const s of sides) s.root.classList.toggle('duo', duo);
  rotate.classList.toggle('duo', duo);
  if (document.fullscreenElement) screen.orientation?.lock?.(duo ? 'portrait' : 'landscape').catch(() => {});
  shown = !shown; // `update` lays both out again
  update();
};
const update = () => {
  // gone when the game stops (index.html `fail`): its Reload must take the taps
  const want = touched && !anyReal() && !document.getElementById('disc')?.offsetParent && !document.getElementById('boot')?.classList.contains('err');
  if (want !== shown) {
    shown = want;
    sides.forEach((s, i) => {
      const on = want && (i === 0 || duo);
      s.root.hidden = !on;
      if (!on) s.reset();
    });
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
const inGame = () => !document.getElementById('disc')?.offsetParent;
addEventListener('touchstart', e => {
  // the touch that just landed (B165: `touches[0]` is the first one still held, so a stick thumb near the edge used to
  // swallow every other finger's touches too)
  const x = e.changedTouches[0]?.clientX;
  if (inGame() && (x < 20 || x > innerWidth - 20)) e.preventDefault();
}, { passive: false, capture: true });
// B113: iOS starts a text selection, the magnifier or the long-press callout from a drag on the stick or the canvas,
// and zooms on a double tap or pinch: pointer events' preventDefault doesn't stop those, the touch events' does. Only
// the canvas and the pad: the page's own inputs and links (file pick, code entry, Reload) keep their taps.
const ours = t => t instanceof Element && (t.tagName === 'CANVAS' || sides.some(s => s.root.contains(t)));
for (const t of ['touchstart', 'touchmove']) addEventListener(t, e => { if (ours(e.target)) e.preventDefault(); }, { passive: false });
for (const t of ['gesturestart', 'gesturechange', 'dblclick', 'contextmenu', 'selectstart']) addEventListener(t, e => { if (ours(e.target)) e.preventDefault(); }, { passive: false });
const guard = () => inGame() && history.pushState({ hst: 1 }, '', location.href);
// pushed on a tap: Chrome skips entries a page added before any user activation
addEventListener('pointerdown', function once() {
  if (!inGame()) return;
  removeEventListener('pointerdown', once, true);
  guard();
  addEventListener('popstate', guard);
}, true);
