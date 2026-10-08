//! The costume noise's VU half against a GS dump (`context/fixtures/noise_gs_s05.txt`, `research/p17m_noise_gs.py`):
//! slot 5 with the deformers frozen (rate 0) and their amp boosted, so the skirt (node 1) and fringe (node 14) of
//! player 0 sway by pixels. Each noise vertex is its position entries moved by `noise::deform`, carried through its
//! bones' matrices fitted to the same frame, and must land where the GS drew it; unmoved entries must not.

use hst_sim::noise;

fn bits(h: &str) -> f32 {
    f32::from_bits(u32::from_str_radix(h, 16).unwrap())
}

#[test]
fn skirt_sway_gs_s05() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../context/fixtures").into());
    let Ok(text) = std::fs::read_to_string(format!("{dir}/noise_gs_s05.txt")) else {
        return eprintln!("noise_gs_s05.txt absent, skipped");
    };
    let table = noise::table();
    let (mut screen, mut deformers) = ([0f64; 3], Vec::new());
    // per vertex: node, drawn (x, y), entries (p, w, weight, 3×4 matrix)
    let mut verts: Vec<(usize, [f64; 2], Vec<([f32; 3], f32, f32, [f64; 12])>)> = Vec::new();
    for line in text.lines() {
        let t: Vec<&str> = line.split_whitespace().collect();
        match t[0] {
            "S" => screen = std::array::from_fn(|k| t[1 + k].parse().unwrap()),
            "N" => deformers.push((t[1].parse::<usize>().unwrap(), bits(t[2]), bits(t[3]), bits(t[4]))),
            "V" => verts.push((t[1].parse().unwrap(), [t[2].parse().unwrap(), t[3].parse().unwrap()], Vec::new())),
            _ => verts.last_mut().unwrap().2.push(([bits(t[1]), bits(t[2]), bits(t[3])], bits(t[4]), bits(t[5]), std::array::from_fn(|k| t[6 + k].parse().unwrap()))),
        }
    }
    let miss = |moved: bool| -> Vec<f64> {
        verts
            .iter()
            .map(|(node, at, entries)| {
                let &(_, freq, prev, amp) = deformers.iter().find(|d| d.0 == *node).unwrap();
                let mut x = [0f64; 3];
                for (p, w, weight, m) in entries {
                    let q = if moved { noise::deform(&table, *p, *weight, freq, prev, amp) } else { *p };
                    let v = [q[0] as f64, q[1] as f64, q[2] as f64, *w as f64];
                    for (r, xr) in x.iter_mut().enumerate() {
                        *xr += (0..4).map(|c| m[4 * r + c] * v[c]).sum::<f64>();
                    }
                }
                let (sx, sy) = (screen[0] + screen[2] * x[0] / x[2], screen[1] + screen[2] * x[1] / x[2]);
                (sx - at[0]).hypot(sy - at[1])
            })
            .collect()
    };
    let (moved, still) = (miss(true), miss(false));
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let max = |v: &[f64]| v.iter().cloned().fold(0.0, f64::max);
    eprintln!("{} vertices: moved mean {:.3} max {:.3} px, unmoved mean {:.3} max {:.3} px", moved.len(), mean(&moved), max(&moved), mean(&still), max(&still));
    assert!(verts.len() > 100);
    // the GS's 1/16-pixel grid and the fit: the frame's noise-free vertices fit to 0.03 px on average
    assert!(mean(&moved) < 0.05 && max(&moved) < 0.3, "the port's sway misses the drawn skirt");
    assert!(mean(&still) > 0.5, "unmoved entries land as close: the check sees no sway");
}
