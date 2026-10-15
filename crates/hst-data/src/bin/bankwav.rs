//! bankwav <iso> <archive> <bank.hd> <out dir> — every one-note take of a sound bank as `<program>_<key>.wav`
//! (16-bit mono at the rate the game plays it), the mod voice layout (open-tennis-mods standard §5). Takes of several
//! notes are listed and skipped. Writes game data: keep `out dir` out of git.
use hst_data::{exe, iso::Iso, snd::{self, Bank}, xb::Archive};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let [_, iso, xb, hd, out] = &a[..] else { return Err("usage: bankwav <iso> <archive> <bank.hd> <out dir>".into()) };
    let mut iso = Iso::open(iso)?;
    let pitch = exe::pitch_table(&iso.read("MODULES2/SG2IOPM1.IRX")?)?;
    let data = iso.read(xb)?;
    let arc = Archive::parse(&data)?;
    let read = |name: &str| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let e = arc.entries.iter().find(|e| e.name.replace('\\', "/").ends_with(name)).ok_or(format!("{name} not in {xb}"))?;
        Ok(arc.read(e)?)
    };
    let (hd, bd) = (read(hd)?, read(&hd.replace(".hd", ".bd"))?);
    let b = Bank::parse(&hd)?;
    std::fs::create_dir_all(out)?;
    for (p, k) in (0..16).flat_map(|p| (0..32).map(move |k| (p, k))) {
        let Some(e) = b.key_ons(p, k) else { continue };
        let [e] = e[..] else {
            println!("{p}_{k}: {} notes, skipped", e.len());
            continue;
        };
        let t = b.tone(e.set as usize, e.note).ok_or("no tone")?;
        let word = (t.root() as u32) << 24 | (e.note as u32) << 16 | (t.fine() as u8 as u32) << 8 | 0x40;
        let rate = (snd::pitch(&pitch, word, 0x0100_1000) as u32 * 48_000 + 2048) / 4096;
        let pcm = snd::adpcm(&bd, t.sample());
        let mut wav = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        wav.extend([16u32.to_le_bytes(), [1, 0, 1, 0], rate.to_le_bytes(), (rate * 2).to_le_bytes(), [2, 0, 16, 0]].concat());
        wav.extend(b"data");
        wav.extend((pcm.len() as u32 * 2).to_le_bytes());
        wav.extend(pcm.iter().flat_map(|x| x.to_le_bytes()));
        std::fs::write(format!("{out}/{p}_{k}.wav"), &wav)?;
        println!("{p}_{k}: {rate} Hz, {:.2} s, velocity {}, tone volume {}", pcm.len() as f32 / rate as f32, e.velocity, t.volume());
    }
    Ok(())
}
