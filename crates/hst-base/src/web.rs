//! The browser build (`web/`, built by `tools/web.sh`). The page imports the user's ISO once: a worker calls
//! `import_disc` with the picked `File` (read by extent with `File.slice`, never whole) and an OPFS file to write the
//! pack into (`Iso::write_pack_to`). Every later visit reads that pack and calls `run`, which mounts it in memory
//! (`hst_data::iso::mount`) where every `Iso::open` finds it. Flags come from the URL: `?play&stage=1` = `--play --stage 1`;
//! `?code=<lobby code>` (a lobby link, B104) starts on that lobby's character pick (`online::linked`); Join by Code's
//! Paste takes the last text pasted on the page (`paste`), and a phone types the code on its own keyboard (`typed_code`).

use hst_data::iso::Iso;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use bevy::prelude::Res;
use wasm_bindgen::prelude::*;
use web_sys::{File, FileReaderSync, FileSystemReadWriteOptions, FileSystemSyncAccessHandle};

/// The path the mounted pack answers to (settings.txt and controls.txt "beside" it are in localStorage).
pub const PACK: &str = "disc.pack";

fn io_err(e: JsValue) -> io::Error {
    io::Error::other(js_err(e))
}

/// The picked ISO, read synchronously (workers only) by `File.slice`.
struct Picked {
    file: File,
    reader: FileReaderSync,
    pos: u64,
}

impl Read for Picked {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let end = (self.pos + buf.len() as u64).min(self.file.size() as u64);
        if end <= self.pos {
            return Ok(0);
        }
        let blob = self.file.slice_with_f64_and_f64(self.pos as f64, end as f64).map_err(io_err)?;
        let ab = self.reader.read_as_array_buffer(&blob).map_err(io_err)?;
        let n = (end - self.pos) as usize;
        js_sys::Uint8Array::new(&ab).copy_to(&mut buf[..n]);
        self.pos = end;
        Ok(n)
    }
}

fn seek(pos: &mut u64, len: u64, to: SeekFrom) -> io::Result<u64> {
    let p = match to {
        SeekFrom::Start(p) => p as i64,
        SeekFrom::Current(d) => *pos as i64 + d,
        SeekFrom::End(d) => len as i64 + d,
    };
    *pos = u64::try_from(p).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "seek before start"))?;
    Ok(*pos)
}

impl Seek for Picked {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        seek(&mut self.pos, self.file.size() as u64, to)
    }
}

/// An OPFS file, written synchronously (workers only).
struct Opfs {
    h: FileSystemSyncAccessHandle,
    pos: u64,
}

impl Write for Opfs {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let at = FileSystemReadWriteOptions::new();
        at.set_at(self.pos as f64);
        let n = self.h.write_with_u8_array_and_options(buf, &at).map_err(io_err)? as usize;
        self.pos += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.h.flush().map_err(io_err)
    }
}

impl Seek for Opfs {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let len = self.h.get_size().map_err(io_err)? as u64;
        seek(&mut self.pos, len, to)
    }
}

/// Worker: check `iso` is the supported disc and write its pack into `out` (emptied first). `progress(i, n)` per
/// file. Returns the pack hash in hex; the page renames the file into place when this returns.
#[wasm_bindgen]
pub fn import_disc(iso: File, out: FileSystemSyncAccessHandle, progress: js_sys::Function) -> Result<String, JsValue> {
    let err = |e: io::Error| JsValue::from_str(&e.to_string());
    out.truncate_with_f64(0.0)?;
    let mut disc = Iso::from_source(Box::new(Picked { file: iso, reader: FileReaderSync::new()?, pos: 0 })).map_err(err)?;
    let mut w = BufWriter::with_capacity(1 << 20, Opfs { h: out, pos: 0 });
    let hash = disc
        .write_pack_to(&mut w, |i, n| {
            let _ = progress.call2(&JsValue::NULL, &(i as u32).into(), &(n as u32).into());
        })
        .map_err(err)?;
    Ok(format!("{hash:016x}"))
}

/// The pack written without OPFS (B457: a private window refuses it): the first 1 MB (the header, whose hash is
/// written last) kept here, every later MB handed to `chunk` as it fills, so the worker's memory holds little of it.
struct Chunks {
    head: Vec<u8>,
    cur: Vec<u8>,
    pos: u64,
    len: u64,
    chunk: js_sys::Function,
}

