//! Function oracle: a small R5900 (PS2 EE) interpreter that calls one of the original's functions with chosen
//! arguments and stops at its return, so tests can compare a port against the original on many inputs with
//! no emulator running.
//!
//! The code comes from the user's own disc at test time (the boot ELF and the GAME overlay, disc checked by
//! `hst_data::exe::Game`), optionally over a RAM dump from `context/ram/` for functions that read game state.
//! Function addresses live in `research/oracle.txt` (research notes), never in this crate.
//!
//! Covered: EE integer ops (64-bit GPRs, the 128-bit ones only for `lq`/`sq` and a few MMI moves), COP1 through
//! `hst_sim::ps2` (PCSX2's FPU model). Not timing, frame flow, VU0 or hardware registers: an op it does not know
//! panics with its address, so add it here when a function needs it.

use hst_sim::ps2;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const RAM: usize = 32 << 20;
/// Return address planted in `ra`: reaching it ends the call.
const DONE: u32 = 0xffff_fff0;

pub struct Ee {
    pub gpr: [u128; 32],
    pub hi: u64,
    pub lo: u64,
    pub hi1: u64,
    pub lo1: u64,
    /// FPU registers as raw bits (`mfc1`/`mtc1` move bits).
    pub fpr: [u32; 32],
    pub acc: u32,
    pub cond: bool,
    pub pc: u32,
    npc: u32,
    pub ram: Vec<u8>,
    pub spr: Vec<u8>,
    /// Every byte address written since the last `call` began.
    pub written: BTreeSet<u32>,
    /// Instructions run by the last `call`.
    pub steps: u64,
}

/// The repo root (for `context/` and `research/`).
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Function addresses by name from `research/oracle.txt` (`name hexaddr` per line, `#` comments).
pub fn addr(name: &str) -> u32 {
    let t = std::fs::read_to_string(root().join("research/oracle.txt")).expect("research/oracle.txt");
    t.lines()
        .filter_map(|l| l.split('#').next()?.split_once(char::is_whitespace))
        .find(|(n, _)| *n == name)
        .map(|(_, a)| u32::from_str_radix(a.trim().trim_start_matches("0x"), 16).unwrap())
        .unwrap_or_else(|| panic!("{name} not in research/oracle.txt"))
}

fn sx32(v: u32) -> u64 {
    v as i32 as i64 as u64
}

/// PCSX2 reads FPU operands with Inf/NaN as ±max and denormals as ±0.
fn fd(b: u32) -> f32 {
    match b & 0x7f80_0000 {
        0x7f80_0000 => f32::from_bits((b & 0x8000_0000) | 0x7f7f_ffff),
        0 => f32::from_bits(b & 0x8000_0000),
        _ => f32::from_bits(b),
    }
}

impl Ee {
    /// The boot program and the GAME overlay as loaded in a match, from `context/iso/`; `None` without a disc.
    pub fn game() -> Option<Self> {
        let iso = root().join("context/iso");
        let (cnf, boot, game) = (
            std::fs::read(iso.join("SYSTEM.CNF")).ok()?,
            std::fs::read(iso.join("SCUS_976.10")).ok()?,
            std::fs::read(iso.join("ZZBIN/GAME.BIN")).ok()?,
        );
        hst_data::exe::Game::new(&cnf, &game).expect("supported disc");
        let mut ee = Self::blank(vec![0; RAM]);
        // boot ELF: program headers' PT_LOAD file bytes (bss stays zero)
        let u = |o: usize| u32::from_le_bytes(boot[o..o + 4].try_into().unwrap()) as usize;
        let (phoff, phn) = (u(0x1c), u16::from_le_bytes([boot[0x2c], boot[0x2d]]) as usize);
        for k in 0..phn {
            let p = phoff + 32 * k;
            if u(p) == 1 && u(p + 16) > 0 {
                let (off, va, n) = (u(p + 4), u(p + 8), u(p + 16));
                ee.ram[va..va + n].copy_from_slice(&boot[off..off + n]);
            }
        }
        let at = 0x32_2d00;
        ee.ram[at..at + game.len()].copy_from_slice(&game);
        Some(ee)
    }

    /// A 32 MB EE RAM dump (`context/ram/<name>`) as the machine's memory, its GAME overlay checked against the
    /// disc's; `None` when either is missing.
    pub fn from_ram(name: &str) -> Option<Self> {
        let ram = std::fs::read(root().join("context/ram").join(name)).ok()?;
        let disc = Self::game()?;
        let at = 0x32_2d00 + 0x100; // code, past the overlay header
        assert_eq!(ram[at..at + 0x1000], disc.ram[at..at + 0x1000], "{name} holds a different GAME overlay");
        Some(Self::blank(ram))
    }

