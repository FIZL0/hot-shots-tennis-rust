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
const PACK: &str = "disc.pack";

fn io_err(e: JsValue) -> io::Error {
    io::Error::other(format!("{e:?}"))
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

/// Page: start the game from the pack's bytes, with the URL's flags.
#[wasm_bindgen]
pub fn run(pack: js_sys::Uint8Array) {
    // straight into one shared buffer: the pack (≈ 171 MB) is never held twice in wasm memory
    let mut bytes = Arc::<[u8]>::new_uninit_slice(pack.length() as usize);
    pack.copy_to_uninit(Arc::get_mut(&mut bytes).unwrap());
    // SAFETY: copy_to_uninit filled every byte
    hst_data::iso::mount(PACK, unsafe { bytes.assume_init() });
    let mut argv = vec![PACK.to_string()];
    if let Ok(q) = web_sys::window().unwrap().location().search() {
        for kv in q.trim_start_matches('?').split('&').filter(|s| !s.is_empty()) {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            // the page's own (`index.html`)
            if k == "webgl2" {
                continue;
            }
            if k == "code" {
                paste(js_sys::decode_uri_component(v).map(String::from).unwrap_or_default());
                continue;
            }
            argv.push(format!("--{k}"));
            argv.extend(js_sys::decode_uri_component(&v.replace('+', " ")).map(String::from).unwrap_or_default().split_whitespace().map(String::from));
        }
    }
    crate::start(argv);
}

/// The page's boot status (`index.html`) goes once the game has drawn its first frames.
pub fn shown(frames: Res<bevy::diagnostic::FrameCount>) {
    if frames.0 == 3 {
        if let Ok(f) = js_sys::Reflect::get(&js_sys::global(), &"hstShown".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
            let _ = f.call0(&JsValue::NULL);
        }
    }
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
    local()?.get_item(&key).map_err(io_err)?.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, key.into_owned()))
}

pub fn write(path: impl AsRef<std::path::Path>, text: impl AsRef<[u8]>) -> io::Result<()> {
    local()?.set_item(&path.as_ref().to_string_lossy(), &String::from_utf8_lossy(text.as_ref())).map_err(io_err)
}

fn local() -> io::Result<web_sys::Storage> {
    web_sys::window().unwrap().local_storage().map_err(io_err)?.ok_or_else(|| io::Error::other("no localStorage"))
}

/// The page's on-screen keyboard's keys (`index.html`; Backspace as `\u{8}`) for Join by Code, since the last call.
static TYPED: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Page: text typed on the phone's keyboard while Join by Code's code entry is up.
#[wasm_bindgen]
pub fn typed_code(text: String) {
    TYPED.lock().unwrap().push_str(&text);
}

pub fn typed() -> String {
    std::mem::take(&mut *TYPED.lock().unwrap())
}

/// Join by Code's code entry shown or gone: the page puts a tap target up that opens the phone's keyboard.
pub fn code_entry(on: bool) {
    let f = js_sys::Reflect::get(&web_sys::window().unwrap(), &"hstCodeEntry".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok());
    if let Some(f) = f {
        let _ = f.call1(&JsValue::NULL, &on.into());
    }
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

/// Drop every card whose name doesn't start with `prefix` (an older drawing's).
pub fn cards_keep(prefix: &str) {
    let mut cards = CARDS.lock().unwrap();
    let Some(cards) = cards.as_mut() else { return };
    let drop = page_fn("hstCardDrop");
    cards.retain(|name, _| {
        let keep = name.starts_with(prefix);
        if !keep && let Some(f) = &drop {
            let _ = f.call1(&JsValue::NULL, &name.as_str().into());
        }
        keep
    });
}

fn page_fn(name: &str) -> Option<js_sys::Function> {
    js_sys::Reflect::get(&web_sys::window().unwrap(), &name.into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok())
}