const HEAD: usize = 1 << 20;

impl Chunks {
    fn send(&mut self) -> io::Result<()> {
        if !self.cur.is_empty() {
            self.chunk.call1(&JsValue::NULL, &js_sys::Uint8Array::from(&self.cur[..])).map_err(io_err)?;
            self.cur.clear();
        }
        Ok(())
    }
}

impl Write for Chunks {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let p = self.pos as usize;
        let n = if p < HEAD {
            // the header's part: overwrites in place (the hash), else grows it to HEAD
            let n = buf.len().min(HEAD - p);
            let end = p + n;
            if self.head.len() < end {
                if p > self.head.len() {
                    return Err(io::Error::other("gap in the pack"));
                }
                self.head.resize(end, 0);
            }
            self.head[p..end].copy_from_slice(&buf[..n]);
            n
        } else if self.pos == self.len {
            self.cur.extend_from_slice(buf);
            if self.cur.len() >= HEAD {
                self.send()?;
            }
            buf.len()
        } else {
            return Err(io::Error::other("rewrite past the pack's header"));
        };
        self.pos += n as u64;
        self.len = self.len.max(self.pos);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for Chunks {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        seek(&mut self.pos, self.len, to)
    }
}

/// Worker, B457: `import_disc` for a browser without OPFS: the pack's parts go to `chunk(bytes)` in order after the
/// first, which is returned with the hash as `[hash, first]` (the page joins them in a Blob, first in front).
#[wasm_bindgen]
pub fn import_disc_to_memory(iso: File, progress: js_sys::Function, chunk: js_sys::Function) -> Result<js_sys::Array, JsValue> {
    let err = |e: io::Error| JsValue::from_str(&e.to_string());
    let mut disc = Iso::from_source(Box::new(Picked { file: iso, reader: FileReaderSync::new()?, pos: 0 })).map_err(err)?;
    let mut w = BufWriter::with_capacity(1 << 20, Chunks { head: Vec::new(), cur: Vec::new(), pos: 0, len: 0, chunk });
    let hash = disc
        .write_pack_to(&mut w, |i, n| {
            let _ = progress.call2(&JsValue::NULL, &(i as u32).into(), &(n as u32).into());
        })
        .map_err(err)?;
    let mut c = w.into_inner().map_err(|e| err(e.into_error()))?;
    c.send().map_err(err)?;
    Ok(js_sys::Array::of2(&format!("{hash:016x}").into(), &js_sys::Uint8Array::from(&c.head[..])))
}

/// The pack as the page reads it in ([`pack_begin`], [`pack_part`]): the buffer and how much of it is filled.
pub type Filling = (Arc<[std::mem::MaybeUninit<u8>]>, usize);

thread_local! {
    pub static FILLING: std::cell::RefCell<Option<Filling>> = const { std::cell::RefCell::new(None) };
}

/// Page: a pack of `len` bytes follows in parts ([`pack_part`]), then [`run`]. B468: the page reads the pack from
/// OPFS a part at a time straight into this one buffer, so it is never held whole in JS as well (≈ 171 MB more until
/// the browser's garbage collector got to it, on a phone that's what a tab kill needs).
#[wasm_bindgen]
pub fn pack_begin(len: usize) {
    FILLING.with(|f| *f.borrow_mut() = Some((Arc::new_uninit_slice(len), 0)));
}

/// Page: the pack's next bytes.
#[wasm_bindgen]
pub fn pack_part(part: js_sys::Uint8Array) -> Result<(), JsValue> {
    FILLING.with(|f| {
        let mut f = f.borrow_mut();
        let (buf, at) = f.as_mut().ok_or("pack_part before pack_begin")?;
        let end = *at + part.length() as usize;
        let dst = Arc::get_mut(buf).unwrap().get_mut(*at..end).ok_or("pack_part past the pack's length")?;
        part.copy_to_uninit(dst);
        *at = end;
        Ok(())
    })
}

/// The page's boot status (`index.html`) goes once the game has drawn its first frames.
pub fn shown(frames: Res<bevy::diagnostic::FrameCount>) {
    if frames.0 == 3 {
        if let Ok(f) = js_sys::Reflect::get(&js_sys::global(), &"hstShown".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
            let _ = f.call0(&JsValue::NULL);
        }
    }
}

