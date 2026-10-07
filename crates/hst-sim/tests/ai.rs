//! The AI tuning table read from the disc's AIParam.csv must equal the game's own copy in RAM (`context/ram/s05.bin`,
//! slot 5: a doubles bot match), and each bot's row and level must be the ones the game gave it.

use hst_data::{iso::Iso, xb::Archive};
use hst_sim::ai::{AiParams, Choice, ROWS, menu_row};

const TABLE: usize = 0x3174c0;
const RECORD: usize = 0x118;

fn load() -> Option<(Vec<u8>, Vec<u8>)> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let ram = std::fs::read(format!("{root}/context/ram/s05.bin")).ok()?;
    let mut iso = Iso::open(format!("{root}/Hot Shots Tennis (USA).iso")).ok()?;
    let data = iso.read("PCDATA/PCDATA.XB").ok()?;
    let arc = Archive::parse(&data).ok()?;
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("aiparam.csv"))?;
    Some((ram, arc.read(e).ok()?))
}

fn word(ram: &[u8], a: usize) -> usize {
    u32::from_le_bytes(ram[a..a + 4].try_into().unwrap()) as usize
}

#[test]
fn table_equals_ram() {
    let Some((ram, csv)) = load() else { return eprintln!("RAM dump or disc missing, skipped") };
    let table = AiParams::table(&csv);
    for (r, a) in table.iter().enumerate() {
        let want = &ram[TABLE + r * RECORD..TABLE + (r + 1) * RECORD];
        assert_eq!(a.bytes(), want, "row {r}: {a:?}");
    }
    assert_eq!(table.len(), ROWS);
}

#[test]
fn bots_get_their_rows() {
    let Some((ram, _)) = load() else { return eprintln!("RAM dump or disc missing, skipped") };
    let gm = word(&ram, 0x422f80);
    for i in 0..4 {
        // the menu's player slot: character, outfit, …, setup word at +8
        let slot = 0x2ef7f4 + 12 * i;
        let (character, outfit) = (ram[slot], ram[slot + 1]);
        let setup = word(&ram, slot + 8) as u32;
        assert_eq!(setup & 0xff, menu_row(character, outfit) as u32, "player {i} setup row");
        // the AI object (player +0x80): row pointer, level, strategy byte, reach
        let ai = word(&ram, word(&ram, gm + 0xa8 + 4 * i) + 0x80);
        let c = Choice::new(setup, character, true);
        assert_eq!(TABLE + c.row * RECORD, word(&ram, ai + 12), "player {i} row");
        assert_eq!(c.level, ram[ai + 16], "player {i} level");
        assert_eq!(c.strategy, ram[ai + 17], "player {i} strategy");
        assert_eq!(c.reach.to_bits(), word(&ram, ai + 20) as u32, "player {i} reach");
    }
}

#[test]
fn rows_by_outfit_and_mode() {
    assert_eq!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map(|o| menu_row(6, o)), [6, 6, 6, 6, 48, 20, 20, 20, 20, 48]);
    assert_eq!(Choice::new(13, 13, false).row, 13);
    assert_eq!(Choice::new(13, 13, true).row, 97);
    // a forced level puts the character in that block; rows clamp to the singles half first
    assert_eq!(Choice::new(0x200 | 50, 4, false), Choice { row: 18, level: 1, strategy: 0, reach: 2.85 });
    assert_eq!(Choice::new(160, 0, true).row, 167);
}

