// On-screen controls for phones and tablets, and the page's other mobile bits. The controls are a virtual pad in the
// browser's standard layout added to `navigator.getGamepads()`, which the game polls like any controller (gilrs), so
// menus and matches read them through the same bindings as a real pad (✕ = shot / OK, ○ = slice / back, △ = lob,
// □, L1/R1, Select = camera, Start = pause). They show on touch screens and hide while a real controller is connected.
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
      .then(() => screen.orientation?.lock?.('landscape'))
      .catch(() => {});
  }
};
for (const e of ['pointerup', 'touchend', 'keydown']) addEventListener(e, unlock, { capture: true });

// the standard mapping's button indices
const B = { cross: 0, circle: 1, square: 2, triangle: 3, l1: 4, r1: 5, select: 8, start: 9 };
const buttons = Array.from({ length: 17 }, () => ({ pressed: false, touched: false, value: 0 }));
const pad = {
  id: 'Touch controls (STANDARD GAMEPAD)',
  index: 16, // past the browser's own slots (0–3)
  connected: true,
  mapping: 'standard',
  timestamp: 0,
  axes: [0, 0, 0, 0],
  buttons,
  vibrationActuator: null,
};
const realPads = navigator.getGamepads?.bind(navigator) ?? (() => []);
const anyReal = () => [...realPads()].some(g => g && g.connected);
let shown = false;
navigator.getGamepads = () => {
  const list = [...realPads()];
  if (shown) list.push(pad);
  return list;
};

const css = `
#touch { position: fixed; inset: 0; z-index: 10; pointer-events: none; user-select: none; -webkit-user-select: none;
  --u: min(7vw, 12.5vh); font: 600 calc(var(--u) * .42) system-ui, sans-serif; color: #fff; }
#touch[hidden] { display: none; }
#touch .zone { position: absolute; left: 0; top: 18%; bottom: 0; width: 45%; pointer-events: auto; touch-action: none; }
#touch .base, #touch .knob { position: absolute; border-radius: 50%; transform: translate(-50%, -50%); }
#touch .base { width: calc(var(--u) * 2.6); height: calc(var(--u) * 2.6); background: #fff2; border: 2px solid #fff5; }
#touch .knob { width: calc(var(--u) * 1.2); height: calc(var(--u) * 1.2); background: #fff6; }
#touch button { position: absolute; pointer-events: auto; touch-action: none; border: 2px solid #fff5; background: #0004;
  color: inherit; font: inherit; border-radius: 50%; width: calc(var(--u) * 1.25); height: calc(var(--u) * 1.25); padding: 0; }
#touch button.on { background: #fff6; }
#touch button.small { border-radius: calc(var(--u) * .3); width: calc(var(--u) * 1.5); height: calc(var(--u) * .7);
  font-size: calc(var(--u) * .3); opacity: .7; }
#rotate { display: none; position: fixed; inset: 0; z-index: 20; background: #000; color: #eee; align-items: center;
  justify-content: center; text-align: center; font: 20px system-ui, sans-serif; padding: 16px; }
@media (orientation: portrait) and (pointer: coarse) { #rotate { display: flex; } }
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

const root = document.createElement('div');
root.id = 'touch';
root.hidden = true;
root.innerHTML = `<style>${css}</style><div class="zone"><div class="base" hidden></div><div class="knob" hidden></div></div>`
  + layout.map(([k, label, pos, cls = '']) => `<button data-b="${k}" class="${cls}" style="${pos}">${label}</button>`).join('');
document.body.append(root);
const rotate = document.createElement('div');
rotate.id = 'rotate';
rotate.textContent = 'Turn your device sideways to play.';
document.body.append(rotate);

const stamp = () => (pad.timestamp = performance.now());
const set = (k, on) => {
  const b = buttons[B[k]];
  b.pressed = b.touched = on;
  b.value = on ? 1 : 0;
  root.querySelector(`[data-b="${k}"]`).classList.toggle('on', on);
  if (on) navigator.vibrate?.(8);
  stamp();
};
for (const el of root.querySelectorAll('button')) {
  const k = el.dataset.b;
  el.addEventListener('pointerdown', e => { e.preventDefault(); el.setPointerCapture(e.pointerId); set(k, true); });
  for (const t of ['pointerup', 'pointercancel', 'lostpointercapture']) el.addEventListener(t, () => buttons[B[k]].pressed && set(k, false));
  el.addEventListener('contextmenu', e => e.preventDefault());
}

// the stick appears where the left thumb lands; full tilt at the base's radius
const zone = root.querySelector('.zone');
const base = zone.querySelector('.base');
const knob = zone.querySelector('.knob');
let stick = null;
const place = (el, x, y) => { el.style.left = `${x}px`; el.style.top = `${y}px`; };
zone.addEventListener('pointerdown', e => {
  if (stick) return;
  e.preventDefault();
  zone.setPointerCapture(e.pointerId);
  const r = zone.getBoundingClientRect();
  stick = { id: e.pointerId, x: e.clientX - r.left, y: e.clientY - r.top };
  place(base, stick.x, stick.y);
  place(knob, stick.x, stick.y);
  base.hidden = knob.hidden = false;
});
zone.addEventListener('pointermove', e => {
  if (stick?.id !== e.pointerId) return;
  const r = zone.getBoundingClientRect();
  const R = base.offsetWidth / 2;
  let dx = e.clientX - r.left - stick.x, dy = e.clientY - r.top - stick.y;
  const d = Math.hypot(dx, dy);
  if (d > R) (dx *= R / d), (dy *= R / d);
  place(knob, stick.x + dx, stick.y + dy);
  pad.axes[0] = dx / R;
  pad.axes[1] = dy / R;
  stamp();
});
const release = e => {
  if (stick?.id !== e.pointerId) return;
  stick = null;
  base.hidden = knob.hidden = true;
  pad.axes[0] = pad.axes[1] = 0;
  stamp();
};
for (const t of ['pointerup', 'pointercancel', 'lostpointercapture']) zone.addEventListener(t, release);

// shown on touch screens (or after any touch) while no real controller is connected; `?touch` forces them on
let touched = coarse.matches || new URLSearchParams(location.search).has('touch');
addEventListener('pointerdown', e => { if (e.pointerType === 'touch') touched = true; }, { capture: true });
const update = () => {
  const want = touched && !anyReal() && !document.getElementById('disc')?.offsetParent;
  if (want !== shown) {
    shown = want;
    root.hidden = !want;
    if (!want) for (const k in B) buttons[B[k]].pressed && set(k, false);
  }
  requestAnimationFrame(update);
};
update();