/// B469 (`?log`): each time the wasm heap grows, its new size and when. It never shrinks, and a phone's browser
/// kills the tab past its memory limit: a growth mid-match is something allocated on first use.
pub fn heap_growth(time: Res<bevy::time::Time<bevy::time::Real>>, mut last: bevy::prelude::Local<usize>) {
    let now = core::arch::wasm32::memory_size(0) * 65536;
    if now != *last {
        *last = now;
        bevy::log::info!("wasm heap {} MB at {:.2} s", now >> 20, time.elapsed_secs());
    }
}

/// B468: the system allocator, counting the bytes live and the most live at once since [`memory_log`] last read it.
struct Counted;

static LIVE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static PEAK: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[global_allocator]
static ALLOC: Counted = Counted;

unsafe impl std::alloc::GlobalAlloc for Counted {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc(l) };
        if !p.is_null() {
            grew(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc_zeroed(l) };
        if !p.is_null() {
            grew(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(p, l) };
        LIVE.fetch_sub(l.size(), std::sync::atomic::Ordering::Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, size: usize) -> *mut u8 {
        let q = unsafe { std::alloc::System.realloc(p, l, size) };
        if !q.is_null() {
            LIVE.fetch_sub(l.size(), std::sync::atomic::Ordering::Relaxed);
            grew(size);
        }
        q
    }
}

fn grew(n: usize) {
    let live = LIVE.fetch_add(n, std::sync::atomic::Ordering::Relaxed) + n;
    PEAK.fetch_max(live, std::sync::atomic::Ordering::Relaxed);
}

/// B468: the wasm heap's size in the console each time it has grown another 32 MB, and once a second while the bytes
/// live (or their peak since the last line) moved 8 MB: `hst: wasm memory N MB (live L MB, peak P MB)`. The heap never
/// shrinks, so its size is the peak so far: what a phone's tab kill (WebKit drops a tab over its memory limit and
/// reloads the page) leaves to read in a remote Web Inspector.
pub fn memory_log(time: Res<bevy::time::Time<bevy::time::Real>>, mut logged: bevy::prelude::Local<(usize, usize, f32)>) {
    let heap = core::arch::wasm32::memory_size(0) * 65536;
    let now = time.elapsed_secs();
    let (live, peak) = (LIVE.load(std::sync::atomic::Ordering::Relaxed), PEAK.load(std::sync::atomic::Ordering::Relaxed));
    let moved = live.abs_diff(logged.1).max(peak.saturating_sub(logged.1)) >= 8 << 20 && now >= logged.2 + 1.0;
    if heap >= logged.0 + (32 << 20) || moved {
        *logged = (heap.max(logged.0), live, now);
        PEAK.store(live, std::sync::atomic::Ordering::Relaxed);
        bevy::log::info!("hst: wasm memory {} MB (live {} MB, peak {} MB)", heap >> 20, live >> 20, peak >> 20);
    }
}

/// B260: tells the page whether a match is on (`hstMatch`): a hidden tab steps the game on a timer only outside one
/// (`index.html`), so a match holds while hidden.
pub fn in_match(on: bool) {
    if IN_MATCH.swap(on, std::sync::atomic::Ordering::Relaxed) != on {
        let _ = js_sys::Reflect::set(&js_sys::global(), &"hstMatch".into(), &on.into());
    }
}

static IN_MATCH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// P58: this frame came from the page's timer stand-in (`index.html`: a hidden tab, or an unfocused window the
/// compositor gives no frames), so the wait before it was the page's, not a stall.
pub fn timer_frame() -> bool {
    js_sys::Reflect::get(&js_sys::global(), &"hstTimerFrame".into()).is_ok_and(|v| v.is_truthy())
}

/// B324: the best frame rate the game has held over a whole second, in fps × 10: [in the menus, in a match].
static BEST_FPS: [std::sync::atomic::AtomicU32; 2] = [const { std::sync::atomic::AtomicU32::new(0) }; 2];

/// B324: counts the frames drawn in each second (real time) and keeps the best second's rate, the menus' and a
/// match's apart (a phone's GPU can keep up in the menus and not in a match). A second that crosses into or out of
/// a match counts for neither.
pub fn frame_rate(time: Res<bevy::time::Time<bevy::time::Real>>, mut w: bevy::prelude::Local<(f64, u32, bool)>) {
    let (now, on) = (time.elapsed_secs_f64(), IN_MATCH.load(std::sync::atomic::Ordering::Relaxed));
    if w.1 == 0 || w.2 != on {
        *w = (now, 1, on);
        return;
    }
    let span = now - w.0;
    if span >= 1.0 {
        let fps = ((w.1 as f64 / span) * 10.0).round() as u32;
        BEST_FPS[on as usize].fetch_max(fps, std::sync::atomic::Ordering::Relaxed);
        *w = (now, 1, on);
    } else {
        w.1 += 1;
    }
}

/// B324: the display's refresh rate (Hz) as the page measured it from requestAnimationFrame before the game was
/// loaded (`hstRefresh`, index.html), when it could.
pub fn refresh() -> Option<f64> {
    js_sys::Reflect::get(&js_sys::global(), &"hstRefresh".into()).ok()?.as_f64().filter(|&hz| hz > 0.0)
}

/// P53: what the page read of the device for the Auto graphics preset (`hstDevice`, index.html): the screen in CSS px,
/// the DPR, the cores, Chromium's `deviceMemory` and WebGPU's `maxBufferSize`.
pub fn device(d: &mut crate::graphics::Device) {
    let Ok(o) = js_sys::Reflect::get(&js_sys::global(), &"hstDevice".into()) else { return };
    let num = |k: &str| js_sys::Reflect::get(&o, &k.into()).ok().and_then(|v| v.as_f64()).filter(|v| v.is_finite() && *v > 0.0).unwrap_or(0.0);
    let (w, h) = (num("w") as u32, num("h") as u32);
    d.css = [w.min(h), w.max(h)];
    d.dpr = num("dpr") as f32;
    d.cores = num("cores") as u32;
    d.ram_mb = (num("ram") * 1024.0) as u32;
    d.gpu_buffer_mb = (num("gpuBuffer") / 1048576.0) as u32;
}

/// B324: on the WebGL2 build, what it reaches here: the best second's frame rate (a match's once one has run, and
/// whether it was a match's) and the display's refresh rate. `None` on the WebGPU build and before a whole second has
/// been measured (words: `options::webgl2_reach`).
pub fn webgl2_reach() -> Option<(f64, bool, Option<f64>)> {
    if cfg!(feature = "webgpu") {
        return None;
    }
    let [menus, play] = [0, 1].map(|i| BEST_FPS[i].load(std::sync::atomic::Ordering::Relaxed));
    let fps = if play > 0 { play } else { menus };
    (fps > 0).then(|| (fps as f64 / 10.0, play > 0, refresh()))
}

/// The last text pasted on the page (the browser's clipboard can't be read synchronously: Join by Code takes this).
static PASTED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Page: text pasted (Ctrl+V / the paste menu), or a LAN answer handed over by another tab (`index.html`): an answer
/// goes to the hosting lobby (`net/rtc.rs`), anything else to Join by Code.
#[wasm_bindgen]
pub fn paste(text: String) {
    if crate::net::rtc::deliver(&text) {
        return;
    }
    *PASTED.lock().unwrap() = Some(text);
}

pub fn pasted() -> Option<String> {
    PASTED.lock().unwrap().clone()
}

/// localStorage under the file's path (`crate::store`): settings.txt and controls.txt last between visits.
pub fn read_to_string(path: impl AsRef<std::path::Path>) -> io::Result<String> {
    let key = path.as_ref().to_string_lossy();
    let Ok(local) = local() else {
        return MEMORY.lock().unwrap().get(&*key).cloned().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, key.into_owned()));
    };
    local.get_item(&key).map_err(io_err)?.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, key.into_owned()))
}

