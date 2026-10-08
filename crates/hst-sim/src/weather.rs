//! Match weather: the per-game schedule the game draws at a match start (weather and wind for up to 65 games,
//! with gusts), and what each weather changes in the court's look.
//!
//! Weathers: 0 clear, 1 cloudy, 2 rain, 3 heavy rain (4 and 5 are drawn like 2 and 3 but never scheduled).
//! Rain (2 and 3) also makes running slower to pick up (`player::Stats::new`) and the dive thud wet
//! (`sound::dive_thud`).

/// Games in a schedule; the game counter wraps around it.
pub const GAMES: usize = 65;

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
    /// The rain is heavy (weather 3).
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Game {
    pub weather: u8,
    /// Wind speed (0 calm; a full gust is 9) and the direction it blows from (degrees).
    pub speed: f32,
    pub degrees: f32,
}

fn r15(rand: &mut impl FnMut() -> u32) -> i32 {
    ((rand() >> 16) & 0x7fff) as i32
}

/// The match's schedule: `games` and `sets` are the games and sets needed to win, `players` 2 or 4 (1 = no match:
/// clear and the base wind throughout). `rand` is the game's random source (its 32-bit output; the 15-bit draws
/// are bits 16..30).
/// ponytail: the game draws from its own Mersenne twister seeded at boot; any 32-bit source gives the same odds.
pub fn schedule(odds: &Odds, wind: &Wind, games: i32, sets: i32, players: i32, mut rand: impl FnMut() -> u32) -> [Game; GAMES] {
    let mut out = [Game::default(); GAMES];
    let (per_set, sets) = ((games * 2 + 1).max(1) as usize, (sets * 2 - 1).max(1) as usize);
    if players > 1 {
        let mut i = 0;
        while i < GAMES {
            if r15(&mut rand) % 100 < odds.cloudy {
                let len = odds.cloudy_len[0] + r15(&mut rand) % (odds.cloudy_len[1] - odds.cloudy_len[0] + 1);
                let n = (len.max(0) as usize).min(GAMES - i);
                out[i..i + n].iter_mut().for_each(|g| g.weather = 1);
                i += n;
            }
            i += 1;
        }
        if r15(&mut rand) % 100 < odds.rain {
            let start = r15(&mut rand) as usize % (per_set * sets);
            let len = (odds.rain_len[0] + r15(&mut rand) % (odds.rain_len[1] - odds.rain_len[0] + 1)) as usize;
            out.iter_mut().skip(start).take(len).for_each(|g| g.weather = if odds.heavy { 3 } else { 2 });
            if !odds.no_border {
                if start > 0 {
                    out[start - 1].weather = 1;
                }
                if start + len < GAMES {
                    out[start + len].weather = 1;
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
        let full = [20, 50].get(wind.kind as usize).copied().unwrap_or(80);
        for s in 0..sets {
            if r15(&mut rand) % 100 < wind.chance {
                for g in out.iter_mut().skip(s * per_set).take(per_set) {
                    g.speed = if r15(&mut rand) % 100 < full { 9.0 } else { (5.0 - wind.speed) * rand() as f32 * 2.3283064e-10 + wind.speed };
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
    let f = |v: f32| (v >= 0.0).then(|| (1.0 - v) * 255.0);
    let v = |v: f32| (v >= 0.0).then_some(v);
    Look { sky: f(row[0]), bg: f(row[1]), far: f(row[2]), z: [v(row[3]), v(row[4])], scale: row[5], grey: weather >= 2 }
}

/// The court shadow's strength S (0..255) in this weather.
pub fn shadow(s: u32, weather: u8) -> u32 {
    match weather {
        1 => (s as f32 * 0.75) as u32,
        2 => (s as f32 * 0.5) as u32,
        3 => (s as f32 * 0.25) as u32,
        _ => s,
    }
}

/// Rain: the players cast a round blob instead of their projected shadow.
pub fn rain(weather: u8) -> bool {
    weather.wrapping_sub(2) < 2
}

/// The sky clouds' layout in this weather, in cloud units (×40 when drawn): (radius, lowest y, highest y, count
/// or `None` for the court's own count). Heavy rain packs 200 low clouds.
pub fn cloud_layout(weather: u8) -> (f32, f32, f32, Option<usize>) {
    if weather == 3 || weather == 5 { (50.0, -2.0, -5.0, Some(200)) } else { (50.0, -5.0, -10.0, None) }
}

/// The clouds' colour scale.
pub fn cloud_tint(weather: u8) -> f32 {
    match weather {
        2 | 4 => 0.8,
        3 | 5 => 0.55,
        _ => 1.0,
    }
}

/// Which background categories draw: (`_acc` accessories, `_clo` background clouds, the sun's lens flare). The
/// `_bg` models and the sky always do.
pub fn shows(weather: u8) -> (bool, bool, bool) {
    (weather != 3 && weather != 5, weather < 2, weather < 2)
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
}
