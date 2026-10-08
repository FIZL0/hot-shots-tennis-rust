use hst_data::iso::Iso;
use hst_sim::weather::{schedule, Mt, Odds, Rand, Wind};

/// Slot 5's match (court 10, doubles, 4 games, 1 set): its schedule as in RAM (match object +0xd8 weathers, +0x11c
/// speed/direction pairs) comes out of the shared MT19937 seeded with newlib `rand()` output 0x28c7c4a1, the one
/// 2641 calls before the match seed (research/p17k_seed_search.py walks the LCG back from the live state).
#[test]
fn slot5_schedule_from_the_game_generator() {
    let Ok(mut iso) = Iso::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso")) else { return eprintln!("no ISO, skipped") };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let exe = hst_data::exe::Game::new(&cnf, &bin).unwrap();
    let o = exe.weather_odds(10);
    let odds = Odds { cloudy: o[0], cloudy_len: [o[1], o[2]], rain: o[3], rain_len: [o[4], o[5]], no_border: o[7] != 0, heavy: o[8] != 0 };
    let ((directions, speed), (chance, kind)) = (exe.wind(10), exe.gusts(10));
    let wind = Wind { chance, kind, directions, speed };
    let mut mt = Mt::new(0x28c7_c4a1);
    let s = schedule(&odds, &wind, 4, 1, 4, || mt.next());
    let weathers: String = s.iter().map(|g| char::from(b'0' + g.weather)).collect();
    assert_eq!(weathers, "00000000000000000000000000000000111000110000000000000000000000001");
    let degrees = [
        315, 0, 315, 0, 0, 0, 315, 315, 45, 315, 0, 0, 45, 45, 45, 315, 45, 45, 45, 315, 0, 45, 45, 45, 315, 45, 0, 45, 315, 45, 315, 0, 0, 45, 45,
        315, 315, 45, 45, 0, 0, 45, 45, 315, 315, 0, 315, 45, 0, 315, 45, 0, 315, 45, 315, 315, 0, 315, 315, 315, 0, 315, 45, 315, 315,
    ];
    assert_eq!(s.map(|g| g.degrees as i32), degrees);
    assert!(s.iter().all(|g| g.speed == 2.0));
}

/// newlib's rand: state 1 at boot; and the match seed (0x387a9ad) sits 2641 outputs after the schedule's.
#[test]
fn rand_chain() {
    let mut r = Rand(0xa8c7_c4a1_ec47_6463);
    for _ in 0..2640 {
        r.next();
    }
    assert_eq!(r.next(), 0x387_a9ad);
    assert_eq!(Rand::default().next(), (0x5851_f42d_4c95_7f2eu64 >> 32) as u32 & 0x7fff_ffff);
}
