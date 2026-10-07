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

    /// A character's shot variants (the `up1`/`dw1`/`dw2`/`dw3` trajectory tables): 0x48-byte entries, ended by
    /// a zero weight. The 13-float record in each entry is blank on disc; `hst_sim::params` builds it.
    pub fn shot_variants(&self, character: usize) -> Vec<ShotVariant> {
        let mut at = 0x40_8e60 + character as u32 * 0x8b8;
        let mut out = Vec::new();
        loop {
            let e = self.at(at, 0x10);
            let weight = f32::from_le_bytes(e[8..12].try_into().unwrap());
            if weight == 0.0 {
                return out;
            }
            let kind = i32::from_le_bytes(e[4..8].try_into().unwrap()) as usize;
            out.push(ShotVariant { class: e[0] as usize, kind, weight, uses: e[12..16].try_into().unwrap() });
            at += 0x48;
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

    /// Per court (13) how ball bounces look on it: the dust and the mark colours and whether dust puffs rise.
    pub fn bounce_looks(&self) -> Vec<BounceLook> {
        let rgb = |r: &[u8], o: usize| std::array::from_fn(|k| f32::from_le_bytes(r[o + 4 * k..o + 4 * k + 4].try_into().unwrap()));
        self.at(0x41_5000, 13 * 0x90).chunks_exact(0x90).map(|r| BounceLook { dust: rgb(r, 0), mark: rgb(r, 0x60), puffs: r[0x80] != 0 }).collect()
    }

    /// How far inside the court lines shots are pulled: per shot class (serve, stroke, volley, smash) and kind
    /// (0..4) the margins (across, along) for a shot mode ≥ 0 then < 0; and per character (0..13) the widest
    /// sideline margin a short angled topspin stroke can get.
    pub fn court_margins(&self) -> ([[[f32; 4]; 5]; 4], [f32; 14]) {
        let m = self.f32s(0x40_8d20, 80);
        let lines = std::array::from_fn(|c| std::array::from_fn(|k| std::array::from_fn(|j| m[c * 20 + k * 4 + j])));
        (lines, self.f32s(0x41_09c4, 14).try_into().unwrap())
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

    /// Court `court`'s (1-based) creature roster: which `npcNN` entry becomes which background figure, in the
    /// game's order (the first entry with a creature's name wins).
    pub fn npc_roster(&self, court: u32) -> Vec<NpcEntry> {
        let n = i32::from_le_bytes(self.at(0x41_27e0 + 4 * court, 4).try_into().unwrap()).max(0) as u32;
        (0..n)
            .filter_map(|k| {
                let e = self.at(0x41_2820 + court * 0xc0 + k * 12, 12);
                let name = u32::from_le_bytes(e[0..4].try_into().unwrap());
                (name != 0).then(|| {
                    let s = self.at(name, 16);
                    NpcEntry { name: String::from_utf8_lossy(&s[..s.iter().position(|&c| c == 0).unwrap_or(16)]).into(), kind: e[4] }
                })
            })
            .collect()
    }

    /// Court `court`'s six walking spectators (npc00..npc05): model number, animation set, and whether they also
    /// come out in doubles.
    pub fn walkers(&self, court: u32) -> [[u8; 3]; 6] {
        let t = self.at(0x41_39c0 + court * 0x12, 0x12);
        std::array::from_fn(|k| [t[3 * k], t[3 * k + 1], t[3 * k + 2]])
    }

    /// Court `court`'s wind (1..11): the directions it may blow from (degrees, the game picks one at random; none
    /// allowed = 180) and its speed (0 = calm).
    pub fn wind(&self, court: u32) -> (Vec<f32>, f32) {
        let row = self.at(0x41_cf90 + (court - 1) * 0x14, 0x14);
        let i32_at = |o: usize| i32::from_le_bytes(row[o..o + 4].try_into().unwrap());
        let degrees = self.at(0x41_d070, 32).chunks_exact(4).map(|c| i32::from_le_bytes(c.try_into().unwrap()) as f32);
        let allowed: Vec<f32> = degrees.zip(&row[8..16]).filter(|(_, f)| **f != 0).map(|(d, _)| d).collect();
        let speed = if i32_at(0) > 0 { i32_at(0x10) as f32 } else { 0.0 };
        (if allowed.is_empty() { vec![180.0] } else { allowed }, speed)
    }

    /// Trigger creature type `ty`'s (0..53) sound (program 7 key, −1 none) and, for the types that play it now and
    /// then, the mean gap between plays in ticks and the gap's jitter.
    pub fn emitter(&self, ty: u8) -> EmitterRow {
        let row = self.at(0x41_aec0 + 0x74 * ty as u32, 0x74);
        EmitterRow {
            sound: i32::from_le_bytes(self.at(0x41_c6e4 + 0x18 * ty as u32, 4).try_into().unwrap()),
            base: i16::from_le_bytes(row[0x54..0x56].try_into().unwrap()),
            jitter: f32::from_le_bytes(row[0x58..0x5c].try_into().unwrap()),
        }
    }

    /// A trigger creature type's parameter row (the part the creature engine uses).
    pub fn trigger(&self, ty: u8) -> TriggerRow {
        let r = self.at(0x41_aec0 + 0x74 * ty as u32, 0x74);
        let h = |o: usize| i16::from_le_bytes([r[o], r[o + 1]]);
        let f = |o: usize| f32::from_le_bytes(r[o..o + 4].try_into().unwrap());
        TriggerRow {
            start: r[0],
            mode: r[1],
            stage: h(8),
            exact: r[0xa] != 0,
            reverse_roll: r[0xc] != 0,
            reset_wait: (h(0xe), f(0x10)),
            end_wait: (h(0x14), f(0x18)),
            retarget: r[0x1c] != 0,
            speed: f(0x20),
            steer: f(0x24),
            every: (h(0x28), f(0x2c)),
            pause: (h(0x30), f(0x34)),
            snap: r[0x38] as i8,
            wake: r[0x3a] != 0,
            repeat: (h(0x3c), f(0x40)),
            orient: r[0x45],
            near: [f(0x48), f(0x4c)],
            stages: h(0x50),
            idle: (h(0x54), f(0x58)),
            sound: i32::from_le_bytes(self.at(0x41_c6e4 + 0x18 * ty as u32, 4).try_into().unwrap()),
        }
    }

    /// The deciding-set crowd voices (trigger type 44), in the order the creatures are made (later ones are
    /// silent): (pan in degrees, ticks between calls).
    pub fn deciding_voices(&self) -> [(i32, i32); 4] {
        let i = |a| i32::from_le_bytes(self.at(a, 4).try_into().unwrap());
        std::array::from_fn(|k| (i(0x42_2240 + 4 * k as u32), i(0x42_2250 + 4 * k as u32)))
    }

    /// How long the scoreboard takes over the score after a point.
    /// `language` and `umpire` pick the call sprite's countdowns (the US disc's match: language 0, umpire 4).
    pub fn scoreboard_timing(&self, language: u8, umpire: u8) -> ScoreboardTiming {
        let i = |a| i32::from_le_bytes(self.at(a, 4).try_into().unwrap());
        let calls = 0x41_0f0c + language as u32 * 0x14 + umpire as u32 * 0x28;
        ScoreboardTiming {
            call_wait: std::array::from_fn(|k| i(calls + 4 * k as u32)),
            point_wait: i(0x41_0e90),
            game_wait: i(0x41_0e98),
            point_hold: self.f32(0x41_0ea8),
            game_hold: self.f32(0x41_0eb0),
            game_rise: i(0x41_0eb8),
            game_drop: i(0x41_0ec0),
        }
    }

    /// The in-match panel's per-player colours, by player 0..3 (GS colour, 128 = the texture's own): the tint
    /// of each player's pill and slot label, and of their rank label. Two-player matches use the first two.
    pub fn hud_colours(&self) -> ([[u8; 3]; 4], [[u8; 3]; 4]) {
        let i = |a| i32::from_le_bytes(self.at(a, 4).try_into().unwrap()) as u8;
        // the tables list pink, orange, blue, green; players take pink, blue, orange, green
        let read = |base: u32| [0u32, 2, 1, 3].map(|k| std::array::from_fn(|c| i(base + 12 * k + 4 * c as u32)));
        (read(0x41_5930), read(0x41_5960))
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

/// One roster line: a creature entry name (`npc06`) and the figure it becomes — a trigger creature type (0..53),
/// a walking spectator (54..59 = npc00..npc05's own rows of [`Game::walkers`]), the umpire (60) or one of court 5's
/// own creatures (61..66).
#[derive(Clone, Debug, PartialEq)]
pub struct NpcEntry {
    pub name: String,
    pub kind: u8,
}

/// A trigger creature type's ambient sound: `sound` played every `base`·(1 − `jitter`·r) ticks, r uniform in [0, 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmitterRow {
    pub sound: i32,
    pub base: i16,
    pub jitter: f32,
}

/// A trigger creature type's parameters. Pairs are (ticks, jitter): the ticks are scaled by 1 − jitter·random.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriggerRow {
    /// Where a reset puts it: 0 home, 1 its start node, 2 a random point, 3 circling a node.
    pub start: u8,
    /// Path mode: 0 random point, 1 loop, 2 ping-pong, 3 cycle, 4 spline, 5 one-shot, 6 circle, 7 none.
    pub mode: u8,
    pub stage: i16,
    /// Waypoints are the nodes themselves (no random offset).
    pub exact: bool,
    /// A reset draws its direction along the path.
    pub reverse_roll: bool,
    /// Pause after a reset.
    pub reset_wait: (i16, f32),
    /// Pause at a loop's end.
    pub end_wait: (i16, f32),
    /// Retarget every `every.0` ticks of a leg.
    pub retarget: bool,
    /// Distance per tick (circling: degrees per tick).
    pub speed: f32,
    pub steer: f32,
    /// Leg length unit, and the pause taken every that many ticks.
    pub every: (i16, f32),
    pub pause: (i16, f32),
    /// Ground snap period (−1 snaps each waypoint, −2 the start).
    pub snap: i8,
    /// Restart the idle animation when a pause ends.
    pub wake: bool,
    /// Idle-animation repeat gap.
    pub repeat: (i16, f32),
    /// Face the direction of travel (2: level).
    pub orient: u8,
    /// Animation slows within these distances of the leg's start and end.
    pub near: [f32; 2],
    pub stages: i16,
    /// Gap between idle animations or sounds (the emitters' and deciding-set crowd's).
    pub idle: (i16, f32),
    /// Its sound (−1 none).
    pub sound: i32,
}

/// Scoreboard timing after a point (frames unless noted).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScoreboardTiming {
    /// Ticks the umpire's call sprite stays up at most, by the judge's call (1 out, 2 fault, 3 double fault,
    /// 4 let; 5 out after net reads past this umpire's row, as the game does); it settles earlier when her
    /// voice line ends.
    pub call_wait: [i32; 6],
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

/// One per-character shot variant: its record is the character's record of (class, kind) moved by |weight|
/// toward archetype 0 (weight > 0) or 2 (weight < 0). `uses[v]` is set when variant table v (0 up1, 1 dw1,
/// 2 dw2, 3 dw3) takes this record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShotVariant {
    pub class: usize,
    pub kind: usize,
    pub weight: f32,
    pub uses: [u8; 4],
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

/// One court's bounce effects look (colours 0..128 per channel).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BounceLook {
    pub dust: [f32; 3],
    pub mark: [f32; 3],
    pub puffs: bool,
}

/// Number of scripted camera shots.
pub const CAMERA_SHOTS: usize = 108;

/// One scripted camera shot.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraShot {
    pub record: Vec<u8>,
    pub script: Vec<(u8, f32)>,
}
