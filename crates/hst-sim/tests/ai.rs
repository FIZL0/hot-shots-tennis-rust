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

/// The rows the game gave each player: slot 5's bots in outfit 0, and `1p3goodcpus` (EE RAM of its save state,
/// `context/fixtures/1p3goodcpus_ee.bin`): Carol, Will, 2 and 10 in outfits 9, 9, 4, 9, the hardest block.
#[test]
fn bots_get_their_rows() {
    let Some((s05, _)) = load() else { return eprintln!("RAM dump or disc missing, skipped") };
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let good = std::fs::read(format!("{root}/context/fixtures/1p3goodcpus_ee.bin"));
    if good.is_err() {
        eprintln!("1p3goodcpus_ee.bin absent, only slot 5");
    }
    for ram in [Some(s05), good.ok()].into_iter().flatten() {
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
                    ([t.stroke, t.volley, t.smash, t.serve] == errs(ob)).then(|| {
                        for _ in 0..hst_sim::ai::CHOICE_DRAWS {
                            roll();
                        }
                        // the reaction's own draws (its value is checked in reactions_match_the_game)
                        let lob = ob[0x138] == 3;
                        row.reaction(&t, &shots.last, lob, ob[0x13d] != 0, false, false, &mut roll);
                        row.guess(&t, shots.last.kind == 0, true, &mut roll)
                    })
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

/// The AI object's state machine against the slot-5 bots (`ai_mind_s05.bin`: tools/record_ai.py 5 5000 … 0x280
/// 0x4230a8,0x423060, so the point's winning team and the shot count follow the MT): each first update of a point
/// takes the state its role gives; each new net pick, hit count, net-rate step and point-over coin flip comes out of
/// the game's own draws; each serve walks to the spot and waits as drawn.
#[test]
fn minds_match_the_game() {
    use hst_sim::ai::{Mind, NET_RATE, Phase, serve_spot, serve_wait};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Some((ram, csv)), Ok(d)) = (load(), std::fs::read(format!("{root}/context/fixtures/ai_mind_s05.bin"))) else {
        return eprintln!("disc or ai_mind_s05.bin missing, skipped");
    };
    // the clamp the point reaction keeps the rate in: doubles, then singles
    for (lo, hi) in [(0x4179b8, 0x4179c0), (0x4178d0, 0x4178d8)] {
        assert_eq!((word(&ram, lo) as i32, word(&ram, hi) as i32), NET_RATE);
    }
    let table = AiParams::table(&csv);
    let (glob, mt, extra, size_ai) = (4, 4 + 0x18, 4 + 0x18 + 0x9d0, 0x280);
    let ai = extra + 16;
    let samples: Vec<&[u8]> = d[20..].chunks_exact(ai + 4 * size_ai).collect();
    let int = |b: &[u8], o: usize| word(b, o) as i32;
    let f = |b: &[u8], o: usize| f32::from_bits(word(b, o) as u32);
    let mind = |o: &[u8]| Mind {
        phase: [Phase::Start, Phase::Serve, Phase::Receive, Phase::Rally][o[0x54] as usize],
        active: o[0x60] != 0,
        net: o[0x274] != 0,
        net_rate: int(o, 0x270),
        net_left: int(o, 0x26c),
    };
    let mut n = [0; 7];
    for w in samples.windows(2) {
        let (a, b) = (w[0], w[1]);
        if word(b, 0) != word(a, 0) + 1 {
            continue;
        }
        let vsync = word(b, 0);
        let mut g = Mt::of(&a[mt..]);
        let draws: Vec<u32> = (0..200).map(|_| g.next()).collect();
        // some offset into this frame's draws gives `want`
        let drawn = |want: &dyn Fn(&mut dyn FnMut() -> u32) -> bool| {
            (0..150).any(|j| {
                let mut n = draws[j..].iter().copied();
                want(&mut || n.next().unwrap())
            })
        };
        for k in 0..4 {
            let (oa, ob) = (&a[ai + k * size_ai..][..size_ai], &b[ai + k * size_ai..][..size_ai]);
            let (ma, mb) = (mind(oa), mind(ob));
            let row = &table[(word(ob, 12) - TABLE) / RECORD];
            if ma.phase == Phase::Start && mb.phase != Phase::Start {
                let (server, receiver) = (int(b, glob + 4) == k as i32, int(b, glob + 0xc) == k as i32);
                assert_eq!(ma.clone().start(server, receiver), mb.phase, "vsync {vsync} AI {k}: first update");
                n[0] += 1;
            }
            if mb.net_rate != ma.net_rate {
                assert_eq!(row.style, 3, "vsync {vsync} AI {k}: rate moved");
                let won = int(b, extra) == k as i32 & 1;
                let ok = drawn(&|r| {
                    let mut m = ma;
                    m.point_result(row.style, won, &mut || r());
                    (m.net_rate, m.net) == (mb.net_rate, mb.net)
                });
                assert!(ok, "vsync {vsync} AI {k}: rate {} -> {} ({won})", ma.net_rate, mb.net_rate);
                n[1] += 1;
            } else if mb.net_left != ma.net_left {
                let shots = (int(a, extra + 8), int(b, extra + 8));
                if mb.net_left == ma.net_left - 1 && shots.1 == shots.0 + 1 {
                    n[2] += 1; // its team's hit
                } else if ma.net_left < 1 && mb.phase == Phase::Rally && ma.phase == Phase::Rally {
                    let ok = drawn(&|r| {
                        let mut m = ma;
                        m.rally(&mut || r());
                        (m.net, m.net_left) == (mb.net, mb.net_left)
                    });
                    assert!(ok, "vsync {vsync} AI {k}: pick {:?} -> {:?}", ma, mb);
                    n[3] += 1;
                } else {
                    // a timing draw at a reset or serve ends with the count
                    let ok = drawn(&|r| {
                        let mut m = ma;
                        m.count(&mut || r());
                        m.net_left == mb.net_left
                    });
                    assert!(ok && mb.net == ma.net, "vsync {vsync} AI {k}: count {:?} -> {:?}", ma, mb);
                    n[4] += 1;
                }
            }
            if ma.active && !mb.active {
                assert!(drawn(&|r| {
                    let mut m = ma;
                    m.point_over(&mut || r());
                    !m.active
                }));
                n[5] += 1;
            }
            // the serve's first step: the spot along the baseline, then the wait on it
            if ob[0x54] == 1 && ob[0x55] == 1 && (oa[0x54], oa[0x55]) != (1, 1) {
                let (ad, level0) = (int(b, glob + 8) != 0, ob[0x5c] == 0);
                let ok = [1.0, -1.0].iter().any(|&side| {
                    drawn(&|r| {
                        let x = serve_spot(level0, true, side, ad, &mut || r());
                        x.to_bits() == f(ob, 0x70).to_bits() && serve_wait(&mut || r()) == int(ob, 0x58)
                    })
                });
                assert!(ok, "vsync {vsync} AI {k}: spot {} wait {} (level {})", f(ob, 0x70), int(ob, 0x58), ob[0x5c]);
                n[6] += 1;
            }
        }
    }
    eprintln!("{n:?}: first updates, rate steps, own hits, picks, counts, point-over stops, serve spots");
    assert!(n.iter().all(|&c| c > 0), "{n:?}");
}

/// Every reaction the slot-5 bots draw on an opponent's hit (`ai_pos_s05.bin`, tools/record_ai_pos.py, which has
/// the players' positions): right after the timing draw and the shot-choice draws, the frames the AI waits (+0x248)
/// must be the game's, from its row, the shot it saw (after a smash, a lob), the fast-ball part and how near the net
/// it stands. A guess replaces it (checked in guesses_match_the_game). The rally state may already have counted one
/// frame off by the sample.
#[test]
fn reactions_match_the_game() {
    use hst_sim::ai::{CHOICE_DRAWS, Seen, Shots};
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Some((_, csv)), Ok(d)) = (load(), std::fs::read(format!("{root}/context/fixtures/ai_pos_s05.bin"))) else {
        return eprintln!("disc or ai_pos_s05.bin missing, skipped");
    };
    let table = AiParams::table(&csv);
    const AI: usize = 0x280;
    let (glob, mt, players) = (4, 4 + 0x18, 4 + 0x18 + 0x9d0 + 0x40 + 0xc0);
    let size = players + 4 * (0x10 + AI);
    let samples: Vec<&[u8]> = d[4 + 32 * 4 + 8..].chunks_exact(size).collect();
    let int = |b: &[u8], o: usize| word(b, o) as i32;
    let f = |b: &[u8], o: usize| f32::from_bits(word(b, o) as u32);
    let (mut checked, mut kinds, mut near) = (0, [0; 5], 0);
    for w in samples.windows(2) {
        let (a, b) = (w[0], w[1]);
        if word(b, 0) != word(a, 0) + 1 {
            continue;
        }
        for k in 0..4 {
            let at = players + k * (0x10 + AI);
            let (oa, ob) = (&a[at + 0x10..][..AI], &b[at + 0x10..][..AI]);
            let hitter = int(ob, 0x130);
            if oa[0x130..0x170] == ob[0x130..0x170] || hitter < 0 || hitter & 1 == k as i32 & 1 || ob[0x24f] != 0 {
                continue;
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
            // where it stood when the hit came
            let z = f(a, at + 8);
            let mut g = Mt::of(&a[mt..]);
            let draws: Vec<u32> = (0..200).map(|_| g.next()).collect();
            let got = (0..150)
                .find_map(|j| {
                    let mut n = draws[j..].iter().copied();
                    let mut roll = || n.next().unwrap();
                    let t = row.timing(Some(&shots), receiver, ob[0x230] != 0, int(ob, 0x28) != 0, &mut roll);
                    ([t.stroke, t.volley, t.smash, t.serve] == errs(ob)).then(|| {
                        for _ in 0..CHOICE_DRAWS {
                            roll();
                        }
                        let human = int(ob, 0x28) == 1;
                        row.reaction(&t, &shots.last, ob[0x138] == 3, ob[0x13d] != 0, human, z.abs() <= 6.4, &mut roll)
                    })
                })
                .unwrap_or_else(|| panic!("vsync {} AI {k}: timing draw not found", word(b, 0)));
            let left = int(ob, 0x248);
            assert!((got - 1..=got).contains(&left), "vsync {} AI {k}: reaction {left}, drawn {got} ({shots:?})", word(b, 0));
            checked += 1;
            kinds[shots.last.kind as usize] += 1;
            near += (z.abs() <= 6.4) as u32;
        }
    }
    eprintln!("{checked} reactions checked, by shot kind {kinds:?}, {near} near the net");
    assert!(checked >= 50 && kinds[4] > 0 && near > 0, "{checked} reactions, kinds {kinds:?}, {near} near the net");
}

/// Every time a slot-5 bot's contact search finds the ball on both sides of it
/// (`context/fixtures/ai_side_s05.bin`, tools/record_ai_side.py: the game's decision hooked), the side the port keeps
/// must be the game's: the nearer stand spot, or the strong side when the run-round roll passed and it is inside the
/// court's half width plus the row's extend (which must also be the game's width).
#[test]
fn stand_sides_match_the_game() {
    use hst_sim::ai::stand_side;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Some((_, csv)), Ok(d)) = (load(), std::fs::read(format!("{root}/context/fixtures/ai_side_s05.bin"))) else {
        return eprintln!("disc or ai_side_s05.bin missing, skipped");
    };
    let table = AiParams::table(&csv);
    let recs: Vec<&[u8]> = d.chunks_exact(0x50).collect();
    let int = |b: &[u8], o: usize| word(b, o) as i32;
    let f = |b: &[u8], o: usize| f32::from_bits(word(b, o) as u32);
    let (mut checked, mut ran, mut kept_nearer_on_roll) = (0, 0, 0);
    for w in recs.windows(2) {
        let (a, b) = (w[0], w[1]);
        if word(a, 0) >> 16 != 1 {
            continue;
        }
        assert_eq!((word(b, 0), word(b, 8)), (word(a, 0) + 0x10000, word(a, 8)), "unpaired decision at vsync {}", word(a, 4));
        let row = &table[(word(a, 68) - TABLE) / RECORD];
        let width = f(a, 24);
        assert_eq!(row.run_round_width(false).to_bits(), width.to_bits(), "vsync {}: width", word(a, 4));
        let roll = word(a, 40) != 0;
        let spot = |i: usize, o: usize| Some((int(a, i), [f(a, o), f(a, o + 4)]));
        let side = |w: Option<f32>| {
            stand_side(spot(12, 48), spot(16, 56), [f(a, 32), f(a, 36)], f(a, 20), f(a, 28), f(a, 64), word(a, 44) as u8, w)
        };
        let got = side(roll.then_some(width));
        let want = Some(int(b, 12) >= 1);
        assert_eq!(got, want, "vsync {} AI {:#x}: {:?}", word(a, 4), word(a, 8), &a[..0x50]);
        checked += 1;
        if roll {
            if side(None) != got {
                ran += 1;
            } else {
                kept_nearer_on_roll += 1;
            }
        }
    }
    eprintln!("{checked} sides checked, {ran} ran round, {kept_nearer_on_roll} rolls kept the nearer side");
    assert!(checked >= 30 && ran > 0, "{checked} sides, {ran} run-rounds");
}

