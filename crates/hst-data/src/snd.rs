//! Sound banks: `.hd` headers ("SShd") beside `.bd` sample data (PS-ADPCM), and the SPU pitch the sound driver
//! sets for a note, and the SPU2 voice that plays it (`Voice`, mixed one step per 48 kHz output sample) at the
//! volume the driver sends (`Level`).
//!
//! ```text
//! .hd  u32 hd size, u32 bd size, u32 0, "SShd"
//!      i32 × 6 section offsets at 0x10 (-1 = absent); 0x1c sequences, 0x24 tone sets
//! sequences  u16 last program, u16 offset × programs           (offsets from the section start)
//!            program: u16 last key, u16 offset × keys          (offsets from the *section* start too)
//!            sequence: { event, varlen delta }* ending ff 2f 00
//!            events (running status): a0|ch note velocity set (key-on with a tone set, velocity 0 = key-off),
//!            f0 00 20 x (marker), f0 00 30 a b, other f0 up to f7, b0 controller value, ff 2f 00 end;
//!            the other status bytes take no data. The driver plays 8 ticks a frame: an event waits
//!            ⌈delta / 8⌉ frames after the one before it.
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
    /// Panned through the centre gains instead of the pan table.
    pub fn centre(&self) -> bool {
        self.raw[0xa] != 0
    }
}

/// One key-on of a sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyOn {
    /// Frames (60 Hz) since the sequence started.
    pub frame: u32,
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

    /// The key-ons (and velocity-0 key-offs) of `(program, key)` as the driver plays them, until the end event.
    pub fn key_ons(&self, program: usize, key: usize) -> Option<Vec<KeyOn>> {
        let seq = self.sequence(program, key)?;
        let (mut i, mut frame, mut status, mut out) = (0, 0u32, 0u8, Vec::new());
        loop {
            if seq.get(i)? & 0x80 != 0 {
                status = seq[i];
                i += 1;
            }
            match status & 0xf0 {
                0xa0 => {
                    let b = seq.get(i..i + 3)?;
                    out.push(KeyOn { frame, channel: status & 0xf, note: b[0], velocity: b[1], set: b[2] });
                    i += 3;
                }
                0xb0 => i += 2,
                0xf0 if status == 0xff => {
                    if *seq.get(i)? == 0x2f {
                        return Some(out);
                    }
                    i += 2 + *seq.get(i + 1)? as usize; // ponytail: no SE sequence has other meta events
                }
                0xf0 => match seq.get(i..i + 2)? {
                    [0, 0x20] => i += 3,
                    [0, 0x30] => i += 4,
                    _ => i += seq[i..].iter().position(|&b| b == 0xf7)? + 1,
                },
                _ => {}
            }
            let mut delta = 0;
            loop {
                let b = *seq.get(i)?;
                i += 1;
                delta = delta << 7 | (b & 0x7f) as u32;
                if b & 0x80 == 0 {
                    break;
                }
            }
            frame += delta.div_ceil(8);
        }
    }

    /// Tone set volume (the set header's byte 1).
    pub fn set_volume(&self, set: usize) -> Option<u8> {
        let s = self.sets?;
        if set > self.u16(s)? {
            return None;
        }
        self.hd.get(s + self.u16(s + 2 + 2 * set)? + 1).copied()
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

/// What the driver builds a voice's L/R volume from. `seq` is the play call's volume per side, `bank` the bank's
/// volume, `pan` the play call's, the channel's and the tone's pan (0x40 centre each); `tone`, `pan[2]`, `centre`
/// and `gain` come from the tone (`Level::tone`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Level {
    pub seq: [u32; 2],
    pub bank: u32,
    pub tone: u32,
    pub velocity: u32,
    pub pan: [u32; 3],
    pub centre: bool,
    pub gain: [u32; 2],
}

impl Level {
    /// The tone's part, at the channel's reset volume 100 and expression 127 (SE sequences change neither).
    /// `gain` is `exe::sound_tables`' cos table.
    pub fn tone(&mut self, set_volume: u8, t: Tone, gain: &[u16]) {
        self.tone = (100 * 127 * set_volume as u64 * t.volume() as u64 / (127 * 127 * 127)) as u32;
        self.centre = t.centre();
        self.pan[2] = if t.centre() { 0x40 } else { t.pan().clamp(1, 127) as u32 };
        let p = (t.pan() as usize).min(0x80); // ponytail: pans above 0x80 not seen
        self.gain = if t.centre() { [gain[p] as u32, gain[0x80 - p] as u32] } else { [0; 2] };
    }