pub fn write(path: impl AsRef<std::path::Path>, text: impl AsRef<[u8]>) -> io::Result<()> {
    let Ok(local) = local() else {
        let (k, v) = (path.as_ref().to_string_lossy().into_owned(), String::from_utf8_lossy(text.as_ref()).into_owned());
        MEMORY.lock().unwrap().insert(k, v);
        return Ok(());
    };
    local.set_item(&path.as_ref().to_string_lossy(), &String::from_utf8_lossy(text.as_ref())).map_err(io_err)
}

fn local() -> io::Result<web_sys::Storage> {
    web_sys::window().unwrap().local_storage().map_err(io_err)?.ok_or_else(|| io::Error::other("no localStorage"))
}

/// B457: the files for this visit only when the browser refuses localStorage (blocked site data, a private window).
static MEMORY: std::sync::Mutex<std::collections::BTreeMap<String, String>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// The page's on-screen keyboard's keys (`index.html`; Backspace as `\u{8}`) for Join by Code, since the last call.
static TYPED: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Page: text typed on the phone's keyboard while Join by Code's code entry is up; Return / Go as `\n` (B207).
#[wasm_bindgen]
pub fn typed_code(text: String) {
    TYPED.lock().unwrap().push_str(&text);
}

pub fn typed() -> String {
    std::mem::take(&mut *TYPED.lock().unwrap())
}

