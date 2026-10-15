//! Disc import: the first start copies what the remaster reads out of the user's ISO into a pack in the user data dir
//! (`hst_data::iso::Iso::write_pack`); from then on the game runs from the pack and the ISO can go. settings.txt,
//! mods/ and cache/ live beside the pack, as they do beside an ISO.
//!
//! `hst [flags]` runs from the pack (importing first if there is none: a file dialog asks for the ISO);
//! `hst --import <iso> [flags]` (re)imports; `hst <iso or pack> [flags]` runs straight from that file, as before.

use hst_data::iso::Iso;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `$XDG_DATA_HOME/hst` (else `~/.local/share/hst`); `%APPDATA%\hst` on Windows; the app's Documents on iOS (the
/// folder the Files app shows, so the ISO, settings.txt and mods/ go in and out there).
pub(crate) fn data_dir() -> PathBuf {
    let env = |k| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(target_os = "ios") {
        return env("HOME").unwrap_or_default().join("Documents");
    }
    if cfg!(windows) {
        return env("APPDATA").unwrap_or_default().join("hst");
    }
    env("XDG_DATA_HOME").unwrap_or_else(|| env("HOME").unwrap_or_default().join(".local/share")).join("hst")
}

pub fn pack_path() -> PathBuf {
    data_dir().join("disc.pack")
}

/// Puts the disc to run from in front of `argv` (see the module doc).
pub fn resolve(argv: &mut Vec<String>) {
    // iOS (M91): no dialogs, and a launch that takes too long gets killed, so no import before the window: the pack if
    // there is one, else straight from the first ISO in the app's folder
    #[cfg(target_os = "ios")]
    if argv.is_empty() {
        let iso = std::fs::read_dir(data_dir()).into_iter().flatten().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("iso")));
        let disc = Some(pack_path()).filter(|p| p.exists()).or(iso).unwrap_or_else(|| crate::ios::no_disc());
        argv.push(disc.to_string_lossy().into_owned());
        return;
    }
    let import = argv.first().is_some_and(|a| a == "--import");
    if import {
        argv.remove(0);
    }
    let iso = if import && argv.first().is_some_and(|a| !a.starts_with("--")) { Some(argv.remove(0)) } else { None };
    if !import && argv.first().is_some_and(|a| !a.starts_with("--")) {
        // B94: a path that won't open (a launcher's placeholder) used to panic in the startup systems with the window
        // already up; fall back to the pack / picker instead
        if Iso::open(&argv[0]).is_ok() {
            return;
        }
        eprintln!("Couldn't open the disc image {}: using the imported disc", argv.remove(0));
    }
    let pack = pack_path();
    if import || !pack.exists() {
        let iso = iso.or_else(pick).unwrap_or_else(|| fail("No disc image picked. Run: hst --import <your Hot Shots Tennis (USA) ISO>"));
        if let Err(e) = import_iso(Path::new(&iso), &pack) {
            fail(&format!("Couldn't import {iso}: {e}"));
        }
    }
    argv.insert(0, pack.to_string_lossy().into_owned());
}

fn import_iso(iso: &Path, pack: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(pack.parent().unwrap())?;
    let mut disc = Iso::open(iso)?;
    let mut last = 0;
    let hash = disc.write_pack(pack, |i, n| {
        if i * 10 / n != last {
            last = i * 10 / n;
            eprintln!("importing the disc: {}%", last * 10);
        }
    })?;
    // the settings beside the old ISO come along (mods/ stay where they are: they can be large)
    let (old, new) = (iso.with_file_name("settings.txt"), pack.with_file_name("settings.txt"));
    if old.exists() && !new.exists() {
        std::fs::copy(old, new)?;
    }
    let mb = |p: &Path| std::fs::metadata(p).map_or(0, |m| m.len() >> 20);
    tell(&format!(
        "Imported the disc into {} ({} MB of the {} MB image, hash {hash:016x}).\nThe game no longer needs the ISO: you can move or delete it.",
        pack.display(),
        mb(pack),
        mb(iso)
    ));
    Ok(())
}

/// The ISO from a file dialog (zenity or kdialog), if one is installed and the user picked a file.
fn pick() -> Option<String> {
    let title = "Hot Shots Tennis: pick your disc image (SCUS-97610)";
    let tries: [(&str, &[&str]); 2] = [
        ("zenity", &["--file-selection", "--title", title, "--file-filter", "Disc image | *.iso *.ISO", "--file-filter", "All files | *"]),
        ("kdialog", &["--title", title, "--getopenfilename", ".", "*.iso *.ISO"]),
    ];
    tries.iter().find_map(|(cmd, args)| {
        let out = Command::new(cmd).args(*args).output().ok().filter(|o| o.status.success())?;
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string()).filter(|s| !s.is_empty())
    })
}

/// A message on stderr, and in a dialog when there is one (a launcher start has no terminal).
fn tell(msg: &str) {
    eprintln!("{msg}");
    if std::env::var_os("HST_BG").is_none() {
        let _ = Command::new("zenity").args(["--info", "--no-wrap", "--text", msg]).status();
    }
}

fn fail(msg: &str) -> ! {
    tell(msg);
    std::process::exit(1)
}
