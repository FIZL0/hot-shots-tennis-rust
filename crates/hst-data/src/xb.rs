//! `.XB` / `.XB0` archives.
//!
//! Layout (all little-endian):
//! ```text
//! u32 magic "xe\0\x01"   u32 count
//! count × { u32 size, u32 (kind << 28) | offset_in_words }
//! block: name list (LZ), entries of { u8 len, u8 hash, name[len], 0 }
//! ```
//! A *block* is `{ u32 out_size, u32 compressed?, data }`. Entry kinds:
//! 3 = raw bytes, 2 = LZ block, 1 = Huffman block, 0 = Huffman block whose output is an LZ block.

use std::fmt;

pub const MAGIC: [u8; 4] = *b"xe\0\x01";

#[derive(Debug)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;

fn err<T>(m: impl Into<String>) -> Result<T> {
    Err(Error(m.into()))
}

fn u32_at(d: &[u8], o: usize) -> Result<u32> {
    d.get(o..o + 4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| Error(format!("read past end at {o:#x}")))
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub size: u32,
    pub kind: u8,
    pub offset: usize,
}

pub struct Archive<'a> {
    data: &'a [u8],
    pub entries: Vec<Entry>,
}

impl<'a> Archive<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.get(..4) != Some(&MAGIC) {
            return err("not an xe archive");
        }
        let count = u32_at(data, 4)? as usize;
        let names = block(data, 8 + count * 8, lz)?;
        let mut entries = Vec::with_capacity(count);
        let mut p = 0;
        for i in 0..count {
            let len = *names.get(p).ok_or_else(|| Error("name list truncated".into()))? as usize;
            let name = names.get(p + 2..p + 2 + len).ok_or_else(|| Error("name truncated".into()))?;
            p += len + 3;
            let size = u32_at(data, 8 + i * 8)?;
            let w = u32_at(data, 12 + i * 8)?;
            entries.push(Entry {
                name: String::from_utf8_lossy(name).into_owned(),
                size,
                kind: (w >> 28) as u8,
                offset: (w & 0x0fff_ffff) as usize * 4,
            });
        }
        Ok(Self { data, entries })
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        let n = name.replace('/', "\\");
        self.entries.iter().find(|e| e.name.eq_ignore_ascii_case(&n))
    }

    pub fn read(&self, e: &Entry) -> Result<Vec<u8>> {
        let out = match e.kind {
            3 => self
                .data
                .get(e.offset..e.offset + e.size as usize)
                .ok_or_else(|| Error("raw entry out of range".into()))?
                .to_vec(),
            2 => block(self.data, e.offset, lz)?,
            1 => block(self.data, e.offset, huffman)?,
            0 => block(&block(self.data, e.offset, huffman)?, 0, lz)?,
            k => return err(format!("unknown entry kind {k}")),
        };
        if out.len() != e.size as usize {
            return err(format!("{}: decoded {} bytes, expected {}", e.name, out.len(), e.size));
        }
        Ok(out)
    }
}

/// Name hash the game uses to speed up lookups: 8-bit rotate-left-xor, paired with the length.
pub fn name_hash(name: &[u8]) -> u8 {
    name.iter().fold(0u8, |h, &c| h.rotate_left(1) ^ c)
}

fn block(d: &[u8], at: usize, decode: fn(&[u8], usize) -> Result<Vec<u8>>) -> Result<Vec<u8>> {
    let size = u32_at(d, at)? as usize;
    let packed = u32_at(d, at + 4)?;
    let src = d.get(at + 8..).ok_or_else(|| Error("block out of range".into()))?;
    if packed == 0 {
        return src.get(..size).map(<[u8]>::to_vec).ok_or_else(|| Error("stored block truncated".into()));
    }
    decode(src, size)
}

/// Byte-oriented LZ77. Control byte low bits: `00` literal run, `x1` short match, `10` long match.
pub fn lz(src: &[u8], size: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(size);
    let mut s = 0;
    let b = |i: usize| src.get(i).copied().map(u32::from).ok_or_else(|| Error("lz input truncated".into()));
    while out.len() < size {
        let c = b(s)?;
        s += 1;
        let (len, dist) = if c & 3 == 0 {
            let n = (c >> 2) as usize + 1;
            out.extend_from_slice(src.get(s..s + n).ok_or_else(|| Error("lz literal truncated".into()))?);
            s += n;
            continue;
        } else if c & 1 == 1 {
            let v = c | b(s)? << 8;
            s += 1;
            ((v >> 1 & 7) + 3, v >> 4)
        } else {
            let v = c | b(s)? << 8 | b(s + 1)? << 16;
            s += 2;
            ((v >> 2 & 0x3ff) + 3, v >> 12)
        };
        let start = out
            .len()
            .checked_sub(dist as usize)
            .ok_or_else(|| Error(format!("lz distance {dist} before start")))?;
        for i in 0..len as usize {
            out.push(out[start + i]); // overlapping copies are intentional (run-length)
        }
    }
    out.truncate(size); // the game writes whole matches; anything past `size` is scratch
    Ok(out)
}

/// Canonical Huffman, codes read LSB-first from little-endian u16s, 10-bit lookup table.
/// Symbols with codes longer than 10 bits share one escape code followed by a raw byte.
pub fn huffman(src: &[u8], size: usize) -> Result<Vec<u8>> {
    const ESC: u8 = 0xff;
    let mut table = [(ESC, 0u8); 1024]; // (code length, symbol)
    let mut p = 0;
    let rd = |p: &mut usize| -> Result<u8> {
        let v = *src.get(*p).ok_or_else(|| Error("huffman table truncated".into()))?;
        *p += 1;
        Ok(v)
    };
    let max_len = rd(&mut p)? as u32;
    let mut code = 0u32;
    for len in 1..=max_len {
        for _ in 0..rd(&mut p)? {
            let sym = rd(&mut p)?;
            let rev = (code & ((1 << len) - 1)).reverse_bits() >> (32 - len);
            let mut i = rev;
            while i < 1024 {
                table[i as usize] = (len as u8, sym);
                i += 1 << len;
            }
            if len < 11 {
                code += 1;
            }
        }
        code <<= 1;
    }
    p += p & 1; // bitstream is 2-byte aligned; blocks are word-aligned so relative == absolute parity
    // The game refills 16 bits ahead, so the last entry of a file reads past its end: treat as zeros.
    let mut half = || -> Result<u32> {
        let v = src.get(p..p + 2).map_or(0, |v| u16::from_le_bytes([v[0], v[1]]) as u32);
        p += 2;
        Ok(v)
    };
    let (mut bits, mut n) = (0u32, 0u32);
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        if n < 16 {
            bits |= half()? << n;
            n += 16;
        }
        let (len, sym) = table[(bits & 0x3ff) as usize];
        if len < 11 {
            out.push(sym);
            bits >>= len;
            n -= len as u32;
        } else {
            bits >>= 10;
            n -= 10;
            if n < 16 {
                bits |= half()? << n;
                n += 16;
            }
            out.push(bits as u8);
            bits >>= 8;
            n -= 8;
        }
    }
    Ok(out)
}
