//! Mod files and mod zips (M16). `mods.rs` reads a mod's files through here: from its folder on disk, or (the browser
//! has no file system) from the ones the page read out of OPFS `mods/` before `run` (`web::add_mod_file`) and the
//! ones a zip import added (`install`). A zip goes through `unpack`'s checks and then the mod's own (`install`), as
//! `plan/M17.md` *Mods are data only* lists them, before anything is kept: a mod that fails leaves nothing behind.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::mods::{self, Mod};

// Every mod file the browser build holds (never an upscaled set, `upscaled`); the main thread can't read OPFS
// synchronously, so the page reads them in before `run` (only the small `.json`s, B336, but a texture pack's all,
// M5a) or when `ready` asks.
static MEM: Mutex<BTreeMap<String, Arc<[u8]>>> = Mutex::new(BTreeMap::new());

/// B336: the browser's mod files known by path and size but not in memory (a phone tab died holding 40 mods, ≈ 10–40
/// MB each): the page reads a mod folder's in from OPFS when [`ready`] asks, and [`unload`] puts them back.
static LAZY: Mutex<BTreeMap<String, u64>> = Mutex::new(BTreeMap::new());
/// Folders [`ready`] asked the page for (not in yet), and the ones read in, most recently wanted last.
static ASKED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
static WANTED: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// How many read-in mods stay in memory; wanting one more unloads the least recently wanted.
// ponytail: a count, not bytes; 6 covers a doubles match's 4 picks and two hovers
const RESIDENT: usize = 6;

/// Whether file `n` of a mod stays in memory when the mod is [`unload`]ed: its manifests (`mod.json`, `store.json`,
/// …) and own select pictures (`previews/`) are read while listing mods; models, textures and voices only when it's
/// played or drawn. `web/index.html` reads the same ones in at the start.
pub fn small(n: &str) -> bool {
    n.ends_with(".json") || n.contains("/previews/")
}

/// A browser mod file the page left in OPFS (`size` bytes): it reads as missing data until [`ready`] has it in.
#[cfg(any(test, feature = "testing", target_arch = "wasm32"))]
pub fn add_lazy(path: &Path, size: u64) {
    LAZY.lock().unwrap().insert(key(path), size);
}

fn under<'a, V>(m: &'a BTreeMap<String, V>, pre: &'a str) -> impl Iterator<Item = (&'a String, &'a V)> {
    m.range(pre.to_string()..).take_while(move |(k, _)| k.starts_with(pre))
}

/// Whether mod folder `dir`'s files are all in memory; if not, asks the page to read them in (`web::mod_load`; it
/// then calls [`loaded`]) and the caller tries again on a later frame. Marks `dir` as just wanted; one wanted past
/// [`RESIDENT`] unloads the oldest. Always true outside the browser (nothing's ever lazy there but a test's
/// [`add_lazy`] files, which it reports without asking anyone).
pub fn ready(dir: &Path) -> bool {
    if cfg!(target_arch = "wasm32") {
        return want(dir);
    }
    under(&LAZY.lock().unwrap(), &format!("{}/", key(dir))).next().is_none()
}

fn want(dir: &Path) -> bool {
    let d = key(dir);
    let pre = format!("{d}/");
    let mut wanted = WANTED.lock().unwrap();
    wanted.retain(|w| *w != d);
    wanted.push(d.clone());
    let over = wanted.len().saturating_sub(RESIDENT);
    let old: Vec<String> = wanted.drain(..over).collect();
    drop(wanted);
    for o in old {
        unload(Path::new(&o));
    }
    if under(&LAZY.lock().unwrap(), &pre).next().is_none() {
        return true;
    }
    if ASKED.lock().unwrap().insert(d.clone()) {
        #[cfg(target_arch = "wasm32")]
        crate::web::mod_load(&d);
    }
    false
}

/// The page is done reading folder `dir` in: a file it couldn't read is dropped (it then reads as missing, and the
/// mod fails to load as a broken one would instead of waiting forever).
pub fn loaded(dir: &str) {
    let pre = format!("{}/", key(Path::new(dir)));
    ASKED.lock().unwrap().remove(&key(Path::new(dir)));
    LAZY.lock().unwrap().retain(|k, _| !k.starts_with(&pre));
}

/// Mod folder `dir`'s files out of memory again but its [`small`] ones (the browser: the page has them in OPFS). A
/// texture pack's all stay (M5a: `textures.rs` reads them as the menu's and a match's textures are made, with no
/// frame to wait for a read-in; `web/index.html` reads them in whole at the start too).
pub fn unload(dir: &Path) {
    if mods::Kind::of_dir(dir) == mods::Kind::Textures {
        return;
    }
    let pre = format!("{}/", key(dir));
    let mut mem = MEM.lock().unwrap();
    let big: Vec<(String, u64)> = under(&mem, &pre).filter(|(k, _)| !small(k)).map(|(k, b)| (k.clone(), b.len() as u64)).collect();
    for (k, _) in &big {
        mem.remove(k);
    }
    LAZY.lock().unwrap().extend(big);
}