/// A file for the player to keep (a desync's report, B154): the page's `hstSaveFile` keeps it in OPFS `reports/` and
/// downloads it; the text goes to the console too, so a page without the hook (or a blocked download) still has it.
pub fn save_file(name: &str, text: &str) -> io::Result<String> {
    bevy::log::info!("{name}: {text}");
    let f = js_sys::Reflect::get(&web_sys::window().unwrap(), &"hstSaveFile".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok());
    let f = f.ok_or_else(|| io::Error::other("no hstSaveFile on the page (the report is in the console)"))?;
    f.call2(&JsValue::NULL, &name.into(), &text.into()).map_err(io_err)?;
    Ok(format!("{name} (downloaded; also in the console)"))
}

/// Join by Code's code entry shown or gone: the page puts a tap target up that opens the phone's keyboard (B323: a
/// URL keyboard, no capitals, for the relay's URL).
pub fn code_entry(on: bool, url: bool) {
    let f = js_sys::Reflect::get(&web_sys::window().unwrap(), &"hstCodeEntry".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok());
    if let Some(f) = f {
        let _ = f.call2(&JsValue::NULL, &on.into(), &url.into());
    }
}

/// B464: Copy Link on a touch screen: the page shows the text on a card with its own Copy and Share buttons (iOS Safari
/// writes the clipboard only inside a tap's own event, which is over by the time the game sees it). False where the
/// page has no card for it (a mouse and keyboard): the clipboard is written directly.
pub fn copy_sheet(text: &str) -> bool {
    page_fn("hstCopySheet").and_then(|f| f.call1(&JsValue::NULL, &text.into()).ok()).is_some_and(|v| v.is_truthy())
}

/// The page's size in CSS px (the canvas fills it): where touches and the mouse are measured. The canvas's own box
/// first (B333: on iOS the window's inner size can lag the screen the canvas was just fitted to), else the window's.
pub fn page_size() -> Option<bevy::math::Vec2> {
    let w = web_sys::window()?;
    if let Some(c) = w.document().and_then(|d| d.query_selector("canvas").ok().flatten()) {
        let r = c.get_bounding_client_rect();
        if r.width() > 0.0 && r.height() > 0.0 {
            return Some(bevy::math::Vec2::new(r.width() as f32, r.height() as f32));
        }
    }
    let (x, y) = (w.inner_width().ok()?.as_f64()?, w.inner_height().ok()?.as_f64()?);
    (x > 0.0 && y > 0.0).then(|| bevy::math::Vec2::new(x as f32, y as f32))
}

/// A menu is up (B221: it takes taps itself) or a match is on: in a menu the page's on-screen pad keeps its stick to a
/// fixed corner circle so taps elsewhere reach the menu (B244); in a match the stick follows the thumb.
pub fn touch_menu(on: bool) {
    if let Some(f) = page_fn("hstTouchMenu") {
        let _ = f.call1(&JsValue::NULL, &on.into());
    }
}

/// B118: the page's on-screen pad's press vibration (Settings → Mobile). Sent when it changes.
pub fn touch_buzz(buzz: bool) {
    static SENT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(u8::MAX);
    let v = buzz as u8;
    if SENT.swap(v, std::sync::atomic::Ordering::Relaxed) != v
        && let Some(f) = page_fn("hstTouchBuzz")
    {
        let _ = f.call1(&JsValue::NULL, &buzz.into());
    }
}

