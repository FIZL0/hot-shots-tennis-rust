//! Background figures from the disc against the game's own objects in RAM on four courts (`context/ram/`: slots
//! 07 and 10 of the PCSX2 states = courts 1 and 2, `s03.bin` = court 4, `s05.bin` = court 10; not in git, skipped
//! when absent along with the disc image). Every creature record's position (links resolved) and yaw must match
//! the game's loaded record bit for bit; the walking spectators and trigger creatures made must be exactly the
//! game's (record, type); every trigger creature's matrix bit-exact, and a walker's wherever it still stands where
//! it was made (all four on court 1).

use hst_data::exe::Game;
use hst_data::iso::Iso;
use hst_data::layout;
use hst_data::xb::Archive;
use hst_sim::npc::{self, Kind};
use std::collections::BTreeMap;

const WALKER: u32 = 0x1d1de0;
const TRIGGER: u32 = 0x1d2180;

struct Ram(Vec<u8>);
impl Ram {
    fn u(&self, a: u32) -> u32 {
        let a = (a & 0x1ff_ffff) as usize;
        u32::from_le_bytes(self.0[a..a + 4].try_into().unwrap())
    }
    fn bits(&self, a: u32, n: u32) -> Vec<u32> {
        (0..n).map(|k| self.u(a + 4 * k)).collect()
    }
}

#[test]
fn creatures_match_ram_on_four_courts() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Ok(mut iso) = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")) else {
        return eprintln!("disc missing, skipped");
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let game = Game::new(&cnf, &bin).unwrap();
    let mut courts = Vec::new();
    for f in ["npc_c01", "npc_c02", "s03", "s05"] {
        let Ok(ram) = std::fs::read(format!("{root}/context/ram/{f}.bin")) else {
            eprintln!("{f}.bin missing, skipped");
            continue;
        };
        let r = Ram(ram);
        let (court, players) = (r.u(0x422f90), r.u(0x422fa4));
        courts.push(court);
        let (mut entries, mut plants) = (Vec::new(), Vec::new());
        for arc in ["CMN.XB", "HOL01.XB"] {
            let data = iso.read(&format!("COURT/{court:02}/{arc}")).unwrap();
            let a = Archive::parse(&data).unwrap();
            for e in &a.entries {
                let name = e.name.to_ascii_lowercase();
                if name.ends_with(&format!("entry_c{court:02}.txt")) {
                    entries = layout::entries(&String::from_utf8_lossy(&a.read(e).unwrap()));
                } else if name.ends_with(&format!("plant_c{court:02}_h01_0.dat")) {
                    plants = layout::plants(&a.read(e).unwrap()).unwrap();
                }
            }
        }

        let layout = r.u(r.u(r.u(0x422f80) + 0x84) + 0x150);
        let (n, recs) = (r.u(layout + 0x430 + 23 * 4), r.u(layout + 0x490 + 23 * 4));
        let pos = npc::positions(&plants);
        assert_eq!(pos.len(), n as usize, "court {court}: creature records");
        let yaws: Vec<f32> = plants.iter().filter(|p| p.category == 23).map(|p| p.yaw).collect();
        for (k, p) in pos.iter().enumerate() {
            let a = recs + 0x90 * k as u32;
            assert_eq!(r.bits(a + 0x40, 3), p.map(f32::to_bits), "court {court} record {k}: position");
            assert_eq!(r.u(a + 0x54), yaws[k].to_bits(), "court {court} record {k}: yaw");
        }

        // the game's figures: record → (type, matrix)
        let mut objs = BTreeMap::new();
        for a in (0x10_0000..0x200_0000u32).step_by(4) {
            let vt = r.u(a);
            if vt == WALKER || vt == TRIGGER {
                let rec = (r.u(a + 0x54) - recs) / 0x90;
                objs.insert(rec as usize, (vt == WALKER, r.u(a + 0x50) as u8, r.bits(a + 0x70, 16)));
            }
        }
        let made = npc::spawn(&entries, &plants, &game.npc_roster(court), &game.walkers(court), players);
        let ours: BTreeMap<usize, (bool, u8)> = made
            .iter()
            .filter_map(|m| match m.kind {
                Kind::Trigger(t) => Some((m.record, (false, t))),
                Kind::Walker(w) => Some((m.record, (true, w + 54))),
                _ => None,
            })
            .collect();
        let theirs: BTreeMap<usize, (bool, u8)> = objs.iter().map(|(&k, v)| (k, (v.0, v.1))).collect();
        assert_eq!(ours, theirs, "court {court}: figures (record → walker?, type)");
        let mut still = 0;
        for m in &made {
            let Some((walker, _, mtx)) = objs.get(&m.record) else { continue };
            let want: Vec<u32> = m.world.iter().flatten().map(|x| x.to_bits()).collect();
            if *walker && mtx[12..15] != want[12..15] {
                continue; // walked off
            }
            still += *walker as usize;
            assert_eq!(*mtx, want, "court {court} record {}: matrix", m.record);
        }
        eprintln!("court {court}: {} records, {} figures, {still} walkers checked at their spot", n, theirs.len());
        if court == 1 {
            assert_eq!(still, 4, "court 1's walkers have not moved");
        }
    }
    assert!(courts.is_empty() || courts.len() >= 3, "fewer than three courts checked: {courts:?}");
}