    /// The voice volume registers, from `exe::sound_tables`' pan table.
    pub fn volume(&self, pan_table: &[u16]) -> [i16; 2] {
        let i = if self.centre { 0x40 } else { (self.pan.iter().sum::<u32>() as i32 - 0x80).clamp(1, 127) as usize };
        let [r, l] = pan_table[i].to_le_bytes();
        [(0, l), (1, r)].map(|(s, pan)| {
            let x = self.seq[s] * self.tone * self.velocity / 0x3f01 * self.bank / 127 * pan as u32;
            let x = if self.centre { x * self.gain[s] / 0x7fff } else { x };
            x.min(0x3fff) as i16
        })
    }
}

/// One SPU2 voice, stepped like PCSX2's mixer (ADPCM decode into a 32-sample queue, gaussian interpolation, ADSR
/// envelope, volume) once per 48 kHz output sample. Addresses are in SPU RAM halfwords from the start of the sample
/// data the voice plays from.
#[derive(Debug, Clone, PartialEq)]
pub struct Voice {
    pub adsr: (u16, u16),
    pub pitch: u16,
    /// Volume registers L/R.
    pub volume: [i16; 2],
    pub loop_start: u32,
    pub next: u32,
    /// Envelope: 0 off, 1 attack, 2 decay, 3 sustain, 4 release; level 0..=0x7fff, counter to 0x8000.
    pub phase: u8,
    pub level: i32,
    pub counter: u32,
    /// Last two decoded samples (ADPCM filter history).
    pub prev: [i32; 2],
    /// Sample position: 12-bit fraction, queue write and read counts.
    pub sp: u32,
    pub write: u32,
    pub read: u32,
    pub fifo: [i32; 32],
    /// The current block's loop flags: 1 end, 2 loop, 4 loop start.
    pub flags: u8,
    /// Last sample after the envelope.
    pub out: i32,
    block: Option<[i16; 28]>,
}

impl Voice {
    pub fn key_on(start: u32, adsr: (u16, u16), pitch: u16, volume: [i16; 2]) -> Self {
        let (phase, level, counter, prev, sp, write, read, flags, out) = (1, 0, 0, [0; 2], 0, 0, 0, 0, 0);
        Self { adsr, pitch, volume, loop_start: start, next: start | 1, phase, level, counter, prev, sp, write, read, fifo: [0; 32], flags, out, block: None }
    }

    pub fn key_off(&mut self) {
        if self.phase != 0 {
            self.phase = 4;
            self.counter = 0;
        }
    }

    fn stop(&mut self) {
        self.phase = 0;
        self.level = 0;
    }

    /// One output sample (L, R), before the core's mix and master volume. `ram` is the sample data.
    pub fn tick(&mut self, ram: &[u8]) -> [i32; 2] {
        if self.phase == 0 {
            return [0; 2];
        }
        let at = (self.next & !7) as usize * 2;
        let Some(b) = ram.get(at..at + 16) else {
            self.stop();
            return [0; 2];
        };
        self.flags = b[1];
        if self.flags & 4 != 0 {
            self.loop_start = self.next & !7;
        }
        if (self.write.wrapping_sub(self.read) as i32) <= 12 {
            let block = *self.block.get_or_insert_with(|| decode(b, &mut self.prev));
            let s = (self.next % 8 - 1) as usize * 4;
            for k in 0..4 {
                self.fifo[(self.write as usize + k) % 32] = block[s + k] as i32;
            }
            self.write += 4;
            self.next += 1;
            if self.next % 8 == 0 {
                if self.flags & 1 != 0 {
                    self.next = self.loop_start;
                    if self.flags & 2 == 0 {
                        self.stop();
                    }
                }
                self.next += 1;
                self.block = None;
            }
        }
        if self.phase == 0 {
            return [0; 2];
        }
        let g = gaussian()[(self.sp as usize & 0xff0) >> 4];
        let x: i32 = (0..4).map(|k| g[k] as i32 * self.fifo[(self.read as usize + k) % 32] >> 15).sum();
        if !self.envelope() {
            self.stop();
        }
        self.out = x * self.level >> 15;
        self.sp += self.pitch.min(0x3fff) as u32;
        self.read += self.sp >> 12;
        self.sp &= 0xfff;
        self.volume.map(|v| self.out * (v << 1) as i32 >> 15)
    }