    fn blank(ram: Vec<u8>) -> Self {
        assert_eq!(ram.len(), RAM);
        Self {
            gpr: [0; 32],
            hi: 0,
            lo: 0,
            hi1: 0,
            lo1: 0,
            fpr: [0; 32],
            acc: 0,
            cond: false,
            pc: 0,
            npc: 0,
            ram,
            spr: vec![0; 0x4000],
            written: BTreeSet::new(),
            steps: 0,
        }
    }

    /// Call `addr` with integer args in a0.. and float args in f12.. and run to its return (panics after
    /// `budget` instructions). Results: `v0()`, `f0()`, `written`, memory reads.
    pub fn call(&mut self, addr: u32, ints: &[u64], floats: &[f32], budget: u64) {
        for (k, &v) in ints.iter().enumerate() {
            self.set(4 + k, v);
        }
        for (k, &v) in floats.iter().enumerate() {
            self.fpr[12 + k] = v.to_bits();
        }
        self.set(29, sx32(0x01ff_c000)); // ponytail: fixed stack near the top of RAM; move if a dump uses it
        self.set(31, sx32(DONE));
        self.pc = addr;
        self.npc = addr + 4;
        self.written.clear();
        self.steps = 0;
        while self.pc != DONE {
            assert!(self.steps < budget, "call {addr:06x}: over {budget} steps (pc {:06x})", self.pc);
            self.step();
            self.steps += 1;
        }
    }

    pub fn v0(&self) -> u64 {
        self.r(2)
    }
    pub fn f0(&self) -> f32 {
        f32::from_bits(self.fpr[0])
    }

    pub fn r(&self, i: usize) -> u64 {
        self.gpr[i] as u64
    }
    /// Write the low 64 bits (the upper 64 are kept, as on the R5900).
    pub fn set(&mut self, i: usize, v: u64) {
        if i != 0 {
            self.gpr[i] = (self.gpr[i] & !(u64::MAX as u128)) | v as u128;
        }
    }

    /// (scratchpad?, offset) of an address.
    fn loc(&self, a: u32) -> (bool, usize) {
        if a >> 28 == 7 {
            (true, (a & 0x3fff) as usize)
        } else if (a & 0x0fff_ffff) < RAM as u32 && a & 0x1000_0000 == 0 {
            (false, (a & 0x0fff_ffff) as usize)
        } else {
            panic!("access to {a:08x} (pc {:06x}): not RAM", self.pc.wrapping_sub(4))
        }
    }
    pub fn read(&self, a: u32, n: usize) -> u128 {
        let (spr, o) = self.loc(a);
        let m = if spr { &self.spr } else { &self.ram };
        (0..n).fold(0u128, |v, k| v | (m[o + k] as u128) << (8 * k))
    }
    pub fn write(&mut self, a: u32, n: usize, v: u128) {
        let (spr, o) = self.loc(a);
        let m = if spr { &mut self.spr } else { &mut self.ram };
        for k in 0..n {
            m[o + k] = (v >> (8 * k)) as u8;
        }
        self.written.extend(a..a + n as u32);
    }
    pub fn f32_at(&self, a: u32) -> f32 {
        f32::from_bits(self.read(a, 4) as u32)
    }

    fn branch(&mut self, take: bool, target: u32, likely: bool) {
        if take {
            self.npc = target;
        } else if likely {
            self.pc = self.npc; // skip the delay slot
            self.npc += 4;
        }
    }