/// `p` as a `MEM` key: `/`-separated, without `.` parts (`./mods/x` and `mods/x` are one file).
pub fn key(p: &Path) -> String {
    p.components().filter(|c| !matches!(c, Component::CurDir)).map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

pub fn add(path: &Path, bytes: Arc<[u8]>) {
    LAZY.lock().unwrap().remove(&key(path));
    MEM.lock().unwrap().insert(key(path), bytes);
}

pub fn read(p: &Path) -> io::Result<Vec<u8>> {
    hst_data::uselog::note("file", || key(p));
    match MEM.lock().unwrap().get(&key(p)) {
        Some(b) => Ok(b.to_vec()),
        None if LAZY.lock().unwrap().contains_key(&key(p)) => Err(io::Error::new(io::ErrorKind::WouldBlock, "still in the browser's storage (the mod is being read in)")),
        None => std::fs::read(p),
    }
}

pub fn read_to_string(p: &Path) -> io::Result<String> {
    String::from_utf8(read(p)?).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn is_file(p: &Path) -> bool {
    MEM.lock().unwrap().contains_key(&key(p)) || LAZY.lock().unwrap().contains_key(&key(p)) || p.is_file()
}

/// File `p`'s size and (on disk) modified time; None for a folder or nothing.
pub fn size(p: &Path) -> Option<(u64, Option<std::time::SystemTime>)> {
    if let Some(b) = MEM.lock().unwrap().get(&key(p)) {
        return Some((b.len() as u64, None));
    }
    if let Some(&n) = LAZY.lock().unwrap().get(&key(p)) {
        return Some((n, None));
    }
    std::fs::metadata(p).ok().filter(|m| m.is_file()).map(|m| (m.len(), m.modified().ok()))
}

/// The files and folders in folder `p`, sorted.
pub fn read_dir(p: &Path) -> Vec<PathBuf> {
    let pre = format!("{}/", key(p));
    let mut out: BTreeSet<PathBuf> = std::fs::read_dir(p).into_iter().flatten().flatten().map(|e| e.path()).collect();
    let first = |k: &String| p.join(k[pre.len()..].split('/').next().unwrap());
    out.extend(under(&MEM.lock().unwrap(), &pre).map(|(k, _)| first(k)));
    out.extend(under(&LAZY.lock().unwrap(), &pre).map(|(k, _)| first(k)));
    out.into_iter().collect()
}

/// Every in-memory file under folder `p` (path, bytes): an import's files for the page to store.
#[cfg(any(test, target_arch = "wasm32"))]
pub fn files_under(p: &Path) -> Vec<(String, Arc<[u8]>)> {
    let pre = format!("{}/", key(p));
    MEM.lock().unwrap().range(pre.clone()..).take_while(|(k, _)| k.starts_with(&pre)).map(|(k, b)| (k.clone(), b.clone())).collect()
}

/// Writes file `p` of an installed mod: in memory in the browser (the page stores it), else on disk.
pub fn write(p: &Path, b: Vec<u8>) -> Result<(), String> {
    if cfg!(target_arch = "wasm32") {
        return {
            add(p, b.into());
            Ok(())
        };
    }
    std::fs::write(p, b).map_err(|e| format!("can't write {}: {e}", p.display()))
}

/// Drops every in-memory (or [`LAZY`]) file under folder `p`.
pub fn remove(p: &Path) {
    let pre = format!("{}/", key(p));
    MEM.lock().unwrap().retain(|k, _| !k.starts_with(&pre));
    LAZY.lock().unwrap().retain(|k, _| !k.starts_with(&pre));
    WANTED.lock().unwrap().retain(|w| *w != key(p) && !w.starts_with(&pre));
}

/// The extensions a mod's files may have (`plan/M17.md`): models, manifests, textures, audio.
pub const EXTENSIONS: [&str; 7] = ["json", "glb", "png", "webp", "tm2", "wav", "ogg"];
/// Caps on a zip's entries, one unpacked file and all of them together: checked while inflating, not taken from the
/// zip's headers. The largest mod the mods repo makes is ≈ 590 MB (Out of Bounds characters).
pub const MAX_ENTRIES: usize = 8192;
pub const MAX_FILE: usize = 768 << 20;
pub const MAX_TOTAL: usize = 1 << 30;
/// M5c: a texture pack's cap on all its files (a whole-disc 4x upscale is ≈ 1.5 GB of PNGs); every other kind's is
/// [`MAX_TOTAL`].
pub const MAX_PACK: usize = 2 << 30;

/// M5c: the cap on all the files of a mod of kind `k`.
pub fn max_total(k: &mods::Kind) -> usize {
    if *k == mods::Kind::Textures { MAX_PACK } else { MAX_TOTAL }
}

/// A mod's files: paths relative to its folder, and their bytes.
#[cfg(any(test, feature = "testing"))]
pub type Files = Vec<(String, Vec<u8>)>;

/// Whether `n` is a plain relative path that stays inside the mod's folder: no root, drive, `..`, `.`, empty part,
/// backslash, control character or Windows device name (`CON`, `COM1`, …).
pub fn plain(n: &str) -> bool {
    let device = |p: &str| {
        let s = p.split('.').next().unwrap_or("").to_ascii_uppercase();
        matches!(s.as_str(), "CON" | "PRN" | "AUX" | "NUL") || (s.len() == 4 && (s.starts_with("COM") || s.starts_with("LPT")) && s.as_bytes()[3].is_ascii_digit())
    };
    !n.is_empty()
        && n.len() <= 255
        && !n.starts_with('/')
        && !n.chars().any(|c| c == '\\' || c == ':' || c.is_control())
        && n.trim_end_matches('/').split('/').all(|p| !p.is_empty() && !p.ends_with(['.', ' ']) && !device(p))
}

fn u16_at(d: &[u8], at: usize) -> Result<usize, String> {
    d.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize).ok_or_else(|| "the zip is cut short".into())
}

fn u32_at(d: &[u8], at: usize) -> Result<u32, String> {
    d.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| "the zip is cut short".into())
}

fn crc32(d: &[u8]) -> u32 {
    let mut t = [0u32; 256];
    for (i, e) in t.iter_mut().enumerate() {
        *e = (0..8).fold(i as u32, |c, _| (c >> 1) ^ (0xEDB8_8320 & (c & 1).wrapping_neg()));
    }
    !d.iter().fold(!0u32, |c, &b| t[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8))
}

/// A mod's `.zip` (stored or deflated entries) unpacked and checked as an archive: every entry a plain file under a
/// [`plain`] path with a known extension, no symlink, device or duplicate (case-blind), within the caps; `mod.json`
/// at the top or in the one folder everything is in (that folder is dropped). macOS's `__MACOSX/` and `.DS_Store`
/// are left out. Anything else wrong rejects the whole zip.
#[cfg(any(test, feature = "testing"))]
pub fn unpack(zip: &[u8]) -> Result<Files, String> {
    unpack_capped(zip, MAX_FILE, MAX_TOTAL)
}

#[cfg(any(test, feature = "testing"))]
fn unpack_capped(zip: &[u8], max_file: usize, max_total: usize) -> Result<Files, String> {
    let mut out = Vec::new();
    now(unpack_with(zip.len() as u64, |at, n| std::future::ready(Ok(zip[at as usize..at as usize + n].to_vec())), |_| true, |n, b| {
        let _: () = out.push((n.to_string(), b));
        Ok(())
    }, max_file, max_total))?;
    Ok(out)
}

/// A future that never waits (a read from memory or a file) run to its end.
#[cfg(not(target_arch = "wasm32"))]
fn now<T>(f: impl Future<Output = T>) -> T {
    use std::task::{Context, Poll, Waker};
    match std::pin::pin!(f).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(t) => t,
        Poll::Pending => unreachable!("a read that waits"),
    }
}

/// One file in a zip's directory, its data not read yet.
struct Entry {
    name: String,
    method: usize,
    crc: u32,
    packed: usize,
    size: u32,
    local: u64,
}

