//! mdlstat <dir> [obj_out model.mdl]  — parse every .MDL under dir; optionally dump one as Wavefront OBJ.
use std::{fmt::Write as _, path::Path};

fn walk(dir: &Path, f: &mut dyn FnMut(&Path)) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() { walk(&p, f) } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("mdl")) { f(&p) }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() == 4 {
        let m = hst_data::mdl::parse(&std::fs::read(&a[3]).unwrap()).unwrap();
        let mut s = String::new();
        let mut base = 1;
        for (mi, pks) in m.materials.iter().enumerate() {
            writeln!(s, "g mat{mi}").unwrap();
            for pk in pks {
                for v in &pk.vertices { writeln!(s, "v {} {} {}\nvt {} {}", v.pos[0], v.pos[1], v.pos[2], v.uv[0], 1.0 - v.uv[1]).unwrap(); }
                for t in &pk.triangles { writeln!(s, "f {0}/{0} {1}/{1} {2}/{2}", t[0] + base, t[1] + base, t[2] + base).unwrap(); }
                base += pk.vertices.len() as u32;
            }
        }
        std::fs::write(&a[2], s).unwrap();
        return;
    }
    let (mut ok, mut bad, mut v, mut t) = (0, 0, 0, 0);
    walk(Path::new(&a[1]), &mut |p| match hst_data::mdl::parse(&std::fs::read(p).unwrap()) {
        Ok(m) => {
            ok += 1;
            for pk in m.materials.iter().flatten() { v += pk.vertices.len(); t += pk.triangles.len(); }
        }
        Err(e) => { bad += 1; if bad < 10 { println!("FAIL {}: {e}", p.display()) } }
    });
    println!("{ok} ok, {bad} failed, {v} vertices, {t} triangles");
}