/// B312: the page's chat button (beside Start on the on-screen pad, `web/touch.js`) shown or gone. Sent when it changes.
pub fn touch_chat(on: bool) {
    static SENT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(u8::MAX);
    if SENT.swap(on as u8, std::sync::atomic::Ordering::Relaxed) != on as u8
        && let Some(f) = page_fn("hstTouchChat")
    {
        let _ = f.call1(&JsValue::NULL, &on.into());
    }
}

/// B312: taps on the page's chat button since the last call (none while it's hidden).
pub fn chat_tapped() -> bool {
    page_fn("hstChatTaps").and_then(|f| f.call0(&JsValue::NULL).ok()).and_then(|v| v.as_f64()).is_some_and(|n| n > 0.0)
}

/// B273: the renderer setting saved (`auto`, `webgpu`, `webgl2`): the page reloads into that build once it's left
/// alone for a moment (a no-op when it's the one running).
pub fn renderer(word: &str) {
    if let Some(f) = page_fn("hstRenderer") {
        let _ = f.call1(&JsValue::NULL, &word.into());
    }
}

/// B247: the page is played by touch (a touch screen, or touched once; `web/touch.js`).
pub fn touched() -> bool {
    page_fn("hstTouched").and_then(|f| f.call0(&JsValue::NULL).ok()).is_some_and(|v| v.is_truthy())
}

/// B329: what the Vibration setting can do in this browser (`web/touch.js`): `vibrate` (Android), `firefox`
/// (Firefox for Android: the API is there but never vibrates, B525) or `none` (iOS Safari: no vibration API).
pub fn vibration() -> String {
    page_fn("hstVibration").and_then(|f| f.call0(&JsValue::NULL).ok()).and_then(|v| v.as_string()).unwrap_or_else(|| "none".into())
}

/// B207: a controller other than the page's touch pad is connected (`web/touch.js`).
pub fn real_pad() -> bool {
    page_fn("hstRealPad").and_then(|f| f.call0(&JsValue::NULL).ok()).is_some_and(|v| v.is_truthy())
}

/// The select's baked cards (B120, `play/main_menu/previews.rs`): OPFS `previews/` files by name, all read by the page
/// before `run` (`add_card`); new ones written back through the page's `hstCard`, old drawings' dropped by `hstCardDrop`.
static CARDS: std::sync::Mutex<Option<std::collections::HashMap<String, Arc<[u8]>>>> = std::sync::Mutex::new(None);

/// Page: one cached card's file.
#[wasm_bindgen]
pub fn add_card(name: String, bytes: js_sys::Uint8Array) {
    CARDS.lock().unwrap().get_or_insert_default().insert(name, bytes.to_vec().into());
}

pub fn card(name: &str) -> Option<Arc<[u8]>> {
    CARDS.lock().unwrap().as_ref()?.get(name).cloned()
}

/// Keep `name` and write it to OPFS (the page's write lands a little later; a closed tab loses only that card).
pub fn put_card(name: String, png: Vec<u8>) -> io::Result<()> {
    let f = page_fn("hstCard").ok_or_else(|| io::Error::other("no hstCard"))?;
    f.call2(&JsValue::NULL, &name.as_str().into(), &js_sys::Uint8Array::from(png.as_slice())).map_err(io_err)?;
    CARDS.lock().unwrap().get_or_insert_default().insert(name, png.into());
    Ok(())
}

/// B372: the store's tile pictures in the card store (`modstore::keep_thumb`): not a drawing, never dropped with them.
pub const PICTURES: &str = "mod-pictures~";

/// Drop every card whose name doesn't start with `prefix` (an older drawing's), but the store's tile pictures.
pub fn cards_keep(prefix: &str) {
    let mut cards = CARDS.lock().unwrap();
    let Some(cards) = cards.as_mut() else { return };
    let drop = page_fn("hstCardDrop");
    cards.retain(|name, _| {
        let keep = name.starts_with(prefix) || name.starts_with(PICTURES);
        if !keep && let Some(f) = &drop {
            let _ = f.call1(&JsValue::NULL, &name.as_str().into());
        }
        keep
    });
}

