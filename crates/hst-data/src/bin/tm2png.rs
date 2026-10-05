//! tm2png <dir>  — convert every .tm2 (and .MTL/.MTI texture set) under dir to .png next to it.
use std::path::Path;

fn save(out: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut enc = png::Encoder::new(std::fs::File::create(out)?, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header()?.write_image_data(rgba)?;
    Ok(())
}

fn walk(dir: &Path, n: &mut [usize; 2]) -> Result<(), Box<dyn std::error::Error>> {
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            walk(&p, n)?;
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("mtl")) {
            let mti = ["MTI", "mti", "Mti"].iter().map(|e| p.with_extension(e)).find(|m| m.exists());
            let mti = mti.map(std::fs::read).transpose()?;
            match hst_data::mtl::parse(&std::fs::read(&p)?, mti.as_deref()) {
                Ok(m) => {
                    for (i, t) in m.textures.iter().enumerate() {
                        save(&p.with_extension(format!("tex{i}.png")), t.width, t.height, &t.rgba)?;
                    }
                    n[0] += 1;
                }
                Err(err) => { println!("FAIL {}: {err}", p.display()); n[1] += 1; }
            }
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("tm2")) {
            match hst_data::tim2::decode(&std::fs::read(&p)?) {
                Ok(pics) => {
                    for (i, pic) in pics.iter().enumerate() {
                        let out = if i == 0 { p.with_extension("png") } else { p.with_extension(format!("{i}.png")) };
                        save(&out, pic.width, pic.height, &pic.rgba)?;
                    }
                    n[0] += 1;
                }
                Err(err) => { println!("FAIL {}: {err}", p.display()); n[1] += 1; }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut n = [0; 2];
    walk(Path::new(&std::env::args().nth(1).ok_or("usage: tm2png <dir>")?), &mut n)?;
    println!("{} ok, {} failed", n[0], n[1]);
    Ok(())
}
