//! The mod store (M17, `plan/M17.md`): players' mods on the public Hugging Face dataset repo [`REPO`]. Browse lists
//! `mods/<id>/` on `main` (merged: Verified) and on every open pull request (an upload goes live as its PR opens; the
//! PR's author is the mod's author); `main` wins an id, then the newest PR. Install downloads `mod.zip` over HTTPS
//! from the store only, checks its size and sha256 against `meta.json`, then installs it through M16's checks
//! (`modfs::install`): into `mods/<id>/` on disk, or the page's OPFS store in the browser. Upload packages an
//! installed mod (`mod.zip`, `meta.json`, `preview.png` and B372's ~256 px `thumb.png`) and commits it to the repo as a pull request through the
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
    /// B373 (M5a/M5c share it): what the mod is, the Mods pages' kind tab it shows on. Absent: a character.
    #[serde(default, skip_serializing_if = "Kind::is_character")]
    pub kind: Kind,
    /// B488: the mod's `mod.json` has `upscale` false (HD textures already): Install never takes an upscaled set.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_upscale: bool,
}

/// B373: a mod's kind (`meta.json` / `mod.json` `"kind"`): absent is a character, so every older entry still reads;
/// one this game doesn't know yet reads as `Other` and shows on no tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Character,
    Court,
    Umpire,
    /// M5a: a texture replacement pack.
    Textures,
    #[serde(other)]
    Other,
}

impl Kind {
    pub fn is_character(&self) -> bool {
        *self == Kind::Character
    }

    /// M8f9: the store's word for a `mod.json`'s kind (`mods::Kind`).
    pub fn of(k: &crate::mods::Kind) -> Kind {
        match k {
            crate::mods::Kind::Character => Kind::Character,
            crate::mods::Kind::Court => Kind::Court,
            crate::mods::Kind::Umpire => Kind::Umpire,
            crate::mods::Kind::Other(_) => Kind::Other,
        }
    }
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

impl Entry {
    /// B248: what tells store mods apart (two authors may share an id): `<author>/<id>`.
    pub fn key(&self) -> String {
        format!("{}/{}", self.meta.author, self.meta.id)
    }
}

pub enum Event {
    Listing(Result<Vec<Entry>, String>),
    /// An install's outcome: whose it is (the store mod's [`Entry::key`], empty for Add mod's pick) and the mod's
    /// name, or why it was refused (nothing kept). B339: several store installs run at once, each saying its own.
    Installed(String, Result<String, String>),
    /// Mod [`Entry::key`]'s `preview.png`, or None (it has none).
    Preview(String, Option<Vec<u8>>),
    /// B372: the Browse tile picture of the store mod whose zip has this sha256 ([`thumb`]): a small PNG, or None (it
    /// has no picture, kept in the cache, or the store couldn't be reached).
    Thumb(String, Option<Vec<u8>>),
    /// B250: how the upload of the installed mod in this folder is going.
    Upload(std::path::PathBuf, Up),
    /// A pasted token checked: the account's name, or why it isn't kept.
    Signed(Result<String, String>),
    /// An install under way ([`Entry::key`]): what to show (how much has arrived, or that it's being checked).
    Progress(String, String),
    /// M19: the signed-in player's own uploads ([`mine`]).
    Mine(Result<Vec<Entry>, String>),
    /// M19: mod `id`'s removal ([`remove`]): what to tell the player.
    Removed(String, Result<String, String>),
}

/// B250: one mod's upload, as the Upload page shows it beside the mod.
#[derive(Clone, Debug, PartialEq)]
pub enum Up {
    /// Waiting for the uploads before it (one Hub commit at a time).
    Queued,
    Uploading,
    /// Up: what to tell the player (the PR's URL in it).
    Done(String),
    /// Left alone: the store has this copy already, or a newer one.
    Skipped(String),
    Failed(String),
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
    let r = client().get(url).send().await.map_err(|e| format!("can't reach the store: {e}"))?;
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

/// `main` first, then each PR newest first; one already listed (same id, and same author or zip) is left out, so
/// two authors' mods sharing an id are both listed (B248).
fn merge(lists: Vec<Vec<Entry>>) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    for e in lists.into_iter().flatten().filter(|e| !crate::mods::dropped(&e.meta.id)) {
        if !out.iter().any(|o| o.meta.id == e.meta.id && (o.meta.author == e.meta.author || o.meta.sha256 == e.meta.sha256)) {
            out.push(e);
        }
    }
    out.sort_by_key(|e| e.meta.name.to_lowercase());
    out
}

/// B380: an Open Tee 1 character ([`crate::mods::retired`]) isn't offered; its uploader still sees it under their
/// own uploads ([`mine`]) to take it down.
fn retired(e: &Entry) -> bool {
    crate::mods::retired(&e.meta.id, &e.meta.source)
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
            return send(Event::Listing(Ok(c.entries.into_iter().filter(|e| !crate::mods::dropped(&e.meta.id) && !retired(e)).collect())));
        }
    }
    spawn(async {
        let (main, prs) = n0_future::future::zip(entries("main".into(), None), pulls()).await;
        let reachable = !main.is_empty() || !prs.is_empty() || get(&format!("{HF}/api/datasets/{REPO}"), 1 << 20).await.is_ok();
        if !reachable {
            return send(Event::Listing(Err("Can't reach the mod store.".into())));
        }
        let pr = n0_future::join_all(prs.into_iter().map(|(n, who)| entries(format!("refs/pr/{n}"), Some(who)))).await;
        let all: Vec<Entry> = merge(std::iter::once(main).chain(pr).collect()).into_iter().filter(|e| !retired(e)).collect();
        let _ = crate::store::write(CACHE, serde_json::to_string(&Cached { at: now(), entries: all.clone() }).unwrap());
        send(Event::Listing(Ok(all)));
    });
}

/// M19: `name`'s own uploads ([`Event::Mine`]): the mods on their open PRs, and those on `main` whose meta.json
/// author is them (an upload writes the account's name there), newest PR first, then `main`. Never cached.
pub fn mine(name: String) {
    spawn(async move { send(Event::Mine(uploads_of(&name).await)) });
}

