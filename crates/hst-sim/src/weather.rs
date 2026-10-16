//! Match weather: the per-game schedule the game draws at a match start (weather and wind for up to 65 games,
//! with gusts), and what each weather changes in the court's look.
//!
//! Weathers: [`CLEAR`], [`CLOUDY`], [`RAIN`], [`HEAVY_RAIN`] ([`RAIN_2`] and [`HEAVY_RAIN_2`] are drawn like the
//! rains but never scheduled).
//! Rain (2 and 3) also makes running slower to pick up (`player::Stats::new`) and the dive thud wet
//! (`sound::dive_thud`).
#![deny(clippy::float_arithmetic)]

use crate::ps2::{self, F};

/// Games in a schedule; the game counter wraps around it.
pub const GAMES: usize = 65;

/// The weathers, by the game's numbers (`Game::weather`).
pub const CLEAR: u8 = 0;
pub const CLOUDY: u8 = 1;
pub const RAIN: u8 = 2;
pub const HEAVY_RAIN: u8 = 3;
pub const RAIN_2: u8 = 4;
pub const HEAVY_RAIN_2: u8 = 5;

/// Heavy rain, scheduled or not.
fn heavy(weather: u8) -> bool {
    matches!(weather, HEAVY_RAIN | HEAVY_RAIN_2)
}

/// Full gust speed, and the percent chance of a game's gust being full per gust kind (`Wind::kind` 0, 1, else).
const FULL_GUST: f32 = 9.0;
const FULL_GUST_CHANCE: [i32; 3] = [20, 50, 80];
/// A partial gust's speed lies between the base speed and this.
const GUST_TOP: f32 = 5.0;
/// 2⁻³²: a 32-bit draw into 0..1.
const U32_SCALE: f32 = 2.328_306_4e-10;

/// A court's weather odds (percent chances; spell lengths in games).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Odds {
    /// Chance per game of a cloudy spell starting, and its length range.
    pub cloudy: i32,
    pub cloudy_len: [i32; 2],
    /// Chance of one rain spell in the match, and its length range.
    pub rain: i32,
    pub rain_len: [i32; 2],
    /// No cloudy game either side of the rain spell.
    pub no_border: bool,
    /// The rain is heavy (`HEAVY_RAIN`).
    pub heavy: bool,
}

/// A court's wind: the chance per set of a gusty set, how gusty (0..2), the directions it may blow from
/// (degrees; none allowed blows from 0) and its base speed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wind {
    pub chance: i32,
    pub kind: i32,
    pub directions: Vec<f32>,
    pub speed: f32,
}

/// One game's weather.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Game {
    /// `CLEAR`, `CLOUDY`, `RAIN`, …
    pub weather: u8,
    /// Wind speed (0 calm; a full gust is 9) and the direction it blows from (degrees).
    pub speed: f32,
    pub degrees: f32,
}

pub use crate::rng::{Mt, Rand};

/// A 15-bit draw: bits 16..30 of the 32-bit output.
fn r15(rand: &mut impl FnMut() -> u32) -> i32 {
    ((rand() >> 16) & 0x7fff) as i32
}

/// A percent chance: a draw's 15 bits mod 100 below `percent`.
fn chance(rand: &mut impl FnMut() -> u32, percent: i32) -> bool {
    r15(rand) % 100 < percent
}

