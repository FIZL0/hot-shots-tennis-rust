//! The mod store (M17, `plan/M17.md`): players' mods on the public Hugging Face dataset repo [`REPO`]. Browse lists
//! `mods/<id>/` on `main` (merged: Verified) and on every open pull request (an upload goes live as its PR opens; the
//! PR's author is the mod's author); `main` wins an id, then the newest PR. Install downloads `mod.zip` over HTTPS
//! from the store only, checks its size and sha256 against `meta.json`, then installs it through M16's checks
//! (`modfs::install`): into `mods/<id>/` on disk, or the page's OPFS store in the browser. Upload packages an
//! installed mod (`mod.zip`, `meta.json`, `preview.png`) and commits it to the repo as a pull request through the
//! Hub API (user 2026-10-13: no upload page), with the player's own Hugging Face write token, pasted once and kept
//! in `hf-token.txt` beside settings.txt / localStorage.
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
/// The player's Hugging Face account: `<name>\n<token>`.
const TOKEN: &str = "hf-token.txt";

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
    /// What the select shows of it, for Browse's card: play style (0..4), △ grades (1..5), hand (±1), costumes, and
    /// the disc character it plays like (0..14). Absent in an upload from before these.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grades: Option<[u8; 5]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand: Option<i8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub costumes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub like: Option<u8>,
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
    Uploaded(Result<String, String>),
    /// A pasted token checked: the account's name, or why it isn't kept.
    Signed(Result<String, String>),
    /// An install under way: what to show (how much has arrived, or that it's being checked).
    Progress(String),
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

/// GET `url` (the store's only): its answer, unless it says it's more than `cap` bytes.
async fn answer(url: &str, cap: u64) -> Result<reqwest::Response, String> {
    debug_assert!(url.starts_with(HF));
    #[cfg(not(target_arch = "wasm32"))]
    let _ = rustls::crypto::ring::default_provider().install_default();
    let r = reqwest::get(url).await.map_err(|e| format!("can't reach the store: {e}"))?;
    let r = r.error_for_status().map_err(|e| format!("the store said {}", e.status().map_or("no".into(), |s| s.to_string())))?;
    if r.content_length().is_some_and(|n| n > cap) {
        return Err("too big".into());
    }
    Ok(r)
}