/// The game's MT19937 as recorded: 624 words at +4, the next word's index at +0x9c4.
struct Mt([u32; 624], usize);
impl Mt {
    fn of(b: &[u8]) -> Mt {
        Mt(std::array::from_fn(|k| word(b, 4 + 4 * k) as u32), word(b, 0x9c4))
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

/// Every timing-error draw of the slot-5 bots (`context/fixtures/ai_s05.bin`, tools/record_ai.py; skipped when
/// absent): from the frame before, the doubles AI's four errors must come out of consecutive draws of the game's
/// generator (other draws of that frame come first), given the opponent shot it just saw, its serve history, whether
/// it receives and whether its team has hit yet. The change of pace must show up in some of them.
#[test]
fn timing_errors_match_the_game() {
    use hst_sim::ai::{Seen, Shots};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Some((_, csv)), Ok(d)) = (load(), std::fs::read(format!("{root}/context/fixtures/ai_s05.bin"))) else {
        return eprintln!("disc or ai_s05.bin missing, skipped");
    };
    let table = AiParams::table(&csv);
    let (glob, mt, ai) = (4, 4 + 0x18, 4 + 0x18 + 0x9d0);
    let size = ai + 4 * 0x250;
    let samples: Vec<&[u8]> = d[20..].chunks_exact(size).collect();
    let int = |b: &[u8], o: usize| word(b, o) as i32;
    let f = |b: &[u8], o: usize| f32::from_bits(word(b, o) as u32);
    let (mut checked, mut paced, mut fast) = (0, 0, 0);
    for w in samples.windows(2) {
        let (a, b) = (w[0], w[1]);
        if word(b, 0) != word(a, 0) + 1 {
            continue; // a missed frame
        }
        for k in 0..4 {
            let (oa, ob) = (&a[ai + k * 0x250..][..0x250], &b[ai + k * 0x250..][..0x250]);
            let errs = |o: &[u8]| [0x234, 0x238, 0x23c, 0x240].map(|x| int(o, x));
            if errs(oa) == errs(ob) {
                continue;
            }
            let row = &table[(word(ob, 12) - TABLE) / RECORD];
            let seen = |r: usize| Seen { kind: int(ob, r + 4), vel: [f(ob, r + 0x30), f(ob, r + 0x34), f(ob, r + 0x38)] };
            let hitter = int(ob, 0x130);
            let shots = (oa[0x130..0x170] != ob[0x130..0x170] && hitter >= 0 && hitter & 1 != k as i32 & 1).then(|| Shots {
                last: seen(0x130),
                before: (int(ob, 0x170) >= 0).then(|| seen(0x170)),
                serve_before: (int(ob, 0x1f0) >= 0).then(|| seen(0x1f0).vel),
            });
            let receiver = int(b, glob + 0xc) == k as i32;
            let (first, third) = (ob[0x230] != 0, int(ob, 0x28) != 0);
            let mut g = Mt::of(&a[mt..]);
            let draws: Vec<u32> = (0..200).map(|_| g.next()).collect();
            let draw = |shots: Option<&Shots>| {
                (0..150).find_map(|j| {
                    let mut n = draws[j..].iter().copied();
                    let t = row.timing(shots, receiver, first, third, &mut || n.next().unwrap());
                    ([t.stroke, t.volley, t.smash, t.serve] == errs(ob)).then_some(t)
                })
            };
            let t = draw(shots.as_ref())
                .unwrap_or_else(|| panic!("vsync {} AI {k}: errors {:?} not drawn ({shots:?})", word(b, 0), errs(ob)));
            checked += 1;
            if t.pace != 0 {
                // without the pace the same draws must not explain them
                assert!(draw(None).is_none(), "vsync {} AI {k}: errors match without the pace", word(b, 0));
                paced += 1;
            }
            fast += (t.fast_ball != 0) as u32;
        }
    }
    eprintln!("{checked} draws checked, {paced} with a change of pace, {fast} fast-ball reactions");
    assert!(checked >= 100, "only {checked} draws");
    assert!(paced > 0, "no change of pace seen");
}

/// Every guess (ヤマ張り) the slot-5 bots draw (`ai_s05.bin`): on each opponent hit, right after the timing draw that
/// `timing_errors_match_the_game` finds, the guess must come out as the game's (none, or which side, with the move
/// frames in place of the reaction). All four bots, so the partner is always a computer player.
#[test]
fn guesses_match_the_game() {
    use hst_sim::ai::{Guess, Seen, Shots};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Some((_, csv)), Ok(d)) = (load(), std::fs::read(format!("{root}/context/fixtures/ai_s05.bin"))) else {
        return eprintln!("disc or ai_s05.bin missing, skipped");
    };
    let table = AiParams::table(&csv);
    let (glob, mt, ai) = (4, 4 + 0x18, 4 + 0x18 + 0x9d0);
    let size = ai + 4 * 0x250;
    let samples: Vec<&[u8]> = d[20..].chunks_exact(size).collect();
    let int = |b: &[u8], o: usize| word(b, o) as i32;
    let f = |b: &[u8], o: usize| f32::from_bits(word(b, o) as u32);
    let (mut serves, mut guesses) = (0, 0);
    for w in samples.windows(2) {
        let (a, b) = (w[0], w[1]);
        if word(b, 0) != word(a, 0) + 1 {
            continue;
        }
        for k in 0..4 {
            let (oa, ob) = (&a[ai + k * 0x250..][..0x250], &b[ai + k * 0x250..][..0x250]);
            let hitter = int(ob, 0x130);
            if oa[0x130..0x170] == ob[0x130..0x170] || hitter < 0 || hitter & 1 == k as i32 & 1 {
                continue; // only an opponent's hit can bring a guess
            }
            let row = &table[(word(ob, 12) - TABLE) / RECORD];
            let seen = |r: usize| Seen { kind: int(ob, r + 4), vel: [f(ob, r + 0x30), f(ob, r + 0x34), f(ob, r + 0x38)] };
            let shots = Shots {
                last: seen(0x130),
                before: (int(ob, 0x170) >= 0).then(|| seen(0x170)),
                serve_before: (int(ob, 0x1f0) >= 0).then(|| seen(0x1f0).vel),
            };
            let receiver = int(b, glob + 0xc) == k as i32;
            let errs = |o: &[u8]| [0x234, 0x238, 0x23c, 0x240].map(|x| int(o, x));
            let mut g = Mt::of(&a[mt..]);
            let draws: Vec<u32> = (0..200).map(|_| g.next()).collect();
            let got = (0..150)
                .find_map(|j| {
                    let mut n = draws[j..].iter().copied();
                    let mut roll = || n.next().unwrap();
                    let t = row.timing(Some(&shots), receiver, ob[0x230] != 0, int(ob, 0x28) != 0, &mut roll);
                    ([t.stroke, t.volley, t.smash, t.serve] == errs(ob))
                        .then(|| row.guess(&t, shots.last.kind == 0, ob[0x13d] != 0, true, &mut roll))
                })
                .unwrap_or_else(|| panic!("vsync {} AI {k}: timing draw not found", word(b, 0)));
            let want = match ob[0x24f] {
                0 => None,
                1 => Some(Guess::Wide),
                _ => Some(Guess::Other),
            };
            assert_eq!(got, want, "vsync {} AI {k}", word(b, 0));
            if want.is_some() {
                // the receive state may already have counted one frame off by the next sample
                let left = int(ob, 0x248);
                assert!((row.guess[1] - 1..=row.guess[1]).contains(&left), "vsync {} AI {k}: move frames {left}", word(b, 0));
                guesses += 1;
            }
            serves += (shots.last.kind == 0) as u32;
        }
    }
    eprintln!("{serves} serves seen by the receiving bots, {guesses} guesses");
    assert!(guesses >= 3 && serves > guesses, "{serves} serves, {guesses} guesses");
}