/// B259: drop (and delete from OPFS) every card `gone` names.
pub fn cards_drop(gone: impl Fn(&str) -> bool) {
    let mut cards = CARDS.lock().unwrap();
    let Some(cards) = cards.as_mut() else { return };
    let drop = page_fn("hstCardDrop");
    cards.retain(|name, _| {
        let keep = !gone(name);
        if !keep && let Some(f) = &drop {
            let _ = f.call1(&JsValue::NULL, &name.as_str().into());
        }
        keep
    });
}

fn page_fn(name: &str) -> Option<js_sys::Function> {
    js_sys::Reflect::get(&web_sys::window().unwrap(), &name.into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok())
}

/// M16's mods (`modfs.rs`, `play/main_menu/mods_screen.rs`): the page reads OPFS `mods/` into the game before `run`
/// (`add_mod_file`), imports a picked zip (`import_mod`, then writes `mod_files` back to OPFS), and reports how that
/// went (`mod_status`); the game asks the page to open its picker, place it over the Add mod row, drop a mod, reload.
static MOD_STATUS: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Page: one installed mod file, `mods/<id>/…`.
#[wasm_bindgen]
pub fn add_mod_file(path: String, bytes: js_sys::Uint8Array) {
    if crate::modfs::plain(&path) {
        crate::modfs::add(std::path::Path::new(&path), shared(&bytes));
    }
}

/// Page: an installed mod file left in OPFS (B336, `modfs::add_lazy`): read in when the game wants the mod (`mod_load`).
#[wasm_bindgen]
pub fn add_mod_lazy(path: String, size: f64) {
    if crate::modfs::plain(&path) {
        crate::modfs::add_lazy(std::path::Path::new(&path), size as u64);
    }
}

/// Asks the page to read mod folder `dir`'s files in from OPFS (`add_mod_file` each, then `mod_loaded`).
pub fn mod_load(dir: &str) {
    match page_fn("hstModLoad") {
        Some(f) => drop(f.call1(&JsValue::NULL, &dir.into())),
        None => crate::modfs::loaded(dir),
    }
}

/// Page: done reading folder `dir` in (`mod_load`).
#[wasm_bindgen]
pub fn mod_loaded(dir: String) {
    crate::modfs::loaded(&dir);
}

/// Page: mod `id` is stored in OPFS: its big files out of memory again until it's wanted (B336: Download all's 40).
#[wasm_bindgen]
pub fn mod_saved(id: String) {
    crate::modfs::unload(&crate::modfs::root().join(id));
}

/// `b` copied once, straight into a shared buffer (B204: `to_vec().into()` held every mod file twice on the way).
fn shared(b: &js_sys::Uint8Array) -> Arc<[u8]> {
    let mut out = Arc::<[u8]>::new_uninit_slice(b.length() as usize);
    b.copy_to_uninit(Arc::get_mut(&mut out).unwrap());
    // SAFETY: copy_to_uninit filled every byte
    unsafe { out.assume_init() }
}

/// Page: a picked mod zip (a `File`), checked and installed a file at a time (`modfs::install_with`, in memory):
/// `{id, name}`, or why it was refused (nothing of it kept).
#[wasm_bindgen]
pub async fn import_mod(zip: web_sys::Blob) -> Result<JsValue, JsValue> {
    let root = crate::modfs::root();
    let m = crate::modfs::install_with(zip.size() as u64, |at, n| read_blob(zip.clone(), at, n), root, None, None, false, false).await.map_err(JsValue::from)?;
    let o = js_sys::Object::new();
    js_sys::Reflect::set(&o, &"id".into(), &m.folder().into())?;
    js_sys::Reflect::set(&o, &"name".into(), &m.name.into())?;
    Ok(o.into())
}

/// Page: an installed mod's file paths inside it, `mod.json` last (a write cut short leaves a folder without one,
/// which the page drops at the next start). The page takes each file's bytes as it writes it (`mod_file`: B204, never
/// the whole mod at once).
#[wasm_bindgen]
pub fn mod_paths(id: String) -> Vec<String> {
    let dir = crate::modfs::root().join(&id);
    let pre = format!("{}/", crate::modfs::key(&dir));
    let mut files: Vec<String> = crate::modfs::files_under(&dir).into_iter().map(|(p, _)| p[pre.len()..].to_string()).collect();
    files.sort_by_key(|f| f == "mod.json");
    files
}

