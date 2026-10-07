//! texnames <iso> <replacements_dir> [-v]  — match every disc texture to a PCSX2 replacement pack by name and
//! report what the pack covers (per archive with -v).
use std::collections::{BTreeMap, HashSet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut iso = hst_data::iso::Iso::open(
        args.next()
            .ok_or("usage: texnames <iso> <replacements_dir> [-v]")?,
    )?;
    let dir = args.next().ok_or("no replacements dir")?;
    let verbose = args.next().is_some_and(|a| a == "-v");
    let pack: HashSet<String> = std::fs::read_dir(&dir)?
        .filter_map(|e| {
            e.ok()?
                .file_name()
                .into_string()
                .ok()?
                .strip_suffix(".png")
                .map(str::to_owned)
        })
        .collect();
    let (mut hit, mut all, mut used) = (BTreeMap::<String, [usize; 2]>::new(), 0, HashSet::new());
    hst_data::texhash::disc_textures(&mut iso, |t| {
        all += 1;
        let m = t.names.iter().find(|n| pack.contains(*n));
        let arc = t.path.rsplit_once('/').map_or("", |p| p.0).to_owned();
        let e = hit.entry(arc).or_default();
        e[1] += 1;
        if let Some(n) = m {
            e[0] += 1;
            used.insert(n.clone());
        }
        if verbose {
            println!(
                "{} {}x{} {} {}",
                t.path,
                t.width,
                t.height,
                m.map_or("-", |s| s),
                t.names.first().map_or("", |s| s)
            );
        }
    });
    for (arc, [h, n]) in &hit {
        println!("{h:4}/{n:<4} {arc}");
    }
    println!(
        "{all} disc textures, {} with a pack file; {} of {} pack files matched",
        hit.values().map(|v| v[0]).sum::<usize>(),
        used.len(),
        pack.len()
    );
    Ok(())
}
