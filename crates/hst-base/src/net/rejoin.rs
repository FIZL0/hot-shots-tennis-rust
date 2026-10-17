//! M126: the online match this machine plays in as a guest, kept so a browser refresh or a crash can rejoin it: the
//! lobby's code and this player's first seat, written when the match starts (`main_menu/online.rs` `launch`), read once
//! by a fresh menu (it goes straight to joining that code, as a lobby link does) and gone once the match is over. In
//! the browser it's the tab's sessionStorage (a reload keeps it, a closed tab doesn't); natively `rejoin.txt` beside
//! settings.txt. A record older than [`FRESH`] is ignored: the match has long given the seat to a bot or ended.
//! The host can't rejoin (there's no new host: `play/recover.rs`), so it never writes one.

/// How long after it was written a record still rejoins (the others wait ~2 minutes for a silent player, 5 for one
/// that switched away).
pub const FRESH: f64 = 600.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Rejoin {
    /// The code the match's lobby was joined with (a short code or a ticket: `net::code_in`'s).
    pub code: String,
    /// This player's first seat (0–3) in the lobby's line-up.
    pub seat: u8,
}

const KEY: &str = "hst-rejoin";

fn now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    return js_sys::Date::now() / 1000.0;
    #[cfg(not(target_arch = "wasm32"))]
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

/// `code seat seconds` in one line.
fn encode(r: &Rejoin, at: f64) -> String {
    format!("{} {} {}", r.code, r.seat, at as u64)
}

fn decode(text: &str, now: f64) -> Option<Rejoin> {
    let mut f = text.split_whitespace();
    let (code, seat, at) = (f.next()?, f.next()?.parse::<u8>().ok()?, f.next()?.parse::<f64>().ok()?);
    (seat < 4 && !code.is_empty() && (0.0..FRESH).contains(&(now - at))).then(|| Rejoin { code: code.into(), seat })
}

/// Remember `r` (the menu's `iso`: natively the record is beside its settings).
pub fn save(iso: &str, r: &Rejoin) {
    write(iso, Some(&encode(r, now())));
}

/// The match to rejoin, if one was saved lately; read once (it's forgotten here, and saved again when the rejoin
/// starts).
pub fn take(iso: &str) -> Option<Rejoin> {
    let r = read(iso).and_then(|t| decode(&t, now()));
    forget(iso);
    r
}

/// The match is over (or was never this machine's to rejoin).
pub fn forget(iso: &str) {
    write(iso, None);
}

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.session_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
fn read(_iso: &str) -> Option<String> {
    storage()?.get_item(KEY).ok()?
}

#[cfg(target_arch = "wasm32")]
fn write(_iso: &str, text: Option<&str>) {
    if let Some(s) = storage() {
        let _ = match text {
            Some(t) => s.set_item(KEY, t),
            None => s.remove_item(KEY),
        };
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn path(iso: &str) -> std::path::PathBuf {
    crate::store::settings(iso).with_file_name(format!("{}.txt", KEY.trim_start_matches("hst-")))
}

#[cfg(not(target_arch = "wasm32"))]
fn read(iso: &str) -> Option<String> {
    std::fs::read_to_string(path(iso)).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn write(iso: &str, text: Option<&str>) {
    let p = path(iso);
    let _ = match text {
        Some(t) => std::fs::write(p, t),
        None if p.exists() => std::fs::remove_file(p),
        None => Ok(()),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_rejoins_while_fresh() {
        let r = Rejoin { code: "AB3Q".into(), seat: 2 };
        let t = encode(&r, 1000.0);
        assert_eq!(decode(&t, 1000.0), Some(r.clone()));
        assert_eq!(decode(&t, 1000.0 + FRESH - 1.0), Some(r));
        assert_eq!(decode(&t, 1000.0 + FRESH), None, "too old");
        assert_eq!(decode(&t, 900.0), None, "from the future (a clock change)");
        assert_eq!(decode("AB3Q 7 1000", 1000.0), None, "no such seat");
        assert_eq!(decode("", 1000.0), None);
    }

    #[test]
    fn a_record_is_read_once() {
        let dir = std::env::temp_dir().join(format!("hst-rejoin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iso = dir.join("hst.iso").display().to_string();
        let r = Rejoin { code: "endpointabc".into(), seat: 1 };
        save(&iso, &r);
        assert_eq!(take(&iso), Some(r.clone()));
        assert_eq!(take(&iso), None, "taken");
        save(&iso, &r);
        forget(&iso);
        assert_eq!(take(&iso), None, "forgotten");
        let _ = std::fs::remove_dir(&dir);
    }
}
