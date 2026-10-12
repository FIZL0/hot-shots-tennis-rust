//! Same floats on every platform: `state_hash` (a recorded bot match, `context/fixtures/camera_s05.bin`, through the
//! match camera, plus the sim's transcendentals) built for x86_64, aarch64 and wasm32 prints the same per-tick
//! hashes. aarch64 is built with `cargo zigbuild` and run through qemu's binfmt; wasm32 runs under `wasmtime` (PATH
//! or `context/tools/wasmtime-*/`). Skipped when the fixture, a target or a runner is missing.

use std::path::{Path, PathBuf};
use std::process::Command;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// Build `state_hash` for `target` (None = host) in its own target dir (the outer `cargo test` holds the main one).
fn build(target: Option<&str>, dir: &Path) -> Option<PathBuf> {
    let cargo = std::env::var("CARGO").unwrap_or("cargo".into());
    let zig = target.is_some_and(|t| t.contains("linux"));
    let mut c = Command::new(&cargo);
    c.args([if zig { "zigbuild" } else { "build" }, "-q", "--release", "-p", "hst-sim", "--bin", "state_hash"]);
    c.arg("--target-dir").arg(dir).current_dir(ROOT);
    if let Some(t) = target {
        c.args(["--target", t]);
    }
    if !c.status().ok()?.success() {
        return None;
    }
    let sub = target.map_or("release".into(), |t| format!("{t}/release"));
    let exe = dir.join(sub).join(if target.is_some_and(|t| t.starts_with("wasm")) { "state_hash.wasm" } else { "state_hash" });
    exe.exists().then_some(exe)
}

fn run(mut c: Command) -> Option<String> {
    let out = c.current_dir(ROOT).output().ok()?;
    out.status.success().then(|| String::from_utf8(out.stdout).unwrap())
}

fn wasmtime() -> Option<PathBuf> {
    if Command::new("wasmtime").arg("--version").output().is_ok_and(|o| o.status.success()) {
        return Some("wasmtime".into());
    }
    let tools = std::fs::read_dir(Path::new(ROOT).join("context/tools")).ok()?;
    tools.flatten().map(|e| e.path().join("wasmtime")).find(|p| p.exists())
}

#[test]
fn same_state_hash_on_x86_64_aarch64_wasm32() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| format!("{ROOT}/context/fixtures"));
    let fixture = format!("{dir}/camera_s05.bin");
    if !Path::new(&fixture).exists() {
        return eprintln!("camera_s05.bin absent, skipped");
    }
    let tdir = PathBuf::from(std::env::var("CARGO_TARGET_DIR").unwrap_or(format!("{ROOT}/target"))).join("platforms");
    let host = build(None, &tdir).expect("host build of state_hash");
    let want = run({
        let mut c = Command::new(host);
        c.arg(&fixture);
        c
    })
    .expect("host run");
    assert!(want.lines().count() > 5000, "the whole match replayed");
    let mut checked = vec!["host"];
    match build(Some("aarch64-unknown-linux-musl"), &tdir).and_then(|exe| {
        let mut c = Command::new(exe);
        c.arg(&fixture);
        run(c)
    }) {
        Some(got) => {
            assert!(got == want, "aarch64 differs: {:?}", got.lines().zip(want.lines()).find(|(a, b)| a != b));
            checked.push("aarch64");
        }
        None => eprintln!("aarch64: no zigbuild/target/qemu, skipped"),
    }
    match (build(Some("wasm32-wasip1"), &tdir), wasmtime()) {
        (Some(wasm), Some(wt)) => {
            let mut c = Command::new(wt);
            c.args(["run", "--dir", &dir]).arg(wasm).arg(&fixture);
            let got = run(c).expect("wasmtime run");
            assert!(got == want, "wasm32 differs: {:?}", got.lines().zip(want.lines()).find(|(a, b)| a != b));
            checked.push("wasm32");
        }
        _ => eprintln!("wasm32: no target/wasmtime, skipped"),
    }
    eprintln!("same {} lines on {checked:?}", want.lines().count());
}
