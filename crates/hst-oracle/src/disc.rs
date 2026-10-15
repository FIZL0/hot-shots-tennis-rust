//! Getting a program into the machine: a PS2 ELF's loadable segments plus raw overlays at their load addresses.
//! No disc, serial or build knowledge here; the caller reads the files and says where they go.

use crate::Ee;

fn le32(b: &[u8]) -> u32 {
    u32::from_le_bytes(b[..4].try_into().unwrap())
}

/// A PS2 executable's loadable segments, as `(virtual address, bytes)`.
///
/// PS2 ELFs are 32-bit little-endian MIPS with everything in `PT_LOAD` segments at their final addresses (no
/// relocation). `memsz` beyond `filesz` is `.bss`, which starts zeroed, so only the file bytes are returned.
pub fn elf_segments(elf: &[u8]) -> Result<Vec<(u32, &[u8])>, String> {
    if elf.len() < 0x34 || &elf[..4] != b"\x7fELF" {
        return Err("not an ELF (no \\x7fELF magic)".into());
    }
    if elf[4] != 1 || elf[5] != 1 {
        return Err("not a 32-bit little-endian ELF, so not a PS2 executable".into());
    }
    let (phoff, phentsize, phnum) = (
        le32(&elf[0x1c..]) as usize,
        u16::from_le_bytes([elf[0x2a], elf[0x2b]]) as usize,
        u16::from_le_bytes([elf[0x2c], elf[0x2d]]) as usize,
    );
    let mut out = Vec::new();
    for k in 0..phnum {
        let p = phoff + phentsize.max(32) * k;
        if p + 32 > elf.len() {
            return Err("ELF program header table runs past the end of the file".into());
        }
        let (ptype, off, va, filesz) =
            (le32(&elf[p..]), le32(&elf[p + 4..]) as usize, le32(&elf[p + 8..]), le32(&elf[p + 16..]) as usize);
        if ptype == 1 && filesz > 0 {
            if off + filesz > elf.len() {
                return Err(format!("ELF segment at {va:#x} runs past the end of the file"));
            }
            out.push((va, &elf[off..off + filesz]));
        }
    }
    if out.is_empty() { Err("ELF has no loadable segments".into()) } else { Ok(out) }
}

impl Ee {
    /// Zeroed RAM with the boot ELF's segments loaded, then each `(load address, bytes)` overlay copied raw over
    /// it in order (what a game's own overlay loader does).
    pub fn from_program(elf: &[u8], overlays: &[(u32, &[u8])]) -> Result<Self, String> {
        let mut ee = Self::bare();
        for (va, bytes) in elf_segments(elf)? {
            ee.load(va, bytes);
        }
        for &(va, bytes) in overlays {
            ee.load(va, bytes);
        }
        Ok(ee)
    }
}