/// [`unpack`] one entry at a time (B204: a big mod is never whole in memory): `read(at, n)` gives `n` bytes of the
/// `len`-byte zip from `at` (always in range), and each file goes to `put(path in the mod, bytes)` as it's inflated.
/// The directory is checked whole before any file is read; a file `keep` refuses is never read (B227: the browser's
/// left-out upscaled set, not even downloaded). The kept files' names, in the zip's order.
pub async fn unpack_with<F: Future<Output = Result<Vec<u8>, String>>>(
    len: u64,
    mut read: impl FnMut(u64, usize) -> F,
    keep: impl Fn(&str) -> bool,
    mut put: impl FnMut(&str, Vec<u8>) -> Result<(), String>,
    max_file: usize,
    max_total: usize,
) -> Result<Vec<String>, String> {
    let mut at = async |from: u64, n: usize| if from.checked_add(n as u64).is_some_and(|e| e <= len) { read(from, n).await } else { Err("the zip is cut short".into()) };
    // the end of central directory record: the last one, within its comment's 64 KB of the end
    let from = len.saturating_sub(22 + 0xffff);
    let tail = at(from, (len - from) as usize).await?;
    let end = (0..=tail.len().saturating_sub(22)).rev().find(|&i| tail[i..].starts_with(b"PK\x05\x06")).ok_or("not a zip file")?;
    let (n, size, dir_at) = (u16_at(&tail, end + 10)?, u32_at(&tail, end + 12)?, u32_at(&tail, end + 16)?);
    if u16_at(&tail, end + 4)? != 0 || u16_at(&tail, end + 6)? != 0 || u16_at(&tail, end + 8)? != n || n == 0xffff || dir_at == u32::MAX {
        return Err("split and zip64 archives aren't supported".into());
    }
    if n > MAX_ENTRIES {
        return Err(format!("more than {MAX_ENTRIES} files"));
    }
    // each entry ≤ 46 + three 64 KB fields
    if size as usize > n * (46 + 3 * 0xffff) {
        return Err("a damaged zip directory".into());
    }
    let dir = at(dir_at as u64, size as usize).await?;
    let (mut i, mut seen, mut files) = (0, HashSet::new(), Vec::new());
    for _ in 0..n {
        if u32_at(&dir, i)? != 0x0201_4b50 {
            return Err("a damaged zip directory".into());
        }
        let (made, flags, method) = (u16_at(&dir, i + 4)?, u16_at(&dir, i + 8)?, u16_at(&dir, i + 10)?);
        let (crc, packed, size) = (u32_at(&dir, i + 16)?, u32_at(&dir, i + 20)?, u32_at(&dir, i + 24)?);
        let (nlen, xlen, clen) = (u16_at(&dir, i + 28)?, u16_at(&dir, i + 30)?, u16_at(&dir, i + 32)?);
        let (attrs, local) = (u32_at(&dir, i + 38)?, u32_at(&dir, i + 42)? as u64);
        let raw = dir.get(i + 46..i + 46 + nlen).ok_or("the zip is cut short")?;
        i += 46 + nlen + xlen + clen;
        let name = std::str::from_utf8(raw).map_err(|_| "a file name that isn't UTF-8")?;
        if !plain(name) {
            return Err(format!("`{name}`: not a plain path inside the mod"));
        }
        // the Unix file type (made on Unix or macOS): plain files and folders only
        let kind = attrs >> 16 & 0o170000;
        if matches!(made >> 8, 3 | 19) && !matches!(kind, 0 | 0o100000 | 0o040000) {
            return Err(format!("`{name}`: a link or special file"));
        }
        if !seen.insert(name.trim_end_matches('/').to_lowercase()) {
            return Err(format!("`{name}` is in the zip twice"));
        }
        if flags & 1 != 0 {
            return Err(format!("`{name}` is encrypted"));
        }
        if name.ends_with('/') || attrs & 0x10 != 0 || kind == 0o040000 {
            if size != 0 {
                return Err(format!("`{name}`: a folder with data"));
            }
            continue;
        }
        let file = name.rsplit('/').next().unwrap_or(name);
        if name.starts_with("__MACOSX/") || file == ".DS_Store" {
            continue;
        }
        let ext = file.rsplit_once('.').map(|e| e.1.to_ascii_lowercase()).unwrap_or_default();
        if !EXTENSIONS.contains(&ext.as_str()) {
            return Err(format!("`{name}`: only {} files can be in a mod", EXTENSIONS.map(|e| format!(".{e}")).join(" ")));
        }
        if !matches!(method, 0 | 8) {
            return Err(format!("`{name}`: compressed with a method zips rarely use (only stored and deflate)"));
        }
        files.push(Entry { name: name.to_string(), method, crc, packed: packed as usize, size, local });
    }
    // `mod.json` at the top, or one folder holding everything (a zipped mod folder) that is dropped
    let top = if files.iter().any(|f| f.name == "mod.json") {
        String::new()
    } else {
        let top = files.first().and_then(|f| f.name.split_once('/')).map(|t| format!("{}/", t.0)).ok_or("no mod.json in the zip")?;
        if !files.iter().all(|f| f.name.starts_with(&top)) || !files.iter().any(|f| f.name == format!("{top}mod.json")) {
            return Err("no mod.json in the zip (it must be at the top, or in the one folder everything is in)".into());
        }
        top
    };
    files.retain(|f| keep(&f.name[top.len()..]));
    let mut total = 0;
    for f in &files {
        let name = &f.name;
        let cap = max_file.min(max_total - total);
        if f.method == 0 && f.packed > cap {
            return Err(format!("`{name}` is too big"));
        }
        // header, data and a little room for the extra field in one read: each read waits a frame in the browser
        let start = 30 + name.len();
        let mut head = at(f.local, (start + f.packed + 256).min(len.saturating_sub(f.local) as usize).max(start)).await?;
        if u32_at(&head, 0)? != 0x0403_4b50 || &head[30..start] != name.as_bytes() {
            return Err(format!("`{name}`: damaged in the zip"));
        }
        let from = start + u16_at(&head, 28)? as usize;
        let data = match head.get(from..from + f.packed) {
            Some(_) => {
                head.truncate(from + f.packed);
                head.split_off(from)
            }
            None => at(f.local + from as u64, f.packed).await?,
        };
        let bytes = match f.method {
            0 => data,
            _ => miniz_oxide::inflate::decompress_to_vec_with_limit(&data, cap).map_err(|e| match e.status {
                miniz_oxide::inflate::TINFLStatus::HasMoreOutput => format!("`{name}` is too big"),
                _ => format!("`{name}`: damaged in the zip"),
            })?,
        };
        if bytes.len() != f.size as usize || crc32(&bytes) != f.crc {
            return Err(format!("`{name}`: damaged in the zip"));
        }
        total += bytes.len();
        put(&name[top.len()..], bytes)?;
    }
    Ok(files.into_iter().map(|f| f.name[top.len()..].to_string()).collect())
}

