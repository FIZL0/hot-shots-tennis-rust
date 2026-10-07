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
                effect: r[12],
                sound: r[13],
            })
            .collect()
    }

    /// Metres the analog stick moves the aim point at full deflection.
    pub fn stick_reach(&self) -> f32 {
        self.f32(0x40_3bd0)
    }

    /// The match's scripted camera shots (cut-aways after a point, court views): per shot number its 0xbc-byte
    /// record and its script of (op, value) pairs up to the end op 0x1a. The scripts are stored back to back.
    pub fn camera_shots(&self) -> Vec<CameraShot> {
        let mut at = 0x40_1d30;
        (0..CAMERA_SHOTS)
            .map(|n| {
                let mut script = Vec::new();
                loop {
                    let e = self.at(at, 8);
                    at += 8;
                    if e[0] == 0x1a {
                        break;
                    }
                    script.push((e[0], f32::from_le_bytes(e[4..8].try_into().unwrap())));
                }
                CameraShot { record: self.at(0x3f_cde0 + n as u32 * 0xbc, 0xbc).to_vec(), script }
            })
            .collect()
    }

    /// The shot numbers the post-point cut-away cycles through: a point, a game-ending point, a doubles team
    /// reaction.
    pub fn cutaway_lists(&self) -> [Vec<u8>; 3] {
        [self.at(0x3f_cce0, 6).to_vec(), self.at(0x3f_cce8, 4).to_vec(), self.at(0x3f_ccf0, 7).to_vec()]
    }

    /// The umpire's word countdowns in ticks (after this long her next word plays even if the last still
    /// sounds): score words (program 0, 14 keys) and announcements (program 2, 5 keys), for the game's language
    /// setting `language`, umpire `umpire` (0..5) and her voice set `set` (0..2).
    pub fn umpire_words(&self, language: u8, umpire: u8, set: u8) -> ([i32; 14], [i32; 5]) {
        let l = if language == 0xb { 5 } else { language as u32 };
        let (u, v) = (umpire as u32, set as u32);
        let i = |a, k: u32| i32::from_le_bytes(self.at(a + 4 * k, 4).try_into().unwrap());
        (
            std::array::from_fn(|k| i(0x41_3e20 + l * 0x230 + u * 0x70 + v * 0x38, k as u32)),
            std::array::from_fn(|k| i(0x41_4b40 + l * 0xc8 + u * 0x28 + v * 0x14, k as u32)),
        )
    }

    /// Per court (0..12): the umpire's chair is on the side that mirrors her turns.
    pub fn umpire_side(&self, court: usize) -> bool {
        self.at(0x41_3e08 + court as u32, 1)[0] != 0
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

/// The sound driver's pitch table (`snd::pitch`): 608 steps, 192 per octave, 0x1000 at the root note. Read from the
/// IOP sound driver `MODULES2/SG2IOPM1.IRX` of the supported disc.
pub fn pitch_table(irx: &[u8]) -> Result<Vec<u16>, Error> {
    if irx.len() != 28093 || !irx.windows(13).any(|w| w == b"sg2iop_driver") {
        return Err(Error("SG2IOPM1.IRX does not match the supported US 1.00 driver".into()));
    }
    Ok(irx[0x4880..0x4880 + 2 * 608].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect())
}

/// The sound library's voice volume tables (`snd::Level`), read from the main program `SCUS_976.10` of the supported
/// disc: `pan` 128 × u16 (high byte left gain, low byte right, 0x7f = full) and `gain` 129 × u16 (32767 · cos of
/// i · 90° / 128, the centre-panned tones' L/R gains).
pub fn sound_tables(elf: &[u8]) -> Result<(Vec<u16>, Vec<u16>), Error> {
    let u16s = |addr: usize, n: usize| -> Vec<u16> { boot_at(elf, addr, 2 * n).chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect() };
    check_boot(elf)?;
    Ok((u16s(0x1b_b880, 128), u16s(0x1b_b988, 129)))
}

/// The sound library's positional stereo tables (`hst_sim::sound::stereo`): left and right gains, 720 × i32 each,
/// 2048 · cos over half-degree steps, −4096 where the gain is zero. From `SCUS_976.10` of the supported disc.
pub fn stereo_tables(elf: &[u8]) -> Result<[Vec<i32>; 2], Error> {
    let i32s = |addr: usize| -> Vec<i32> { boot_at(elf, addr, 4 * 720).chunks_exact(4).map(|c| i32::from_le_bytes(c.try_into().unwrap())).collect() };
    check_boot(elf)?;
    Ok([i32s(0x1b_db78), i32s(0x1b_d038)])
}

/// The volume the sound library gives each of its 24 bank slots (slot 0 the court's SE bank: 118), by the slot's
/// category. From `SCUS_976.10` of the supported disc.
pub fn bank_volumes(elf: &[u8]) -> Result<Vec<u32>, Error> {
    let u32s = |addr: usize, n: usize| -> Vec<u32> { boot_at(elf, addr, 4 * n).chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect() };
    check_boot(elf)?;
    let volume = u32s(0x1b_bf00, 5);
    Ok(u32s(0x1c_e970, 24).into_iter().map(|c| volume[c as usize]).collect())
}

fn check_boot(elf: &[u8]) -> Result<(), Error> {
    if elf.len() != 864_720 || elf.get(..4) != Some(b"\x7fELF") || elf.get(0x3c..0x40) != Some(&0x10_0000u32.to_le_bytes()) {
        return Err(Error("SCUS_976.10 does not match the supported US 1.00 program".into()));
    }
    Ok(())
}

/// `n` bytes of the boot program at `addr`: one loadable segment from file offset 0x100 to vaddr 0x100000.
fn boot_at(elf: &[u8], addr: usize, n: usize) -> &[u8] {
    &elf[addr - 0x10_0000 + 0x100..][..n]
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
    /// Bounce effect kind (0 none; 2 a soft surface heard once in a row; 5 silent).
    pub effect: u8,
    /// Court program 2 key a bounce plays (0 none).
    pub sound: u8,
}

/// Number of scripted camera shots.
pub const CAMERA_SHOTS: usize = 108;

/// One scripted camera shot.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraShot {
    pub record: Vec<u8>,
    pub script: Vec<(u8, f32)>,
}
