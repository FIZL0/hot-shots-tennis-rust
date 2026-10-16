// M96: the site (index.html #disc, site.css). Shown on every visit (with a disc imported, its play section is one
// big Play button), and with `?site` from the game's error screen (Settings & files without starting the game). Nothing here loads the game's wasm: the ISO import is worker.js's, and Settings &
// files reads and writes the same browser storage the game does (localStorage under the file names `crate::store`
// uses, OPFS for the disc pack, mods, cards and reports). `landing` resolves once the game should start: a disc was
// imported, or Play was pressed.

const $ = s => document.querySelector(`#disc ${s}`);
const el = (tag, props = {}, ...kids) => {
  const e = Object.assign(document.createElement(tag), props);
  e.append(...kids.filter(k => k != null));
  return e;
};
const pill = (text, onclick, cls = '') => el('button', { className: `pill small ${cls}`, textContent: text, onclick });
const size = n => n < 1e3 ? `${n} B` : n < 1e6 ? `${(n / 1e3).toFixed(0)} KB` : n < 1e9 ? `${(n / 1e6).toFixed(n < 1e7 ? 1 : 0)} MB` : `${(n / 1e9).toFixed(2)} GB`;

// localStorage, as the game keeps its files there (web.rs read_to_string / write): settings.txt and online.txt beside
// the pack ("./"), the Hugging Face login under its bare name (modstore.rs TOKEN), the store's listing cache
const SETTINGS = './settings.txt', ONLINE = './online.txt', TOKEN = 'hf-token.txt', STORE_CACHE = 'mods-store.json';
const get = k => { try { return localStorage.getItem(k); } catch { return null; } };
const put = (k, v) => { try { localStorage.setItem(k, v); return true; } catch { return false; } };

// `key = value` lines (a `#` starts a comment), as the game's parsers read them
const value = (text, key) => {
  for (const l of (text ?? '').split('\n')) {
    const [k, v] = (l.split('#')[0] ?? '').split(/=(.*)/s);
    if (v !== undefined && k.trim() === key) return v.trim();
  }
  return undefined;
};
// `text` with `key`'s line set to `v` (the first one replaced, else appended)
const withValue = (text, key, v) => {
  const lines = (text ?? '').split('\n');
  const i = lines.findIndex(l => (l.split('#')[0] ?? '').split('=')[0].trim() === key && l.includes('='));
  if (i >= 0) lines[i] = `${key} = ${v}`;
  else {
    if (lines.at(-1) === '') lines.pop();
    lines.push(`${key} = ${v}`, '');
  }
  return lines.join('\n');
};

// B457: what a browser without OPFS (private browsing) gets
const MEMORY_NOTE = 'This browser isn\'t letting the page keep files (a private window?), so the game runs from memory: '
  + 'pick the ISO again on each visit, and settings and mods last only until the window closes.';

export async function landing(dir, hasPack) {
  const root = document.getElementById('disc');
  root.hidden = false;
  if (location.hash) document.getElementById(location.hash.slice(1))?.scrollIntoView();
  const have = on => {
    for (const e of root.querySelectorAll('[data-new]')) e.hidden = on;
    for (const e of root.querySelectorAll('[data-have]')) e.hidden = !on;
  };
  have(hasPack);
  // B457: storage refused (a private window): said once, beside the ISO
  if (dir.memory && !document.getElementById('nostore')) $('#status').before(el('p', { id: 'nostore', className: 'note', textContent: MEMORY_NOTE }));
  const parts = { dir, have, hasPack };
  username();
  account();
  prefs();
  mods(parts);
  storage(parts);
  stats();
  return new Promise(resolve => {
    const go = () => {
      // `?site` dropped and the tab marked, so a reload goes into the game
      try { sessionStorage.setItem('hst-playing', '1'); } catch {}
      const q = new URLSearchParams(location.search);
      q.delete('site');
      history.replaceState(null, '', `${location.pathname}${q.size ? `?${q}` : ''}`);
      root.hidden = true;
      resolve();
    };
    $('#playnow').onclick = go;
    const auto = $('#autoplay');
    auto.checked = get('hst-autoplay') === '1';
    auto.onchange = () => put('hst-autoplay', auto.checked ? '1' : '0');
    for (const a of root.querySelectorAll('.band [data-have]')) a.onclick = e => { e.preventDefault(); go(); };
    importer(go, dir);
  });
}