async fn uploads_of(name: &str) -> Result<Vec<Entry>, String> {
    let (main, prs) = n0_future::future::zip(entries("main".into(), None), pulls()).await;
    if main.is_empty() && prs.is_empty() && get(&format!("{HF}/api/datasets/{REPO}"), 1 << 20).await.is_err() {
        return Err("Can't reach the mod store.".into());
    }
    let prs = prs.into_iter().filter(|p| p.1.eq_ignore_ascii_case(name));
    let pr = n0_future::join_all(prs.map(|(n, who)| entries(format!("refs/pr/{n}"), Some(who)))).await;
    let main: Vec<Entry> = main.into_iter().filter(|e| e.meta.author.eq_ignore_ascii_case(name)).collect();
    Ok(pr.into_iter().flatten().chain(std::iter::once(main).flatten()).collect())
}

/// `refs/pr/<n>`'s `n`.
pub fn pr_of(rev: &str) -> Option<u32> {
    rev.strip_prefix("refs/pr/")?.parse().ok()
}

/// M19: takes the player's own upload `e` off the store ([`Event::Removed`]): an open PR is closed (it leaves Browse
/// at once); a mod on `main` gets a PR deleting `mods/<id>/` (a non-member can't commit to `main`), so it leaves
/// when a maintainer merges that.
// ponytail: closing a PR takes every mod on it; the game's uploads are one mod per PR
pub fn remove(e: &Entry, token: String) {
    let (id, name, rev) = (e.meta.id.clone(), e.meta.name.clone(), e.rev.clone());
    spawn(async move {
        let got = async {
            let said = match pr_of(&rev) {
                Some(n) => {
                    let body = serde_json::json!({ "status": "closed", "comment": format!("Taken down by its uploader from the game ({id}).") });
                    call(client().post(format!("{HF}/api/datasets/{REPO}/discussions/{n}/status")).bearer_auth(&token).header("Content-Type", "application/json").body(body.to_string())).await?;
                    format!("{name} is off the store: its pull request #{n} is closed.")
                }
                None => {
                    let pr = commit(&token, &format!("Remove mod {name} ({id})"), Vec::new(), &[format!("mods/{id}")], None).await?;
                    format!("Asked to delete {name}: it leaves the store when a maintainer merges {pr}")
                }
            };
            let _ = crate::store::write(CACHE, "");
            Ok(said)
        };
        let got = got.await;
        send(Event::Removed(id, got));
    });
}

/// Fetches mod `e`'s preview ([`Event::Preview`], under [`Entry::key`]).
pub fn preview(e: &Entry) {
    let (id, url) = (e.key(), file_url(&e.rev, &format!("mods/{}/preview.png", e.meta.id)));
    spawn(async move { send(Event::Preview(id, get(&url, MAX_PREVIEW).await.ok())) });
}

/// B372: a Browse tile's picture side: `thumb.png` is the preview shrunk to this on its longest side.
pub const THUMB: u32 = 256;

/// B372: the tile picture cache's file for the zip with sha256 `sha` (in folder `dir`, beside `mods/`; the browser's
/// OPFS card store under `<dir>~`): keyed by the zip, so an update's new picture is fetched again.
pub fn thumb_file(dir: &Path, sha: &str) -> std::path::PathBuf {
    dir.join(format!("{}.png", sha.chars().filter(char::is_ascii_hexdigit).collect::<String>()))
}

/// B372: the cached tile picture of the zip with sha256 `sha`: Some(None) when the store has none for it (an empty
/// file), None when it isn't cached.
pub fn cached_thumb(dir: &Path, sha: &str) -> Option<Option<Vec<u8>>> {
    let f = thumb_file(dir, sha);
    #[cfg(target_arch = "wasm32")]
    let b = crate::web::card(&f.to_string_lossy().replace(['/', '\\'], "~")).map(|b| b.to_vec());
    #[cfg(not(target_arch = "wasm32"))]
    let b = std::fs::read(&f).ok();
    b.map(|b| (!b.is_empty()).then_some(b))
}

/// B372: keeps the tile picture `png` (None: the store has none) of the zip with sha256 `sha` for the next visit.
pub fn keep_thumb(dir: &Path, sha: &str, png: Option<&[u8]>) {
    let f = thumb_file(dir, sha);
    let b = png.unwrap_or_default().to_vec();
    #[cfg(target_arch = "wasm32")]
    let _ = crate::web::put_card(f.to_string_lossy().replace(['/', '\\'], "~"), b);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&f, b));
}

/// B372: PNG `png` shrunk to at most `side` px on its longest side (as PNG again; one already that small is kept as
/// it is); None if it doesn't decode.
pub fn shrink(png: &[u8], side: u32) -> Option<Vec<u8>> {
    use image::ImageEncoder;
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?;
    let (w, h) = (img.width(), img.height());
    if w.max(h) <= side {
        return Some(png.to_vec());
    }
    let k = side as f32 / w.max(h) as f32;
    let (nw, nh) = (((w as f32 * k).round() as u32).max(1), ((h as f32 * k).round() as u32).max(1));
    let small = image::imageops::resize(&img.into_rgba8(), nw, nh, image::imageops::FilterType::Triangle);
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out).write_image(&small, nw, nh, image::ExtendedColorType::Rgba8).ok()?;
    Some(out)
}

/// B372: fetches store mod `e`'s tile picture ([`Event::Thumb`] under its sha256): the cache in `dir` first, then its
/// `thumb.png`, else (an upload from before thumbnails) its `preview.png` shrunk here once; what came is cached.
pub fn thumb(e: &Entry, dir: &Path) {
    let (sha, dir) = (e.meta.sha256.clone(), dir.to_path_buf());
    let base = format!("mods/{}", e.meta.id);
    let (small, big) = (file_url(&e.rev, &format!("{base}/thumb.png")), file_url(&e.rev, &format!("{base}/preview.png")));
    spawn(async move {
        if let Some(c) = cached_thumb(&dir, &sha) {
            return send(Event::Thumb(sha, c));
        }
        let got = match get(&small, MAX_PREVIEW).await {
            Ok(b) => Ok(Some(b)),
            Err(_) => match get(&big, MAX_PREVIEW).await {
                Ok(b) => Ok(shrink(&b, THUMB)),
                // (the store answered: it has no picture for this one)
                Err(e) if e.contains("404") => Ok(None),
                Err(e) => Err(e),
            },
        };
        match got {
            Ok(png) => {
                keep_thumb(&dir, &sha, png.as_deref());
                send(Event::Thumb(sha, png));
            }
            Err(_) => send(Event::Thumb(sha, None)),
        }
    });
}

