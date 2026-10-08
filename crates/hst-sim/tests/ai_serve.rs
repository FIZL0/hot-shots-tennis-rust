//! The bots' serve and singles kind lock, call by call against the original (`tools/record_ai_serve.py`): every
//! call that draws (or reads the frame before the contact) is recorded on entry and exit with the AI generator's
//! draw count, so each port function is fed the generator as it stood and must leave what the game left.
//!
//! `context/fixtures/ai_serve_singles.bin`: the bot-only singles match (`context/recordings/bots_singles.p2m2`'s
//! save state, no input), 7000 frames from the load. P0 Carol (character 6, left-handed), P1 Lola (character 10),
//! both in costume 9 (block 3, the hardest): AIParam rows 48 and 52, level 3.
//! `context/fixtures/ai_serve_s05.bin`: save slot 5, the bot-only doubles match. Characters 0, 2 / 1, 5 in
//! costume 0: rows 84, 86 / 85, 89, level 3 (P11a). Doubles has no kind lock and its serve aim has no net dash.

use hst_data::{exe, iso::Iso, xb::Archive};
use hst_sim::ai::AiParams;
use hst_sim::aim::button;
use hst_sim::serve::ai_pick;

const TABLE: u32 = 0x3174c0;
const RECORD: u32 = 0x118;
const MT_SIZE: usize = 0x9c8;
/// Record offsets (see the tool): arguments, return, the stick and button behind a1/a2, the pick frame, the AI.
const ARGS: usize = 0x10;
const STICK: usize = 0x24;
const BUTTON: usize = 0x34;
const FRAME: usize = 0x38;
const AI: usize = 0x40;
const SIDE: usize = 0x2c0;
const CHAR: usize = 0x2cc;
const GOING: usize = 0x2f4;
const AD: usize = 0x318;
const SECOND: usize = 0x338;
const REACH: usize = 0x340;
const HAND: usize = 0x360;
const PATH: usize = 0x380;

fn root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
}

/// The AIParam table and the characters' second-serve strong-toss chances, from the disc.
fn disc() -> Option<(Vec<AiParams>, [i32; 14])> {
    let mut iso = Iso::open(format!("{}/Hot Shots Tennis (USA).iso", root())).ok()?;
    let data = iso.read("PCDATA/PCDATA.XB").ok()?;
    let arc = Archive::parse(&data).ok()?;
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("aiparam.csv"))?;
    let table = AiParams::table(&arc.read(e).ok()?);
    let (cnf, bin) = (iso.read("SYSTEM.CNF").ok()?, iso.read("ZZBIN/GAME.BIN").ok()?);
    Some((table, exe::Game::new(&cnf, &bin).ok()?.second_toss()))
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_bits(u32_at(b, o))
}

/// The AI generator (MT19937) from its RAM block: state at +4, index at +0x9c4.
#[derive(Clone)]
struct Mt([u32; 624], usize);
impl Mt {
    fn of(b: &[u8]) -> Mt {
        Mt(std::array::from_fn(|k| u32_at(b, 4 + 4 * k)), u32_at(b, 0x9c4) as usize)
    }
    fn next(&mut self) -> u32 {
        if self.1 >= 624 {
            for k in 0..624 {
                let y = (self.0[k] & 0x8000_0000) | (self.0[(k + 1) % 624] & 0x7fff_ffff);
                self.0[k] = self.0[(k + 397) % 624] ^ (y >> 1) ^ if y & 1 != 0 { 0x9908_b0df } else { 0 };
            }
            self.1 = 0;
        }
        let mut y = self.0[self.1];
        self.1 += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }
}

/// One recorded call: entry and exit, the draws before it and in it.
struct Call<'a> {
    tag: u32,
    vsync: u32,
    before: u32,
    draws: u32,
    e: &'a [u8],
    x: &'a [u8],
}

impl Call<'_> {
    fn ai(&self, o: usize) -> u8 {
        self.e[AI + o]
    }
    fn row<'t>(&self, table: &'t [AiParams]) -> &'t AiParams {
        &table[((u32_at(self.e, AI + 0xc) - TABLE) / RECORD) as usize]
    }
}

/// The fixture's generator at its first draw and its calls.
fn calls(d: &[u8]) -> (Mt, Vec<Call<'_>>) {
    assert_eq!(&d[..4], b"AISV");
    let mt = Mt::of(&d[8..8 + MT_SIZE]);
    let mut recs = vec![];
    let mut o = 8 + MT_SIZE;
    while o < d.len() {
        let (tag, size) = (u32_at(d, o), u32_at(d, o + 4) as usize);
        recs.push((tag, &d[o..o + size]));
        o += size;
    }
    let calls = recs
        .chunks(2)
        .map(|p| {
            let ((tag, e), (xtag, x)) = (p[0], p[1]);
            assert_eq!(xtag, tag | 0x100);
            let before = u32_at(e, 0xc);
            Call { tag, vsync: u32_at(e, 8), before, draws: u32_at(x, 0xc) - before, e, x }
        })
        .collect();
    (mt, calls)
}