// ---------------------------------------------------------------- the ISO

function importer(done, dir) {
  const input = $('#iso'), status = $('#status'), bar = $('.progress'), fill = bar.firstElementChild;
  input.onchange = () => {
    const file = input.files[0];
    input.value = '';
    if (!file) return;
    status.className = '';
    // B466: iOS fetches an iCloud file before the worker's first read returns; nothing to count until then
    status.textContent = `Reading ${file.name}…`;
    bar.hidden = false;
    bar.classList.add('busy');
    fill.style.width = '0';
    const w = new Worker('./worker.js', { type: 'module' });
    w.onmessage = ({ data }) => {
      if (data.progress) {
        bar.classList.remove('busy');
        const p = Math.floor(100 * data.progress[0] / data.progress[1]);
        fill.style.width = `${p}%`;
        status.textContent = `Importing… ${p}%`;
      } else if (data.error) {
        w.terminate();
        bar.hidden = true;
        status.className = 'err';
        status.textContent = `Couldn't import ${file.name}: ${data.error}`;
      } else if (data.hash) {
        w.terminate();
        // B457: no OPFS, the pack in memory for this visit (index.html's memory folder)
        if (data.pack) dir.put('disc.pack', data.pack);
        fill.style.width = '100%';
        status.textContent = 'Imported. The ISO is no longer needed; you can delete it. Starting…';
        done();
      }
    };
    w.onerror = e => { bar.hidden = true; status.className = 'err'; status.textContent = `Couldn't import ${file.name}: ${e.message ?? e}`; };
    w.postMessage(dir.memory ? { file, memory: true } : file);
  };
}

// ---------------------------------------------------------------- player

// the glyphs the game's name entry and font have (main_menu.rs FONT), and a space
const GLYPHS = new Set([...'aàáâãäåæbcçdðeèéêëfghiìíîïjklmnñoòóôõöøœpqrsštuùúûũüvwxyýÿzAÀÁÂÃÄÅÆBCÇDÐEÈÉÊËFGHIÌÍÎÏJKLMNÑOÒÓÔÕÖØŒPQRSŠTUÙÚÛŨÜVWXYÝŸZ1234567890-=˜€?!"#£%&`\'^_[]().®©™¿¡„:;+*@{},<>/¥«»$ß ']);

function username() {
  const input = $('#username'), note = $('#namenote');
  const was = value(get(ONLINE), 'name');
  input.value = was ?? '';
  const say = note.textContent;
  $('#namesave').onclick = () => {
    // net.rs Online::parse: 24 characters; `#` would start a comment in online.txt
    const name = [...input.value].filter(c => GLYPHS.has(c) && c !== '#').join('').trim().slice(0, 24);
    input.value = name;
    if (!name) { note.textContent = 'Type a name first.'; return; }
    note.textContent = put(ONLINE, withValue(get(ONLINE), 'name', name)) ? `Saved: you're ${name}.` : 'This browser won\'t let the page save it.';
    setTimeout(() => note.textContent = say, 4000);
  };
  input.onkeydown = e => e.key === 'Enter' && $('#namesave').click();
}

// ---------------------------------------------------------------- accounts

const HF = 'https://huggingface.co';
// modstore.rs TOKENS: where a player makes a write token to paste
const TOKENS = `${HF}/settings/tokens/new?tokenType=write&description=Hot%20Shots%20Tennis%20mods`;

// modstore.rs account(): `name\ntoken\n`
const signedIn = () => {
  const [name, token] = (get(TOKEN) ?? '').trim().split('\n');
  return token?.trim() ? { name, token: token.trim() } : null;
};