/// B335: the installed store mod in `dir` (id `id`)'s listing picture, to `done` (None: not fetched); false if it
/// isn't a store mod.
pub fn installed_preview(dir: &Path, id: &str, done: impl FnOnce(Option<Vec<u8>>) + Send + 'static) -> bool {
    let Some(s) = stamp(dir) else { return false };
    let url = file_url(&s.rev, &format!("mods/{id}/preview.png"));
    spawn(async move { done(get(&url, MAX_PREVIEW).await.ok()) });
    true
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
    /// B248: the store author it was listed under (with its id, what makes it this store mod); empty before B248.
    #[serde(default)]
    pub author: String,
}

pub fn stamp(dir: &Path) -> Option<Stamp> {
    serde_json::from_slice(&modfs::read(&dir.join("store.json")).ok()?).ok()
}

/// B255: the installed mod in `dir`'s `mod.json` version, as an upload writes it into [`Meta::version`].
pub fn version(dir: &Path) -> String {
    let j: serde_json::Value = modfs::read(&dir.join("mod.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    j["version"].as_str().map_or_else(|| j["version"].to_string(), str::to_string).replace("null", "1")
}

/// B255: version `a` is newer than `b`: dot-separated numbers compared in turn ("1.10" > "1.9"; empty is "1").
pub fn newer(a: &str, b: &str) -> bool {
    let v = |s: &str| {
        let mut n: Vec<u64> = (if s.is_empty() { "1" } else { s }).split('.').map(|p| p.trim().parse().unwrap_or(0)).collect();
        // "1.0" is "1"
        while n.last() == Some(&0) {
            n.pop();
        }
        n
    };
    v(a) > v(b)
}

/// Downloads, checks and installs `e` into `root` ([`Event::Installed`]), saying how far it's got
/// ([`Event::Progress`]). The zip streams to a file (B204: never whole in memory) and is hashed as it arrives; only
/// a zip of the listed size and sha256 is unpacked, from that file, a file at a time. The zip goes afterwards.
/// Without its `upscaled` set (Upscaled Downloads off, the mod's `upscale` false, and always in the browser) the parts kept are read straight
/// from the store instead ([`ranged`]).
pub fn install(e: &Entry, root: &Path, upscaled: bool) {
    let (e, root) = (e.clone(), root.to_path_buf());
    spawn(async move {
        let got = async {
            #[cfg(not(target_arch = "wasm32"))]
            // B488: a mod that doesn't want upscaling never takes its upscaled set
            let m = if upscaled && !e.meta.no_upscale { whole(&e, &root).await? } else { ranged(&e, &root, true).await? };
            #[cfg(target_arch = "wasm32")]
            // the browser never takes the upscaled set (its setting is locked off there)
            let m = {
                let _ = upscaled;
                ranged(&e, &root, false).await?
            };
            let s = Stamp { sha256: e.meta.sha256.clone(), rev: e.rev.clone(), verified: e.verified, author: e.meta.author.clone() };
            modfs::write(&m.dir.join("store.json"), serde_json::to_vec(&s).unwrap())?;
            keep(&m)
        };
        send(Event::Installed(e.key(), got.await));
    });
}

/// `e`'s whole `mod.zip` downloaded, checked and installed on disk.
#[cfg(not(target_arch = "wasm32"))]
async fn whole(e: &Entry, root: &Path) -> Result<Mod, String> {
    let mut zip = Zip::create(root, &format!("{}-{}", e.meta.author, e.meta.id)).await?;
    let (size, hash) = download(e, &mut zip).await?;
    matches(size, &hash, &e.meta)?;
    send(Event::Progress(e.key(), format!("Checking {}...", e.meta.name)));
    zip.install(root, &e.meta.id, &e.meta.author).await
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
            send(Event::Progress(e.key(), format!("Downloading {}... {} / {} MB ({}%)", e.meta.name, got >> 20, total >> 20, got * 100 / total)));
        }
    }
    Ok((got, hex(&hash.finalize())))
}

/// A download's zip until it's installed: `mods/.download-<author>-<id>.zip` beside the game (B339: one each, as
/// several download at once), removed when dropped.
#[cfg(not(target_arch = "wasm32"))]
struct Zip(std::fs::File, std::path::PathBuf);

#[cfg(not(target_arch = "wasm32"))]
impl Zip {
    async fn create(root: &Path, whose: &str) -> Result<Self, String> {
        let p = root.join(format!(".download-{whose}.zip"));
        let f = std::fs::create_dir_all(root).and_then(|_| std::fs::File::create(&p)).map_err(|e| format!("can't write {}: {e}", p.display()))?;
        Ok(Zip(f, p))
    }
    async fn write(&mut self, b: &[u8]) -> Result<(), String> {
        std::io::Write::write_all(&mut self.0, b).map_err(|e| format!("can't write {}: {e}", self.1.display()))
    }
    async fn install(&self, root: &Path, id: &str, owner: &str) -> Result<Mod, String> {
        modfs::install_file(&self.1, root, Some(id), Some(owner))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Zip {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.1);
    }
}

