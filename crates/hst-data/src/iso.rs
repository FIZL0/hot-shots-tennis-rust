//! Minimal ISO 9660 reader: enough to pull files straight out of the retail disc image.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

const SECTOR: u64 = 2048;

pub struct Iso {
    file: File,
    /// Upper-case path with `/` separators and no `;1` suffix -> (byte offset, size).
    pub files: BTreeMap<String, (u64, u32)>,
}

impl Iso {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut iso = Self { file: File::open(path)?, files: BTreeMap::new() };
        let pvd = iso.read_at(16 * SECTOR, 2048)?;
        if &pvd[1..6] != b"CD001" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not an ISO 9660 image"));
        }
        let (lba, size) = extent(&pvd[156..]);
        iso.walk(String::new(), lba, size, 0)?;
        Ok(iso)
    }

    fn read_at(&mut self, off: u64, len: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; len];
        self.file.seek(SeekFrom::Start(off))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn walk(&mut self, prefix: String, lba: u32, size: u32, depth: u32) -> io::Result<()> {
        if depth > 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "directory nesting too deep"));
        }
        let dir = self.read_at(lba as u64 * SECTOR, size as usize)?;
        let mut p = 0;
        while p < dir.len() {
            let len = dir[p] as usize;
            if len == 0 {
                p = (p / SECTOR as usize + 1) * SECTOR as usize; // records never straddle sectors
                continue;
            }
            let rec = dir.get(p..p + len).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bad dir record"))?;
            let name_len = rec[32] as usize;
            let raw = &rec[33..33 + name_len];
            if raw != [0] && raw != [1] {
                let name = String::from_utf8_lossy(raw);
                let name = name.split(';').next().unwrap().to_ascii_uppercase();
                let path = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
                let (l, s) = extent(rec);
                if rec[25] & 2 != 0 {
                    self.walk(path, l, s, depth + 1)?;
                } else {
                    self.files.insert(path, (l as u64 * SECTOR, s));
                }
            }
            p += len;
        }
        Ok(())
    }

    /// Read a file by path; case-insensitive, `/` or `\` separators, leading separator optional.
    pub fn read(&mut self, path: &str) -> io::Result<Vec<u8>> {
        let key = path.replace('\\', "/").trim_start_matches('/').to_ascii_uppercase();
        let &(off, size) = self
            .files
            .get(&key)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{path} not on disc")))?;
        self.read_at(off, size as usize)
    }
}

/// Little-endian halves of the both-endian LBA and size fields of a directory record.
fn extent(rec: &[u8]) -> (u32, u32) {
    let le = |o: usize| u32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
    (le(2), le(10))
}