    /// One envelope step; false when the voice ends.
    fn envelope(&mut self) -> bool {
        let (a1, a2) = (self.adsr.0 as i32, self.adsr.1 as i32);
        let (decr, exp, shift, step, target) = match self.phase {
            1 => (false, a1 >> 15 != 0, a1 >> 10 & 31, 7 - (a1 >> 8 & 3), 0x7fff),
            2 => (true, true, a1 >> 4 & 15, -8, ((a1 & 15) + 1) << 11),
            3 => {
                let decr = a2 >> 14 & 1 != 0;
                let step = 7 - (a2 >> 6 & 3);
                (decr, a2 >> 15 != 0, a2 >> 8 & 31, if decr { !step } else { step }, 0)
            }
            _ => (true, a2 >> 5 & 1 != 0, a2 & 31, -8, 0),
        };
        let mut counter_inc = 0x8000u32 >> (shift - 11).max(0);
        let mut level_inc = step << (11 - shift).max(0);
        if exp && !decr && self.level > 0x6000 {
            counter_inc >>= 2;
        }
        if exp && decr {
            level_inc = (level_inc * self.level >> 15) as i16 as i32;
        }
        self.counter += counter_inc.max(1);
        if self.counter >= 0x8000 {
            self.counter = 0;
            self.level = (self.level + level_inc).clamp(0, 0x7fff);
        }
        if self.phase == 3 {
            return self.level != 0;
        }
        if (!decr && self.level >= target) || (decr && self.level <= target) {
            self.phase += 1;
        }
        self.phase <= 4
    }
}

/// One 16-byte PS-ADPCM block to 28 samples; `prev` is the filter history, newest first.
fn decode(b: &[u8], prev: &mut [i32; 2]) -> [i16; 28] {
    const FILTER: [(i32, i32); 5] = [(0, 0), (60, 0), (115, -52), (98, -55), (122, -60)];
    let shift = (b[0] & 15) as u32 + 16;
    let (f1, f2) = FILTER.get((b[0] >> 4) as usize).copied().unwrap_or((0, 0));
    std::array::from_fn(|i| {
        let n = (b[2 + i / 2] >> (i % 2 * 4) & 15) as i32;
        let pcm = ((n << 28 >> shift) + ((f1 * prev[0] + f2 * prev[1] + 32) >> 6)).clamp(-0x8000, 0x7fff);
        *prev = [pcm, prev[0]];
        pcm as i16
    })
}

/// The SPU's 4-tap gaussian interpolation weights per 1/256 sample phase (the table PCSX2 builds, after nocash).
pub fn gaussian() -> &'static [[i16; 4]; 256] {
    static TABLE: std::sync::OnceLock<[[i16; 4]; 256]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        use std::f64::consts::PI;
        let mut t = [0f64; 512];
        for n in 0..512 {
            let k = n as f64 + 0.5;
            let s = (PI * k * 2.048 / 1024.0).sin();
            let c = ((PI * k * 2.0 / 1023.0).cos() - 1.0) * 0.5 + ((PI * k * 4.0 / 1023.0).cos() - 1.0) * 0.08;
            t[511 - n] = s * (c + 1.0) / k;
        }
        let scale = 0x7f80 as f64 * 128.0 / t.iter().sum::<f64>();
        t.iter_mut().for_each(|x| *x *= scale);
        let mut out = [[0; 4]; 256];
        for p in 0..256 {
            let taps = [t[p], t[p + 256], t[511 - p], t[255 - p]];
            let diff = (taps.iter().sum::<f64>() - 0x7f80 as f64) / 4.0;
            out[255 - p] = taps.map(|x| (x - diff).round() as i16);
        }
        out
    })
}