/// GET `url` (the store's only), at most `cap` bytes.
async fn get(url: &str, cap: u64) -> Result<Vec<u8>, String> {
    let b = answer(url, cap).await?.bytes().await.map_err(|e| format!("download cut short: {e}"))?;
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
    m.style = m.style.filter(|&s| s < 4);
    m.grades = m.grades.filter(|g| g.iter().all(|&g| (1..=5).contains(&g)));
    m.hand = m.hand.filter(|h| h.abs() == 1);
    m.costumes = m.costumes.filter(|&c| c < 1000);
    m.like = m.like.filter(|&c| c < 14);
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

/// A download of `size` bytes with sha256 `hash` is the zip `meta` describes.
#[cfg(any(test, not(target_arch = "wasm32")))]
fn matches(size: u64, hash: &str, meta: &Meta) -> Result<(), String> {
    if size != meta.size {
        return Err(format!("the download is {size} bytes, the store says {}", meta.size));
    }
    if hash != meta.sha256 {
        return Err("the download doesn't match its sha256 (damaged or changed)".into());
    }
    Ok(())
}

fn hex(d: &[u8]) -> String {
    d.iter().map(|x| format!("{x:02x}")).collect()
}

fn sha256(b: &[u8]) -> String {
    hex(&Sha256::digest(b))
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

/// Downloads, checks and installs `e` into `root` ([`Event::Installed`]), saying how far it's got
/// ([`Event::Progress`]). The zip streams to a file (B204: never whole in memory) and is hashed as it arrives; only
/// a zip of the listed size and sha256 is unpacked, from that file, a file at a time. The zip goes afterwards. The
/// browser reads the parts it keeps straight from the store instead ([`ranged`]).
pub fn install(e: &Entry, root: &Path) {
    let (e, root) = (e.clone(), root.to_path_buf());
    spawn(async move {
        let got = async {
            #[cfg(not(target_arch = "wasm32"))]
            let m = {
                let mut zip = Zip::create(&root).await?;
                let (size, hash) = download(&e, &mut zip).await?;
                matches(size, &hash, &e.meta)?;
                send(Event::Progress(format!("Checking {}...", e.meta.name)));
                zip.install(&root, &e.meta.id).await?
            };
            #[cfg(target_arch = "wasm32")]
            let m = ranged(&e, &root).await?;
            let s = Stamp { sha256: e.meta.sha256.clone(), rev: e.rev.clone(), verified: e.verified };
            modfs::write(&m.dir.join("store.json"), serde_json::to_vec(&s).unwrap())?;
            keep(&m)
        };
        send(Event::Installed(got.await));
    });
}

/// `e`'s `mod.zip` from the store into `zip`: its size and sha256.
#[cfg(not(target_arch = "wasm32"))]
async fn download(e: &Entry, zip: &mut Zip) -> Result<(u64, String), String> {
    use n0_future::StreamExt;
    let cap = e.meta.size.min(MAX_ZIP);
    let r = answer(&file_url(&e.rev, &format!("mods/{}/mod.zip", e.meta.id)), cap).await?;
    let total = r.content_length().unwrap_or(e.meta.size).max(1);
    let (mut got, mut hash, mut chunks) = (0u64, Sha256::new(), r.bytes_stream());
    while let Some(c) = chunks.next().await {
        let c = c.map_err(|e| format!("download cut short: {e}"))?;
        let mb = got >> 20;
        got += c.len() as u64;
        if got > cap {
            return Err("too big".into());
        }
        hash.update(&c);
        zip.write(&c).await?;
        if got >> 20 != mb {
            send(Event::Progress(format!("Downloading {}... {} / {} MB ({}%)", e.meta.name, got >> 20, total >> 20, got * 100 / total)));
        }
    }
    Ok((got, hex(&hash.finalize())))
}

/// A download's zip until it's installed: `mods/.download.zip` beside the game, removed when dropped.
#[cfg(not(target_arch = "wasm32"))]
struct Zip(std::fs::File, std::path::PathBuf);

#[cfg(not(target_arch = "wasm32"))]
impl Zip {
    async fn create(root: &Path) -> Result<Self, String> {
        let p = root.join(".download.zip");
        let f = std::fs::create_dir_all(root).and_then(|_| std::fs::File::create(&p)).map_err(|e| format!("can't write {}: {e}", p.display()))?;
        Ok(Zip(f, p))
    }
    async fn write(&mut self, b: &[u8]) -> Result<(), String> {
        std::io::Write::write_all(&mut self.0, b).map_err(|e| format!("can't write {}: {e}", self.1.display()))
    }
    async fn install(&self, root: &Path, id: &str) -> Result<Mod, String> {
        modfs::install_file(&self.1, root, Some(id))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Zip {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.1);
    }
}

/// B227: the browser installs `e`'s `mod.zip` from the store a part at a time (HTTP range requests), reading only
/// the files it keeps (`modfs::kept_in_memory`: never the upscaled set, ≈ 90% of a store mod), so nothing of it is
/// stored whole. Its sha256 needs all of it, so here its size is checked against the store's, and each file's CRC-32
/// as it's unpacked (`modfs::unpack_with`); the connection is HTTPS to the store either way.
#[cfg(any(test, target_arch = "wasm32"))]
async fn ranged(e: &Entry, root: &Path) -> Result<Mod, String> {
    send(Event::Progress(format!("Downloading {}...", e.meta.name)));
    let r = range(&file_url(&e.rev, &format!("mods/{}/mod.zip", e.meta.id)), 0, 1).await?;
    // the zip's size from "bytes 0-0/<size>"; the rest is asked of the URL the store redirected to (its CDN)
    let len: u64 = r.headers().get("content-range").and_then(|v| v.to_str().ok()?.rsplit_once('/')?.1.parse().ok()).ok_or("the store didn't say the zip's size")?;
    if len != e.meta.size {
        return Err(format!("the download is {len} bytes, the store says {}", e.meta.size));
    }
    if len > MAX_ZIP {
        return Err("too big".into());
    }
    let (url, got) = (r.url().to_string(), std::cell::Cell::new(0u64));
    let read = |at: u64, n: usize| {
        let (url, got, name) = (url.clone(), &got, &e.meta.name);
        async move {
            if n == 0 {
                return Ok(Vec::new());
            }
            let b = range(&url, at, n).await?.bytes().await.map_err(|e| format!("download cut short: {e}"))?;
            if b.len() != n {
                return Err("download cut short".into());
            }
            let mb = got.get() >> 20;
            got.set(got.get() + n as u64);
            if got.get() >> 20 != mb {
                send(Event::Progress(format!("Downloading {name}... {} MB", got.get() >> 20)));
            }
            Ok(b.to_vec())
        }
    };
    modfs::install_with(len, read, root, Some(&e.meta.id), false).await
}

/// Bytes `at..at + n` of `url` (the store's file, or where it redirected).
#[cfg(any(test, target_arch = "wasm32"))]
async fn range(url: &str, at: u64, n: usize) -> Result<reqwest::Response, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = rustls::crypto::ring::default_provider().install_default();
    let r = reqwest::Client::new().get(url).header("Range", format!("bytes={at}-{}", at + n as u64 - 1)).send().await.map_err(|e| format!("can't reach the store: {e}"))?;
    let r = r.error_for_status().map_err(|e| format!("the store said {}", e.status().map_or("no".into(), |s| s.to_string())))?;
    if r.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err("the store sent the whole zip, not the part asked for".into());
    }
    Ok(r)
}

