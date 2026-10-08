//! noidump <model.mdl> [<model.mtl>] — every packet's drawn vertices with their position entries, for checking the costume noise
//! against a GS dump (`research/p17m_noise_gs.py`). Lines: `P material group drawn`, then per drawn vertex
//! `V kick r g b a` (its vertex colour) and per entry `E x y z w node noise nx ny nz` (its normal); with the MTL first `M index r g b a +0x10 +0x14 name mode header-hex` per material.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let m = hst_data::mdl::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    if let Some(path) = a.get(2) {
        for (i, x) in hst_data::mtl::parse(&std::fs::read(path).unwrap(), std::fs::read(path.replace(".MTL", ".MTI")).ok().as_deref()).unwrap().materials.iter().enumerate() {
            let f = |o: usize| f32::from_le_bytes(x.header[o..o + 4].try_into().unwrap());
            println!("M {i} {} {} {} {} {} {} {} {} {}", x.color[0], x.color[1], x.color[2], x.color[3], f(0x10), f(0x14), x.name, i16::from_le_bytes([x.header[0x1e], x.header[0x1f]]), x.header.iter().map(|b| format!("{b:02x}")).collect::<String>());
        }
    }
    for (mi, pks) in m.materials.iter().enumerate() {
        for pk in pks {
            let n = pk.bones.len().saturating_sub(1).min(pk.uvs.len());
            println!("P {mi} {} {n}", pk.group);
            for v in 0..n {
                let (lo, hi) = (pk.bones[v] as usize, (pk.bones[v + 1] as usize).min(pk.vertices.len()));
                let c = pk.colors.get(v).copied().unwrap_or([0x80; 4]);
                println!("V {} {} {} {} {}", pk.entry_flags.get(lo).is_some_and(|f| f & 0x8000 == 0) as u8, c[0], c[1], c[2], c[3]);
                for e in lo..hi {
                    let flags = pk.entry_flags.get(e).copied().unwrap_or(0);
                    let node = pk.palette.get((flags >> 3 & 7) as usize).or(pk.palette.first()).copied().unwrap_or(0);
                    let p = pk.vertices[e].pos;
                    let (w, q) = (pk.entry_weight.get(e).copied().unwrap_or(1.0), pk.vertices[e].normal);
                    println!("E {:08x} {:08x} {:08x} {:08x} {node} {:08x} {} {} {}", p[0].to_bits(), p[1].to_bits(), p[2].to_bits(), w.to_bits(), pk.noise.get(e).copied().unwrap_or(0.0).to_bits(), q[0], q[1], q[2]);
                }
            }
        }
    }
}
