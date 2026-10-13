//! The mod store (M17, `plan/M17.md`): players' mods on the public Hugging Face dataset repo [`REPO`]. Browse lists
//! `mods/<id>/` on `main` (merged: Verified) and on every open pull request (an upload goes live as its PR opens; the
//! PR's author is the mod's author); `main` wins an id, then the newest PR. Install downloads `mod.zip` over HTTPS
//! from the store only, checks its size and sha256 against `meta.json`, then installs it through M16's checks
//! (`modfs::install`): into `mods/<id>/` on disk, or the page's OPFS store in the browser. Upload packages an
//! installed mod (`mod.zip`, `meta.json`, `preview.png`) and opens the repo's upload page; the game holds no token.
//!
//! Network work runs off the frame (a thread natively, the page's event loop in the browser); results come back
//! through [`take`].

use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::modfs;
use crate::mods::Mod;

/// The store: one public dataset repo.
pub const REPO: &str = "FIZL0/opentennis-mods";
const HF: &str = "https://huggingface.co";
/// The listing is kept this long (anonymous rate limits), in `mods-store.json` beside settings.txt / localStorage.
const CACHE_SECS: u64 = 3600;
const CACHE: &str = "mods-store.json";
/// The largest `mod.zip` taken (a stored zip of M16's biggest unpacked mod, plus its directory).
const MAX_ZIP: u64 = modfs::MAX_TOTAL as u64 + (16 << 20);
/// `meta.json` and `preview.png` caps.
const MAX_META: u64 = 64 << 10;
const MAX_PREVIEW: u64 = 4 << 20;
/// Where Upload writes its files on desktop.
#[cfg(not(target_arch = "wasm32"))]
const OUTBOX: &str = "mods-upload";

/// `mods/<id>/meta.json`: what Browse shows and what Install checks.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Meta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
    /// `mod.zip`'s bytes and sha256 (lowercase hex).
    pub size: u64,
    pub sha256: String,
    /// The mod standard's version (`mod.json` `standard`).
    #[serde(default)]
    pub skeleton: u32,
}

/// One mod in the store.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub meta: Meta,
    /// `main` or `refs/pr/<n>`.
    pub rev: String,
    /// On `main` (its PR was merged).
    pub verified: bool,
}

pub enum Event {
    Listing(Result<Vec<Entry>, String>),
    /// An install's outcome: the mod's name, or why it was refused (nothing kept).
    Installed(Result<String, String>),
    /// Mod `id`'s `preview.png`, or None (it has none).
    Preview(String, Option<Vec<u8>>),
    /// Upload's outcome: what to tell the player.
    Packaged(Result<String, String>),
}

static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

fn send(e: Event) {
    EVENTS.lock().unwrap().push(e);
}