/// B227: `e`'s `mod.zip` installed from the store a part at a time (HTTP range requests) without its upscaled set
/// (`modfs::upscaled`, ≈ 90% of a store mod), which is never downloaded; on `disk`, else in memory (the browser). Its sha256 needs all of it, so here its size is checked against the store's, and each file's CRC-32
/// as it's unpacked (`modfs::unpack_with`); the connection is HTTPS to the store either way.
async fn ranged(e: &Entry, root: &Path, disk: bool) -> Result<Mod, String> {
    send(Event::Progress(e.key(), format!("Downloading {}...", e.meta.name)));
    let asked = file_url(&e.rev, &format!("mods/{}/mod.zip", e.meta.id));
    let mut r = ask(&asked, 0, 1).await?;
    if r.status() != reqwest::StatusCode::PARTIAL_CONTENT && r.url().as_str() != asked {
        // B242: a phone's fetch can drop the Range header when it follows the store's redirect; ask its CDN directly
        let cdn = r.url().to_string();
        drop(r); // stops the whole zip coming
        r = ask(&cdn, 0, 1).await?;
    }
    let r = partial(r, "the size check")?;
    // the zip's size from "bytes 0-0/<size>"; the rest is asked of the URL the store redirected to (its CDN)
    let len: u64 = r.headers().get("content-range").and_then(|v| v.to_str().ok()?.rsplit_once('/')?.1.parse().ok()).ok_or("the store didn't say the zip's size")?;
    if len != e.meta.size {
        return Err(format!("the download is {len} bytes, the store says {}", e.meta.size));
    }
    if len > MAX_ZIP {
        return Err("too big".into());
    }
    let (url, got) = (r.url().to_string(), std::sync::atomic::AtomicU64::new(0));
    let key = e.key();
    let read = |at: u64, n: usize| {
        let (url, got, name, key) = (url.clone(), &got, &e.meta.name, &key);
        async move {
            if n == 0 {
                return Ok(Vec::new());
            }
            let b = partial(ask(&url, at, n).await?, &format!("the part at {at}"))?.bytes().await.map_err(|e| format!("download cut short: {e}"))?;
            if b.len() != n {
                return Err("download cut short".into());
            }
            let was = got.fetch_add(n as u64, std::sync::atomic::Ordering::Relaxed);
            if (was + n as u64) >> 20 != was >> 20 {
                send(Event::Progress(key.clone(), format!("Downloading {name}... {} MB", (was + n as u64) >> 20)));
            }
            Ok(b.to_vec())
        }
    };
    modfs::install_with(len, read, root, Some(&e.meta.id), Some(&e.meta.author), disk, false).await
}

/// Bytes `at..at + n` of `url` (the store's file, or where it redirected), asked for; `partial` checks they came.
async fn ask(url: &str, at: u64, n: usize) -> Result<reqwest::Response, String> {
    let q = client().get(url).header("Range", format!("bytes={at}-{}", at + n as u64 - 1));
    // B242: never the browser's HTTP cache, which may answer a range with a whole zip it kept
    #[cfg(target_arch = "wasm32")]
    let q = q.fetch_cache_no_store();
    let r = q.send().await.map_err(|e| format!("can't reach the store: {e}"))?;
    r.error_for_status().map_err(|e| format!("the store said {}", e.status().map_or("no".into(), |s| s.to_string())))
}

/// `r` if it's the part asked for (206), else which request (`what`) got what from where, so a report names it.
fn partial(r: reqwest::Response, what: &str) -> Result<reqwest::Response, String> {
    if r.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(format!("the store sent the whole zip, not the part asked for ({what}: {} from {})", r.status().as_u16(), r.url().host_str().unwrap_or("?")));
    }
    Ok(r)
}

/// Desktop Add mod: the system's file picker for a mod `.zip`, installed like a download (`modfs::install_file`'s
/// checks) into `root/<id>/` on disk. The browser build picks through the page instead (`web.rs`).
// ponytail: macOS wants dialogs on the main thread; this runs on a worker (fine on Linux/Windows).
#[cfg(not(any(target_arch = "wasm32", target_os = "ios")))]
pub fn pick(root: &Path) {
    let root = root.to_path_buf();
    spawn(async move {
        #[cfg(not(target_os = "android"))]
        let f = rfd::AsyncFileDialog::new().add_filter("Mod", &["zip"]).set_title("Add mod").pick_file().await.map(|f| f.path().to_path_buf());
        // ponytail: no picker on Android (rfd has none; Android's needs Java/JNI); the mod store's downloads still work
        #[cfg(target_os = "android")]
        let f: Option<std::path::PathBuf> = None;
        let Some(f) = f else {
            return send(Event::Installed(String::new(), Err("no file picked".into())));
        };
        let got = (|| {
            let len = std::fs::metadata(&f).map_err(|e| e.to_string())?.len();
            if len > MAX_ZIP {
                return Err(format!("{len} bytes is bigger than a mod may be"));
            }
            keep(&modfs::install_file(&f, &root, None, None)?)
        })();
        send(Event::Installed(String::new(), got));
    });
}

/// iOS has no picker yet (M91): mods come in through the app's folder in the Files app.
#[cfg(target_os = "ios")]
pub fn pick(_root: &Path) {
    send(Event::Installed(String::new(), Err("put the mod .zip in Hot Shots Tennis's folder in the Files app".into())));
}

/// Keeps an installed mod: natively it's on disk already; in the browser the page writes it to OPFS and says how
/// that went.
fn keep(m: &Mod) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    crate::web::mod_save(&m.folder(), &m.name);
    Ok(m.name.clone())
}

/// Every file of the mod in `dir` (path inside it, bytes), sorted; Install's `store.json` left out.
fn mod_files(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    if !modfs::ready(dir) {
        return Err("reading the mod in from this browser's storage: press again in a moment".into());
    }
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
    // M8f9: a court or umpire goes up with its kind and none of a character's card facts
    let kind = Kind::of(&crate::mods::Kind::of(&j));
    let player = kind == Kind::Character;
    let meta = Meta {
        id: m.id.clone(),
        name: plain_text(&m.name, 40),
        source: plain_text(&m.source, 80),
        author: plain_text(j["author"].as_str().unwrap_or(""), 40),
        version: plain_text(&j["version"].as_str().map_or_else(|| j["version"].to_string(), str::to_string).replace("null", "1"), 16),
        size: zip.len() as u64,
        sha256: sha256(&zip),
        skeleton: j["standard"].as_u64().unwrap_or(1) as u32,
        style: stats.filter(|_| player).map(|s| s.0 as u8),
        grades: stats.filter(|_| player).map(|s| s.1),
        hand: player.then_some(if m.hand < 0.0 { -1 } else { 1 }),
        costumes: player.then_some(m.costumes.len() as u32),
        like: player.then_some(m.donor as u8),
        kind,
        no_upscale: !m.upscale,
    };
    let preview = preview_of(&files, kind);
    Ok(Package { zip, meta, preview })
}

/// M8f9: an upload's `preview.png` among its `files`: an umpire's `previews/umpire.png` (the standard's §3 picture,
/// B400), a court's `previews/court.png`; else (and a character's) its first picture under `previews/`.
fn preview_of(files: &[(String, Vec<u8>)], kind: Kind) -> Option<Vec<u8>> {
    let own = match kind {
        Kind::Umpire => Some("previews/umpire.png"),
        Kind::Court => Some("previews/court.png"),
        _ => None,
    };
    own.and_then(|n| files.iter().find(|f| f.0 == n)).or_else(|| files.iter().find(|f| f.0.starts_with("previews/") && f.0.ends_with(".png"))).map(|f| f.1.clone())
}