/// Every aim choice of the slot-5 bots (`context/fixtures/ai_aim_s05.bin`, tools/record_ai_aim.py; skipped when
/// absent): from the AI object, the four players' spots and the generator at the doubles chooser's entry, the stick,
/// the plan byte and the generator's index at its exit must come out as the game's.
#[test]
fn aims_match_the_game() {
    use hst_sim::aim::Pair;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Some((_, csv)), Ok(d)) = (load(), std::fs::read(format!("{root}/context/fixtures/ai_aim_s05.bin"))) else {
        return eprintln!("disc or ai_aim_s05.bin missing, skipped");
    };
    let table = AiParams::table(&csv);
    let f = |b: &[u8], o: usize| f32::from_bits(word(b, o) as u32);
    let at = |b: &[u8], o: usize| [f(b, o), f(b, o + 4)];
    let mut kinds = [0; 5];
    let recs: Vec<&[u8]> = d.chunks_exact(0xc90).collect();
    for w in recs.chunks_exact(2) {
        let (a, b) = (w[0], w[1]);
        assert_eq!((word(a, 0), word(b, 0), word(a, 8)), (3, 0x13, word(b, 8)), "unpaired at vsync {}", word(a, 4));
        let ai = &a[0x10..0x290];
        let row = &table[(word(ai, 0xc) - TABLE) / RECORD];
        let l = Pair {
            me: at(a, 0x290),
            mate: at(a, 0x298),
            opp: [at(a, 0x2a0), at(a, 0x2a8)],
            side: f(a, 0x2b0),
            singles: ai[0x38] != 0,
            kind: ai[0x9a],
            volley_level: ai[0x5d],
            level: ai[0x10],
            formation: a[0x2b4],
            smash_third: word(ai, 0x28) != 0,
        };
        let mut mt = Mt::of(&a[0x2c0..]);
        let got = row.pair_aim(&l, &mut || mt.next());
        let want: Vec<u32> = (0..4).map(|k| word(b, 0x10 + 0xa0 + 4 * k) as u32).collect();
        let ctx = format!("vsync {} AI {:#x}: {l:?}", word(a, 4), word(a, 8));
        assert_eq!((got.stick.map(f32::to_bits).to_vec(), got.plan), (want, b[0x10 + 0xb2]), "{ctx}");
        assert_eq!(mt.1, word(&b[0x2c0..], 0x9c4), "{ctx}: draws");
        kinds[l.kind as usize] += 1;
    }
    eprintln!("aims by contact kind: {kinds:?}");
    assert!(kinds.iter().all(|&n| n > 0), "{kinds:?}");
}