    fn step(&mut self) {
        let w = self.read(self.pc, 4) as u32;
        let here = self.pc;
        self.pc = self.npc;
        self.npc += 4;
        let (op, rs, rt, rd, sa, fun) =
            (w >> 26, (w >> 21 & 31) as usize, (w >> 16 & 31) as usize, (w >> 11 & 31) as usize, w >> 6 & 31, w & 63);
        let imm = w as u16 as i16 as i64 as u64;
        let (s, t) = (self.r(rs), self.r(rt));
        let ea = (s as u32).wrapping_add(imm as u32);
        let bt = self.pc.wrapping_add((imm << 2) as u32);
        let bad = || panic!("unknown op {w:08x} at {here:06x}");
        match op {
            0 => match fun {
                0 => self.set(rd, sx32((t as u32) << sa)),
                2 => self.set(rd, sx32((t as u32) >> sa)),
                3 => self.set(rd, sx32(((t as i32) >> sa) as u32)),
                4 => self.set(rd, sx32((t as u32) << (s & 31))),
                6 => self.set(rd, sx32((t as u32) >> (s & 31))),
                7 => self.set(rd, sx32(((t as i32) >> (s & 31)) as u32)),
                8 => self.npc = s as u32,
                9 => {
                    self.set(rd, sx32(self.pc + 4));
                    self.npc = s as u32;
                }
                0xa if t == 0 => self.set(rd, s),
                0xb if t != 0 => self.set(rd, s),
                0xa | 0xb | 0xf => {}
                0x10 => self.set(rd, self.hi),
                0x11 => self.hi = s,
                0x12 => self.set(rd, self.lo),
                0x13 => self.lo = s,
                0x14 => self.set(rd, t << (s & 63)),
                0x16 => self.set(rd, t >> (s & 63)),
                0x17 => self.set(rd, ((t as i64) >> (s & 63)) as u64),
                0x18 | 0x19 => {
                    let p = if fun == 0x18 { (s as i32 as i64 * t as i32 as i64) as u64 } else { (s as u32 as u64) * (t as u32 as u64) };
                    (self.lo, self.hi) = (sx32(p as u32), sx32((p >> 32) as u32));
                    self.set(rd, self.lo);
                }
                0x1a => (self.lo, self.hi) = div(s as u32, t as u32),
                0x1b => (self.lo, self.hi) = divu(s as u32, t as u32),
                0x20 | 0x21 => self.set(rd, sx32((s as u32).wrapping_add(t as u32))),
                0x22 | 0x23 => self.set(rd, sx32((s as u32).wrapping_sub(t as u32))),
                0x24 => self.set(rd, s & t),
                0x25 => self.set(rd, s | t),
                0x26 => self.set(rd, s ^ t),
                0x27 => self.set(rd, !(s | t)),
                0x2a => self.set(rd, ((s as i64) < (t as i64)) as u64),
                0x2b => self.set(rd, (s < t) as u64),
                0x2c | 0x2d => self.set(rd, s.wrapping_add(t)),
                0x2e | 0x2f => self.set(rd, s.wrapping_sub(t)),
                0x38 => self.set(rd, t << sa),
                0x3a => self.set(rd, t >> sa),
                0x3b => self.set(rd, ((t as i64) >> sa) as u64),
                0x3c => self.set(rd, t << (sa + 32)),
                0x3e => self.set(rd, t >> (sa + 32)),
                0x3f => self.set(rd, ((t as i64) >> (sa + 32)) as u64),
                _ => bad(),
            },
            1 => {
                let take = if rt & 1 == 0 { (s as i64) < 0 } else { (s as i64) >= 0 };
                if rt & 0x10 != 0 {
                    self.set(31, sx32(self.pc + 4));
                }
                match rt {
                    0 | 1 | 0x10 | 0x11 => self.branch(take, bt, false),
                    2 | 3 | 0x12 | 0x13 => self.branch(take, bt, true),
                    _ => bad(),
                }
            }
            2 | 3 => {
                if op == 3 {
                    self.set(31, sx32(self.pc + 4));
                }
                self.npc = (self.pc & 0xf000_0000) | (w & 0x03ff_ffff) << 2;
            }
            4 | 0x14 => self.branch(s == t, bt, op == 0x14),
            5 | 0x15 => self.branch(s != t, bt, op == 0x15),
            6 | 0x16 => self.branch(s as i64 <= 0, bt, op == 0x16),
            7 | 0x17 => self.branch(s as i64 > 0, bt, op == 0x17),
            8 | 9 => self.set(rt, sx32((s as u32).wrapping_add(imm as u32))),
            0xa => self.set(rt, ((s as i64) < imm as i64) as u64),
            0xb => self.set(rt, (s < imm) as u64),
            0xc => self.set(rt, s & (w & 0xffff) as u64),
            0xd => self.set(rt, s | (w & 0xffff) as u64),
            0xe => self.set(rt, s ^ (w & 0xffff) as u64),
            0xf => self.set(rt, sx32(w << 16)),
            0x11 => self.cop1(w, here),
            0x18 | 0x19 => self.set(rt, s.wrapping_add(imm)),
            0x1c => self.mmi(w, here),
            0x1e => {
                let v = self.read(ea & !15, 16);
                if rt != 0 {
                    self.gpr[rt] = v;
                }
            }
            0x1f => self.write(ea & !15, 16, self.gpr[rt]),
            0x20 => self.set(rt, self.read(ea, 1) as u8 as i8 as i64 as u64),
            0x21 => self.set(rt, self.read(ea, 2) as u16 as i16 as i64 as u64),
            0x23 => self.set(rt, sx32(self.read(ea, 4) as u32)),
            0x24 => self.set(rt, self.read(ea, 1) as u64),
            0x25 => self.set(rt, self.read(ea, 2) as u64),
            0x27 => self.set(rt, self.read(ea, 4) as u64),
            0x37 => self.set(rt, self.read(ea, 8) as u64),
            0x28 => self.write(ea, 1, t as u128),
            0x29 => self.write(ea, 2, t as u128),
            0x2b => self.write(ea, 4, t as u128),
            0x3f => self.write(ea, 8, t as u128),
            0x31 => self.fpr[rt] = self.read(ea, 4) as u32,
            0x39 => self.write(ea, 4, self.fpr[rt] as u128),
            0x22 | 0x26 | 0x1a | 0x1b => self.load_lr(op, rt, ea),
            0x2a | 0x2e | 0x2c | 0x2d => self.store_lr(op, t, ea),
            0x2f | 0x33 => {} // cache, pref
            _ => bad(),
        }
    }