/// Desktop Add mod: the system's file picker for a mod `.zip`, installed like a download (`modfs::install_file`'s
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
            keep(&modfs::install_file(f.path(), &root, None)?)
        })();
        send(Event::Installed(got));
    });
}

/// Keeps an installed mod: natively it's on disk already; in the browser the page writes it to OPFS and says how
/// that went.
fn keep(m: &Mod) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    crate::web::mod_save(&m.id, &m.name);
    Ok(m.name.clone())
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

/// Mod `m` packaged for the store, after the checks Install runs on every player's machine; `stats` is its play
/// style and △ grades as the select shows them, when the select has it.
pub fn package(m: &Mod, stats: Option<(usize, [u8; 5])>) -> Result<Package, String> {
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
        style: stats.map(|s| s.0 as u8),
        grades: stats.map(|s| s.1),
        hand: Some(if m.hand < 0.0 { -1 } else { 1 }),
        costumes: Some(m.costumes.len() as u32),
        like: Some(m.donor as u8),
    };
    let preview = files.iter().find(|f| f.0.starts_with("previews/") && f.0.ends_with(".png")).map(|f| f.1.clone());
    Ok(Package { zip, meta, preview })
}

/// The signed-in account: its name and token.
pub fn account() -> Option<(String, String)> {
    let t = crate::store::read_to_string(TOKEN).ok()?;
    let (name, token) = t.trim().split_once('\n')?;
    let token = token.trim();
    (!token.is_empty()).then(|| (name.into(), token.into()))
}

/// The page where the player makes a token (write access) to paste.
pub const TOKENS: &str = "https://huggingface.co/settings/tokens/new?tokenType=write&description=Hot%20Shots%20Tennis%20mods";

/// `text` as a Hugging Face token (`hf_…`), if it holds one.
pub fn token_in(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').find(|w| w.starts_with("hf_") && w.len() >= 30 && w.len() <= 200).map(str::to_string)
}

