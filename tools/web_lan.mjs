// M2k: two browser tabs play a LAN match over WebRTC with the codes passed the way phones would (the host's offer link
// opened by the guest, the guest's answer link opened in a third tab that hands it to the host by BroadcastChannel).
// Usage: node tools/web_lan.mjs ISO|- OUT [seconds of match]   (serve web/ on :8765 first: python3 -m http.server -d
// web 8765). ISO `-`: OUT/profile already holds the pack. WEB_HIDE: a tab-out mid-match (below). CHROME_FLAGS adds chromium flags (`--use-angle=vulkan
// --enable-features=Vulkan`: the real GPU; SwiftShader draws two matches at ~5 fps). Console and screenshots into OUT.
import { spawn } from 'node:child_process';
import { writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const [iso, out, secs = '60'] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const web = process.env.WEB_PORT ?? 8765, cdp = process.env.CDP_PORT ?? 9334;
const base = `http://localhost:${web}/`;
const sleep = ms => new Promise(r => setTimeout(r, ms));
// every chromium we start dies with this script (a crash, timeout or kill would otherwise leave a hidden hst playing music)
const kids = new Set();
process.on('exit', () => kids.forEach(c => c.kill()));
for (const s of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(s, () => process.exit(1));
const log = [];
const say = s => { log.push(s); console.log(s); };

const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${cdp}`, `--user-data-dir=${resolve(out, 'profile')}`,
  '--ignore-gpu-blocklist', '--window-size=960,540', '--disable-background-timer-throttling', '--disable-renderer-backgrounding',
  '--disable-backgrounding-occluded-windows', ...(process.env.CHROME_FLAGS ?? '').split(' ').filter(Boolean), 'about:blank'], { stdio: 'ignore' });
  kids.add(chrome);
process.on('exit', () => { writeFileSync(resolve(out, 'console.txt'), log.join('\n') + '\n'); chrome.kill(); });
let version;
for (let i = 0; i < 50 && !version; i++) {
  await sleep(200);
  version = await fetch(`http://127.0.0.1:${cdp}/json/version`).then(r => r.json()).catch(() => null);
}

// each page in a window of its own: a tab behind another draws no frames, as on a phone
const browser = new WebSocket(version.webSocketDebuggerUrl);
await new Promise(r => (browser.onopen = r));
const newWindow = () => new Promise(r => {
  browser.onmessage = ({ data }) => r(JSON.parse(data).result.targetId);
  browser.send(JSON.stringify({ id: 1, method: 'Target.createTarget', params: { url: 'about:blank', newWindow: true } }));
});

// a browser-level CDP call (one at a time)
let bid = 1;
const bsend = (method, params = {}) => new Promise(r => {
  const id = ++bid;
  browser.onmessage = ({ data }) => { const m = JSON.parse(data); if (m.id === id) r(m.result); };
  browser.send(JSON.stringify({ id, method, params }));
});

/// A page: its console lines, `send` (CDP), `keys`.
async function tab(name, url) {
  const target = await newWindow();
  const ws = new WebSocket(`ws://127.0.0.1:${cdp}/devtools/page/${target}`);
  await new Promise(r => (ws.onopen = r));
  let id = 0;
  const waiting = new Map();
  const lines = [];
  ws.onmessage = ({ data }) => {
    const m = JSON.parse(data);
    if (m.id && waiting.has(m.id)) waiting.get(m.id)(m), waiting.delete(m.id);
    let l;
    if (m.method === 'Runtime.consoleAPICalled') l = `${m.params.type}: ${m.params.args.map(a => a.value ?? a.description).join(' ')}`;
    if (m.method === 'Runtime.exceptionThrown') l = `exception: ${m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text}`;
    if (l) {
      l = l.replace(/%c/g, '').replace(/ color: [^%]*$/, '');
      lines.push(l);
      log.push(`[${name}] ${l}`);
    }
  };
  const send = (method, params = {}) => new Promise(r => { waiting.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
  const evaluate = async e => (await send('Runtime.evaluate', { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
  await send('Runtime.enable');
  await send('Page.enable');
  // keeps the page's peer connections and counts frames, for `stats`
  await send('Page.addScriptToEvaluateOnNewDocument', { source: `{
    const P = RTCPeerConnection; window.__pcs = [];
    window.RTCPeerConnection = function (...a) { const pc = new P(...a); __pcs.push(pc); return pc; };
    window.RTCPeerConnection.prototype = P.prototype;
    // other agents' virtual pads would steer the menus
    Navigator.prototype.getGamepads = () => [];
    addEventListener('gamepadconnected', e => e.stopImmediatePropagation(), true);
    window.__frames = 0; const f = () => { __frames++; requestAnimationFrame(f); }; requestAnimationFrame(f);
  }` });
  await send('Page.navigate', { url });
  const keys = async (...ks) => {
    for (const key of ks) {
      await send('Page.bringToFront');
      for (const type of ['rawKeyDown', 'keyUp']) {
        await send('Input.dispatchKeyEvent', { type, key, code: key, windowsVirtualKeyCode: { Enter: 13, ArrowDown: 40, ArrowUp: 38, ArrowLeft: 37, ArrowRight: 39, Escape: 27 }[key] });
        await sleep(120);
      }
      await sleep(700);
      // WEB_STEPS=1: a screenshot after each key (finding the way through changed menus)
      if (process.env.WEB_STEPS) await shot(`step${++steps}-${key}`);
    }
  };
  let steps = 0;
  const shot = async n => {
    const s = await send('Page.captureScreenshot', { format: 'png' });
    if (s.result) writeFileSync(resolve(out, `${name}-${n}.png`), Buffer.from(s.result.data, 'base64'));
  };
  const wait = async (re, s) => {
    for (let i = 0; i < s * 4; i++) {
      const hit = lines.map(l => l.match(re)).find(Boolean);
      if (hit) return hit;
      await sleep(250);
    }
    return null;
  };
  // frames drawn so far, then each connection's state and its data channels' messages sent/received
  const stats = () => evaluate(`(async () => [__frames, ...await Promise.all(__pcs.map(async pc => pc.connectionState + ' ' +
    [...(await pc.getStats()).values()].filter(s => s.type == 'data-channel').map(s => s.label + ':' + s.messagesSent + '/' + s.messagesReceived).join(' ')))].join(' | '))()`);
  return { name, target, lines, send, evaluate, keys, shot, wait, ws, stats };
}

const host = await tab('host', `${base}?webgl2`);
if (iso !== '-') {
  for (let i = 0; i < 100 && !(await host.evaluate("!document.getElementById('disc')?.hidden")); i++) await sleep(200);
  const doc = await host.send('DOM.getDocument');
  const node = await host.send('DOM.querySelector', { nodeId: doc.result.root.nodeId, selector: '#iso' });
  await host.send('DOM.setFileInputFiles', { nodeId: node.result.nodeId, files: [resolve(iso)] });
  for (let i = 0; i < 600 && !/Imported|Couldn't/.test(await host.evaluate("document.getElementById('status').textContent")); i++) await sleep(1000);
  say(`import: ${await host.evaluate("document.getElementById('status').textContent")}`);
}
say(`host menu: ${!!(await host.wait(/loading screen down/, 120))}`);
await sleep(3000);
// Online → Relay (n0 → Off) → Host → singles → controllers → character → confirm's Start: the lobby
await host.keys('ArrowDown', 'Enter', 'ArrowDown', 'ArrowDown', 'ArrowDown', 'Enter', 'ArrowUp', 'ArrowUp', 'Enter',
  'Enter', 'Enter', 'Enter', 'Enter', 'Enter', 'ArrowRight', 'Enter');
const offer = await host.wait(/LAN offer: (\S+)/, 30);
await host.shot('lobby');
say(`offer: ${offer?.[1]}`);
if (!offer) process.exit(1);

// the guest "scanned" it: the page opens with the code; Online → Join by Code → singles → controllers → character
const guest = await tab('guest', `${offer[1].replace(/^http:\/\/localhost:\d+\//, base)}&webgl2`);
say(`guest menu: ${!!(await guest.wait(/loading screen down/, 120))}`);
await sleep(3000);
await guest.keys('ArrowDown', 'Enter');
const answer = await guest.wait(/LAN answer: (\S+)/, 30);
await guest.shot('lobby');
say(`answer: ${answer?.[1]}`);
if (!answer) process.exit(1);

// the host "scanned" the answer: a third tab with it, which hands it over and stops
const relay = await tab('answer', answer[1].replace(/^http:\/\/localhost:\d+\//, base));
await sleep(2000);
say(`answer tab: ${await relay.evaluate('document.body.innerText')}`);
await sleep(5000);
await guest.keys(...(process.env.WEB_GUEST ?? 'Enter,Enter,Enter').split(','));
await host.keys(...(process.env.WEB_HOST ?? 'ArrowDown,Enter').split(','));

const started = await Promise.all([host, guest].map(t => t.wait(/starting the online match/, 60)));
say(`started: ${started.map(Boolean)}`);
for (let s = 2; s <= 20; s += 2) {
  await sleep(2000);
  for (const t of [host, guest]) say(`${s}s ${t.name}: ${await t.stats()}`);
}
// WEB_HIDE=ms[:host[:frozen]]: the guest's (host's) window is then minimised for ms (hidden: no frames, its network
// and timers go on, as a tab put behind another) and comes back (B299); `frozen`: the page is frozen instead
// (Page.setWebLifecycleState: no timers or network either). Screenshots 1 and 3 s after.
if (process.env.WEB_HIDE) {
  const [ms, who, how] = process.env.WEB_HIDE.split(':');
  const t = who === 'host' ? host : guest;
  say(`hiding ${t.name} for ${ms} ms: ${await t.stats()}`);
  const { windowId } = await bsend('Browser.getWindowForTarget', { targetId: t.target });
  const hide = on => how === 'frozen' ? t.send('Page.setWebLifecycleState', { state: on ? 'frozen' : 'active' })
    : bsend('Browser.setWindowBounds', { windowId, bounds: { windowState: on ? 'minimized' : 'normal' } });
  await hide(true);
  say(`${t.name} hidden: visibility ${await t.evaluate('document.visibilityState')}`);
  await sleep(+ms);
  await hide(false);
  say(`${t.name} back: ${await t.stats()}`);
  for (const s of [1, 3]) {
    await sleep(s === 1 ? 1000 : 2000);
    for (const u of [host, guest]) await u.shot(`back-${s}s`);
  }
}
for (let s = 30; s <= +secs; s += 10) {
  await sleep(10000);
  for (const t of [host, guest]) await t.shot(`match-${s}s`);
}
for (const t of [host, guest]) say(`${t.name}: ${t.lines.filter(l => /rollback tick|online tick|match over|desync|rollback:|panicked|exception/i.test(l)).join(' | ')}`);
chrome.kill();
process.exit(0);