    /// lwl/lwr/ldl/ldr (little-endian): the bytes from `ea` to its word's end / start.
    fn load_lr(&mut self, op: u32, rt: usize, ea: u32) {
        let n = if op >= 0x22 { 4 } else { 8 };
        let (al, sh) = (ea & !(n as u32 - 1), (ea as usize) & (n - 1));
        let left = op == 0x22 || op == 0x1a;
        let mut v = self.r(rt).to_le_bytes();
        for i in 0..n {
            if left && i <= sh {
                v[n - 1 - sh + i] = self.read(al + i as u32, 1) as u8;
            } else if !left && i >= sh {
                v[i - sh] = self.read(al + i as u32, 1) as u8;
            }
        }
        let v = u64::from_le_bytes(v);
        // the 32-bit forms sign-extend except lwr with a partial word
        self.set(rt, if n == 4 && (left || sh == 0) { sx32(v as u32) } else { v });
    }
    fn store_lr(&mut self, op: u32, t: u64, ea: u32) {
        let n = if op <= 0x2e && op != 0x2c && op != 0x2d { 4 } else { 8 };
        let (al, sh) = (ea & !(n as u32 - 1), (ea as usize) & (n - 1));
        let left = op == 0x2a || op == 0x2c;
        let b = t.to_le_bytes();
        for i in 0..n {
            if left && i <= sh {
                self.write(al + i as u32, 1, b[n - 1 - sh + i] as u128);
            } else if !left && i >= sh {
                self.write(al + i as u32, 1, b[i - sh] as u128);
            }
        }
    }

