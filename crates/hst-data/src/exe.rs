//! Tuning data that only exists inside the game program (the GAME overlay, `ZZBIN/GAME.BIN`), read from the
//! user's own disc at runtime.
//!
//! This is the one place that knows retail locations, and only for *data*: each read names what it is, the
//! disc is identified first (US 1.00, SCUS-97610), and nothing here is code addresses or game logic.

use crate::xb::Error;

/// SYSTEM.CNF boot line of the supported disc.
const BOOT: &str = "SCUS_976.10";
/// The GAME overlay: Metrowerks header "MWo3", overlay id 2, 0x100280 bytes, loaded at 0x322d00.
const GAME_SIZE: usize = 0x10_0280;
const GAME_LOAD: u32 = 0x32_2d00;

/// The GAME overlay of the supported disc.
pub struct Game<'a> {
    bin: &'a [u8],
}

impl<'a> Game<'a> {
    /// `system_cnf` and `game_bin` are `SYSTEM.CNF` and `ZZBIN/GAME.BIN` from the disc.
    pub fn new(system_cnf: &[u8], game_bin: &'a [u8]) -> Result<Self, Error> {
        if !String::from_utf8_lossy(system_cnf).contains(BOOT) {
            return Err(Error("unsupported disc: only Hot Shots Tennis USA (SCUS-97610) is known".into()));
        }
        let ok = game_bin.len() == GAME_SIZE
            && game_bin.get(..4) == Some(b"MWo3")
            && game_bin.get(4..8) == Some(&2u32.to_le_bytes())
            && game_bin.get(8..12) == Some(&GAME_LOAD.to_le_bytes());
        if !ok {
            return Err(Error("GAME.BIN does not match the supported US 1.00 overlay".into()));
        }
        Ok(Self { bin: game_bin })
    }

    fn at(&self, addr: u32, len: usize) -> &'a [u8] {
        let o = (addr - GAME_LOAD) as usize;
        &self.bin[o..o + len]
    }
    fn f32s(&self, addr: u32, n: usize) -> Vec<f32> {
        self.at(addr, n * 4).chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect()
    }
    fn f32(&self, addr: u32) -> f32 {
        self.f32s(addr, 1)[0]
    }

    /// Shot parameter source data (see `hst_sim::params`).
    pub fn shot_params(&self) -> ShotParamSource {
        ShotParamSource {
            // per shot class: 0x451 floats = up to 5 kinds × 0xdd floats = 17 records × 13 floats
            base: self.f32s(0x40_4810, 4 * 0x451),
            kinds: self.at(0x40_3b90, 16).chunks_exact(4).map(|c| i32::from_le_bytes(c.try_into().unwrap())).collect(),
            // 17 rows of 0x7c bytes; rows 3..16 (= character + 3) hold the blend weights
            weights: self.f32s(0x40_3f60, 17 * 0x7c / 4),
            middle_mix: self.f32(0x41_0a18),
        }
    }

    /// Tolerance the line calls add to every court line (metres).
    pub fn line_margin(&self) -> f32 {
        self.f32(0x40_3bec)
    }

    /// How the ball bounces off each collision material id (256 rows of 16 bytes).
    pub fn surfaces(&self) -> Vec<Surface> {
        self.at(0x41_0b70, 256 * 16)
            .chunks_exact(16)
            .map(|r| Surface {
                court: r[0] != 0,
                special: r[1] != 0,
                restitution: f32::from_le_bytes(r[4..8].try_into().unwrap()),
                spin_loss: f32::from_le_bytes(r[8..12].try_into().unwrap()),
            })
            .collect()
    }

    /// Metres the analog stick moves the aim point at full deflection.
    pub fn stick_reach(&self) -> f32 {
        self.f32(0x40_3bd0)
    }

    /// How long the scoreboard takes over the score after a point.
    pub fn scoreboard_timing(&self) -> ScoreboardTiming {
        let i = |a| i32::from_le_bytes(self.at(a, 4).try_into().unwrap());
        ScoreboardTiming {
            point_wait: i(0x41_0e90),
            game_wait: i(0x41_0e98),
            point_hold: self.f32(0x41_0ea8),
            game_hold: self.f32(0x41_0eb0),
            game_rise: i(0x41_0eb8),
            game_drop: i(0x41_0ec0),
        }
    }
}

/// Scoreboard timing after a point (frames unless noted).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScoreboardTiming {
    /// Wait before a point's score is shown, when no player is mid-reaction.
    pub point_wait: i32,
    /// Wait before a game's or set's score is shown.
    pub game_wait: i32,
    /// Seconds a point (or tiebreak point) score stays up.
    pub point_hold: f32,
    /// Seconds a game or set score stays up.
    pub game_hold: f32,
    /// Game/set show: frames the new score rises, then falls.
    pub game_rise: i32,
    pub game_drop: i32,
}

/// Raw inputs of the shot parameter table builder.
pub struct ShotParamSource {
    pub base: Vec<f32>,
    pub kinds: Vec<i32>,
    pub weights: Vec<f32>,
    pub middle_mix: f32,
}

/// One row of the collision material table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    /// Bounces by the court's own surface table (the playing surface).
    pub court: bool,
    /// Soft obstacle (the net): counted separately, the first touch kills most of the slide.
    pub special: bool,
    pub restitution: f32,
    /// Share of spin lost per bounce (and how fast spin relaxes) off the court surface.
    pub spin_loss: f32,
}
