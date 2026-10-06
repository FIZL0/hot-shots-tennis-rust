//! Recorded game state, one sample per frame, as written by `tools/record_p2m2.py` while PCSX2 plays an input
//! recording. A sample is the vsync counter followed by fixed RAM regions: the pad manager, the match globals,
//! the match object, the ball object and two regions of each of the four player objects. Tests replay these
//! frames through the port and diff them; `cargo run -p hst-sim --bin replay` dumps them.

pub const GLOBALS_ADDR: usize = 0x422f80;
const PAD: usize = 4;
const GLOBALS: usize = PAD + 0x90;
const GM: usize = GLOBALS + 0x180;
const BALL: usize = GM + 0x100;
const PLAYERS: usize = BALL + 0x290;
/// Player object regions in the sample: +0x1380 (0x200 bytes), then +0x3c00 (0x400 bytes).
const PLAYER_LEN: usize = 0x200 + 0x400;
pub const SAMPLE: usize = PLAYERS + 4 * PLAYER_LEN;

/// Decoded pad state the game reads (stored active-high; libpad bits: 0x8 Start, 0x10 Up, 0x20 Right, 0x40 Down,
/// 0x80 Left, 0x400 L1, 0x800 R1, 0x1000 Triangle, 0x2000 Circle, 0x4000 Cross, 0x8000 Square).
/// Sticks are 0..255 with 0x80 centre (the game's deadzone 0x51..0xae already applied).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pad {
    pub buttons: u16,
    pub rx: u8,
    pub ry: u8,
    pub lx: u8,
    pub ly: u8,
}

#[derive(Clone, Copy)]
pub struct Frame<'a>(pub &'a [u8]);

/// Every whole sample in a fixture file.
pub fn frames(data: &[u8]) -> Vec<Frame<'_>> {
    data.chunks_exact(SAMPLE).map(Frame).collect()
}

fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

impl<'a> Frame<'a> {
    pub fn vsync(self) -> u32 {
        i32_at(self.0, 0) as u32
    }
    /// Port 0 or 1 (slot 0).
    pub fn pad(self, port: usize) -> Pad {
        let b = &self.0[PAD + 0x30 + port * 0x48..];
        Pad { buttons: u16::from_le_bytes([b[0], b[1]]), rx: b[2], ry: b[3], lx: b[4], ly: b[5] }
    }
    /// A word of the match globals by its RAM address (0x422f80..0x423100).
    pub fn global(self, addr: usize) -> i32 {
        i32_at(self.0, GLOBALS + addr - GLOBALS_ADDR)
    }
    pub fn global_u8(self, addr: usize) -> u8 {
        self.0[GLOBALS + addr - GLOBALS_ADDR]
    }
    /// The match object's first 0x100 bytes (phase gm+0x55, sub-phase gm+0x56, ...).
    pub fn gm(self) -> &'a [u8] {
        &self.0[GM..BALL]
    }
    /// The ball object recorded (`gm+0x98`: the path predictor, not the live ball).
    pub fn ball(self) -> &'a [u8] {
        &self.0[BALL..PLAYERS]
    }
    /// Player `i`'s field at object offset `off` (inside +0x1380..0x1580 or +0x3c00..0x4000).
    pub fn player_f32(self, i: usize, off: usize) -> f32 {
        let base = PLAYERS + i * PLAYER_LEN;
        match off {
            0x1380..0x1580 => f32_at(self.0, base + off - 0x1380),
            0x3c00..0x4000 => f32_at(self.0, base + 0x200 + off - 0x3c00),
            _ => panic!("player offset {off:#x} not recorded"),
        }
    }
    /// Player `i`'s position: translation row of its model matrix (+0x3d40), game space (Y down).
    pub fn player_pos(self, i: usize) -> [f32; 3] {
        [0x3d70, 0x3d74, 0x3d78].map(|o| self.player_f32(i, o))
    }
    /// Score block: points, games, sets per team.
    pub fn score(self) -> ([i32; 2], [i32; 2], [i32; 2]) {
        let g = |a| self.global(a);
        ([g(0x423064), g(0x423068)], [g(0x42306c), g(0x423070)], [g(0x423074), g(0x423078)])
    }
}