    fn cop1(&mut self, w: u32, here: u32) {
        let (fmt, ft, fs, fdn, fun) = (w >> 21 & 31, (w >> 16 & 31) as usize, (w >> 11 & 31) as usize, (w >> 6 & 31) as usize, w & 63);
        let bad = || panic!("unknown COP1 op {w:08x} at {here:06x}");
        match fmt {
            0 => self.set(ft, sx32(self.fpr[fs])),
            2 => self.set(ft, sx32(if fs == 31 { 0x0100_0001 | (self.cond as u32) << 23 } else { 0x2e00 })),
            4 => self.fpr[fs] = self.r(ft) as u32,
            6 => {}
            8 => {
                let bt = self.pc.wrapping_add(((w as u16 as i16 as i32) << 2) as u32);
                self.branch(self.cond == (ft & 1 == 1), bt, ft & 2 != 0);
            }
            0x10 => {
                let (a, b, acc) = (fd(self.fpr[fs]), fd(self.fpr[ft]), fd(self.acc));
                let (sb, tb) = (self.fpr[fs], self.fpr[ft]);
                let put = |e: &mut Self, v: f32| e.fpr[fdn] = v.to_bits();
                match fun {
                    0 => put(self, ps2::add(a, b)),
                    1 => put(self, ps2::sub(a, b)),
                    2 => put(self, ps2::mul(a, b)),
                    3 => put(self, ps2::div(a, b)),
                    4 => put(self, ps2::sqrt(b)),
                    5 => self.fpr[fdn] = sb & 0x7fff_ffff,
                    6 => self.fpr[fdn] = sb,
                    7 => self.fpr[fdn] = sb ^ 0x8000_0000,
                    0x16 => {
                        if tb & 0x7f80_0000 == 0 {
                            self.fpr[fdn] = ((sb ^ tb) & 0x8000_0000) | 0x7f7f_ffff;
                        } else {
                            put(self, ps2::div(a, ps2::sqrt(b)));
                        }
                    }
                    0x18 => self.acc = ps2::add(a, b).to_bits(),
                    0x19 => self.acc = ps2::sub(a, b).to_bits(),
                    0x1a => self.acc = ps2::mul(a, b).to_bits(),
                    0x1c => put(self, ps2::madd(acc, a, b)),
                    0x1d => put(self, ps2::msub(acc, a, b)),
                    0x1e => self.acc = ps2::madd(acc, a, b).to_bits(),
                    0x1f => self.acc = ps2::msub(acc, a, b).to_bits(),
                    0x24 => {
                        self.fpr[fdn] = if sb & 0x7f80_0000 <= 0x4e80_0000 {
                            a as i32 as u32 // truncates
                        } else if sb & 0x8000_0000 != 0 {
                            0x8000_0000
                        } else {
                            0x7fff_ffff
                        }
                    }
                    0x28 => put(self, if a >= b { a } else { b }),
                    0x29 => put(self, if a <= b { a } else { b }),
                    0x30 => self.cond = false,
                    0x32 => self.cond = a == b,
                    0x34 => self.cond = a < b,
                    0x36 => self.cond = a <= b,
                    _ => bad(),
                }
            }
            0x14 if fun == 0x20 => self.fpr[fdn] = ps2::chop(self.fpr[fs] as i32 as f64).to_bits(),
            _ => bad(),
        }
    }

    fn mmi(&mut self, w: u32, here: u32) {
        let (rs, rt, rd, sa, fun) = ((w >> 21 & 31) as usize, (w >> 16 & 31) as usize, (w >> 11 & 31) as usize, w >> 6 & 31, w & 63);
        let (s, t) = (self.r(rs), self.r(rt));
        let (qs, qt) = (self.gpr[rs], self.gpr[rt]);
        let q = |e: &mut Self, v: u128| {
            if rd != 0 {
                e.gpr[rd] = v
            }
        };
        let lo64 = u64::MAX as u128;
        match (fun, sa) {
            (0x10, _) => self.set(rd, self.hi1),
            (0x11, _) => self.hi1 = s,
            (0x12, _) => self.set(rd, self.lo1),
            (0x13, _) => self.lo1 = s,
            (0x18, _) | (0x19, _) => {
                let p = if fun == 0x18 { (s as i32 as i64 * t as i32 as i64) as u64 } else { (s as u32 as u64) * (t as u32 as u64) };
                (self.lo1, self.hi1) = (sx32(p as u32), sx32((p >> 32) as u32));
                self.set(rd, self.lo1);
            }
            (0x1a, _) => (self.lo1, self.hi1) = div(s as u32, t as u32),
            (0x1b, _) => (self.lo1, self.hi1) = divu(s as u32, t as u32),
            (0x09, 0x0e) => q(self, (qt & lo64) | (qs & lo64) << 64), // pcpyld
            (0x09, 0x12) => q(self, qs & qt),                         // pand
            (0x09, 0x13) => q(self, qs ^ qt),                         // pxor
            (0x29, 0x0e) => q(self, (qt >> 64) | (qs >> 64) << 64),   // pcpyud
            (0x29, 0x12) => q(self, qs | qt),                         // por
            (0x29, 0x13) => q(self, !(qs | qt)),                      // pnor
            _ => panic!("unknown MMI op {w:08x} at {here:06x}"),
        }
    }
}

fn div(s: u32, t: u32) -> (u64, u64) {
    let (s, t) = (s as i32, t as i32);
    match t {
        0 => (if s < 0 { 1 } else { u64::MAX }, s as i64 as u64),
        -1 if s == i32::MIN => (sx32(s as u32), 0),
        _ => (sx32((s / t) as u32), sx32((s % t) as u32)),
    }
}
fn divu(s: u32, t: u32) -> (u64, u64) {
    if t == 0 { (u64::MAX, sx32(s)) } else { (sx32(s / t), sx32(s % t)) }
}
