//! Z2's asset-use log: with `HST_ASSET_LOG=FILE` every asset the game touches is appended to `FILE` once per run, as
//! `kind<TAB>name` lines: `disc` a disc file read, `xb` an archive entry decoded (`DISC/PATH:entry`), and whatever
//! the layers above note (`hst_base::asset_use`: textures made, replaced, drawn; images drawn; sounds played; mod files read).
//! Several runs may append to one file; `research/assets_used.py` folds them into used/unused lists. Off (the
//! default) it costs one relaxed atomic load per call.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};

/// 0 not checked yet, 1 off, 2 on.
static STATE: AtomicU8 = AtomicU8::new(0);

struct Log {
    file: std::fs::File,
    seen: HashSet<(String, String)>,
    /// An archive's header (entry table) -> the disc file it was read from, so its entries are named by both.
    archives: HashMap<u64, String>,
    /// An entry's decoded bytes (XXH3) -> its name, for [`used`].
    entries: HashMap<u64, String>,
}

static LOG: Mutex<Option<Log>> = Mutex::new(None);

/// Whether the log is on (`HST_ASSET_LOG` set when first asked).
pub fn on() -> bool {
    match STATE.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let file = std::env::var_os("HST_ASSET_LOG")
                .and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok());
            let on = file.is_some();
            if let Some(file) = file {
                *LOG.lock().unwrap() = Some(Log { file, seen: HashSet::new(), archives: HashMap::new(), entries: HashMap::new() });
            }
            STATE.store(if on { 2 } else { 1 }, Ordering::Relaxed);
            on
        }
    }
}

/// Log `kind name` (once per run); `name` is only built when the log is on.
pub fn note(kind: &str, name: impl FnOnce() -> String) {
    if !on() {
        return;
    }
    let name = name();
    let mut g = LOG.lock().unwrap();
    let Some(log) = g.as_mut() else { return };
    if log.seen.insert((kind.to_string(), name.clone())) {
        let _ = writeln!(log.file, "{kind}\t{name}");
    }
}

/// An archive's identity: a hash of its magic, count and entry table (enough to tell the disc's archives apart).
fn archive_key(data: &[u8]) -> Option<u64> {
    if data.get(..4) != Some(&crate::xb::MAGIC) {
        return None;
    }
    let count = u32::from_le_bytes(data.get(4..8)?.try_into().ok()?) as usize;
    let head = data.get(..(8 + count * 8).min(data.len()))?;
    Some(twox_hash::XxHash3_64::oneshot(head))
}

/// A disc file was read (`Iso::read`): logged, and remembered as an archive's name if it is one.
pub(crate) fn disc(path: &str, data: &[u8]) {
    if !on() {
        return;
    }
    note("disc", || path.to_string());
    if let Some(k) = archive_key(data) {
        if let Some(log) = LOG.lock().unwrap().as_mut() {
            log.archives.entry(k).or_insert_with(|| path.to_string());
        }
    }
}

/// An archive entry was decoded (`Archive::read`); `data` is the whole archive, `out` the entry (an archive inside
/// an archive is named `DISC/PATH:entry` in turn).
pub(crate) fn entry(data: &[u8], name: &str, out: &[u8]) {
    if !on() {
        return;
    }
    let disc = archive_key(data).and_then(|k| LOG.lock().unwrap().as_ref().and_then(|l| l.archives.get(&k).cloned()));
    let full = format!("{}:{}", disc.as_deref().unwrap_or("?"), name.replace('\\', "/"));
    let h = twox_hash::XxHash3_64::oneshot(out);
    if let Some(log) = LOG.lock().unwrap().as_mut() {
        if let Some(k) = archive_key(out) {
            log.archives.entry(k).or_insert_with(|| full.clone());
        }
        log.entries.entry(h).or_insert_with(|| full.clone());
    }
    note("xb", || full);
}

/// The archive entry whose decoded bytes are `bytes`, if one was decoded this run (and the log is on).
pub fn name_of(bytes: &[u8]) -> Option<String> {
    if !on() {
        return None;
    }
    let h = twox_hash::XxHash3_64::oneshot(bytes);
    LOG.lock().unwrap().as_ref().and_then(|l| l.entries.get(&h).cloned())
}

/// Log `kind name+suffix`, `name` the archive entry whose decoded bytes are `bytes` (`?` if none is).
pub fn used(kind: &str, bytes: &[u8], suffix: impl FnOnce() -> String) {
    if !on() {
        return;
    }
    let name = name_of(bytes).unwrap_or_else(|| "?".into());
    note(kind, || name + &suffix());
}