/// What finished since the last call.
pub fn take() -> Vec<Event> {
    std::mem::take(&mut *EVENTS.lock().unwrap())
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn(f: impl std::future::Future<Output = ()> + Send + 'static) {
    std::thread::spawn(move || tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn spawn(f: impl std::future::Future<Output = ()> + 'static) {
    wasm_bindgen_futures::spawn_local(f);
}

/// `rev` as one URL path part.
fn rev_part(rev: &str) -> String {
    rev.replace('/', "%2F")
}

fn file_url(rev: &str, path: &str) -> String {
    format!("{HF}/datasets/{REPO}/resolve/{}/{path}", rev_part(rev))
}

/// GET `url` (the store's only), at most `cap` bytes.
async fn get(url: &str, cap: u64) -> Result<Vec<u8>, String> {
    debug_assert!(url.starts_with(HF));
    #[cfg(not(target_arch = "wasm32"))]
    let _ = rustls::crypto::ring::default_provider().install_default();
    let r = reqwest::get(url).await.map_err(|e| format!("can't reach the store: {e}"))?;
    let r = r.error_for_status().map_err(|e| format!("the store said {}", e.status().map_or("no".into(), |s| s.to_string())))?;
    if r.content_length().is_some_and(|n| n > cap) {
        return Err("too big".into());
    }
    let b = r.bytes().await.map_err(|e| format!("download cut short: {e}"))?;
    if b.len() as u64 > cap {
        return Err("too big".into());
    }
    Ok(b.to_vec())
}

/// The mod ids under `mods/` at `rev` (none when the folder isn't there).
async fn ids(rev: &str) -> Vec<String> {
    // ponytail: one tree page (HF's is ~1000 entries); follow its Link header when the store outgrows it
    let Ok(b) = get(&format!("{HF}/api/datasets/{REPO}/tree/{}/mods", rev_part(rev)), 4 << 20).await else { return Vec::new() };
    ids_in(&b)
}

fn ids_in(tree: &[u8]) -> Vec<String> {
    #[derive(Deserialize)]
    struct Node {
        #[serde(rename = "type")]
        kind: String,
        path: String,
    }
    let nodes: Vec<Node> = serde_json::from_slice(tree).unwrap_or_default();
    nodes.into_iter().filter(|n| n.kind == "directory").filter_map(|n| n.path.strip_prefix("mods/").map(str::to_string)).filter(|id| modfs::good_id(id)).collect()
}

/// Open pull requests, newest first: (number, author's account name).
async fn pulls() -> Vec<(u32, String)> {
    let mut out = Vec::new();
    for p in 0..20 {
        let url = format!("{HF}/api/datasets/{REPO}/discussions?type=pull_request&status=open&p={p}");
        let Ok(b) = get(&url, 4 << 20).await else { break };
        let page = pulls_in(&b);
        if page.is_empty() {
            break;
        }
        out.extend(page);
    }
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out
}

fn pulls_in(page: &[u8]) -> Vec<(u32, String)> {
    let v: serde_json::Value = serde_json::from_slice(page).unwrap_or_default();
    let list = v["discussions"].as_array().cloned().unwrap_or_default();
    list.iter()
        .filter(|d| d["isPullRequest"].as_bool() == Some(true) && d["status"] == "open")
        .filter_map(|d| Some((d["num"].as_u64()? as u32, d["author"]["name"].as_str().unwrap_or("").to_string())))
        .collect()
}

/// `s` as plain text: no control characters, at most `n` characters.
fn plain_text(s: &str, n: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(n).collect::<String>().trim().to_string()
}

/// `mods/<id>/meta.json`, checked: its id is its folder's, a sha256, a size within the cap; text capped.
fn meta_of(b: &[u8], id: &str) -> Option<Meta> {
    let mut m: Meta = serde_json::from_slice(b).ok()?;
    let hex = m.sha256.len() == 64 && m.sha256.bytes().all(|c| c.is_ascii_hexdigit());
    if m.id != id || !hex || m.size == 0 || m.size > MAX_ZIP {
        return None;
    }
    m.sha256.make_ascii_lowercase();
    m.name = plain_text(&m.name, 40);
    m.source = plain_text(&m.source, 80);
    m.author = plain_text(&m.author, 40);
    m.version = plain_text(&m.version, 16);
    (!m.name.is_empty()).then_some(m)
}

async fn entries(rev: String, author: Option<String>) -> Vec<Entry> {
    let ids = ids(&rev).await;
    let metas = n0_future::join_all(ids.iter().map(|id| get_meta(&rev, id))).await;
    metas
        .into_iter()
        .flatten()
        .map(|mut meta| {
            if let Some(a) = &author {
                meta.author = plain_text(a, 40);
            }
            Entry { meta, verified: rev == "main", rev: rev.clone() }
        })
        .collect()
}

async fn get_meta(rev: &str, id: &str) -> Option<Meta> {
    meta_of(&get(&file_url(rev, &format!("mods/{id}/meta.json")), MAX_META).await.ok()?, id)
}

/// `main` first, then each PR newest first; an id already listed is left out.
fn merge(lists: Vec<Vec<Entry>>) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    for e in lists.into_iter().flatten() {
        if !out.iter().any(|o| o.meta.id == e.meta.id) {
            out.push(e);
        }
    }
    out.sort_by_key(|e| e.meta.name.to_lowercase());
    out
}

fn now() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[derive(Serialize, Deserialize)]
struct Cached {
    at: u64,
    entries: Vec<Entry>,
}

/// Lists the store ([`Event::Listing`]): the cached listing if it's under an hour old and not `fresh`.
pub fn list(fresh: bool) {
    if !fresh {
        let c = crate::store::read_to_string(CACHE).ok().and_then(|s| serde_json::from_str::<Cached>(&s).ok());
        if let Some(c) = c.filter(|c| now().saturating_sub(c.at) < CACHE_SECS) {
            return send(Event::Listing(Ok(c.entries)));
        }
    }
    spawn(async {
        let (main, prs) = n0_future::future::zip(entries("main".into(), None), pulls()).await;
        let reachable = !main.is_empty() || !prs.is_empty() || get(&format!("{HF}/api/datasets/{REPO}"), 1 << 20).await.is_ok();
        if !reachable {
            return send(Event::Listing(Err("Can't reach the mod store.".into())));
        }
        let pr = n0_future::join_all(prs.into_iter().map(|(n, who)| entries(format!("refs/pr/{n}"), Some(who)))).await;
        let all = merge(std::iter::once(main).chain(pr).collect());
        let _ = crate::store::write(CACHE, serde_json::to_string(&Cached { at: now(), entries: all.clone() }).unwrap());
        send(Event::Listing(Ok(all)));
    });
}

/// Fetches mod `e`'s preview ([`Event::Preview`]).
pub fn preview(e: &Entry) {
    let (id, url) = (e.meta.id.clone(), file_url(&e.rev, &format!("mods/{}/preview.png", e.meta.id)));
    spawn(async move { send(Event::Preview(id, get(&url, MAX_PREVIEW).await.ok())) });
}

/// `bytes` is the zip `meta` describes: its size and sha256.
fn matches(bytes: &[u8], meta: &Meta) -> Result<(), String> {
    if bytes.len() as u64 != meta.size {
        return Err(format!("the download is {} bytes, the store says {}", bytes.len(), meta.size));
    }
    if sha256(bytes) != meta.sha256 {
        return Err("the download doesn't match its sha256 (damaged or changed)".into());
    }
    Ok(())
}

fn sha256(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

/// What an installed mod came from (`mods/<id>/store.json`, written beside it by Install).
#[derive(Serialize, Deserialize)]
pub struct Stamp {
    pub sha256: String,
    pub rev: String,
    pub verified: bool,
}

pub fn stamp(dir: &Path) -> Option<Stamp> {
    serde_json::from_slice(&modfs::read(&dir.join("store.json")).ok()?).ok()
}

/// Downloads, checks and installs `e` into `root` ([`Event::Installed`]).
pub fn install(e: &Entry, root: &Path) {
    let (e, root) = (e.clone(), root.to_path_buf());
    spawn(async move {
        let got = async {
            let zip = get(&file_url(&e.rev, &format!("mods/{}/mod.zip", e.meta.id)), e.meta.size.min(MAX_ZIP)).await?;
            matches(&zip, &e.meta)?;
            keep(&put(&zip, &e, &root)?)
        };
        send(Event::Installed(got.await));
    });
}

/// Desktop Add mod: the system's file picker for a mod `.zip`, installed like a download (`modfs::install`'s
/// checks) into `root/<id>/` on disk. The browser build picks through the page instead (`web.rs`).
// ponytail: macOS wants dialogs on the main thread; this runs on a worker (fine on Linux/Windows).
#[cfg(not(target_arch = "wasm32"))]
pub fn pick(root: &Path) {
    let root = root.to_path_buf();
    spawn(async move {
        let Some(f) = rfd::AsyncFileDialog::new().add_filter("Mod", &["zip"]).set_title("Add mod").pick_file().await else {
            return send(Event::Installed(Err("no file picked".into())));
        };
        let got = (|| {
            let len = std::fs::metadata(f.path()).map_err(|e| e.to_string())?.len();
            if len > MAX_ZIP {
                return Err(format!("{len} bytes is bigger than a mod may be"));
            }
            let zip = std::fs::read(f.path()).map_err(|e| e.to_string())?;
            keep(&modfs::install(&zip, &root)?)
        })();
        send(Event::Installed(got));
    });
}

/// A checked zip installed in memory as `root/<id>/` (`modfs::install`) with its [`Stamp`].
fn put(zip: &[u8], e: &Entry, root: &Path) -> Result<Mod, String> {
    let m = modfs::install(zip, root)?;
    if m.id != e.meta.id {
        modfs::remove(&m.dir);
        return Err(format!("the zip holds `{}`, not `{}`", m.id, e.meta.id));
    }
    let s = Stamp { sha256: e.meta.sha256.clone(), rev: e.rev.clone(), verified: e.verified };
    modfs::add(&m.dir.join("store.json"), serde_json::to_vec(&s).unwrap().into());
    Ok(m)
}

/// Keeps an installed mod: natively on disk; in the browser the page writes it to OPFS and says how that went.
fn keep(m: &Mod) -> Result<String, String> {
    #[cfg(not(target_arch = "wasm32"))]
    to_disk(&m.dir)?;
    #[cfg(target_arch = "wasm32")]
    crate::web::mod_save(&m.id, &m.name);
    Ok(m.name.clone())
}

/// The in-memory mod in `dir` written over `dir` on disk, then dropped from memory.
#[cfg(not(target_arch = "wasm32"))]
fn to_disk(dir: &Path) -> Result<(), String> {
    let files = modfs::files_under(dir);
    let tmp = dir.with_extension("download");
    let _ = std::fs::remove_dir_all(&tmp);
    let pre = format!("{}/", modfs::key(dir));
    let write = || -> std::io::Result<()> {
        for (k, b) in &files {
            let p = tmp.join(k.strip_prefix(&pre).unwrap_or(k));
            std::fs::create_dir_all(p.parent().unwrap())?;
            std::fs::write(p, b)?;
        }
        let _ = std::fs::remove_dir_all(dir);
        std::fs::rename(&tmp, dir)
    };
    let r = write().map_err(|e| format!("can't write {}: {e}", dir.display()));
    let _ = std::fs::remove_dir_all(&tmp);
    // on disk now, or (failed) not kept at all
    modfs::remove(dir);
    r
}

/// Every file of the mod in `dir` (path inside it, bytes), sorted; Install's `store.json` left out.
fn mod_files(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    fn walk(d: &Path, pre: &str, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), String> {
        for p in modfs::read_dir(d) {
            let name = format!("{pre}{}", p.file_name().unwrap_or_default().to_string_lossy());
            if modfs::is_file(&p) {
                if name != "store.json" {
                    let b = modfs::read(&p).map_err(|e| format!("{name}: {e}"))?;
                    out.push((name, b));
                }
            } else {
                walk(&p, &format!("{name}/"), out)?;
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out)?;
    Ok(out)
}

/// An upload's three files.
pub struct Package {
    pub zip: Vec<u8>,
    pub meta: Meta,
    pub preview: Option<Vec<u8>>,
}

/// Mod `m` packaged for the store, after the checks Install runs on every player's machine.
pub fn package(m: &Mod) -> Result<Package, String> {
    let files = mod_files(&m.dir)?;
    let names: Vec<String> = files.iter().map(|f| f.0.clone()).collect();
    if let Some(bad) = names.iter().find(|n| !modfs::plain(n) || !modfs::EXTENSIONS.iter().any(|e| n.to_ascii_lowercase().ends_with(&format!(".{e}")))) {
        return Err(format!("`{bad}` can't be in a mod (only {} files)", modfs::EXTENSIONS.map(|e| format!(".{e}")).join(" ")));
    }
    modfs::check_all(&m.dir, &names)?;
    let zip = modfs::zip(&files.iter().map(|f| (f.0.as_str(), &f.1[..])).collect::<Vec<_>>(), true, 0o100644);
    if zip.len() as u64 > MAX_ZIP || files.iter().map(|f| f.1.len()).sum::<usize>() > modfs::MAX_TOTAL {
        return Err("too big for the store (1 GB unpacked at most)".into());
    }
    let j: serde_json::Value = files.iter().find(|f| f.0 == "mod.json").and_then(|f| serde_json::from_slice(&f.1).ok()).unwrap_or_default();
    let meta = Meta {
        id: m.id.clone(),
        name: plain_text(&m.name, 40),
        source: plain_text(&m.source, 80),
        author: plain_text(j["author"].as_str().unwrap_or(""), 40),
        version: plain_text(&j["version"].as_str().map_or_else(|| j["version"].to_string(), str::to_string).replace("null", "1"), 16),
        size: zip.len() as u64,
        sha256: sha256(&zip),
        skeleton: j["standard"].as_u64().unwrap_or(1) as u32,
    };
    let preview = files.iter().find(|f| f.0.starts_with("previews/") && f.0.ends_with(".png")).map(|f| f.1.clone());
    Ok(Package { zip, meta, preview })
}

/// The repo's upload page for mod `id` (the player drops the three files there; it opens a pull request).
pub fn upload_url(id: &str) -> String {
    format!("{HF}/datasets/{REPO}/upload/main/mods/{id}")
}

/// Packages mod `m` and hands the files to the player ([`Event::Packaged`]): natively into `mods-upload/<id>/`, in
/// the browser as downloads; then opens the upload page.
pub fn upload(m: &Mod) {
    let m = m.clone();
    spawn(async move {
        let got = package(&m).and_then(|p| {
            let url = upload_url(&m.id);
            let at = hand_over(&m.id, &p)?;
            open(&url);
            Ok(format!("{} packed ({} MB) {at}. Upload all three on the page that opened, into mods/{}/: {url}", m.name, p.meta.size.div_ceil(1 << 20), m.id))
        });
        send(Event::Packaged(got));
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn hand_over(id: &str, p: &Package) -> Result<String, String> {
    let dir = std::path::PathBuf::from(OUTBOX).join(id);
    let w = || -> std::io::Result<()> {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("mod.zip"), &p.zip)?;
        std::fs::write(dir.join("meta.json"), serde_json::to_vec_pretty(&p.meta).unwrap())?;
        let _ = std::fs::remove_file(dir.join("preview.png"));
        if let Some(png) = &p.preview {
            std::fs::write(dir.join("preview.png"), png)?;
        }
        Ok(())
    };
    w().map_err(|e| format!("can't write {}: {e}", dir.display()))?;
    open(&std::fs::canonicalize(&dir).unwrap_or(dir.clone()).to_string_lossy());
    Ok(format!("into {}", dir.display()))
}

#[cfg(target_arch = "wasm32")]
fn hand_over(_: &str, p: &Package) -> Result<String, String> {
    crate::web::download("mod.zip", &p.zip);
    crate::web::download("meta.json", &serde_json::to_vec_pretty(&p.meta).unwrap());
    if let Some(png) = &p.preview {
        crate::web::download("preview.png", png);
    }
    Ok("as downloads".into())
}

/// Opens `what` (a URL or folder) in the player's browser / file manager.
#[cfg(not(target_arch = "wasm32"))]
pub fn open(what: &str) {
    let mut c = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    } else {
        std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" })
    };
    if let Err(e) = c.arg(what).spawn() {
        bevy::log::warn!("opening {what}: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
pub fn open(what: &str) {
    let _ = web_sys::window().unwrap().open_with_url_and_target(what, "_blank");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(id: &str, zip: &[u8]) -> Meta {
        Meta { id: id.into(), name: "Lauryn".into(), size: zip.len() as u64, sha256: sha256(zip), ..Default::default() }
    }

    /// The store's API answers as HF sends them (shapes copied from live replies, 2026-10-13).
    #[test]
    fn reads_tree_and_pulls() {
        let tree = br#"[{"type":"directory","oid":"a","size":0,"path":"mods/oob_pc16_gloria"},{"type":"file","oid":"b","size":3,"path":"mods/x.json"},{"type":"directory","oid":"c","size":0,"path":"mods/../evil"}]"#;
        assert_eq!(ids_in(tree), ["oob_pc16_gloria"]);
        assert!(ids_in(br#"{"error":"mods does not exist on \"main\""}"#).is_empty());
        let page = br#"{"discussions":[{"num":82,"author":{"name":"Cathleen"},"status":"open","isPullRequest":true},{"num":7,"author":{"name":"x"},"status":"open","isPullRequest":false}],"count":2,"start":0}"#;
        assert_eq!(pulls_in(page), [(82, "Cathleen".to_string())]);
    }

    /// A meta.json is taken only for its own folder, with a real sha256 and a size within the cap; its text is
    /// stripped of control characters and capped.
    #[test]
    fn checks_meta() {
        let ok = meta("lauryn", b"zip");
        let j = |m: &Meta| serde_json::to_vec(m).unwrap();
        assert_eq!(meta_of(&j(&ok), "lauryn"), Some(ok.clone()));
        assert_eq!(meta_of(&j(&ok), "other"), None);
        assert_eq!(meta_of(&j(&Meta { sha256: "zz".repeat(32), ..ok.clone() }), "lauryn"), None);
        assert_eq!(meta_of(&j(&Meta { size: MAX_ZIP + 1, ..ok.clone() }), "lauryn"), None);
        assert_eq!(meta_of(b"not json", "lauryn"), None);
        let loud = meta_of(&j(&Meta { name: format!("A\u{1b}[31mB{}", "x".repeat(100)), ..ok.clone() }), "lauryn").unwrap();
        assert!(!loud.name.contains('\u{1b}') && loud.name.chars().count() == 40);
    }

    /// `main` wins an id over any PR, a newer PR over an older one.
    #[test]
    fn main_then_newest_pr() {
        let e = |id: &str, rev: &str| Entry { meta: Meta { id: id.into(), name: id.into(), ..Default::default() }, rev: rev.into(), verified: rev == "main" };
        let got = merge(vec![vec![e("a", "main")], vec![e("a", "refs/pr/9"), e("b", "refs/pr/9")], vec![e("b", "refs/pr/3"), e("c", "refs/pr/3")]]);
        let got: Vec<_> = got.iter().map(|e| (e.meta.id.as_str(), e.rev.as_str(), e.verified)).collect();
        assert_eq!(got, [("a", "main", true), ("b", "refs/pr/9", false), ("c", "refs/pr/3", false)]);
    }

    /// Upload → Install round trip: a real mod packaged, its zip checked against its meta (a flipped byte or a
    /// short download refused), installed in memory with its stamp; a zip whose mod isn't the listed id is refused
    /// and leaves nothing.
    #[test]
    fn package_then_install() {
        let mods = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods");
        let Some(dir) = std::fs::read_dir(mods).ok().and_then(|d| d.flatten().map(|e| e.path()).filter(|p| p.join("mod.json").is_file()).min()) else {
            return eprintln!("no mods in {mods} (skipped)");
        };
        let m = crate::mods::read(&dir).unwrap();
        let p = package(&m).unwrap();
        eprintln!("packed {} ({} bytes, {} files)", m.id, p.meta.size, mod_files(&m.dir).unwrap().len());
        assert_eq!(p.meta.id, m.id);
        matches(&p.zip, &p.meta).unwrap();
        let mut bad = p.zip.clone();
        bad[100] ^= 1;
        assert!(matches(&bad, &p.meta).unwrap_err().contains("sha256"));
        assert!(matches(&p.zip[1..], &p.meta).is_err());
        let root = Path::new("t_store_mods");
        let e = Entry { meta: p.meta.clone(), rev: "refs/pr/1".into(), verified: false };
        let got = put(&p.zip, &e, root).unwrap();
        assert_eq!(stamp(&got.dir).unwrap().rev, "refs/pr/1");
        // the stamp stays out of a re-upload's zip
        assert!(!mod_files(&got.dir).unwrap().iter().any(|f| f.0 == "store.json"));
        modfs::remove(&got.dir);
        let wrong = Entry { meta: Meta { id: "someone_else".into(), ..e.meta.clone() }, ..e.clone() };
        assert!(put(&p.zip, &wrong, root).unwrap_err().contains("not `someone_else`"));
        assert!(!modfs::is_file(&root.join(&m.id).join("mod.json")));
        // natively the install lands on disk (over an older copy) and leaves memory
        let disk = std::env::temp_dir().join(format!("hst-store-{}", std::process::id()));
        std::fs::create_dir_all(disk.join(&m.id).join("old")).unwrap();
        let got = put(&p.zip, &e, &disk).unwrap();
        to_disk(&got.dir).unwrap();
        assert!(!disk.join(&m.id).join("old").exists() && !disk.join(format!("{}.download", m.id)).exists());
        assert!(modfs::files_under(&got.dir).is_empty());
        assert_eq!(crate::mods::read(&got.dir).unwrap().id, m.id);
        assert_eq!(stamp(&got.dir).unwrap().sha256, p.meta.sha256);
        assert_eq!(mod_files(&got.dir).unwrap().len(), mod_files(&m.dir).unwrap().len());
        std::fs::remove_dir_all(&disk).unwrap();
    }

    /// The real store answers (`HST_LIVE=1`): a listing comes back (empty while nobody has uploaded), and a mod
    /// with no preview gives None.
    #[test]
    fn live_store() {
        if std::env::var("HST_LIVE").is_err() {
            return eprintln!("HST_LIVE unset (skipped)");
        }
        list(true);
        let e = Entry { meta: Meta { id: "nobody_here".into(), ..Default::default() }, rev: "main".into(), verified: true };
        preview(&e);
        let (mut listed, mut shot) = (None, None);
        for _ in 0..600 {
            for ev in take() {
                match ev {
                    Event::Listing(r) => listed = Some(r),
                    Event::Preview(id, b) => shot = Some((id, b)),
                    _ => {}
                }
            }
            if listed.is_some() && shot.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let listed = listed.expect("no listing in 60 s").expect("the store can't be read");
        eprintln!("store: {} mods: {:?}", listed.len(), listed.iter().map(|e| (&e.meta.id, &e.rev)).collect::<Vec<_>>());
        assert_eq!(shot, Some(("nobody_here".into(), None)));
    }
}