/// The match's schedule: `games` and `sets` are the games and sets needed to win, `players` 2 or 4 (1 = no match:
/// clear and the base wind throughout). `rand` is the game's random source (its 32-bit output; the 15-bit draws
/// are bits 16..30): the game's is `Mt::new(seed).next()`.
pub fn schedule(odds: &Odds, wind: &Wind, games: i32, sets: i32, players: i32, mut rand: impl FnMut() -> u32) -> [Game; GAMES] {
    let mut out = [Game::default(); GAMES];
    let (per_set, sets) = ((games * 2 + 1).max(1) as usize, (sets * 2 - 1).max(1) as usize);
    if players > 1 {
        let mut i = 0;
        while i < GAMES {
            if chance(&mut rand, odds.cloudy) {
                let len = odds.cloudy_len[0] + r15(&mut rand) % (odds.cloudy_len[1] - odds.cloudy_len[0] + 1);
                let n = (len.max(0) as usize).min(GAMES - i);
                out[i..i + n].iter_mut().for_each(|g| g.weather = CLOUDY);
                i += n;
            }
            i += 1;
        }
        if chance(&mut rand, odds.rain) {
            let start = r15(&mut rand) as usize % (per_set * sets);
            let len = (odds.rain_len[0] + r15(&mut rand) % (odds.rain_len[1] - odds.rain_len[0] + 1)) as usize;
            out.iter_mut().skip(start).take(len).for_each(|g| g.weather = if odds.heavy { HEAVY_RAIN } else { RAIN });
            if !odds.no_border {
                if start > 0 {
                    out[start - 1].weather = CLOUDY;
                }
                if start + len < GAMES {
                    out[start + len].weather = CLOUDY;
                }
            }
        }
    }
    let dirs: &[f32] = if wind.directions.is_empty() { &[0.0] } else { &wind.directions };
    let base = if wind.chance > 0 { wind.speed } else { 0.0 };
    for g in &mut out {
        g.speed = base;
        g.degrees = dirs[r15(&mut rand) as usize % dirs.len()];
    }
    if players > 1 {
        let full = FULL_GUST_CHANCE[(wind.kind as u32).min(2) as usize];
        for s in 0..sets {
            if chance(&mut rand, wind.chance) {
                for g in out.iter_mut().skip(s * per_set).take(per_set) {
                    g.speed = if chance(&mut rand, full) {
                        FULL_GUST
                    } else {
                        // adda base; madd (5 − base)·(2⁻³²·u)
                        let u = U32_SCALE * F(ps2::utof(rand()));
                        (F::ZERO + wind.speed).madd(GUST_TOP - F(wind.speed), u).0
                    };
                    g.degrees = dirs[r15(&mut rand) as usize % dirs.len()];
                }
            }
        }
    }
    out
}

/// What a weather does to the court's fog and light. The game's look row per weather is
/// `[sky, bg, far, near z, far z, light]`: the first five override the court's own fog values when ≥ 0
/// (the first three as how much fog: F = (1 − v)·255), the last scales the light and the fog colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// The main fog's far F and its z range.
    pub far: Option<f32>,
    pub z: [Option<f32>; 2],
    /// The background models' and the sky's F (their fog is flat: F at every depth).
    pub bg: Option<f32>,
    pub sky: Option<f32>,
    /// Scales the light intensity and the fog colour.
    pub scale: f32,
    /// Rain: the light turns grey (its RGB mean) and the fog white.
    pub grey: bool,
}

impl Look {
    /// Clear weather: the court's own fog and light.
    pub const CLEAR: Look = Look { far: None, z: [None, None], bg: None, sky: None, scale: 1.0, grey: false };
}

pub fn look(weather: u8, row: [f32; 6]) -> Look {
    let f = |v: f32| (v >= 0.0).then(|| ((1.0 - F(v)) * 255.0).0);
    let v = |v: f32| (v >= 0.0).then_some(v);
    Look { sky: f(row[0]), bg: f(row[1]), far: f(row[2]), z: [v(row[3]), v(row[4])], scale: row[5], grey: weather >= RAIN }
}

/// The court shadow's strength S (0..255) in this weather.
pub fn shadow(s: u32, weather: u8) -> u32 {
    match weather {
        CLOUDY => (F(s as f32) * 0.75).0 as u32,
        RAIN => (F(s as f32) * 0.5).0 as u32,
        HEAVY_RAIN => (F(s as f32) * 0.25).0 as u32,
        _ => s,
    }
}

/// Rain: the players cast a round blob instead of their projected shadow.
pub fn rain(weather: u8) -> bool {
    matches!(weather, RAIN | HEAVY_RAIN)
}

/// The sky clouds' layout in this weather, in cloud units (×40 when drawn): (radius, lowest y, highest y, count
/// or `None` for the court's own count). Heavy rain packs `rain` (`exe::Game::rain_clouds`: 200 low clouds).
pub fn cloud_layout(weather: u8, rain: (usize, [f32; 3])) -> (f32, f32, f32, Option<usize>) {
    let (n, [r, y0, y1]) = rain;
    if heavy(weather) { (r, y0, y1, Some(n)) } else { (50.0, -5.0, -10.0, None) }
}

/// A new point's weather change from `old` to `new` remakes the clouds (singles and doubles alike), unless both
/// are heavy rain.
pub fn clouds_remade(old: u8, new: u8) -> bool {
    old != new && !(heavy(old) && heavy(new))
}

