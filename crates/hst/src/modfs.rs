//! Mod files and mod zips (M16). `mods.rs` reads a mod's files through here: from its folder on disk, or (the browser
//! has no file system) from the ones the page read out of OPFS `mods/` before `run` (`web::add_mod_file`) and the
//! ones a zip import added (`install`). A zip goes through `unpack`'s checks and then the mod's own (`install`), as
//! `plan/M17.md` *Mods are data only* lists them, before anything is kept: a mod that fails leaves nothing behind.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::mods::{self, Mod};

// ponytail: every mod file of the browser build is held in memory (the main thread can't read OPFS synchronously);
// the ceiling is wasm32's 4 GB beside the 171 MB pack. Reading on demand from a worker if mods outgrow it.
static MEM: Mutex<BTreeMap<String, Arc<[u8]>>> = Mutex::new(BTreeMap::new());

/// `p` as a `MEM` key: `/`-separated, without `.` parts (`./mods/x` and `mods/x` are one file).
pub(crate) fn key(p: &Path) -> String {
    p.components().filter(|c| !matches!(c, Component::CurDir)).map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

pub fn add(path: &Path, bytes: Arc<[u8]>) {
    MEM.lock().unwrap().insert(key(path), bytes);
}

pub fn read(p: &Path) -> io::Result<Vec<u8>> {
    match MEM.lock().unwrap().get(&key(p)) {
        Some(b) => Ok(b.to_vec()),
        None => std::fs::read(p),
    }
}

pub fn read_to_string(p: &Path) -> io::Result<String> {
    String::from_utf8(read(p)?).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn is_file(p: &Path) -> bool {
    MEM.lock().unwrap().contains_key(&key(p)) || p.is_file()
}

/// The files and folders in folder `p`, sorted.
pub fn read_dir(p: &Path) -> Vec<PathBuf> {
    let pre = format!("{}/", key(p));
    let mut out: BTreeSet<PathBuf> = std::fs::read_dir(p).into_iter().flatten().flatten().map(|e| e.path()).collect();
    let mem = MEM.lock().unwrap();
    out.extend(mem.range(pre.clone()..).take_while(|(k, _)| k.starts_with(&pre)).map(|(k, _)| p.join(k[pre.len()..].split('/').next().unwrap())));
    out.into_iter().collect()
}

/// Every in-memory file under folder `p` (path, bytes): an import's files for the page to store.
pub fn files_under(p: &Path) -> Vec<(String, Arc<[u8]>)> {
    let pre = format!("{}/", key(p));
    MEM.lock().unwrap().range(pre.clone()..).take_while(|(k, _)| k.starts_with(&pre)).map(|(k, b)| (k.clone(), b.clone())).collect()
}

/// Drops every in-memory file under folder `p`.
pub fn remove(p: &Path) {
    let pre = format!("{}/", key(p));
    MEM.lock().unwrap().retain(|k, _| !k.starts_with(&pre));
}

/// The extensions a mod's files may have (`plan/M17.md`): models, manifests, textures, audio.
pub const EXTENSIONS: [&str; 7] = ["json", "glb", "png", "webp", "tm2", "wav", "ogg"];
/// Caps on a zip's entries, one unpacked file and all of them together: checked while inflating, not taken from the
/// zip's headers. The largest mod the mods repo makes is ≈ 590 MB (Out of Bounds characters).
pub const MAX_ENTRIES: usize = 8192;
pub const MAX_FILE: usize = 768 << 20;
pub const MAX_TOTAL: usize = 1 << 30;

/// A mod's files: paths relative to its folder, and their bytes.
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
pub fn unpack(zip: &[u8]) -> Result<Files, String> {
    unpack_capped(zip, MAX_FILE, MAX_TOTAL)
}

fn unpack_capped(zip: &[u8], max_file: usize, max_total: usize) -> Result<Files, String> {
    // the end of central directory record: the last one, within its comment's 64 KB of the end
    let from = zip.len().saturating_sub(22 + 0xffff);
    let end = (from..=zip.len().saturating_sub(22)).rev().find(|&i| zip[i..].starts_with(b"PK\x05\x06")).ok_or("not a zip file")?;
    let (n, at) = (u16_at(zip, end + 10)?, u32_at(zip, end + 16)?);
    if u16_at(zip, end + 4)? != 0 || u16_at(zip, end + 6)? != 0 || u16_at(zip, end + 8)? != n || n == 0xffff || at == u32::MAX {
        return Err("split and zip64 archives aren't supported".into());
    }
    if n > MAX_ENTRIES {
        return Err(format!("more than {MAX_ENTRIES} files"));
    }
    let (mut at, mut total, mut seen, mut out) = (at as usize, 0usize, HashSet::new(), Vec::new());
    for _ in 0..n {
        if u32_at(zip, at)? != 0x0201_4b50 {
            return Err("a damaged zip directory".into());
        }
        let (made, flags, method) = (u16_at(zip, at + 4)?, u16_at(zip, at + 8)?, u16_at(zip, at + 10)?);
        let (crc, packed, size) = (u32_at(zip, at + 16)?, u32_at(zip, at + 20)?, u32_at(zip, at + 24)?);
        let (nlen, xlen, clen) = (u16_at(zip, at + 28)?, u16_at(zip, at + 30)?, u16_at(zip, at + 32)?);
        let (attrs, local) = (u32_at(zip, at + 38)?, u32_at(zip, at + 42)? as usize);
        let raw = zip.get(at + 46..at + 46 + nlen).ok_or("the zip is cut short")?;
        at += 46 + nlen + xlen + clen;
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
        if u32_at(zip, local)? != 0x0403_4b50 || zip.get(local + 30..local + 30 + nlen) != Some(raw) {
            return Err(format!("`{name}`: damaged in the zip"));
        }
        let start = local + 30 + nlen + u16_at(zip, local + 28)?;
        let data = zip.get(start..start + packed as usize).ok_or("the zip is cut short")?;
        let cap = max_file.min(max_total - total);
        let bytes = match method {
            0 if data.len() <= cap => data.to_vec(),
            0 => return Err(format!("`{name}` is too big")),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(data, cap).map_err(|e| match e.status {
                miniz_oxide::inflate::TINFLStatus::HasMoreOutput => format!("`{name}` is too big"),
                _ => format!("`{name}`: damaged in the zip"),
            })?,
            _ => return Err(format!("`{name}`: compressed with a method zips rarely use (only stored and deflate)")),
        };
        if bytes.len() != size as usize || crc32(&bytes) != crc {
            return Err(format!("`{name}`: damaged in the zip"));
        }
        total += bytes.len();
        out.push((name.to_string(), bytes));
    }
    if !out.iter().any(|f| f.0 == "mod.json") {
        // one folder holding everything (a zipped mod folder)
        let top = out.first().and_then(|f| f.0.split_once('/')).map(|t| format!("{}/", t.0)).ok_or("no mod.json in the zip")?;
        if !out.iter().all(|f| f.0.starts_with(&top)) || !out.iter().any(|f| f.0 == format!("{top}mod.json")) {
            return Err("no mod.json in the zip (it must be at the top, or in the one folder everything is in)".into());
        }
        out.iter_mut().for_each(|f| f.0.drain(..top.len()).for_each(drop));
    }
    Ok(out)
}

/// A mod id fit for a folder name: 1..=64 ASCII letters, digits, `_` or `-`.
pub fn good_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Every file of the mod in `dir` read the way the game reads it, as hostile input: `mods::read`, each `.glb`
/// through `mods::open`'s checks (every costume also §7's), each image decoded within limits, each `.wav` as PCM16.
pub fn check_all(dir: &Path, files: &[String]) -> Result<Mod, String> {
    let m = mods::read(dir)?;
    mods::check(&m)?;
    for f in files {
        let p = dir.join(f);
        let bytes = || read(&p).map_err(|e| format!("{f}: {e}"));
        match f.rsplit_once('.').map(|e| e.1.to_ascii_lowercase()).as_deref() {
            Some("glb") => mods::open(&p).map(drop)?,
            Some(ext @ ("png" | "webp")) => image_ok(&bytes()?, ext).map_err(|e| format!("{f}: {e}"))?,
            Some("wav") if crate::audio::pcm(&bytes()?).is_none() => return Err(format!("{f}: not a PCM16 wav")),
            Some("json") => drop(serde_json::from_slice::<serde_json::Value>(&bytes()?).map_err(|e| format!("{f}: not JSON: {e}"))?),
            _ => {}
        }
    }
    Ok(m)
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

/// A mod zip installed into `root` (the `mods/` folder) as `root/<id>/`, in memory: unpacked ([`unpack`]), checked
/// whole ([`check_all`]) under `root/.import/`, then put in place of an installed mod with the same id. On any
/// failure nothing is kept. Its files are then [`files_under`] `root/<id>` for the page to store.
pub fn install(zip: &[u8], root: &Path) -> Result<Mod, String> {
    let files = unpack(zip)?;
    let manifest = &files.iter().find(|f| f.0 == "mod.json").ok_or("no mod.json")?.1;
    let j: serde_json::Value = serde_json::from_slice(manifest).map_err(|e| format!("mod.json: not JSON: {e}"))?;
    let id = j["id"].as_str().filter(|id| good_id(id)).ok_or("mod.json: `id` must be 1 to 64 letters, digits, `_` or `-`")?.to_string();
    let stage = root.join(".import");
    remove(&stage);
    let names: Vec<String> = files.iter().map(|f| f.0.clone()).collect();
    for (n, b) in files {
        add(&stage.join(n), b.into());
    }
    let got = check_all(&stage, &names);
    let staged = files_under(&stage);
    remove(&stage);
    got.map_err(|e| e.replace(&format!("{}/", stage.display()), ""))?;
    let dir = root.join(&id);
    remove(&dir);
    let (from, to) = (key(&stage), key(&dir));
    let mut mem = MEM.lock().unwrap();
    for (k, b) in staged {
        mem.insert(format!("{to}{}", &k[from.len()..]), b);
    }
    drop(mem);
    mods::read(&dir)
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The real mods (`context/mods`, links into the mods repo's `out/mods`).
    pub(crate) const MODS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/mods");

    /// Every file under `dir` (relative path, bytes), sorted.
    pub(crate) fn dir_files(dir: &Path) -> Files {
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