/// Checks a pasted token with Hugging Face and keeps it ([`Event::Signed`]).
pub fn sign_in(token: String) {
    spawn(async move {
        let got = async {
            let r = call(client().get(format!("{HF}/api/whoami-v2")).bearer_auth(&token)).await.map_err(|e| format!("Hugging Face didn't take that token ({e})"))?;
            let name = r["name"].as_str().ok_or("Hugging Face sent no account name")?.to_string();
            if r["auth"]["accessToken"]["role"] == "read" {
                return Err("that token can only read: make one with write access".into());
            }
            crate::store::write(TOKEN, format!("{name}\n{token}\n")).map_err(|e| format!("can't keep the token: {e}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(TOKEN, std::fs::Permissions::from_mode(0o600));
            }
            Ok(name)
        };
        send(Event::Signed(got.await));
    });
}

/// Forgets the token.
pub fn sign_out() {
    let _ = crate::store::write(TOKEN, "");
}

/// Packages mod `m` and commits it to the store as a pull request with the player's token ([`Event::Uploaded`]).
/// `card` is the select's picture of it, sent when the mod ships no `previews/*.png`.
pub fn publish(m: &Mod, stats: Option<(usize, [u8; 5])>, card: Option<Vec<u8>>, token: String) {
    let m = m.clone();
    spawn(async move {
        let got = async {
            let p = package(&m, stats)?;
            let dir = format!("mods/{}", m.id);
            let mut files = vec![(format!("{dir}/mod.zip"), p.zip), (format!("{dir}/meta.json"), serde_json::to_vec_pretty(&p.meta).unwrap())];
            if let Some(png) = p.preview.or(card).filter(|b| b.len() as u64 <= MAX_PREVIEW) {
                files.push((format!("{dir}/preview.png"), png));
            }
            let pr = commit(&token, &format!("Add mod {} ({})", m.name, m.id), files).await?;
            // the next Browse reads the store again, this upload included
            let _ = crate::store::write(CACHE, "");
            Ok(format!("{} is up ({} MB): it's in everyone's Browse mods now. {pr}", m.name, p.meta.size.div_ceil(1 << 20)))
        };
        send(Event::Uploaded(got.await));
    });
}

/// An HTTP client; natively the TLS provider goes in first (building one without it panics).
fn client() -> reqwest::Client {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::new()
}

/// A request's JSON answer, or what went wrong (the status and the start of the body).
async fn call(r: reqwest::RequestBuilder) -> Result<serde_json::Value, String> {
    let r = r.send().await.map_err(|e| format!("can't reach Hugging Face: {e}"))?;
    let status = r.status();
    let body = r.bytes().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = serde_json::from_slice::<serde_json::Value>(&body).ok().and_then(|j| j["error"].as_str().map(str::to_string)).unwrap_or_else(|| String::from_utf8_lossy(&body[..body.len().min(200)]).into());
        return Err(format!("{status}: {msg}"));
    }
    Ok(serde_json::from_slice(&body).unwrap_or_default())
}

/// The Hub's commit API (as `huggingface_hub`'s `create_commit`): asks which files go through LFS, uploads those,
/// then commits all of them to a new pull request. Returns the PR's URL.
async fn commit(token: &str, summary: &str, files: Vec<(String, Vec<u8>)>) -> Result<String, String> {
    use data_encoding::BASE64;
    use serde_json::json;
    let c = client();
    let api = format!("{HF}/api/datasets/{REPO}");
    let ask = json!({ "files": files.iter().map(|(path, b)| json!({ "path": path, "size": b.len(), "sample": BASE64.encode(&b[..b.len().min(512)]) })).collect::<Vec<_>>() });
    // `create_pr`: an account with read access only may still open a PR (403 without it)
    let modes = call(c.post(format!("{api}/preupload/main?create_pr=1")).bearer_auth(token).header("Content-Type", "application/json").body(ask.to_string())).await?;
    let lfs = |path: &str| modes["files"].as_array().into_iter().flatten().any(|f| f["path"] == path && f["uploadMode"] == "lfs");
    let mut lines = vec![json!({ "key": "header", "value": { "summary": summary, "description": "Uploaded from the game." } })];
    for (path, b) in &files {
        lines.push(if lfs(path) {
            let oid = sha256(b);
            lfs_put(&c, token, &oid, b).await?;
            json!({ "key": "lfsFile", "value": { "path": path, "algo": "sha256", "oid": oid, "size": b.len() } })
        } else {
            json!({ "key": "file", "value": { "path": path, "content": BASE64.encode(b), "encoding": "base64" } })
        });
    }
    let body = lines.iter().map(|l| l.to_string() + "\n").collect::<String>();
    let r = call(c.post(format!("{api}/commit/main?create_pr=1")).bearer_auth(token).header("Content-Type", "application/x-ndjson").body(body)).await?;
    Ok(r["pullRequestUrl"].as_str().or(r["commitUrl"].as_str()).unwrap_or_default().to_string())
}