/// Replays every call of a fixture through the port; returns how many of each kind were checked.
fn replay(name: &str) -> Option<[usize; 5]> {
    let d = std::fs::read(format!("{}/context/fixtures/{name}", root())).ok()?;
    let (table, second) = disc()?;
    let (mt0, calls) = calls(&d);
    let mut n = [0; 5];
    for c in &calls {
        let mut mt = mt0.clone();
        (0..c.before).for_each(|_| _ = mt.next());
        let mut used = 0;
        let mut roll = || {
            used += 1;
            mt.next()
        };
        let row = c.row(&table);
        let at = format!("{name} v{} tag {}", c.vsync, c.tag);
        let going = u32_at(c.e, GOING) == 1;
        let side = f32_at(c.e, SIDE);
        let mut checked = true;
        match c.tag {
            // singles kind lock: the button, then (the frame before the contact) the stick for the kept button
            1 => {
                let b = u32_at(c.e, BUTTON) as u8;
                if b != 0 {
                    let got = row.lock_button(b, &mut roll);
                    assert_eq!(got as u32, u32_at(c.x, BUTTON), "{at}: button {b}");
                    assert_eq!(got as u32, u32_at(c.x, AI + 0x2c), "{at}: kept");
                    n[0] += 1;
                }
                if going {
                    let stick = std::array::from_fn(|k| f32_at(c.e, STICK + 4 * k));
                    let got = row.lock_stick(u32_at(c.x, AI + 0x2c) as u8, stick, side, &mut roll);
                    let want: [f32; 4] = std::array::from_fn(|k| f32_at(c.x, STICK + 4 * k));
                    assert_eq!(got.map(f32::to_bits), want.map(f32::to_bits), "{at}: stick {stick:?}");
                    n[1] += 1;
                }
            }
            // serve state: the toss (2 → 3), the swing (pressed into 5), the aim (5, the frame before the contact)
            2 | 3 => {
                let (from, to) = (c.ai(0x55), c.x[AI + 0x55]);
                if from == 2 && to == 3 {
                    let ch = u32_at(c.e, CHAR) as usize;
                    let toss = AiParams::serve_toss(u32_at(c.e, SECOND) != 0, second[ch], c.ai(0x5c), &mut roll);
                    assert_eq!(toss, c.x[AI + 0xb1], "{at}: toss");
                    assert_eq!(button(toss) as u32, u32_at(c.x, BUTTON), "{at}: toss button");
                    n[2] += 1;
                } else if from != 5 && to == 5 {
                    let swing = row.serve_swing(c.ai(0xb1), &mut roll);
                    assert_eq!(swing, c.x[AI + 0xb2], "{at}: swing");
                    assert_eq!(button(swing) as u32, u32_at(c.x, BUTTON), "{at}: swing button");
                    n[3] += 1;
                } else if from == 5 && going {
                    let (level, swing) = (c.ai(0x5c), c.ai(0xb2));
                    let (ad, lefty) = (u32_at(c.e, AD) != 0, c.e[HAND] != 0);
                    let want: [f32; 4] = std::array::from_fn(|k| f32_at(c.x, STICK + 4 * k));
                    let got = if c.tag == 2 {
                        let reach = f32_at(c.e, AI + 0x14);
                        let (s, dash, spot) = row.serve_aim_singles(level, swing, ad, lefty, side, reach, &mut roll);
                        assert_eq!(dash, c.x[AI + 0x256] != 0 && c.ai(0x256) == 0, "{at}: dash");
                        if let Some([x, z]) = spot {
                            assert_eq!([x, z].map(f32::to_bits), [0x70, 0x78].map(|o| u32_at(c.x, AI + o)), "{at}: dash spot");
                        }
                        s
                    } else {
                        row.serve_aim(level, swing, ad, lefty, side, &mut roll)
                    };
                    assert_eq!(got.map(f32::to_bits), want.map(f32::to_bits), "{at}: aim lv {level} swing {swing} {got:?} {want:?}");
                    n[4] += 1;
                } else {
                    checked = false;
                }
            }
            // the contact pick: the frame on the predicted path nearest the ideal height
            4 => {
                let [toss, quick] = [u32_at(c.e, ARGS + 4), u32_at(c.e, ARGS + 8)];
                let r = &c.e[REACH..];
                let window = if toss == 2 { [0x10, 0x14, 0x18] } else { [4, 8, 0xc] }.map(|o| f32_at(r, o));
                let (start, end) = (u32_at(c.e, AI + 0x30) as usize, u32_at(c.e, AI + 0x3c) as usize);
                let p = &c.e[PATH..];
                let path = (start..end).map(|k| (f32_at(p, 0x30 * k + 4), f32_at(p, 0x30 * k + 0x14), u32_at(p, 0x30 * k + 0x20) as i32));
                let got = ai_pick(window, quick != 0 && toss != 2, path).map(|k| k + start);
                assert_eq!(got, Some(u32_at(c.x, FRAME) as usize), "{at}: pick");
            }
            _ => unreachable!(),
        }
        drop(roll);
        if checked {
            assert_eq!(used, c.draws, "{at}: draws");
        }
    }
    Some(n)
}

#[test]
fn singles_serves_and_kind_lock() {
    let Some(n) = replay("ai_serve_singles.bin") else { return eprintln!("fixture or disc missing, skipped") };
    eprintln!("button locks, stick locks, tosses, swings, aims: {n:?}");
    assert!(n.iter().all(|&k| k > 0), "{n:?}");
}

#[test]
fn doubles_serves() {
    let Some(n) = replay("ai_serve_s05.bin") else { return eprintln!("fixture or disc missing, skipped") };
    eprintln!("tosses, swings, aims: {:?}", &n[2..]);
    assert!(n[2..].iter().all(|&k| k > 0), "{n:?}");
}