function account() {
  const box = $('#hf');
  const who = signedIn();
  box.replaceChildren();
  if (who) {
    box.append(el('div', { className: 'r' },
      el('span', {}, 'Hugging Face ', el('span', { className: 'v' }, '(mod uploads)')),
      el('span', { className: 'end' }, el('span', { className: 'ok', textContent: `Signed in as ${who.name}` }),
        pill('Sign out', () => { if (confirm('Sign out of Hugging Face? The game forgets your token; you can paste it or a new one again.')) { put(TOKEN, ''); account(); } }))));
    return;
  }
  const input = el('input', { type: 'password', placeholder: 'Paste a token (hf_…)', autocomplete: 'off', spellcheck: false });
  const msg = el('p', { className: 'note' });
  const go = pill('Sign in', async () => {
    // modstore.rs token_in
    const token = input.value.split(/[^A-Za-z0-9_]/).find(w => w.startsWith('hf_') && w.length >= 30 && w.length <= 200);
    if (!token) { msg.className = 'bad'; msg.textContent = 'That doesn\'t look like a Hugging Face token (hf_…).'; return; }
    go.disabled = true;
    msg.className = 'note';
    msg.textContent = 'Checking with Hugging Face…';
    try {
      // modstore.rs sign_in: the account's name, and a token that can write
      const r = await fetch(`${HF}/api/whoami-v2`, { headers: { Authorization: `Bearer ${token}` } });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const j = await r.json();
      if (!j.name) throw new Error('no account name');
      if (j.auth?.accessToken?.role === 'read') { msg.className = 'bad'; msg.textContent = 'That token can only read: make one with write access.'; go.disabled = false; return; }
      if (!put(TOKEN, `${j.name}\n${token}\n`)) throw new Error('this browser won\'t let the page keep it');
      account();
    } catch (e) {
      msg.className = 'bad';
      msg.textContent = `Hugging Face didn't take that token (${e.message ?? e}).`;
      go.disabled = false;
    }
  });
  input.onkeydown = e => e.key === 'Enter' && go.click();
  box.append(
    el('div', { className: 'r' }, el('span', {}, 'Hugging Face ', el('span', { className: 'v' }, '(mod uploads)')), el('span', { className: 'v', textContent: 'Not signed in' })),
    el('div', { className: 'r' }, el('span', { className: 'note' }, 'Make a token with write access, then paste it here. ',
      el('a', { href: TOKENS, target: '_blank', rel: 'noopener', textContent: 'Make a token' })),
      el('span', { className: 'end' }, input, go)),
    msg);
}

// ---------------------------------------------------------------- settings.txt

const ON = ['on', 'off'];
// the game's settings.txt keys (main_menu.rs Settings::text, graphics.rs Graphics::text, audio.rs Volumes), grouped
// as its Settings screen pages; [key, label, choices (word or [word, label]) | 'on' | 'volume', default]
const GROUPS = [
  ['Display', [
    ['aspect', 'Aspect ratio', ['Auto', '4:3', '16:9', '16:10', '21:9'], 'Auto'],
    ['renderer', 'Renderer', [['auto', 'Auto'], ['webgpu', 'WebGPU'], ['webgl2', 'WebGL2']], 'auto'],
    ['menu_size', 'Menu size', [['auto', 'Auto'], ...[50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150].map(n => [`${n}`, `${n}%`])], 'auto'],
    ['button_icons', 'Button icons', [['auto', 'Auto'], ['ps', 'PlayStation'], ['xbox', 'Xbox'], ['switch', 'Switch'], ['generic', 'Generic']], 'auto'],
    ['show_fps', 'Show FPS', [['off', 'Off'], ['on', 'On'], ['detailed', 'Detailed']], 'off'],
  ]],
  ['Graphics', [
    ['frame_limit', 'Frame limit', [...[30, 60, 120, 144, 240, 360, 480].map(n => [`${n}`, `${n} fps`]), ['uncapped', 'Uncapped']], 'uncapped'],
    ['vsync', 'VSync', 'on', 'off'],
    ['render_scale', 'Render scale', [50, 67, 75, 85, 100].map(n => [`${n}`, `${n}%`]), '100'],
    ['shadows', 'Shadows', [['off', 'Off'], ['low', 'Low'], ['high', 'High']], 'high'],
    ['anti_aliasing', 'Anti-aliasing', [['off', 'Off'], ['2x', '2x'], ['4x', '4x']], '4x'],
    ['original', 'Original look (640×448)', 'on', 'off'],
    ['fewer_shadows', 'Fewer shadows', 'on', 'on'],
    ['cull', 'Skip off-screen', 'on', 'on'],
    ['mesh', 'Mesh detail', [25, 50, 75, 100].map(n => [`${n}`, `${n}%`]), '100'],
  ]],
  ['Sound', [
    ['master_volume', 'Master volume', 'volume', '10'],
    ['music_volume', 'Music volume', 'volume', '10'],
    ['sfx_volume', 'Sound effects volume', 'volume', '10'],
    ['voice_volume', 'Voices volume', 'volume', '10'],
    ['menu_music', 'Menu music', 'on', 'on'],
    ['music', 'Match music', 'on', 'off'],
  ]],
  ['Game & online', [
    ['key_hints', 'Key hints', 'on', 'on'],
    ['chat', 'Online chat', 'on', 'on'],
    ['online_mods', 'Lobby mods', [['ask', 'Ask'], ['always', 'Download, always'], ['never', 'Never download']], 'ask'],
    ['vibration', 'Vibration (touch)', 'on', 'on'],
    ['update_check', 'Update check', 'on', 'on'],
  ]],
  ['Mods', [
    ['upscaled_textures', 'Upscaled textures', 'on', 'on'],
    ['upscaled_downloads', 'Upscaled downloads', 'on', 'on'],
    ['downloads_at_once', 'Downloads at once', [['auto', 'Auto'], ['1', '1'], ['2', '2'], ['3', '3'], ['4', '4']], 'auto'],
  ]],
];
// written by the game but not settings to change here (the confirm screen's last umpire, the desktop window)
const HIDDEN = new Set(['umpire', 'windowed', 'widescreen', 'uncapped_fps']);

