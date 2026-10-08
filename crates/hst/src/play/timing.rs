//! Timing grade effects on a rally launch (`hst_sim::swing`'s timing): per player its character's timing stats and
//! the variant trajectory tables (up1/dw1/dw2/dw3) with their shot records; per stroke or volley the timing error
//! set up at contact, which at the launch scatters the aim and picks the table mode; per slow contact the mis-hit
//! roll (a dull hit's lower elevation, a framed hit's wild lob).

use hst_data::{iso::Iso, xb::Archive};
use hst_sim::ball::V3;
use hst_sim::params::{self, ShotParams};
use hst_sim::player::ReachStats;
use hst_sim::ps2;
use hst_sim::shot::{Bounds, Lookup, Table, lookup};
use hst_sim::swing::{self, TimingError};

use super::{Contact, Game, SWEET_FRAME};

/// A variant: (class, kind, variant), its table and its shot record.
type Variant = ((usize, usize, usize), Table, [f32; 13]);

/// One player's character: its timing stats, bias per frame from the press, the volley-class down-2 thresholds
/// and its listed variants ((class, kind, variant) with the table and the variant's shot record).
pub struct Timing {
    character: usize,
    stats: ReachStats,
    bias: Vec<i32>,
    down2: [f32; 14],
    variants: Vec<Variant>,
}

/// Character `c`'s timing data from the disc (TParam, the executable's variant list, the trajectory archives).
pub fn load(iso: &mut Iso, params: &ShotParams, c: usize) -> Timing {
    let stats = ReachStats::from_tparam(&super::tparam(iso, c).join(","));
    let (cnf, bin) = (iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"), iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"));
    let exe = hst_data::exe::Game::new(&cnf, &bin).expect("supported disc");
    let data = ["A", "B"].map(|ab| iso.read(&format!("TRAJ/TRAJ{c:02}{ab}.XB")).expect("trajectory archive"));
    let archives: Vec<Archive> = data
        .iter()
        .map(|d| Archive::parse(d).expect("xb archive"))
        .collect();
    let mut variants = vec![];
    for x in exe.shot_variants(c) {
        let stem = if x.class == 1 { "strk" } else { "voly" };
        for v in (0..4).filter(|&v| x.uses[v] == 1) {
            let file = format!("tr_pc{c:02}_{stem}{}{}.dat", x.kind, ["_up1", "_dw1", "_dw2", "_dw3"][v]);
            let table = archives.iter().find_map(|a| {
                let e = a.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&file))?;
                Table::parse(&a.read(e).ok()?)
            });
            let record = params.variant(x.class, x.kind, params::record_of(c), x.weight);
            // ponytail: a listed table missing from the archives (character 10's voly1_up1) launches from the base
            // table; what the original loads there is not looked into
            if let Some(table) = table {
                variants.push(((x.class, x.kind, v), table, record));
            }
        }
    }
    let bias = swing::timing(stats.after, stats.before).1;
    Timing { character: c, stats, bias, down2: exe.down2_blend(), variants }
}

/// The timing error of player `i`'s ground stroke or volley at contact `c` (`branch` 1 or 2), the lob button
/// `lob`.
// ponytail: the contact height is the ball's at the contact frame; dives take no error (their grade is set apart)
pub fn error(g: &Game, i: usize, c: &Contact, branch: u8, lob: bool) -> TimingError {
    let (t, p) = (&g.timing[i], &g.players[i]);
    let bias = t.bias.get((c.offset + SWEET_FRAME) as usize).copied().unwrap_or(0);
    let forehand = c.swing.forehand;
    let right = forehand == (p.hand >= 0.0);
    swing::timing_error(&t.stats, branch, lob, bias, c.offset, -c.swing.ball[1], right, forehand, c.swing.body, [0.0; 2], p.end)
}

