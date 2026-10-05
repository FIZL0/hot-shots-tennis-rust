//! xbdump <iso> [out_dir]  — decode every .XB/.XB0 on the disc; with out_dir, extract them.
use hst_data::{iso::Iso, xb::Archive};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut iso = Iso::open(args.next().ok_or("usage: xbdump <iso> [out_dir]")?)?;
    let out = args.next().map(PathBuf::from);
    let paths: Vec<String> = iso.files.keys().filter(|p| p.ends_with(".XB") || p.ends_with(".XB0")).cloned().collect();
    let (mut ok, mut bad) = (0, 0);
    for path in &paths {
        let data = iso.read(path)?;
        let arc = match Archive::parse(&data) {
            Ok(a) => a,
            Err(e) => { println!("FAIL {path}: {e}"); bad += 1; continue; }
        };
        for e in &arc.entries {
            match arc.read(e) {
                Ok(bytes) => {
                    ok += 1;
                    if let Some(dir) = &out {
                        // names look like `..\\data\\x.MDL`; keep only normal components so nothing escapes `dir`
                        let rel: PathBuf = e.name.split(['\\', '/']).filter(|c| !matches!(*c, "" | "." | "..")).collect();
                        let dst = dir.join(path).join(rel);
                        std::fs::create_dir_all(dst.parent().unwrap())?;
                        std::fs::write(dst, bytes)?;
                    }
                }
                Err(err) => { println!("FAIL {path}:{} kind={} {err}", e.name, e.kind); bad += 1; }
            }
        }
    }
    println!("{} archives, {ok} entries ok, {bad} failed", paths.len());
    Ok(())
}