function prefs() {
  const box = $('#prefs');
  const text = get(SETTINGS);
  const none = text == null || !text.trim();
  box.replaceChildren();
  if (none) box.append(el('p', { className: 'note', textContent: 'The game makes its settings the first time it starts (a phone gets its own graphics preset then). Start it once and they show up here.' }));
  const set = (key, v) => {
    if (!put(SETTINGS, withValue(get(SETTINGS), key, v))) alert('This browser won\'t let the page save settings.');
  };
  const control = (key, kind, def) => {
    const now = value(text, key) ?? def;
    if (kind === 'on') {
      const t = el('button', { className: 'toggle', role: 'switch', disabled: none, title: key });
      t.setAttribute('aria-checked', String(now !== 'off'));
      t.onclick = () => { const on = t.getAttribute('aria-checked') !== 'true'; t.setAttribute('aria-checked', String(on)); set(key, on ? 'on' : 'off'); };
      return t;
    }
    if (kind === 'volume') {
      const out = el('span', { className: 'v', textContent: now });
      const r = el('input', { type: 'range', min: 0, max: 10, step: 1, value: now, disabled: none, title: key });
      r.oninput = () => out.textContent = r.value;
      r.onchange = () => set(key, r.value);
      return el('span', { className: 'end' }, r, out);
    }
    const opts = kind.map(c => Array.isArray(c) ? c : [c, c]);
    // a value set some other way (a flag, an edit) stays listed
    if (!opts.some(([w]) => w === now)) opts.push([now, now]);
    const s = el('select', { disabled: none, title: key }, ...opts.map(([w, label]) => el('option', { value: w, textContent: label, selected: w === now })));
    s.onchange = () => set(key, s.value);
    return s;
  };
  const known = new Set(GROUPS.flatMap(([, rows]) => rows.map(r => r[0])));
  const other = [];
  for (const l of (text ?? '').split('\n')) {
    const [k, v] = (l.split('#')[0] ?? '').split(/=(.*)/s);
    if (v !== undefined && !known.has(k.trim()) && !HIDDEN.has(k.trim())) other.push([k.trim(), v.trim()]);
  }
  const groups = [...GROUPS];
  if (other.length) groups.push(['Other', other.map(([k, v]) => [k, k, [v], v])]);
  groups.forEach(([name, rows], i) => {
    const d = el('details', { className: 'group', open: i === 0 }, el('summary', { textContent: name }));
    for (const [key, label, kind, def] of rows) d.append(el('div', { className: 'r' }, el('span', { textContent: label }), control(key, kind, def)));
    box.append(d);
  });
  if (!none) box.append(el('div', { className: 'r' }, el('span', { className: 'note', textContent: 'Back to the defaults (a phone gets its graphics preset again).' }),
    pill('Reset settings', () => { if (confirm('Reset every setting to its default?')) { try { localStorage.removeItem(SETTINGS); } catch {} prefs(); } }, 'red')));
}

