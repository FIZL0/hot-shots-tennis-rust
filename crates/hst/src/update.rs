//! B317: a newer build is out. The game never updates or reloads by itself: the main menu shows a notice (`home.rs`)
//! whose button refreshes the page (browser) or opens the release page to download it (desktop).
//!
//! Browser: `web/index.html` reads the site's `version.txt` (`tools/web.sh` writes the commit there) when the page
//! loads and again every few minutes; once the site's differs from the one this page loaded with, it sets
//! `hstLatest`. Desktop: builds made by GitHub Actions (`pages.yml`, the rolling "Latest alpha") ask GitHub which
//! commit the alpha release is at, at start and every half hour; a local build never asks (its commit is never the
//! alpha's). `HST_LATEST=<commit>` in the environment fakes a newer one (the browser: set `hstLatest` in the console).

/// The release page the desktop notice's Download opens.
pub const RELEASES: &str = "https://github.com/FIZL0/hot-shots-tennis-rust/releases/tag/alpha";

/// The newer build's commit (short), if one is out.
#[cfg(target_arch = "wasm32")]
pub fn latest() -> Option<String> {
    js_sys::Reflect::get(&web_sys::window()?.into(), &"hstLatest".into()).ok()?.as_string().filter(|s| !s.is_empty())
}

/// The newer build's commit (short), if one is out.
#[cfg(not(target_arch = "wasm32"))]
pub fn latest() -> Option<String> {
    use std::sync::{Mutex, Once};
    static LATEST: Mutex<Option<String>> = Mutex::new(None);
    static START: Once = Once::new();
    START.call_once(|| {
        if cfg!(test) {
            return;
        }
        if let Some(fake) = std::env::var("HST_LATEST").ok().filter(|s| !s.is_empty()) {
            *LATEST.lock().unwrap() = Some(fake);
            return;
        }
        // this build's full commit, only in an Actions build
        let Some(ours) = option_env!("GITHUB_SHA") else { return };
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            loop {
                match rt.block_on(alpha()) {
                    Ok(sha) if sha != ours => *LATEST.lock().unwrap() = Some(sha[..7.min(sha.len())].to_string()),
                    Ok(_) => {}
                    Err(e) => bevy::log::info!("update check: {e}"),
                }
                std::thread::sleep(std::time::Duration::from_secs(30 * 60));
            }
        });
    });
    LATEST.lock().unwrap().clone()
}

/// The commit the "Latest alpha" release was built from (GitHub's API asks for a User-Agent).
#[cfg(not(target_arch = "wasm32"))]
async fn alpha() -> Result<String, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let r = reqwest::Client::new()
        .get("https://api.github.com/repos/FIZL0/hot-shots-tennis-rust/releases/tags/alpha")
        .header("User-Agent", "hot-shots-tennis")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(r.status().to_string());
    }
    let j: serde_json::Value = serde_json::from_slice(&r.bytes().await.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    // `gh release create --target $GITHUB_SHA`: the full commit
    j["target_commitish"].as_str().filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())).map(str::to_string).ok_or_else(|| "no commit".into())
}

/// The notice's button: the page reloads (once its mod saves are done, `web.rs`) into the new build, which the
/// service worker fetches first (`web/sw.js`), or the release page opens in the browser.
pub fn act() {
    #[cfg(target_arch = "wasm32")]
    crate::web::reload();
    #[cfg(not(target_arch = "wasm32"))]
    crate::modstore::open(RELEASES);
}

/// The button's word.
pub const BUTTON: &str = if cfg!(target_arch = "wasm32") { "Refresh" } else { "Download" };