/// Page: file `path` of installed mod `id`.
#[wasm_bindgen]
pub fn mod_file(id: String, path: String) -> Option<js_sys::Uint8Array> {
    let p = crate::modfs::root().join(id).join(path);
    crate::modfs::read(&p).ok().map(|b| js_sys::Uint8Array::from(&b[..]))
}

/// A JS error as words: an `Error`'s name and message, a string as is (B223: iOS showed "JsValue(undefined)").
pub fn js_err(e: JsValue) -> String {
    if let Some(s) = e.as_string() {
        return s;
    }
    if let Some(e) = e.dyn_ref::<js_sys::Error>() {
        return format!("{}: {}", String::from(e.name()), String::from(e.message()));
    }
    if e.is_undefined() || e.is_null() {
        return "the browser gave no reason".into();
    }
    js_sys::JSON::stringify(&e).ok().and_then(|s| s.as_string()).unwrap_or_else(|| format!("{e:?}"))
}

/// `n` bytes of `b` from `at`.
pub async fn read_blob(b: web_sys::Blob, at: u64, n: usize) -> Result<Vec<u8>, String> {
    let part = b.slice_with_f64_and_f64(at as f64, (at + n as u64) as f64).map_err(js_err)?;
    let ab = wasm_bindgen_futures::JsFuture::from(part.array_buffer()).await.map_err(js_err)?;
    let u = js_sys::Uint8Array::new(&ab);
    if u.length() as usize != n {
        return Err("the zip is cut short".into());
    }
    Ok(u.to_vec())
}

/// Lets the browser draw a frame (a long job split up between them).
pub async fn breathe() {
    let p = js_sys::Promise::new(&mut |done, _| {
        let _ = web_sys::window().unwrap().set_timeout_with_callback(&done);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(p).await;
}

/// Page: mod `id` out of the game again (its save to OPFS failed).
#[wasm_bindgen]
pub fn drop_mod(id: String) {
    crate::modfs::remove(&crate::modfs::root().join(id));
}

/// Page: the Mods screen's description bar (an import checked, saved, or refused).
#[wasm_bindgen]
pub fn mod_status(text: String) {
    *MOD_STATUS.lock().unwrap() = Some(text);
}

pub fn mod_status_take() -> Option<String> {
    MOD_STATUS.lock().unwrap().take()
}

/// Opens the page's mod picker (a key's or pad's ✕ on Add mod; a tap lands on the picker itself).
pub fn mod_pick() {
    if let Some(f) = page_fn("hstModPick") {
        let _ = f.call0(&JsValue::NULL);
    }
}

/// The page's mod picker over `rect` (left, top, width, height in % of the window), or gone.
pub fn mod_picker(rect: Option<[f32; 4]>) {
    if let Some(f) = page_fn("hstModPicker") {
        let r: JsValue = rect.map_or(JsValue::NULL, |r| r.iter().map(|&v| JsValue::from(v)).collect::<js_sys::Array>().into());
        let _ = f.call1(&JsValue::NULL, &r);
    }
}

/// Drops mod `id` from OPFS.
pub fn mod_drop(id: &str) {
    if let Some(f) = page_fn("hstModDrop") {
        let _ = f.call1(&JsValue::NULL, &id.into());
    }
}

/// Reloads the page once its mod saves and removals in OPFS are done (B338: a reload right after a store install cut
/// the save short and the mod didn't come in).
pub fn reload() {
    match page_fn("hstReload") {
        Some(f) => drop(f.call0(&JsValue::NULL)),
        None => drop(web_sys::window().unwrap().location().reload()),
    }
}

/// Back to the site page (`?site`), once mod saves are done like [`reload`].
pub fn launcher() {
    match page_fn("hstReload") {
        Some(f) => drop(f.call1(&JsValue::NULL, &true.into())),
        None => drop(web_sys::window().unwrap().location().set_search("site")),
    }
}

/// M17: a mod the store installed (in memory) written to OPFS by the page, which then reports it (`mod_status`).
pub fn mod_save(id: &str, name: &str) {
    if let Some(f) = page_fn("hstModSave") {
        let _ = f.call2(&JsValue::NULL, &id.into(), &name.into());
    }
}