/// Each guess the slot-5 bots judge (`ai_guess_s05.bin`: tools/record_ai.py 5 4000 … 0x260, so the spot the AI
/// stood on at the draw, +0x250, is in it): when the guess clears as the AI finds its contact point (+0x70), the
/// verdict from that spot must be the one the game acted on: right sets the boost flag, wrong waits again
/// for the stuck frames, neither goes on to the hit.
#[test]
fn guess_verdicts_match_the_game() {
    use hst_sim::ai::{Guess, Verdict};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Some((_, csv)), Ok(d)) = (load(), std::fs::read(format!("{root}/context/fixtures/ai_guess_s05.bin"))) else {
        return eprintln!("disc or ai_guess_s05.bin missing, skipped");
    };
    let table = AiParams::table(&csv);
    let (size_ai, ai) = (0x260, 4 + 0x18 + 0x9d0);
    let samples: Vec<&[u8]> = d[20..].chunks_exact(ai + 4 * size_ai).collect();
    let f = |b: &[u8], o: usize| f32::from_bits(word(b, o) as u32);
    let mut judged = 0;
    for w in samples.windows(2) {
        let (a, b) = (w[0], w[1]);
        for k in 0..4 {
            let (oa, ob) = (&a[ai + k * size_ai..][..size_ai], &b[ai + k * size_ai..][..size_ai]);
            if oa[0x24f] == 0 || ob[0x24f] != 0 || oa[0x70..0x80] == ob[0x70..0x80] {
                continue; // not cleared by finding the contact point
            }
            let row = &table[(word(ob, 12) - TABLE) / RECORD];
            let guess = if oa[0x24f] == 1 { Guess::Wide } else { Guess::Other };
            let from = [f(ob, 0x250), f(ob, 0x258)];
            // the side sign: the AI stands on −z for side +1
            let side = if from[1] > 0.0 { -1.0 } else { 1.0 };
            let v = guess.verdict(side, from, [f(ob, 0x70), f(ob, 0x78)]);
            let want = if ob[0x24e] != 0 {
                Verdict::Right
            } else if ob[0x56] == 0 {
                // a wrong guess sends the receive state back to waiting (+0x56 = 0) for the stuck frames
                assert_eq!(word(ob, 0x248) as i32, row.guess[2]);
                Verdict::Wrong
            } else {
                Verdict::Neither
            };
            assert_eq!(v, want, "vsync {} AI {k}", word(b, 0));
            judged += 1;
        }
    }
    eprintln!("{judged} guesses judged");
    assert!(judged >= 1);
}