/// M8f9/B400: an umpire picture's head square (`mods_umpire::head`) as its own PNG, transparent where the square
/// runs past the picture; None when it isn't a picture of anyone.
fn head_of(png: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?.into_rgba8();
    let [x, y, s, _] = crate::mods_umpire::head(&img)?;
    let (x, y, n) = (x.round() as i64, y.round() as i64, (s.round() as u32).max(1));
    let mut out = image::RgbaImage::new(n, n);
    for (ox, oy, p) in out.enumerate_pixels_mut() {
        let (sx, sy) = (x + ox as i64, y + oy as i64);
        if (0..img.width() as i64).contains(&sx) && (0..img.height() as i64).contains(&sy) {
            *p = *img.get_pixel(sx as u32, sy as u32);
        }
    }
    let mut b = Vec::new();
    out.write_to(&mut std::io::Cursor::new(&mut b), image::ImageFormat::Png).ok()?;
    Some(b)
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

/// One upload: an installed mod, its play style and △ grades as the select shows them, and the select's picture of
/// it (sent when the mod ships no `previews/*.png`).
pub type Upload = (Mod, Option<(usize, [u8; 5])>, Option<Vec<u8>>);

/// B250: uploads `mods` one after another in the background as account `name` ([`Event::Upload`] for each: Uploading,
/// then Done / Skipped / Failed): each is packaged and committed to the store as its own pull request, or, if `name`
/// uploaded that id before, over that upload ([`plan`]).
pub fn publish(mods: Vec<Upload>, name: String, token: String) {
    spawn(async move {
        let mut own = None;
        for (m, stats, card) in mods {
            send(Event::Upload(m.dir.clone(), Up::Uploading));
            let got = upload(&m, stats, card, &name, &token, &mut own).await;
            send(Event::Upload(m.dir.clone(), got.unwrap_or_else(Up::Failed)));
        }
        // the next Browse reads the store again, these uploads included
        let _ = crate::store::write(CACHE, "");
    });
}

/// What uploading a mod does, given the uploader's own copy of its id on the store (`old`, newest first).
#[derive(Debug, PartialEq)]
enum Plan {
    /// A new pull request: never uploaded, or merged (the PR's files replace `mods/<id>/` on `main`).
    New,
    /// A commit onto the uploader's open pull request.
    Onto(u32),
    Skip(String),
}

/// Same zip: skipped; the store's version newer: skipped; else over the open PR, or a new one.
fn plan(old: Option<&Entry>, new: &Meta) -> Plan {
    // "1.10" > "1.9"; a missing version is 1, as `package` writes it
    let v = |s: &str| {
        let mut n: Vec<u64> = s.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).map(|p| p.parse().unwrap_or(u64::MAX)).collect();
        if n.is_empty() {
            n.push(1);
        }
        while n.len() > 1 && n.last() == Some(&0) {
            n.pop();
        }
        n
    };
    match old {
        None => Plan::New,
        Some(o) if o.meta.sha256 == new.sha256 => Plan::Skip("this copy is on the store already".into()),
        Some(o) if v(&o.meta.version) > v(&new.version) => Plan::Skip(format!("the store has a newer version ({})", o.meta.version)),
        Some(o) => pr_of(&o.rev).map_or(Plan::New, Plan::Onto),
    }
}

/// Mod `m` packaged and committed as `name`; `own` is `name`'s uploads, read once per batch.
async fn upload(m: &Mod, stats: Option<(usize, [u8; 5])>, card: Option<Vec<u8>>, name: &str, token: &str, own: &mut Option<Vec<Entry>>) -> Result<Up, String> {
    let mut p = package(m, stats)?;
    // M19: the uploader's account, so My public mods finds it once merged (Browse shows a PR's author anyway)
    p.meta.author = plain_text(name, 40);
    if own.is_none() {
        *own = Some(uploads_of(name).await?);
    }
    let old = own.iter().flatten().find(|e| e.meta.id == m.id);
    let (onto, verb) = match plan(old, &p.meta) {
        Plan::Skip(why) => return Ok(Up::Skipped(format!("{} not uploaded: {why}.", m.name))),
        Plan::New if old.is_some() => (None, "Update"),
        Plan::New => (None, "Add"),
        Plan::Onto(n) => (Some(n), "Update"),
    };
    let dir = format!("mods/{}", m.id);
    let mut files = vec![(format!("{dir}/mod.zip"), p.zip), (format!("{dir}/meta.json"), serde_json::to_vec_pretty(&p.meta).unwrap())];
    if let Some(png) = p.preview.or(card).filter(|b| b.len() as u64 <= MAX_PREVIEW) {
        // B372: Browse's tile picture, the preview small (Browse fetches it first); M8f9/B400: an umpire's is its
        // head, as the umpire select's card has it
        let tile = if p.meta.kind == Kind::Umpire { head_of(&png) } else { None };
        if let Some(t) = shrink(tile.as_deref().unwrap_or(&png), THUMB) {
            files.push((format!("{dir}/thumb.png"), t));
        }
        files.push((format!("{dir}/preview.png"), png));
    }
    let pr = commit(token, &format!("{verb} mod {} ({})", m.name, m.id), files, &[], onto).await?;
    let mb = p.meta.size.div_ceil(1 << 20);
    Ok(Up::Done(match onto {
        Some(_) => format!("{} updated ({mb} MB) on your pull request: {pr}", m.name),
        None if verb == "Update" => format!("{} updated ({mb} MB): live now, and on main once {pr} is merged.", m.name),
        None => format!("{} is up ({mb} MB): it's in everyone's Browse mods now. {pr}", m.name),
    }))
}