/// A mod id fit for a folder name: 1..=64 ASCII letters, digits, `_` or `-`.
pub fn good_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Every file of the mod in `dir` read the way the game reads it, as hostile input: `mods::read_any` (M8f9: a court
/// or umpire mod as its kind), each `.glb`
/// through `mods::open`'s checks (every costume also §7's), each image decoded within limits, each `.wav` as PCM16.
pub fn check_all(dir: &Path, files: &[String]) -> Result<Mod, String> {
    let m = check_mod(dir)?;
    files.iter().try_for_each(|f| check_file(dir, f))?;
    Ok(m)
}

fn check_mod(dir: &Path) -> Result<Mod, String> {
    let m = mods::read_any(dir)?;
    mods::check(&m)?;
    Ok(m)
}

fn check_file(dir: &Path, f: &str) -> Result<(), String> {
    let p = dir.join(f);
    let bytes = || read(&p).map_err(|e| format!("{f}: {e}"));
    match f.rsplit_once('.').map(|e| e.1.to_ascii_lowercase()).as_deref() {
        Some("glb") => mods::open(&p).map(drop)?,
        Some(ext @ ("png" | "webp")) => image_ok(&bytes()?, ext).map_err(|e| format!("{f}: {e}"))?,
        Some("wav") if crate::audio::pcm(&bytes()?).is_none() => return Err(format!("{f}: not a PCM16 wav")),
        Some("json") => drop(serde_json::from_slice::<serde_json::Value>(&bytes()?).map_err(|e| format!("{f}: not JSON: {e}"))?),
        _ => {}
    }
    Ok(())
}

/// An image decodes as its extension's format within 8192×8192 and 256 MB (WebP, which this build can't decode, is
/// never drawn either).
fn image_ok(b: &[u8], ext: &str) -> Result<(), String> {
    let fmt = image::ImageFormat::from_extension(ext).ok_or("not an image")?;
    let mut r = image::ImageReader::with_format(io::Cursor::new(b), fmt);
    let mut lim = image::Limits::default();
    (lim.max_image_width, lim.max_image_height, lim.max_alloc) = (Some(8192), Some(8192), Some(256 << 20));
    r.limits(lim);
    match r.decode() {
        Ok(_) => Ok(()),
        Err(image::ImageError::Unsupported(_)) if fmt == image::ImageFormat::WebP => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// A mod zip installed into `root` (the `mods/` folder) as `root/<id>/`, in memory ([`install_with`]).
#[cfg(any(test, feature = "testing"))]
pub fn install(zip: &[u8], root: &Path) -> Result<Mod, String> {
    now(install_with(zip.len() as u64, |at, n| std::future::ready(Ok(zip[at as usize..at as usize + n].to_vec())), root, None, None, false, false))
}

/// The mod zip file `zip` installed into `root/<id>/` on disk ([`install_with`]); `want` is the id it must hold and
/// `owner` its store author (a store download's; both `None` for a local zip).
#[cfg(not(target_arch = "wasm32"))]
pub fn install_file(zip: &Path, root: &Path, want: Option<&str>, owner: Option<&str>) -> Result<Mod, String> {
    use io::{Read, Seek};
    let mut f = std::fs::File::open(zip).map_err(|e| format!("can't open the zip: {e}"))?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    let read = |at, n| {
        let mut b = vec![0; n];
        std::future::ready(f.seek(io::SeekFrom::Start(at)).and_then(|_| f.read_exact(&mut b)).map(|_| b).map_err(|e| format!("can't read the zip: {e}")))
    };
    now(install_with(len, read, root, want, owner, true, true))
}

/// B248: the folder under `root` mod `id` (staged in `stage`) installs into: the first of `<id>`, `<id>-2`, `<id>-3`, …
/// that's free or holds this same mod (replaced: an update). The same mod is a store download of the same id by the
/// same `owner` (store author; an install from before B248 has none and counts as anyone's), or with no `owner` a
/// local zip whose `mod.json` is byte for byte the installed one's. So two mods sharing an id (or a name) both stay.
pub fn folder_for(root: &Path, id: &str, stage: &Path, owner: Option<&str>) -> PathBuf {
    let same = |d: &Path| {
        let stamp = crate::modstore::stamp(d);
        mods::read_any(d).is_ok_and(|m| m.id == id)
            && match owner {
                Some(o) => stamp.is_some_and(|s| s.author.is_empty() || s.author == o),
                None => stamp.is_none() && read(&d.join("mod.json")).ok() == read(&stage.join("mod.json")).ok(),
            }
    };
    (1..).map(|n| root.join(if n == 1 { id.to_string() } else { format!("{id}-{n}") })).find(|d| !is_file(&d.join("mod.json")) || same(d)).unwrap()
}

/// A mod fetched from an online peer (`mods_online`'s cache, checked there) copied into `root` as an installed one, in
/// the folder Install would pick ([`folder_for`]: kept before, it's that folder again, left as it is); the browser's
/// page then stores it, as Install's.
pub fn keep(from: &Path, root: &Path) -> Result<Mod, String> {
    fn copy(from: &Path, to: &Path) -> Result<(), String> {
        for p in read_dir(from) {
            let t = to.join(p.file_name().unwrap_or_default());
            if !is_file(&p) {
                copy(&p, &t)?;
                continue;
            }
            if !cfg!(target_arch = "wasm32") {
                std::fs::create_dir_all(to).map_err(|e| format!("can't write {}: {e}", to.display()))?;
            }
            write(&t, read(&p).map_err(|e| format!("{}: {e}", p.display()))?)?;
        }
        Ok(())
    }
    let id = mods::read_any(from)?.id;
    let dir = folder_for(root, &id, from, None);
    if is_file(&dir.join("mod.json")) {
        return mods::read_any(&dir);
    }
    copy(from, &dir)?;
    let m = mods::read_any(&dir)?;
    #[cfg(target_arch = "wasm32")]
    crate::web::mod_save(&m.folder(), &m.name);
    Ok(m)
}

/// Whether `dir` is a folder Install made under `root` (`<id>` or `<id>-<n>`, [`folder_for`]), not one put in by hand.
pub fn installed_by_game(root: &Path, dir: &Path, id: &str) -> bool {
    let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    dir.parent() == Some(root) && (name == id || name.strip_prefix(&format!("{id}-")).is_some_and(|n| n.parse::<u32>().is_ok_and(|n| n >= 2)))
}

/// Whether a mod's file `n` (inside its folder) is in its upscaled set (B227): ≈ 90% of the store mods' bytes (512
/// of Anya's 525 MB). The browser never installs it (a few installed mods outgrew a phone's tab; it draws the mod's
/// own textures, as an online peer does, `mods_online.rs`), nor does the desktop with Upscaled Downloads off.
// ponytail: no upscaled mod textures in the browser; reading the picked costume's set from OPFS before a match
// (async, the main thread can't read OPFS synchronously) if they're ever wanted there
pub fn upscaled(n: &str) -> bool {
    n.starts_with("textures/upscaled/")
}

/// A mod zip (`len` bytes, read as [`unpack_with`] reads it) installed into `root` (the `mods/` folder) as
/// `root/<id>/`: unpacked a file at a time under `root/.import/` (a store mod's under `root/.import-<owner>-<id>/`) (on disk, or in memory: the browser's page then
/// stores [`files_under`] `root/<id>`), checked whole ([`check_all`]), then put in place of an installed mod with the
/// same id ([`folder_for`]; else beside it). `want` is the id it must hold (a store download's), `owner` its store
/// author. `upscaled` false leaves the mod's upscaled set
/// ([`upscaled`]) out, unread. On any failure nothing of it is kept.
pub async fn install_with<F: Future<Output = Result<Vec<u8>, String>>>(
    len: u64,
    zip: impl FnMut(u64, usize) -> F,
    root: &Path,
    want: Option<&str>,
    owner: Option<&str>,
    disk: bool,
    with_upscaled: bool,
) -> Result<Mod, String> {
    // B339: a store mod's own stage, as several install at once
    let stage = root.join(match (owner, want) {
        (Some(o), Some(w)) => format!(".import-{o}-{w}"),
        _ => ".import".into(),
    });
    let clear = || {
        remove(&stage);
        if disk {
            let _ = std::fs::remove_dir_all(&stage);
        }
    };
    clear();
    let checked = async {
        let put = |n: &str, b: Vec<u8>| {
            let p = stage.join(n);
            if !disk {
                return {
                    add(&p, b.into());
                    Ok(())
                };
            }
            std::fs::create_dir_all(p.parent().unwrap()).and_then(|_| std::fs::write(&p, b)).map_err(|e| format!("can't write {}: {e}", p.display()))
        };
        // M5c: a pack's cap while unpacking (the kind is in mod.json, read after), then the kind's own
        let mut total = 0;
        let count = |n: &str, b: Vec<u8>| {
            total += b.len();
            put(n, b)
        };
        let names = unpack_with(len, zip, |n| with_upscaled || !upscaled(n), count, MAX_FILE, MAX_PACK).await?;
        let j: serde_json::Value = serde_json::from_slice(&read(&stage.join("mod.json")).map_err(|e| e.to_string())?).map_err(|e| format!("mod.json: not JSON: {e}"))?;
        if total > max_total(&mods::Kind::of(&j)) {
            return Err(format!("more than {} MB unpacked", MAX_TOTAL >> 20));
        }
        let id = j["id"].as_str().filter(|id| good_id(id)).ok_or("mod.json: `id` must be 1 to 64 letters, digits, `_` or `-`")?.to_string();
        if let Some(w) = want.filter(|w| *w != id) {
            return Err(format!("the zip holds `{id}`, not `{w}`"));
        }
        check_mod(&stage)?;
        #[cfg(target_arch = "wasm32")]
        let mut last = js_sys::Date::now();
        for f in &names {
            // the browser draws a frame every 50 ms of checks (each yield waits a whole frame)
            #[cfg(target_arch = "wasm32")]
            if js_sys::Date::now() - last > 50.0 {
                crate::web::breathe().await;
                last = js_sys::Date::now();
            }
            check_file(&stage, f)?;
        }
        Ok(id)
    };
    let id = match checked.await {
        Ok(id) => id,
        Err(e) => {
            clear();
            return Err(e.replace(&format!("{}/", stage.display()), ""));
        }
    };
    // one install at a time picks its folder and moves in (B339: two of one id mustn't both take `<id>-2`)
    static PLACING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _placing = PLACING.lock().unwrap_or_else(|e| e.into_inner());
    let dir = folder_for(root, &id, &stage, owner);
    if disk {
        let _ = std::fs::remove_dir_all(&dir);
        if let Err(e) = std::fs::rename(&stage, &dir) {
            clear();
            return Err(format!("can't write {}: {e}", dir.display()));
        }
    } else {
        remove(&dir);
        let (from, to) = (key(&stage), key(&dir));
        let mut mem = MEM.lock().unwrap();
        let staged: Vec<String> = mem.range(format!("{from}/")..).take_while(|(k, _)| k.starts_with(&format!("{from}/"))).map(|(k, _)| k.clone()).collect();
        for k in staged {
            let b = mem.remove(&k).unwrap();
            mem.insert(format!("{to}{}", &k[from.len()..]), b);
        }
    }
    mods::read_any(&dir)
}

/// A zip of `files` (stored or deflated), each with Unix mode `mode`, written by hand like any zip tool would (M17 upload packages, and the tests' archives).
pub fn zip(files: &[(&str, &[u8])], deflate: bool, mode: u32) -> Vec<u8> {
    let (mut out, mut dir) = (Vec::new(), Vec::new());
    for (name, data) in files {
        let packed = if deflate { miniz_oxide::deflate::compress_to_vec(data, 6) } else { data.to_vec() };
        let head = |sig: u32, central: bool, at: usize| {
            let mut h = sig.to_le_bytes().to_vec();
            if central {
                h.extend((3u16 << 8 | 20).to_le_bytes());
            }
            h.extend(20u16.to_le_bytes());
            h.extend(0u16.to_le_bytes());
            h.extend((if deflate { 8u16 } else { 0 }).to_le_bytes());
            h.extend([0; 4]);
            h.extend(crc32(data).to_le_bytes());
            h.extend((packed.len() as u32).to_le_bytes());
            h.extend((data.len() as u32).to_le_bytes());
            h.extend((name.len() as u16).to_le_bytes());
            h.extend(0u16.to_le_bytes());
            if central {
                h.extend([0; 6]);
                h.extend((mode << 16).to_le_bytes());
                h.extend((at as u32).to_le_bytes());
            }
            h.extend(name.as_bytes());
            h
        };
        dir.extend(head(0x0201_4b50, true, out.len()));
        out.extend(head(0x0403_4b50, false, 0));
        out.extend(&packed);
    }
    let at = out.len();
    out.extend(&dir);
    out.extend(b"PK\x05\x06\0\0\0\0");
    out.extend((files.len() as u16).to_le_bytes().repeat(2));
    out.extend((dir.len() as u32).to_le_bytes());
    out.extend((at as u32).to_le_bytes());
    out.extend([0, 0]);
    out
}

/// The mods folder: `mods/` beside the disc, as the select's roster reads it (`play/main_menu.rs setup` sets it, through
/// `mods_screen`; a relative `mods` broke when the game was started from elsewhere, e.g. a desktop shortcut).
static ROOT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

pub fn set_root(dir: std::path::PathBuf) {
    // `./mods` (a disc path with no folder, the browser's) stays `mods`, the OPFS paths' form
    let _ = ROOT.set(dir.strip_prefix(".").map(Into::into).unwrap_or(dir));
}

pub fn root() -> &'static Path {
    ROOT.get().map(|p| p.as_path()).unwrap_or(Path::new("mods"))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// The real mods (`context/mods`, links into the mods repo's `out/mods`).
    pub const MODS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/mods");

    /// Every file under `dir` (relative path, bytes), sorted.
    pub fn dir_files(dir: &Path) -> Files {
        fn walk(d: &Path, pre: &str, out: &mut Files) {
            let mut es: Vec<_> = std::fs::read_dir(d).into_iter().flatten().flatten().collect();
            es.sort_by_key(|e| e.file_name());
            for e in es {
                let n = format!("{pre}{}", e.file_name().to_string_lossy());
                if e.path().is_dir() {
                    walk(&e.path(), &format!("{n}/"), out);
                } else {
                    out.push((n, std::fs::read(e.path()).unwrap()));
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, "", &mut out);
        out
    }

    fn zip_of(files: &Files) -> Vec<u8> {
        zip(&files.iter().map(|f| (f.0.as_str(), &f.1[..])).collect::<Vec<_>>(), false, 0o100644)
    }


    const MANIFEST: &[u8] = br#"{"standard":1,"id":"x","name":"X","costumes":["c.glb"],"donor":3,"hand":"right","params":{"base":3},"ai_row":3,"face":"none"}"#;

    #[test]
    fn unpacks_a_plain_zip() {
        for deflate in [false, true] {
            let z = zip(&[("m/mod.json", MANIFEST), ("m/c.glb", b"glTF"), ("m/voice/0_0.wav", b"RIFF")], deflate, 0o100644);
            let f = unpack(&z).unwrap();
            assert_eq!(f.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(), ["mod.json", "c.glb", "voice/0_0.wav"]);
            assert_eq!(f[0].1, MANIFEST);
        }
    }

    #[test]
    fn keeps_a_downloaded_mod_once() {
        let t = std::env::temp_dir().join(format!("hst-keep-{}", std::process::id()));
        let (from, root) = (t.join("cache/abc"), t.join("mods"));
        std::fs::create_dir_all(from.join("voice")).unwrap();
        std::fs::write(from.join("mod.json"), MANIFEST).unwrap();
        std::fs::write(from.join("c.glb"), b"glTF").unwrap();
        std::fs::write(from.join("voice/0_0.wav"), b"RIFF").unwrap();
        let m = keep(&from, &root).unwrap();
        assert_eq!((m.id.as_str(), std::fs::read(root.join("x/voice/0_0.wav")).unwrap()), ("x", b"RIFF".to_vec()));
        keep(&from, &root).unwrap();
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1, "the same mod again isn't a second copy");
        std::fs::remove_dir_all(&t).unwrap();
    }

    /// The bad-archive half of the bad-mod set (`plan/M17.md`): each one is refused with a message, nothing kept.
    #[test]
    fn refuses_bad_archives() {
        let m: (&str, &[u8]) = ("mod.json", MANIFEST);
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("traversal", zip(&[m, ("../evil.json", b"{}")], false, 0o100644)),
            ("nested traversal", zip(&[m, ("a/../../evil.json", b"{}")], false, 0o100644)),
            ("absolute", zip(&[m, ("/etc/evil.json", b"{}")], false, 0o100644)),
            ("drive", zip(&[m, ("C:/evil.json", b"{}")], false, 0o100644)),
            ("backslash", zip(&[m, ("a\\..\\evil.json", b"{}")], false, 0o100644)),
            ("device", zip(&[m, ("con.json", b"{}")], false, 0o100644)),
            ("symlink", zip(&[m, ("link.glb", b"/etc/passwd")], false, 0o120777)),
            ("device file", zip(&[m, ("dev.glb", b"")], false, 0o020644)),
            ("duplicate", zip(&[m, ("a.png", b"1"), ("A.PNG", b"2")], false, 0o100644)),
            ("code", zip(&[m, ("run.js", b"alert(1)")], false, 0o100644)),
            ("no extension", zip(&[m, ("run", b"#!/bin/sh")], false, 0o100644)),
            ("no manifest", zip(&[("c.glb", b"glTF")], false, 0o100644)),
            ("two folders", zip(&[("a/mod.json", MANIFEST), ("b/c.glb", b"")], false, 0o100644)),
            ("garbage", b"PK\x03\x04 not really a zip".to_vec()),
            ("empty", Vec::new()),
        ];
        for (what, z) in cases {
            let e = unpack(&z).err().unwrap_or_else(|| panic!("{what} accepted"));
            eprintln!("{what}: {e}");
        }
        // a compression bomb (1 MB of zeros in ≈ 1 KB) against small caps: per file, and in total over two files
        let zeros = vec![0u8; 1 << 20];
        assert!(unpack_capped(&zip(&[m, ("bomb.glb", &zeros)], true, 0o100644), 64 << 10, 1 << 30).unwrap_err().contains("too big"));
        assert!(unpack_capped(&zip(&[m, ("a.glb", &zeros), ("b.glb", &zeros)], true, 0o100644), 2 << 20, (2 << 20) - 1).unwrap_err().contains("too big"));
        assert!(unpack_capped(&zip(&[m, ("bomb.glb", &zeros)], false, 0o100644), 64 << 10, 1 << 30).unwrap_err().contains("too big"));
        // too many entries
        let names: Vec<String> = (0..=MAX_ENTRIES).map(|i| format!("{i}.json")).collect();
        let many: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"{}"[..])).chain([m]).collect();
        assert!(unpack(&zip(&many, false, 0o100644)).is_err());
        // a header that lies about the size (stored: data longer than declared) and a bad checksum
        let mut z = zip(&[m], false, 0o100644);
        let i = z.windows(MANIFEST.len()).position(|w| w == MANIFEST).unwrap();
        z[i + 3] ^= 1;
        assert!(unpack(&z).unwrap_err().contains("damaged"));
        // cut short anywhere: an error, never a panic
        let z = zip(&[m, ("c.glb", b"glTF")], true, 0o100644);
        (0..z.len()).for_each(|n| drop(unpack(&z[..n])));
    }

    /// Seeded random mutations of a zip: errors, never a panic or a runaway allocation.
    #[test]
    fn mutated_zips_never_panic() {
        let z = zip(&[("m/mod.json", MANIFEST), ("m/c.glb", &[7u8; 3000][..])], true, 0o100644);
        let mut s = 0x9e37_79b9_7f4a_7c15u64;
        let mut rnd = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        for _ in 0..20_000 {
            let mut m = z.clone();
            for _ in 0..1 + rnd() % 8 {
                let i = rnd() as usize % m.len();
                m[i] = rnd() as u8;
            }
            let _ = unpack(&m);
        }
    }

    /// The bad-mod half of the bad-mod set: a real mod with one file broken is refused, and nothing of it is kept
    /// (nor of the import), while an installed mod with the same id stays as it was.
    #[test]
    fn bad_mods_leave_nothing() {
        let dir = Path::new(MODS).join("opentee2_pc16_lauryn");
        if !dir.is_dir() {
            return eprintln!("no context/mods/opentee2_pc16_lauryn, skipped");
        }
        let good = dir_files(&dir);
        let costume = "model/pc16_lauryn_own.glb";
        let glb = &good.iter().find(|f| f.0 == costume).unwrap().1;
        // the costume's JSON chunk edited, the binary chunk kept
        let edit = |f: &dyn Fn(&mut serde_json::Value)| {
            let n = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
            let mut j: serde_json::Value = serde_json::from_slice(&glb[20..20 + n]).unwrap();
            f(&mut j);
            let mut text = serde_json::to_vec(&j).unwrap();
            text.resize(text.len().next_multiple_of(4), b' ');
            let bin = &glb[20 + n..];
            let mut out = b"glTF".to_vec();
            out.extend(2u32.to_le_bytes());
            out.extend(((12 + 8 + text.len() + bin.len()) as u32).to_le_bytes());
            out.extend((text.len() as u32).to_le_bytes());
            out.extend(b"JSON");
            out.extend(text);
            out.extend(bin);
            out
        };
        let with = |name: &str, bytes: Vec<u8>| -> Files { good.iter().map(|f| (f.0.clone(), if f.0 == name { bytes.clone() } else { f.1.clone() })).collect() };
        let png = good.iter().find(|f| f.0.ends_with(".png")).unwrap().0.clone();
        let wav = good.iter().find(|f| f.0.ends_with(".wav")).unwrap().0.clone();
        let manifest = String::from_utf8(good.iter().find(|f| f.0 == "mod.json").unwrap().1.clone()).unwrap();
        let cases: Vec<(&str, Files)> = vec![
            ("garbage glb", with(costume, b"glTF\x02\0\0\0garbage".to_vec())),
            ("truncated glb", with(costume, glb[..glb.len() / 2].to_vec())),
            ("glb cut in its JSON", with(costume, glb[..40].to_vec())),
            ("external buffer", with(costume, edit(&|j| j["buffers"][0]["uri"] = "https://example.com/x.bin".into()))),
            ("external image", with(costume, edit(&|j| j["images"][0] = serde_json::json!({"uri": "https://example.com/x.png"})))),
            ("accessor past its data", with(costume, edit(&|j| j["accessors"][0]["count"] = 1_000_000_000.into()))),
            ("bad index", with(costume, edit(&|j| j["meshes"][0]["primitives"][0]["attributes"]["POSITION"] = 99_999.into()))),
            ("node loop", with(costume, edit(&|j| j["nodes"][1]["children"] = serde_json::json!([0])))),
            ("garbage png", with(&png, b"\x89PNG\r\n\x1a\n garbage".to_vec())),
            ("truncated png", with(&png, good.iter().find(|f| f.0 == png).unwrap().1[..100].to_vec())),
            ("garbage wav", with(&wav, b"RIFF....WAVEfmt garbage".to_vec())),
            ("costume outside the mod", with("mod.json", manifest.replace(costume, "../x.glb").into_bytes())),
            ("voice outside the mod", with("mod.json", manifest.replace("\"voice/\"", "\"/etc/\"").into_bytes())),
            ("long name", with("mod.json", manifest.replace("\"Lauryn\"", &format!("\"{}\"", "L".repeat(65))).into_bytes())),
            ("bad id", with("mod.json", manifest.replace("\"opentee2_pc16_lauryn\"", "\"../../x\"").into_bytes())),
        ];
        let root = Path::new("t_bad/mods");
        for (what, files) in &cases {
            let e = install(&zip_of(files), root).err().unwrap_or_else(|| panic!("{what} accepted"));
            eprintln!("{what}: {e}");
            assert!(files_under(Path::new("t_bad")).is_empty(), "{what} left files");
        }
        // over an installed mod: a bad one with its id changes nothing
        install(&zip_of(&good), root).unwrap();
        let before = files_under(root);
        assert!(install(&zip_of(&cases[0].1), root).is_err());
        assert_eq!(files_under(root).len(), before.len());
        assert!(files_under(root).iter().zip(&before).all(|(a, b)| a.0 == b.0 && Arc::ptr_eq(&a.1, &b.1)));
    }

    /// B227: an in-memory (browser) install keeps a mod's files but not its upscaled set, which isn't even checked.
    #[test]
    fn browser_install_leaves_upscaled_out() {
        let dir = Path::new(MODS).join("test_pc00");
        if !dir.is_dir() {
            return eprintln!("no context/mods/test_pc00, skipped");
        }
        let mut files = dir_files(&dir);
        files.push(("textures/upscaled/c00/image0.png".into(), b"not a png".to_vec()));
        let root = Path::new("t_upscaled/mods");
        let m = install(&zip_of(&files), root).unwrap();
        let kept = files_under(&m.dir);
        assert_eq!(kept.len(), files.len() - 1);
        assert!(kept.iter().all(|f| !f.0.contains("/textures/upscaled/")));
        remove(Path::new("t_upscaled"));
    }

    /// M5a: a texture pack's zip installs as one (in memory, as the browser does), each PNG checked, and stays in
    /// memory when unloaded (its textures are read without a read-in); a pack with a broken PNG leaves nothing.
    #[test]
    fn texture_pack_installs_and_stays_in() {
        let tmp = std::env::temp_dir().join(format!("hst-packzip-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        hst_data::texhash::write_png(&tmp.join("t.png"), 1, 1, &[1, 2, 3, 4]).unwrap();
        let png = std::fs::read(tmp.join("t.png")).unwrap();
        let json = br#"{"standard": 1, "kind": "textures", "id": "my_pack", "name": "My Pack"}"#;
        let root = Path::new("t_packzip/mods");
        let m = install(&zip(&[("mod.json", json), ("textures/COURT/x.0-00000000000000aa.png", &png)], true, 0o644), root).unwrap();
        assert_eq!((m.id.as_str(), mods::Kind::of_dir(&m.dir)), ("my_pack", mods::Kind::Textures));
        unload(&m.dir);
        assert_eq!(read(&m.dir.join("textures/COURT/x.0-00000000000000aa.png")).unwrap(), png);
        assert_eq!(crate::mods_textures::list(root).len(), 1);
        let bad = zip(&[("mod.json", json), ("textures/x.0-00000000000000bb.png", b"not a png")], true, 0o644);
        remove(Path::new("t_packzip"));
        assert!(install(&bad, root).unwrap_err().contains("x.0-00000000000000bb.png"));
        assert!(crate::mods_textures::list(root).is_empty());
        remove(Path::new("t_packzip"));
        std::fs::remove_dir_all(&tmp).ok();
    }

    /// B248: two mods sharing an id both install (`<id>`, `<id>-2`); the same mod again replaces its own copy: a
    /// local zip byte for byte the same `mod.json`, a store download by the same author (or an install from before
    /// the author was kept). Only the game's own folders count as installed by it.
    #[test]
    fn same_id_mods_both_install() {
        let dir = Path::new(MODS).join("test_pc00");
        if !dir.is_dir() {
            return eprintln!("no context/mods/test_pc00, skipped");
        }
        let files = dir_files(&dir);
        let renamed = |name: &str| -> Files {
            let j = |b: &[u8]| {
                let mut j: serde_json::Value = serde_json::from_slice(b).unwrap();
                j["name"] = name.into();
                serde_json::to_vec(&j).unwrap()
            };
            files.iter().map(|f| (f.0.clone(), if f.0 == "mod.json" { j(&f.1) } else { f.1.clone() })).collect()
        };
        let (a, b) = (zip_of(&renamed("Same")), zip_of(&renamed("Same ")));
        let root = Path::new("t_same/mods");
        let folder = |m: Mod| m.dir.file_name().unwrap().to_string_lossy().to_string();
        let id = mods::read(&dir).unwrap().id;
        assert_eq!(folder(install(&a, root).unwrap()), id);
        assert_eq!(folder(install(&b, root).unwrap()), format!("{id}-2"));
        assert_eq!(folder(install(&a, root).unwrap()), id);
        assert_eq!(mods::list(root).len(), 2);
        // store downloads: by author
        let store = |zip: &[u8], who: &str| {
            let m = now(install_with(zip.len() as u64, |at, n| std::future::ready(Ok(zip[at as usize..at as usize + n].to_vec())), root, Some(&id), Some(who), false, false)).unwrap();
            add(&m.dir.join("store.json"), serde_json::to_vec(&serde_json::json!({ "sha256": "", "rev": "main", "verified": true, "author": who })).unwrap().into());
            folder(m)
        };
        assert_eq!(store(&a, "x"), format!("{id}-3"));
        assert_eq!(store(&b, "y"), format!("{id}-4"));
        assert_eq!(store(&b, "x"), format!("{id}-3"));
        assert_eq!(mods::list(root).len(), 4);
        add(&root.join(format!("{id}-4/store.json")), br#"{"sha256":"","rev":"main","verified":true}"#.to_vec().into());
        assert_eq!(store(&a, "z"), format!("{id}-4"));
        assert!(installed_by_game(root, &root.join(format!("{id}-4")), &id) && installed_by_game(root, &root.join(&id), &id));
        assert!(!installed_by_game(root, &root.join(format!("{id}-x")), &id) && !installed_by_game(root, &root.join(format!("{id}-1")), &id));
        assert!(!installed_by_game(root, &root.join("pack").join(&id), &id));
        remove(Path::new("t_same"));
    }

    #[test]
    fn a_lazy_mod_is_read_in_when_wanted_and_unloaded_past_the_budget() {
        let d = |n: usize| PathBuf::from(format!("t_lazy/m{n}"));
        add(&d(0).join("mod.json"), b"{}".to_vec().into());
        add_lazy(&d(0).join("model/c00.glb"), 3);
        // listed and sized as a file, but no data until the page reads it in
        assert!(is_file(&d(0).join("model/c00.glb")) && size(&d(0).join("model/c00.glb")).unwrap().0 == 3);
        assert_eq!(read_dir(&d(0)), [d(0).join("mod.json"), d(0).join("model")]);
        assert_eq!(read(&d(0).join("model/c00.glb")).unwrap_err().kind(), io::ErrorKind::WouldBlock);
        assert!(!want(&d(0)) && ASKED.lock().unwrap().contains(&key(&d(0))));
        add(&d(0).join("model/c00.glb"), vec![1, 2, 3].into());
        loaded(&key(&d(0)));
        assert!(want(&d(0)) && read(&d(0).join("model/c00.glb")).unwrap() == [1, 2, 3]);
        // RESIDENT more wanted: m0 unloads, its mod.json stays
        for n in 1..=RESIDENT {
            assert!(want(&d(n)));
        }
        assert!(read(&d(0).join("model/c00.glb")).is_err() && read(&d(0).join("mod.json")).is_ok());
        // a file the page couldn't read: missing, not waited on
        assert!(!want(&d(0)));
        loaded(&key(&d(0)));
        assert!(want(&d(0)) && !is_file(&d(0).join("model/c00.glb")));
        remove(Path::new("t_lazy"));
        assert!(read_dir(Path::new("t_lazy")).is_empty());
    }

    #[test]
    fn plain_paths() {
        for ok in ["mod.json", "model/c00.glb", "textures/upscaled/c00/image0.png", "voice/0_1.wav"] {
            assert!(plain(ok), "{ok}");
        }
        for bad in ["", "/a", "a//b", "./a", "a/.", "..", "a/../b", "a\\b", "c:x", "nul", "COM1.glb", "a/aux.png", "a. ", "a\u{0}b"] {
            assert!(!plain(bad), "{bad}");
        }
        assert!(good_id("fore_pc00_phoebe") && !good_id("../x") && !good_id("") && !good_id(&"a".repeat(65)));
    }
}