/// One file into the repo's LFS store (git-lfs batch API: a single PUT, or parts for a big file), then verified.
async fn lfs_put(c: &reqwest::Client, token: &str, oid: &str, b: &[u8]) -> Result<(), String> {
    use serde_json::json;
    let lfs = "application/vnd.git-lfs+json";
    // no `ref`: it makes the Hub check write access to that branch, and a PR only needs read (as huggingface_hub)
    let ask = json!({ "operation": "upload", "transfers": ["basic", "multipart"], "hash_algo": "sha256", "objects": [{ "oid": oid, "size": b.len() }] });
    let r = call(c.post(format!("{HF}/datasets/{REPO}.git/info/lfs/objects/batch")).bearer_auth(token).header("Accept", lfs).header("Content-Type", lfs).body(ask.to_string())).await?;
    let o = &r["objects"][0];
    if let Some(e) = o["error"]["message"].as_str() {
        return Err(format!("LFS: {e}"));
    }
    // no upload action: the store has these bytes already
    let Some(up) = o["actions"].get("upload") else { return Ok(()) };
    let href = up["href"].as_str().ok_or("LFS: no upload URL")?;
    let header = up["header"].as_object().cloned().unwrap_or_default();
    match header.get("chunk_size").and_then(|v| v.as_str()?.parse::<usize>().ok().or(v.as_u64().map(|n| n as usize))) {
        Some(size) if size > 0 => {
            let mut parts: Vec<(usize, &str)> = header.iter().filter_map(|(k, v)| Some((k.parse().ok()?, v.as_str()?))).collect();
            parts.sort();
            let mut done = Vec::new();
            for (i, (n, url)) in parts.into_iter().enumerate() {
                let chunk = &b[(i * size).min(b.len())..((i + 1) * size).min(b.len())];
                let r = c.put(url).body(chunk.to_vec()).send().await.map_err(|e| format!("upload cut short: {e}"))?;
                let r = r.error_for_status().map_err(|e| format!("upload refused: {e}"))?;
                let etag = r.headers().get("etag").and_then(|v| v.to_str().ok()).ok_or("upload: no ETag back")?.to_string();
                done.push(json!({ "partNumber": n, "etag": etag }));
            }
            call(c.post(href).header("Accept", lfs).header("Content-Type", lfs).body(json!({ "oid": oid, "parts": done }).to_string())).await?;
        }
        _ => {
            let mut put = c.put(href).body(b.to_vec());
            for (k, v) in &header {
                if let Some(v) = v.as_str() {
                    put = put.header(k.as_str(), v);
                }
            }
            put.send().await.map_err(|e| format!("upload cut short: {e}"))?.error_for_status().map_err(|e| format!("upload refused: {e}"))?;
        }
    }
    if let Some(v) = o["actions"].get("verify") {
        let href = v["href"].as_str().ok_or("LFS: no verify URL")?;
        call(c.post(href).bearer_auth(token).header("Accept", lfs).header("Content-Type", lfs).body(json!({ "oid": oid, "size": b.len() }).to_string())).await?;
    }
    Ok(())
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
    /// B227: the browser's install of the store's Anya (PR 2, 513 MB) by range requests: fetches only her kept files
    /// (no `textures/upscaled/`), in memory. Network: `tools/check.sh -p hst ranged_install -- --ignored` after
    /// touching `ranged`.
    #[test]
    #[ignore]
    fn ranged_install_leaves_the_upscaled_set_on_the_store() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let meta: Meta = rt.block_on(async { serde_json::from_slice(&get(&file_url("refs/pr/2", "mods/oob_pc12_anya/meta.json"), MAX_META).await.unwrap()).unwrap() });
        let e = Entry { meta, rev: "refs/pr/2".into(), verified: false };
        let root = Path::new("t_ranged/mods");
        let m = rt.block_on(ranged(&e, root)).unwrap();
        let files = modfs::files_under(&m.dir);
        let mb = files.iter().map(|f| f.1.len()).sum::<usize>() >> 20;
        let fetched = take().into_iter().filter_map(|e| if let Event::Progress(p) = e { Some(p) } else { None }).last().unwrap();
        eprintln!("{} files, {mb} MB kept of a {} MB zip; last progress: {fetched}", files.len(), e.meta.size >> 20);
        assert!(files.iter().all(|f| !f.0.contains("/textures/upscaled/")) && files.iter().any(|f| f.0.ends_with(".glb")));
        assert!(mb < 64);
        modfs::remove(Path::new("t_ranged"));
    }

    use super::*;

    fn meta(id: &str, zip: &[u8]) -> Meta {
        Meta { id: id.into(), name: "Lauryn".into(), size: zip.len() as u64, sha256: sha256(zip), ..Default::default() }
    }

    /// A token is picked out of whatever was copied with it; anything else isn't one.
    #[test]
    fn finds_the_token() {
        let t = "hf_abcdefghijklmnopqrstuvwxyz0123";
        assert_eq!(token_in(&format!("  {t}\n")).as_deref(), Some(t));
        assert_eq!(token_in(&format!("token: \"{t}\"")).as_deref(), Some(t));
        assert_eq!(token_in("hf_short"), None);
        assert_eq!(token_in("hello there"), None);
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
        let p = package(&m, Some((2, [1, 2, 3, 4, 5]))).unwrap();
        eprintln!("packed {} ({} bytes, {} files)", m.id, p.meta.size, mod_files(&m.dir).unwrap().len());
        assert_eq!(p.meta.id, m.id);
        matches(p.zip.len() as u64, &sha256(&p.zip), &p.meta).unwrap();
        let mut bad = p.zip.clone();
        bad[100] ^= 1;
        assert!(matches(bad.len() as u64, &sha256(&bad), &p.meta).unwrap_err().contains("sha256"));
        assert!(matches(p.zip.len() as u64 - 1, &sha256(&p.zip[1..]), &p.meta).is_err());
        // streamed to its file, then installed on disk from it (over an older copy) a file at a time; a zip whose mod
        // isn't the listed id leaves the older copy as it was; nothing is kept in memory, the zip goes when done
        let disk = std::env::temp_dir().join(format!("hst-store-{}", std::process::id()));
        std::fs::create_dir_all(disk.join(&m.id).join("old")).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let got = rt
            .block_on(async {
                let mut z = Zip::create(&disk).await.unwrap();
                for c in p.zip.chunks(64 << 10) {
                    z.write(c).await.unwrap();
                }
                assert!(z.install(&disk, "someone_else").await.unwrap_err().contains("not `someone_else`"));
                assert!(disk.join(&m.id).join("old").exists() && !disk.join(".import").exists());
                z.install(&disk, &m.id).await
            })
            .unwrap();
        assert!(!disk.join(".download.zip").exists() && !disk.join(".import").exists());
        assert!(!disk.join(&m.id).join("old").exists());
        assert!(modfs::files_under(&disk).is_empty());
        assert_eq!(crate::mods::read(&got.dir).unwrap().id, m.id);
        let s = Stamp { sha256: p.meta.sha256.clone(), rev: "refs/pr/1".into(), verified: false };
        modfs::write(&got.dir.join("store.json"), serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(stamp(&got.dir).unwrap().rev, "refs/pr/1");
        // the stamp stays out of a re-upload's zip
        assert!(!mod_files(&got.dir).unwrap().iter().any(|f| f.0 == "store.json"));
        assert_eq!(mod_files(&got.dir).unwrap().len(), mod_files(&m.dir).unwrap().len());
        std::fs::remove_dir_all(&disk).unwrap();
    }

    /// The real store answers (`HST_LIVE=1`): a listing comes back (empty while nobody has uploaded), and a mod
    /// with no preview gives None.
    #[test]
    fn live_sign_in() {
        if std::env::var("HST_LIVE").is_err() {
            return eprintln!("HST_LIVE unset (skipped)");
        }
        sign_in("hf_notarealtokennotarealtoken0000".into());
        for _ in 0..300 {
            if let Some(Event::Signed(r)) = take().into_iter().next() {
                eprintln!("signed: {r:?}");
                return assert!(r.is_err());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        panic!("no answer in 30 s");
    }

    /// A real store mod installed (`HST_LIVE_INSTALL=<id>`, B204) into a temporary folder, its progress and (Linux) the
    /// peak memory printed: it should stay well under the mod's size.
    #[test]
    fn live_install() {
        let Ok(id) = std::env::var("HST_LIVE_INSTALL") else { return eprintln!("HST_LIVE_INSTALL unset (skipped)") };
        list(true);
        let root = std::env::temp_dir().join(format!("hst-live-{}", std::process::id()));
        let started = std::time::Instant::now();
        loop {
            for ev in take() {
                match ev {
                    Event::Listing(r) => install(r.unwrap().iter().find(|e| e.meta.id == id).expect("not in the store"), &root),
                    Event::Progress(s) if s.ends_with("0%)") || !s.contains('%') => eprintln!("{:.1}s {s}", started.elapsed().as_secs_f32()),
                    Event::Installed(r) => {
                        eprintln!("{:.1}s {r:?}", started.elapsed().as_secs_f32());
                        let n = walk_len(&root.join(&id));
                        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
                        eprintln!("{n} files; {}", status.lines().find(|l| l.starts_with("VmHWM")).unwrap_or("no VmHWM"));
                        std::fs::remove_dir_all(&root).unwrap();
                        return assert!(r.is_ok() && n > 0);
                    }
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    fn walk_len(d: &Path) -> usize {
        std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| if e.path().is_dir() { walk_len(&e.path()) } else { 1 }).sum()
    }

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