/// Player `i`'s smash scatter (`swing::smash_scatter`) at contact `c`, `plain` (not the △ smash), from its aim's
/// nudge and held depth (`Game::aim`).
pub fn smash(g: &Game, i: usize, c: &Contact, plain: bool) -> (f32, f32) {
    let t = &g.timing[i];
    let bias = t.bias.get((c.offset + SWEET_FRAME) as usize).copied().unwrap_or(0);
    let scale = swing::smash_scale(plain, c.offset, g.players[i].aim_from[1]);
    swing::smash_scatter(&t.stats, c.grade, bias, -c.swing.ball[1], g.aim.nudge, scale, g.aim.held)
}

/// A timed launch: the scattered target, the table lookup and the variant's shot record
/// when the mode picked one.
pub struct Timed {
    pub target: V3,
    pub lookup: Lookup,
    pub record: Option<[f32; 13]>,
}

/// Player `who`'s mis-hit roll as it strikes a `grade` ground stroke (`branch` 1), volley (2) or dive (3) of `kind`
/// (swing animation `anim`, `forehand` side) from the ball where it is: tired by its stamina after the stroke's
/// cost, awkward on the body-shot swing or the other-hand side.
pub fn mis_hit(g: &mut Game, who: usize, (branch, grade, kind): (u8, u8, i32), anim: u8, forehand: bool) -> swing::MisHit {
    let p = &g.players[who];
    let rally = g.phase == super::Phase::Rally && g.players.len() > 1;
    let stamina = if rally { hst_sim::player::stroke_stamina(&p.stats, p.body.stamina, branch, forehand, 0) } else { p.body.stamina };
    let height = -g.flight.ball.pos[1];
    let rng = &mut g.rng.shared;
    swing::mis_hit(grade, branch, kind, stamina < 10, anim & 1 != 0 || anim == 0x1a, height, || rng.next())
}

/// A framed hit's lob for player `who`: (aim, side, depth error), as `swing::wild_aim`.
pub fn wild(g: &mut Game, who: usize) -> (V3, f32, f32) {
    let (doubles, end, rng) = (g.rules.players > 2, g.players[who].end, &mut g.rng.shared);
    swing::wild_aim(doubles, end, || rng.next())
}

/// What timing error `e` does to player `who`'s stroke (class 1) or volley (class 2) of `kind` from `at` toward the
/// pulled-in aim `target`: from `src`'s tables (the incoming hitter's on a `counter`, blend 0 then). A framed hit
/// launches with its own (side, depth) error instead, blend 0.
#[allow(clippy::too_many_arguments)]
pub fn launch(g: &Game, who: usize, src: usize, class: u8, kind: i32, (branch, grade): (u8, u8), e: TimingError, framed: Option<(f32, f32)>, counter: bool, at: V3, target: V3) -> Timed {
    let (t, s) = (&g.timing[who], &g.timing[src]);
    let (sx, sz, mut blend) = match framed {
        Some((side, depth)) => (side, depth, 0.0),
        None => swing::timing_launch(&t.stats, branch, kind, grade, e, g.aim.nudge, g.aim.short_only),
    };
    if branch == 1 && class == 2 {
        blend = swing::high_blend(&t.stats);
    }
    if counter {
        blend = 0.0;
    }
    let mode = swing::table_mode(class, t.character, grade, kind, blend, &t.down2);
    let variant = if kind == 3 { swing::lob_variant(&s.stats, grade) } else { swing::mode_variant(mode) };
    let (base, bounds) = if class == 1 { (0, Bounds::stroke(kind, at[2])) } else { (1, Bounds::volley(kind, at[2])) };
    let picked = variant.and_then(|v| s.variants.iter().find(|x| x.0 == (class as usize, kind as usize, v)));
    let table = picked.map_or(&g.rally_tables[src][base][kind as usize], |x| &x.1);
    let sc = hst_sim::serve::scatter_along(sx, sz, at, target);
    let l = lookup(table, &bounds, [ps2::sub(at[0], sc[0]), at[1], ps2::sub(at[2], sc[2])], target);
    Timed { target: [ps2::add(target[0], sc[0]), target[1], ps2::add(target[2], sc[2])], lookup: l, record: picked.map(|x| x.2) }
}
