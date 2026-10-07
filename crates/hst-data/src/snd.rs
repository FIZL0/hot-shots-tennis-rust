//! Sound banks: `.hd` headers ("SShd") beside `.bd` sample data (PS-ADPCM), and the SPU pitch the sound driver
//! sets for a note.
//!
//! ```text
//! .hd  u32 hd size, u32 bd size, u32 0, "SShd"
//!      i32 × 6 section offsets at 0x10 (-1 = absent); 0x1c sequences, 0x24 tone sets
//! sequences  u16 last program, u16 offset × programs           (offsets from the section start)
//!            program: u16 last key, u16 offset × keys          (offsets from the *section* start too)
//!            sequence: { event, varlen delta }* ending ff 2f 00
//!            events: a0|ch note velocity set (key-on with a tone set), 80/90/b0/e0 3 bytes, c0/d0 2, ff t len data
//! tone sets  u16 last set, u16 offset × sets
//!            set: 8-byte header (1 volume, 6 lowest note, 7 highest), then 16 bytes per note from the lowest
//!            tone: 0 group, 1 priority, 2 root note, 3 fine tune (i8), u16 sample address / 8 at 4,
//!                  u16 ADSR1 at 6, u16 ADSR2 at 8, a centre-pan flag, b volume, c pan, d e alternates, f flags
//! ```

use crate::xb::Error;

pub struct Bank<'a> {
    hd: &'a [u8],
    sequences: Option<usize>,
    sets: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    pub raw: [u8; 16],
}

impl Tone {
    pub fn priority(&self) -> u8 {
        self.raw[1]
    }
    pub fn root(&self) -> u8 {
        self.raw[2]
    }
    pub fn fine(&self) -> i8 {
        self.raw[3] as i8
    }
    /// Byte offset of the sample in the `.bd`.
    pub fn sample(&self) -> usize {
        u16::from_le_bytes([self.raw[4], self.raw[5]]) as usize * 8
    }
    pub fn adsr(&self) -> (u16, u16) {
        (u16::from_le_bytes([self.raw[6], self.raw[7]]), u16::from_le_bytes([self.raw[8], self.raw[9]]))
    }
    pub fn volume(&self) -> u8 {
        self.raw[0xb]
    }
    pub fn pan(&self) -> u8 {
        self.raw[0xc]
    }
}

/// One key-on of a sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyOn {
    /// Ticks since the sequence started.
    pub time: u32,
    pub channel: u8,
    pub note: u8,
    pub velocity: u8,
    pub set: u8,
}

impl<'a> Bank<'a> {
    pub fn parse(hd: &'a [u8]) -> Result<Self, Error> {
        if hd.get(0xc..0x10) != Some(b"SShd") || hd.len() < 0x28 {
            return Err(Error("snd: not an SShd bank".into()));
        }
        let section = |o: usize| usize::try_from(i32::from_le_bytes(hd[o..o + 4].try_into().unwrap())).ok();
        Ok(Self { hd, sequences: section(0x1c), sets: section(0x24) })
    }

    fn u16(&self, o: usize) -> Option<usize> {
        self.hd.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
    }

    /// The sequence the game plays for `(program, key)`.
    pub fn sequence(&self, program: usize, key: usize) -> Option<&'a [u8]> {
        let s = self.sequences?;
        if program > self.u16(s)? {
            return None;
        }
        let p = s + self.u16(s + 2 + 2 * program)?;
        if key > self.u16(p)? {
            return None;
        }
        self.hd.get(s + self.u16(p + 2 + 2 * key)?..)
    }

    /// The key-ons of `(program, key)`, until the end-of-track event.
    pub fn key_ons(&self, program: usize, key: usize) -> Option<Vec<KeyOn>> {
        let seq = self.sequence(program, key)?;
        let (mut i, mut time, mut out) = (0, 0u32, Vec::new());
        loop {
            let s = *seq.get(i)?;
            i += match s & 0xf0 {
                0xa0 => {
                    let b = seq.get(i + 1..i + 4)?;
                    out.push(KeyOn { time, channel: s & 0xf, note: b[0], velocity: b[1], set: b[2] });
                    4
                }
                0xc0 | 0xd0 => 2,
                0x80 | 0x90 | 0xb0 | 0xe0 => 3,
                0xf0 if s == 0xff && seq.get(i + 1) == Some(&0x2f) => return Some(out),
                0xf0 if s == 0xff => 3 + *seq.get(i + 2)? as usize,
                _ => return None,
            };
            loop {
                let b = *seq.get(i)?;
                i += 1;
                time = time << 7 | (b & 0x7f) as u32;
                if b & 0x80 == 0 {
                    break;
                }
            }
        }
    }

    /// The tone `note` sounds with in tone set `set`.
    pub fn tone(&self, set: usize, note: u8) -> Option<Tone> {
        let s = self.sets?;
        if set > self.u16(s)? {
            return None;
        }
        let h = s + self.u16(s + 2 + 2 * set)?;
        let low = *self.hd.get(h + 6)?;
        let o = h + 8 + 16 * note.checked_sub(low)? as usize;
        Some(Tone { raw: self.hd.get(o..o + 16)?.try_into().unwrap() })
    }
}

/// SPU pitch register for a voice. `word` packs root note << 24 | note << 16 | fine tune << 8 | bend (0x40 centre);
/// `scale` packs bend range << 24 | 12-bit pitch scale (0x1000 = 1). `table` is the driver's 192-steps-per-octave
/// pitch table (`exe::pitch_table`). At the root note the voice plays at 44.1 kHz (0x0eb3).
pub fn pitch(table: &[u16], word: u32, scale: u32) -> u16 {
    let [bend, fine, note, root] = word.to_le_bytes();
    let bend = (bend as i32 - 0x40) * (scale >> 24) as i32 >> 2;
    let at = |step: i32| table[(step * 16 + bend + 0xd0 + fine as i8 as i32) as usize] as u32;
    let p = if root > note {
        let d = (root - note) as i32;
        at(12 - d % 12) >> (d / 12 + 1)
    } else {
        let d = (note - root) as i32;
        at(d % 12) << (d / 12)
    };
    ((scale & 0xffff) * (p * 441 / 480) >> 12) as u16
}
