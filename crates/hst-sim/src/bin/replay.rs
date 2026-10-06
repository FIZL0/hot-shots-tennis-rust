//! Dump a recorded fixture (`tools/record_p2m2.py`) as CSV, one row per frame:
//! `replay <fixture.bin> [first vsync] [last vsync]`. Gaps in the vsync sequence are reported on stderr.

use hst_sim::replay::frames;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else { return eprintln!("usage: replay <fixture.bin> [first vsync] [last vsync]") };
    let data = std::fs::read(path).expect("read fixture");
    let all = frames(&data);
    let bound = |i: usize, d: u32| args.get(i).map_or(d, |s| s.parse().expect("vsync"));
    let (from, to) = (bound(2, 0), bound(3, u32::MAX));
    for w in all.windows(2).filter(|w| w[1].vsync() != w[0].vsync() + 1) {
        eprintln!("gap: vsync {} -> {}", w[0].vsync(), w[1].vsync());
    }
    if let (Some(a), Some(b)) = (all.first(), all.last()) {
        eprintln!("{} frames, vsync {}..={}, {} players", all.len(), a.vsync(), b.vsync(), a.global(0x422fa4));
    }
    println!("vsync,phase,sub,shots,points,games,sets,buttons,lx,ly,p0x,p0z,p1x,p1z,ball_x,ball_y,ball_z,call");
    for f in all.iter().filter(|f| (from..=to).contains(&f.vsync())) {
        let (p, g, s) = f.score();
        let (pad, b) = (f.pad(0), f.ball());
        let bf = |o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let [p0x, _, p0z] = f.player_pos(0);
        let [p1x, _, p1z] = f.player_pos(1);
        println!(
            "{},{},{},{},{}-{},{}-{},{}-{},{:#06x},{},{},{p0x},{p0z},{p1x},{p1z},{},{},{},{}",
            f.vsync(), f.gm()[0x55], f.gm()[0x56], f.global(0x423060), p[0], p[1], g[0], g[1], s[0], s[1],
            pad.buttons, pad.lx, pad.ly, bf(0xe0), bf(0xe4), bf(0xe8), b[0xa5]
        );
    }
}
