// Windows: hst.exe's icon (icon/hst.ico, from web/icon.svg) as a resource, compiled with MinGW's windres (tools/build.sh).
fn main() {
    println!("cargo:rerun-if-changed=icon/hst.ico");
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
