//! The browser build (`web/`, built by `tools/web.sh`). The page imports the user's ISO once: a worker calls
//! `import_disc` with the picked `File` (read by extent with `File.slice`, never whole) and an OPFS file to write the
//! pack into (`Iso::write_pack_to`). Every later visit reads that pack and calls `run`, which mounts it in memory
//! (`hst_data::iso::mount`) where every `Iso::open` finds it. Flags come from the URL: `?play&stage=1` = `--play --stage 1`.

use hst_data::iso::Iso;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::{File, FileReaderSync, FileSystemReadWriteOptions, FileSystemSyncAccessHandle};

/// The path the mounted pack answers to (settings.txt etc. "beside" it don't exist: defaults).
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
            argv.push(format!("--{k}"));
            argv.extend(js_sys::decode_uri_component(&v.replace('+', " ")).map(String::from).unwrap_or_default().split_whitespace().map(String::from));
        }
    }
    crate::start(argv);
}