// ---------------------------------------------------------------- OPFS

const sub = async (d, name) => { try { return await d.getDirectoryHandle(name); } catch { return null; } };
const bytes = async d => {
  let n = 0;
  if (!d) return 0;
  for await (const h of d.values()) n += h.kind === 'file' ? (await h.getFile()).size : await bytes(h);
  return n;
};
const fileSize = async (d, name) => { try { return (await (await d.getFileHandle(name)).getFile()).size; } catch { return 0; } };
const drop = async (d, name) => { try { await d.removeEntry(name, { recursive: true }); return true; } catch (e) { return e.name === 'NotFoundError' || e; } };

// M16: installed mods (OPFS mods/<id>/, mod.json beside their files), with remove and remove all
async function mods(parts) {
  const box = $('#modlist');
  const dir = await sub(parts.dir, 'mods');
  const list = [];
  if (dir) for await (const [id, h] of dir.entries()) {
    if (h.kind !== 'directory') continue;
    let m = {};
    try { m = JSON.parse(await (await (await h.getFileHandle('mod.json')).getFile()).text()); } catch {}
    list.push({ id, name: m.name || id, kind: m.kind || 'character', source: m.source ?? '', size: await bytes(h) });
  }
  list.sort((a, b) => a.name.localeCompare(b.name));
  box.replaceChildren();
  if (!list.length) {
    box.append(el('p', { className: 'note', textContent: 'No mods installed. Add them in the game: Mods › Browse, or a .zip of your own.' }));
    return;
  }
  const remove = async ids => {
    for (const id of ids) {
      const r = await drop(dir, id);
      if (r !== true) alert(`Couldn't remove ${id}: ${r.message ?? r}. Close the game's tab and try again.`);
    }
    mods(parts);
    storage(parts);
  };
  box.append(
    el('div', { className: 'r' }, el('span', { textContent: `${list.length} mod${list.length > 1 ? 's' : ''}, ${size(list.reduce((n, m) => n + m.size, 0))}` }),
      pill('Remove all', () => confirm(`Remove all ${list.length} mods from this browser?`) && remove(list.map(m => m.id)), 'red')),
    el('div', { className: 'mods' }, ...list.map(m => el('div', { className: 'mod' },
      el('span', { className: 'kind', textContent: m.kind }),
      el('div', {}, el('b', { textContent: m.name, title: m.id }), el('span', { textContent: [m.source, size(m.size)].filter(Boolean).join(' · ') })),
      pill('Remove', () => confirm(`Remove ${m.name}?`) && remove([m.id]))))));
}