/// An HTTP client; natively the TLS provider goes in first (building one without it panics), and on Android its own
/// certificate check (B454, `android::tls`).
pub fn client() -> reqwest::Client {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = rustls::crypto::ring::default_provider().install_default();
    #[cfg(target_os = "android")]
    return reqwest::Client::builder().tls_backend_preconfigured(crate::android::tls()).build().expect("the HTTP client");
    #[cfg(not(target_os = "android"))]
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
/// then commits all of them, and the deletion of the folders `gone`, to a new pull request, or onto open pull request
/// `onto` (B250: its author may commit to `refs/pr/<n>`). Returns the PR's URL.
async fn commit(token: &str, summary: &str, files: Vec<(String, Vec<u8>)>, gone: &[String], onto: Option<u32>) -> Result<String, String> {
    use data_encoding::BASE64;
    use serde_json::json;
    let c = client();
    let api = format!("{HF}/api/datasets/{REPO}");
    let to = onto.map_or("main?create_pr=1".into(), |n| rev_part(&format!("refs/pr/{n}")));
    let ask = json!({ "files": files.iter().map(|(path, b)| json!({ "path": path, "size": b.len(), "sample": BASE64.encode(&b[..b.len().min(512)]) })).collect::<Vec<_>>() });
    // `create_pr`: an account with read access only may still open a PR (403 without it)
    let modes = if files.is_empty() { json!({}) } else { call(c.post(format!("{api}/preupload/{to}")).bearer_auth(token).header("Content-Type", "application/json").body(ask.to_string())).await? };
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
    lines.extend(gone.iter().map(|path| json!({ "key": "deletedFolder", "value": { "path": path } })));
    let body = lines.iter().map(|l| l.to_string() + "\n").collect::<String>();
    let r = call(c.post(format!("{api}/commit/{to}")).bearer_auth(token).header("Content-Type", "application/x-ndjson").body(body)).await?;
    if let Some(n) = onto {
        return Ok(format!("{HF}/datasets/{REPO}/discussions/{n}"));
    }
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
        let m = rt.block_on(ranged(&e, root, false)).unwrap();
        let files = modfs::files_under(&m.dir);
        let mb = files.iter().map(|f| f.1.len()).sum::<usize>() >> 20;
        let fetched = take().into_iter().filter_map(|e| if let Event::Progress(_, p) = e { Some(p) } else { None }).last().unwrap();
        eprintln!("{} files, {mb} MB kept of a {} MB zip; last progress: {fetched}", files.len(), e.meta.size >> 20);
        assert!(files.iter().all(|f| !f.0.contains("/textures/upscaled/")) && files.iter().any(|f| f.0.ends_with(".glb")));
        assert!(mb < 64);
        modfs::remove(Path::new("t_ranged"));
        // the desktop's, with Upscaled Downloads off: the same files, on disk
        let disk = std::env::temp_dir().join(format!("hst-ranged-{}", std::process::id()));
        let m = rt.block_on(ranged(&e, &disk, true)).unwrap();
        assert!(m.dir.join("mod.json").is_file() && !m.dir.join("textures/upscaled").exists() && modfs::files_under(&disk).is_empty());
        std::fs::remove_dir_all(&disk).unwrap();
    }

    use super::*;

    /// Whether `dir` holds a download's zip or stage still (a dot file or folder).
    fn dot(dir: &Path) -> bool {
        std::fs::read_dir(dir).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with('.'))
    }

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

    /// B373: a meta.json without "kind" (every upload before it) is a character's, and a character's is written
    /// without one; the other kinds round-trip, and one this game doesn't know yet still reads (as `Other`).
    #[test]
    fn reads_kind() {
        let ok = meta("lauryn", b"zip");
        let j = serde_json::to_string(&ok).unwrap();
        assert!(!j.contains("kind"));
        assert_eq!(meta_of(j.as_bytes(), "lauryn").unwrap().kind, Kind::Character);
        for (k, word) in [(Kind::Court, "court"), (Kind::Umpire, "umpire"), (Kind::Textures, "textures")] {
            let j = serde_json::to_string(&Meta { kind: k, ..ok.clone() }).unwrap();
            assert!(j.contains(&format!("\"kind\":\"{word}\"")), "{j}");
            assert_eq!(meta_of(j.as_bytes(), "lauryn").unwrap().kind, k);
        }
        let later = j.replacen('{', "{\"kind\":\"stadium\",", 1);
        assert_eq!(meta_of(later.as_bytes(), "lauryn").unwrap().kind, Kind::Other);
    }

    /// `main` wins an id over any PR, a newer PR over an older one.
    #[test]
    fn main_then_newest_pr() {
        let e = |id: &str, rev: &str| Entry { meta: Meta { id: id.into(), name: id.into(), ..Default::default() }, rev: rev.into(), verified: rev == "main" };
        let got = merge(vec![vec![e("a", "main")], vec![e("a", "refs/pr/9"), e("b", "refs/pr/9")], vec![e("b", "refs/pr/3"), e("c", "refs/pr/3")]]);
        let got: Vec<_> = got.iter().map(|e| (e.meta.id.as_str(), e.rev.as_str(), e.verified)).collect();
        assert_eq!(got, [("a", "main", true), ("b", "refs/pr/9", false), ("c", "refs/pr/3", false)]);
        // B248: another author's mod with a listed id is listed too; the same zip under another author isn't
        let by = |id: &str, rev: &str, who: &str, sha: &str| Entry { meta: Meta { author: who.into(), sha256: sha.into(), ..e(id, rev).meta }, ..e(id, rev) };
        let got = merge(vec![vec![by("a", "main", "x", "1")], vec![by("a", "refs/pr/9", "y", "2"), by("a", "refs/pr/8", "z", "1"), by("a", "refs/pr/7", "y", "3")]]);
        assert_eq!(got.iter().map(|e| e.rev.as_str()).collect::<Vec<_>>(), ["main", "refs/pr/9"]);
        // B381: HST's four human chair umpires aren't listed, Robot Tennis is
        let ids = ["hst_umpire_00_chika", "hst_umpire_01_suzuki", "hst_umpire_02_lily", "hst_umpire_03_robottennis", "hst_umpire_04_anna", "getagrip_pc12_suzuki"];
        let got = merge(vec![ids.iter().map(|id| e(id, "main")).collect()]);
        assert_eq!(got.iter().map(|e| e.meta.id.as_str()).collect::<Vec<_>>(), ["getagrip_pc12_suzuki", "hst_umpire_03_robottennis"]);
    }

    /// B250: a re-upload by its uploader goes onto their open PR, or a new PR once merged; the same zip, or an older
    /// version than the store's, is skipped.
    #[test]
    fn re_upload_plan() {
        let new = Meta { id: "a".into(), version: "1.10".into(), sha256: "1".repeat(64), ..Default::default() };
        let old = |rev: &str, version: &str, sha: &str| Entry { meta: Meta { version: version.into(), sha256: sha.repeat(64), ..new.clone() }, rev: rev.into(), verified: rev == "main" };
        assert_eq!(plan(None, &new), Plan::New);
        assert_eq!(plan(Some(&old("refs/pr/4", "1.9", "2")), &new), Plan::Onto(4));
        assert_eq!(plan(Some(&old("main", "1", "2")), &new), Plan::New);
        // same version, another zip (a fix not bumped): still goes up
        assert_eq!(plan(Some(&old("refs/pr/4", "1.10.0", "2")), &new), Plan::Onto(4));
        assert!(matches!(plan(Some(&old("refs/pr/4", "1.10", "1")), &new), Plan::Skip(w) if w.contains("already")));
        assert!(matches!(plan(Some(&old("main", "2", "2")), &new), Plan::Skip(w) if w.contains("newer")));
        assert!(matches!(plan(Some(&old("main", "", "2")), &Meta { version: "".into(), ..new.clone() }), Plan::New));
    }

    /// Upload → Install round trip: a real mod packaged, its zip checked against its meta (a flipped byte or a
    /// short download refused), installed in memory with its stamp; a zip whose mod isn't the listed id is refused
    /// and leaves nothing.
    #[test]
    fn package_then_install() {
        let mods = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods");
        // the first character mod (court and umpire mods share the folder)
        let Some(dir) = std::fs::read_dir(mods).ok().and_then(|d| d.flatten().map(|e| e.path()).filter(|p| crate::mods::read(p).is_ok()).min()) else {
            return eprintln!("no mods in {mods} (skipped)");
        };
        let m = crate::mods::read(&dir).unwrap();
        let p = package(&m, Some((2, [1, 2, 3, 4, 5]))).unwrap();
        eprintln!("packed {} ({} bytes, {} files)", m.id, p.meta.size, mod_files(&m.dir).unwrap().len());
        assert_eq!(p.meta.id, m.id);
        // B488: a mod that skips upscaling says so in its meta (Install then never takes an upscaled set); a meta
        // without it, as every upload before, wants it
        let hd = package(&crate::mods::Mod { upscale: false, ..m.clone() }, None).unwrap().meta;
        assert!(hd.no_upscale && p.meta.no_upscale == !m.upscale);
        assert!(serde_json::from_slice::<Meta>(&serde_json::to_vec(&hd).unwrap()).unwrap().no_upscale);
        assert!(!serde_json::from_str::<Meta>(r#"{"id":"x","name":"X","size":1,"sha256":"00"}"#).unwrap().no_upscale);
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
                let mut z = Zip::create(&disk, "-x").await.unwrap();
                for c in p.zip.chunks(64 << 10) {
                    z.write(c).await.unwrap();
                }
                assert!(z.install(&disk, "someone_else", "").await.unwrap_err().contains("not `someone_else`"));
                assert!(disk.join(&m.id).join("old").exists() && !disk.join(".import--someone_else").exists());
                z.install(&disk, &m.id, "").await
            })
            .unwrap();
        assert!(!dot(&disk));
        assert!(!disk.join(&m.id).join("old").exists());
        assert!(modfs::files_under(&disk).is_empty());
        assert_eq!(crate::mods::read(&got.dir).unwrap().id, m.id);
        let s = Stamp { sha256: p.meta.sha256.clone(), rev: "refs/pr/1".into(), verified: false, author: String::new() };
        modfs::write(&got.dir.join("store.json"), serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(stamp(&got.dir).unwrap().rev, "refs/pr/1");
        // the stamp stays out of a re-upload's zip
        assert!(!mod_files(&got.dir).unwrap().iter().any(|f| f.0 == "store.json"));
        assert_eq!(mod_files(&got.dir).unwrap().len(), mod_files(&m.dir).unwrap().len());
        std::fs::remove_dir_all(&disk).unwrap();
    }

    /// M8f9: an umpire mod (made here) and a court mod (the mods repo's smallest, when linked in) package with their
    /// kind and none of a character's card facts, their own picture as the preview (an umpire's tile its head
    /// square), then install from the zip as any mod does and list as their kind (what the court / umpire select
    /// reads after Reload).
    #[test]
    fn courts_and_umpires_go_up_and_come_down() {
        let base = std::env::temp_dir().join(format!("hst-store-kinds-{}", std::process::id()));
        let ump = base.join("src/gag_ump09");
        std::fs::create_dir_all(ump.join("previews")).unwrap();
        std::fs::write(ump.join("mod.json"), r#"{"standard":1,"kind":"umpire","id":"gag_ump09","name":"Nine","source":"Hot Shots Tennis: Get a Grip (PSP)","donor":2,"version":"1.1"}"#).unwrap();
        // a 40×100 figure on a 60×120 picture, its head 16 wide on top
        let mut img = image::RgbaImage::new(60, 120);
        for (x, y, px) in img.enumerate_pixels_mut() {
            if (10..30).contains(&y) && (22..38).contains(&x) || (30..110).contains(&y) && (10..50).contains(&x) {
                *px = image::Rgba([200, 90, 40, 255]);
            }
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        std::fs::write(ump.join("previews/umpire.png"), &png).unwrap();
        std::fs::write(ump.join("previews/other.png"), &png[..]).unwrap();
        let mut srcs = vec![(ump, crate::mods::Kind::Umpire, Some("previews/umpire.png"))];
        let hst = concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/hst-mods");
        let court = std::fs::read_dir(hst).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| crate::mods::Kind::of_dir(p) == crate::mods::Kind::Court).min_by_key(|p| walk_len(p));
        match court {
            Some(c) => srcs.push((c, crate::mods::Kind::Court, Some("previews/court.png"))),
            None => eprintln!("no court mods in {hst} (the court half skipped)"),
        }
        for (src, kind, pic) in srcs {
            let m = crate::mods::read_any(&src).unwrap();
            let p = package(&m, Some((2, [1, 2, 3, 4, 5]))).unwrap();
            assert_eq!(p.meta.kind, Kind::of(&kind));
            assert_eq!((p.meta.style, p.meta.grades, p.meta.hand, p.meta.costumes, p.meta.like), (None, None, None, None, None), "{}", m.id);
            let j = serde_json::to_string(&p.meta).unwrap();
            assert_eq!(meta_of(j.as_bytes(), &m.id).unwrap(), p.meta);
            if let Some(f) = pic.filter(|f| src.join(f).is_file()) {
                assert_eq!(p.preview.as_deref(), Some(&std::fs::read(src.join(f)).unwrap()[..]), "{}", m.id);
            }
            let root = base.join("mods");
            let got = modfs::install(&p.zip, &root).unwrap();
            assert_eq!((got.id.as_str(), crate::mods::Kind::of_dir(&got.dir)), (m.id.as_str(), kind.clone()));
            match kind {
                crate::mods::Kind::Umpire => assert_eq!(crate::mods_umpire::list(&root).iter().map(|u| (u.id.as_str(), u.donor)).collect::<Vec<_>>(), [("gag_ump09", 2)]),
                _ => assert_eq!(crate::mods_court::list(&root).iter().map(|c| c.id.clone()).collect::<Vec<_>>(), [m.id.clone()]),
            }
            // not a character: the roster leaves it out
            assert!(crate::mods::list(&root).is_empty());
            assert_eq!(crate::mods::list_kind(&root, &kind).len(), 1);
        }
        // the umpire's tile: its head's square (0.36 of 100 tall, from just over its top), transparent past the top
        let head = image::load_from_memory(&head_of(&png).unwrap()).unwrap().into_rgba8();
        assert_eq!(head.dimensions(), (36, 36));
        assert_eq!((head.get_pixel(18, 0)[3], head.get_pixel(18, 10)[3], head.get_pixel(1, 10)[3]), (0, 255, 0));
        let _ = std::fs::remove_dir_all(&base);
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
    /// peak memory printed: it should stay well under the mod's size. `HST_LIVE_ROOT=<dir>` installs there and keeps it
    /// (M17: then play it with `--slot-mod 0 <dir>/<id>`).
    #[test]
    fn live_install() {
        let Ok(id) = std::env::var("HST_LIVE_INSTALL") else { return eprintln!("HST_LIVE_INSTALL unset (skipped)") };
        list(true);
        let keep = std::env::var("HST_LIVE_ROOT").ok();
        let root = keep.clone().map_or_else(|| std::env::temp_dir().join(format!("hst-live-{}", std::process::id())), Into::into);
        let started = std::time::Instant::now();
        loop {
            for ev in take() {
                match ev {
                    Event::Listing(r) => install(r.unwrap().iter().find(|e| e.meta.id == id).expect("not in the store"), &root, true),
                    Event::Progress(_, s) if s.ends_with("0%)") || !s.contains('%') => eprintln!("{:.1}s {s}", started.elapsed().as_secs_f32()),
                    Event::Installed(_, r) => {
                        eprintln!("{:.1}s {r:?}", started.elapsed().as_secs_f32());
                        let n = walk_len(&root.join(&id));
                        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
                        eprintln!("{n} files; {}", status.lines().find(|l| l.starts_with("VmHWM")).unwrap_or("no VmHWM"));
                        if keep.is_none() {
                            std::fs::remove_dir_all(&root).unwrap();
                        }
                        return assert!(r.is_ok() && n > 0);
                    }
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    /// M19 against the real store (`HST_LIVE_MINE=<account>`): that account's uploads, read only.
    #[test]
    fn live_mine() {
        let Ok(name) = std::env::var("HST_LIVE_MINE") else { return eprintln!("HST_LIVE_MINE unset (skipped)") };
        mine(name.clone());
        for _ in 0..600 {
            if let Some(Event::Mine(r)) = take().into_iter().next() {
                let r = r.unwrap();
                eprintln!("{name}: {:?}", r.iter().map(|e| (&e.meta.id, &e.rev, &e.meta.author)).collect::<Vec<_>>());
                return assert!(r.iter().all(|e| e.meta.author.eq_ignore_ascii_case(&name)));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        panic!("no answer in 60 s");
    }

    /// B339: the store's `HST_LIVE_PARALLEL` smallest mods (a number) installed side by side into one folder, every
    /// other one whole (Upscaled Downloads on), the rest a part at a time: each says its own progress and outcome by key, each lands in its own folder, and no
    /// download zip or stage is left. Network: `HST_LIVE_PARALLEL=3 tools/check.sh -p hst live_parallel` after
    /// touching `install` or `modfs::install_with`'s stage.
    #[test]
    fn live_parallel() {
        let Some(n) = std::env::var("HST_LIVE_PARALLEL").ok().and_then(|n| n.parse::<usize>().ok()) else { return eprintln!("HST_LIVE_PARALLEL unset (skipped)") };
        let root = std::env::temp_dir().join(format!("hst-parallel-{}", std::process::id()));
        list(true);
        let (started, mut want, mut seen) = (std::time::Instant::now(), Vec::new(), std::collections::HashSet::new());
        let mut done = Vec::new();
        while started.elapsed().as_secs() < 600 {
            for ev in take() {
                match ev {
                    Event::Listing(r) => {
                        let mut l = r.unwrap();
                        l.sort_by_key(|e| e.meta.size);
                        l.dedup_by(|a, b| a.meta.id == b.meta.id);
                        for (i, e) in l.iter().take(n).enumerate() {
                            eprintln!("installing {} ({} MB)", e.key(), e.meta.size >> 20);
                            // the whole zip (each to its own file) and the ranged one, side by side
                            install(e, &root, i % 2 == 1);
                            want.push(e.key());
                        }
                    }
                    Event::Progress(k, _) => _ = seen.insert(k),
                    Event::Installed(k, r) => {
                        eprintln!("{:.1}s {k}: {r:?}", started.elapsed().as_secs_f32());
                        done.push((k, r));
                    }
                    _ => {}
                }
            }
            if !want.is_empty() && done.len() == want.len() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let dirs = crate::mods::list(&root).len();
        let left = dot(&root);
        std::fs::remove_dir_all(&root).unwrap();
        assert!(done.iter().all(|d| d.1.is_ok() && want.contains(&d.0)) && done.len() == n, "{done:?}");
        assert!(want.iter().all(|k| seen.contains(k)) && dirs == n && !left);
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
        assert_eq!(shot, Some((e.key(), None)));
    }
}
