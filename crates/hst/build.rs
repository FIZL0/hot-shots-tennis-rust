// Windows: hst.exe's icon (icon/hst.ico, from web/icon.svg) as a resource, compiled with MinGW's windres (tools/build.sh).
// B302: the git commit for the main menu's version corner (HST_COMMIT), "unknown" outside a git checkout.
fn main() {
    println!("cargo:rerun-if-changed=icon/hst.ico");
    let git = |args: &[&str]| {
        std::process::Command::new("git").args(args).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    for f in ["HEAD", "logs/HEAD"] {
        if let Some(p) = git(&["rev-parse", "--git-path", f]) {
            println!("cargo:rerun-if-changed={p}");
        }
    }
    let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    let commit = git(&["rev-parse", "--short=7", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=HST_COMMIT={commit}{}", if dirty { "+" } else { "" });
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = format!("{}/hst-icon.o", std::env::var("OUT_DIR").unwrap());
    let ok = std::process::Command::new("x86_64-w64-mingw32-windres")
        .args(["--include-dir", "icon", "icon/hst.rc", "-O", "coff", "-o", &out])
        .status()
        .is_ok_and(|s| s.success());
    if ok {
        println!("cargo:rustc-link-arg-bins={out}");
    } else {
        println!("cargo:warning=no x86_64-w64-mingw32-windres: hst.exe has no icon");
    }
}