// what the page keeps: the disc pack, mods, the select's cards, reports, the offline copy, the settings; with the
// usage the browser reports, and clear / forget / erase
async function storage(parts) {
  const box = $('#storage'), dir = parts.dir;
  const est = await navigator.storage?.estimate?.().catch(() => null) ?? null;
  const pack = await fileSize(dir, 'disc.pack') + await fileSize(dir, 'disc.part');
  const modBytes = await bytes(await sub(dir, 'mods'));
  const cards = await bytes(await sub(dir, 'previews'));
  const reports = await bytes(await sub(dir, 'reports'));
  let local = 0;
  try { for (let i = 0; i < localStorage.length; i++) { const k = localStorage.key(i); local += 2 * (k.length + (localStorage.getItem(k)?.length ?? 0)); } } catch {}
  const used = est?.usage ?? 0;
  const known = pack + modBytes + cards + reports;
  const cached = est?.usageDetails?.caches ?? Math.max(0, used - known);
  const segs = [['Disc data', pack, '#2fae4a'], ['Mods', modBytes, '#5fb9e6'], ['Cards', cards, '#f5b81c'], ['Offline copy', cached, '#9b8cff'], ['Reports & settings', reports + local, '#c9ccd3']];
  const total = Math.max(used, segs.reduce((n, s) => n + s[1], 0), 1);
  const persisted = await navigator.storage?.persisted?.().catch(() => false);
  const busy = async (what, f) => { const r = await f(); if (r !== true && r !== undefined) alert(`Couldn't ${what}: ${r.message ?? r}. Close the game's tab and try again.`); storage(parts); };
  const row = (label, n, note, ...buttons) => el('div', { className: 'r' },
    el('span', {}, label, ' ', el('span', { className: 'v', textContent: n == null ? '' : size(n) })),
    el('span', { className: 'end' }, note ? el('span', { className: 'note', textContent: note }) : null, ...buttons));
  // (nulls left out: replaceChildren would show them as text)
  box.replaceChildren(...[
    dir.memory ? el('p', { className: 'note', textContent: MEMORY_NOTE }) : null,
    el('p', {}, el('b', { textContent: size(used || known + local) }), est?.quota ? ` used of ${size(est.quota)} this browser allows.` : ' used.'),
    el('div', { className: 'bar' }, ...segs.filter(s => s[1] > 0).map(([l, n, c]) => el('i', { title: `${l}: ${size(n)}`, style: `width:${100 * n / total}%;background:${c}` }))),
    el('p', { className: 'note legend' }, ...segs.map(([l, , c]) => el('span', {}, el('span', { className: 'key', style: `background:${c}` }), l))),
    row('Disc data', pack, pack ? null : 'none imported', pack ? pill('Forget disc', () => {
      if (!confirm('Forget the imported disc? You\'ll pick your ISO again to play. Settings and mods stay.')) return;
      busy('forget the disc', async () => { const a = await drop(dir, 'disc.pack'); await drop(dir, 'disc.part'); if (a === true) parts.have(false); return a; });
    }, 'red') : null),
    row('Mods', modBytes, null, el('a', { className: 'pill small', href: '#modlist', textContent: 'Manage', onclick: e => { e.preventDefault(); $('#modlist').scrollIntoView({ behavior: 'smooth', block: 'center' }); } })),
    row('Character cards', cards, 'the select\'s pictures, made again as needed', pill('Clear', () => busy('clear the cards', () => drop(dir, 'previews')), '')),
    row('Offline copy', est?.usageDetails?.caches ?? null, 'the game\'s files, downloaded again', pill('Clear', () => busy('clear the offline copy', async () => {
      for (const k of await caches?.keys?.() ?? []) await caches.delete(k);
      try { localStorage.removeItem(STORE_CACHE); } catch {}
      return true;
    }))),
    row('Desync reports', reports, null, reports ? pill('Clear', () => busy('clear the reports', () => drop(dir, 'reports'))) : null),
    !dir.memory && persisted === false && navigator.storage?.persist ? row('Keep it', null, 'the browser may clear all this when space runs low', pill('Keep my data', () => navigator.storage.persist().then(ok => { if (!ok) alert('The browser said no (Chrome agrees once the site is installed or bookmarked).'); storage(parts); }))) : null,
    row('Everything', null, 'disc data, mods, settings, login', pill('Erase everything', () => {
      if (!confirm('Erase everything OpenTennis keeps in this browser: the disc data, mods, cards, settings, your name and your Hugging Face login?')) return;
      busy('erase everything', async () => {
        let res = true;
        const names = [];
        for await (const name of dir.keys()) names.push(name);
        for (const name of names) { const r = await drop(dir, name); if (r !== true) res = r; }
        try { localStorage.clear(); sessionStorage.clear(); } catch {}
        for (const k of await caches?.keys?.() ?? []) await caches.delete(k);
        parts.have(false);
        username(); account(); prefs(); mods(parts);
        return res;
      });
    }, 'red')),
  ].filter(Boolean));
}

// ---------------------------------------------------------------- donate

// M97's counts (web/stats.json, tools/release.sh)
async function stats() {
  const s = await fetch('stats.json', { cache: 'no-cache' }).then(r => r.ok ? r.json() : null, () => null);
  if (!s) return;
  const n = x => x >= 1e9 ? `${(x / 1e9).toFixed(1)} billion` : x >= 1e6 ? `${(x / 1e6).toFixed(1)} million` : x.toLocaleString('en');
  const box = $('#stats');
  box.append(...[[s.prompts, 'prompts'], [s.tokens, 'tokens'], [s.commits, 'commits'], [s.sessions, 'sessions']]
    .filter(([x]) => typeof x === 'number').map(([x, l]) => el('div', { className: 'stat' }, el('b', { textContent: n(x) }), el('span', { textContent: l }))));
  if (s.since) box.after(el('p', { className: 'note', textContent: `Since ${s.since}${s.updated ? `, counted ${s.updated}` : ''}.` }));
  box.hidden = false;
}