/// The clouds' colour scale.
pub fn cloud_tint(weather: u8) -> f32 {
    match weather {
        RAIN | RAIN_2 => 0.8,
        HEAVY_RAIN | HEAVY_RAIN_2 => 0.55,
        _ => 1.0,
    }
}

/// Which background categories draw: (`_acc` accessories, `_clo` background clouds, the sun's lens flare). The
/// `_bg` models and the sky always do.
pub fn shows(weather: u8) -> (bool, bool, bool) {
    (!heavy(weather), weather < RAIN, weather < RAIN)
}

/// π/180, and 1/60 (per second → per frame).
const DEGREE: f32 = 0.017453292;
const PER_FRAME: f32 = 0.016666668;

/// The wind as the game sets it at each game: the row of a turn about Y by `degrees` (wrapped into ±π) that points
/// where it blows, times `speed` / 60 (metres a frame). Footstep puffs drift by it.
pub fn wind(degrees: f32, speed: f32) -> [f32; 4] {
    use crate::world::{PI, TWO_PI};
    const NEG_PI: f32 = -PI;
    let mut a = F(degrees) * DEGREE;
    if PI < a {
        a = a - TWO_PI;
    } else if a < NEG_PI {
        a = a + TWO_PI;
    }
    let s = F(speed) * PER_FRAME;
    crate::world::rot_y(a.0)[2].map(|v| (F(v) * s).0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_matches_game() {
        // PINE, slot 5 (court 10) forced to weather 3: far 0x42cbffff, bg 76.5, sky 0, z 40..150, light ×0.65
        let l = look(3, [1.0, 0.7, 0.6, 40.0, 150.0, 0.65]);
        assert_eq!((l.sky, l.bg, l.far, l.z), (Some(0.0), Some(76.5), Some(f32::from_bits(0x42cb_ffff)), [Some(40.0), Some(150.0)]));
        assert!(l.grey && l.scale == 0.65);
        let l = look(0, [-1.0, -1.0, -1.0, -1.0, -1.0, 1.0]);
        assert_eq!((l.sky, l.far, l.z, l.grey), (None, None, [None, None], false));
        assert_eq!((shadow(76, 1), shadow(76, 2), shadow(76, 3), shadow(76, 4)), (57, 38, 19, 76));
    }

    #[test]
    fn schedule_shapes() {
        let mut x = 0x2468_ace1u32;
        let mut rand = || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x
        };
        // court 11: cloudy 20%, rain 30% (2..4 games, heavy, cloudy either side)
        let odds = Odds { cloudy: 20, cloudy_len: [2, 3], rain: 30, rain_len: [2, 4], no_border: false, heavy: true };
        let wind = Wind { chance: 2, kind: 2, directions: vec![180.0, 270.0], speed: 2.0 };
        let mut rained = 0;
        for _ in 0..200 {
            let s = schedule(&odds, &wind, 4, 1, 4, &mut rand);
            assert!(s.iter().all(|g| g.weather <= 3 && (g.speed == 2.0 || g.speed == 9.0 || (2.0..5.0).contains(&g.speed))));
            assert!(s.iter().all(|g| g.degrees == 180.0 || g.degrees == 270.0));
            // gusts only reach the one set's 9 games
            assert!(s[9..].iter().all(|g| g.speed == 2.0));
            if let Some(a) = s.iter().position(|g| g.weather == 3) {
                rained += 1;
                let b = a + s[a..].iter().take_while(|g| g.weather == 3).count();
                assert!(a < 9 && (2..=4).contains(&(b - a)) && (a == 0 || s[a - 1].weather == 1) && s[b].weather == 1);
            }
        }
        assert!((30..90).contains(&rained));
        // no match: clear throughout, calm if the court has no wind
        let s = schedule(&odds, &Wind::default(), 4, 1, 1, &mut rand);
        assert!(s.iter().all(|g| *g == Game::default()));
    }

    #[test]
    fn wind_vector() {
        // slot 5 (court 10, 315°, speed 2): the game's +0x1a20 row as recorded in foot_s05.bin
        assert_eq!(wind(315.0, 2.0).map(f32::to_bits), [0xbcc1_165e, 0, 0x3cc1_1653, 0]);
    }
}
