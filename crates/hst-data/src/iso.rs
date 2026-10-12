//! Minimal ISO 9660 reader: enough to pull files straight out of the retail disc image. It also opens a disc
//! *pack* (`write_pack`): the files the remaster reads, copied out of the user's ISO once so the ISO can go.

use std::collections::BTreeMap;
use std::fs::File;
use std::hash::Hasher;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use twox_hash::XxHash3_64;

const SECTOR: u64 = 2048;

pub struct Iso {
    file: File,
    /// Upper-case path with `/` separators and no `;1` suffix -> (byte offset, size).
    pub files: BTreeMap<String, (u64, u32)>,
    /// A pack's content hash (XXH3-64 of everything after its header); `None` for an ISO.
    pub pack_hash: Option<u64>,
}

/// Pack layout: magic, u64 hash, u32 count, per file (u16 name length, name, u64 offset, u32 size), then the data.
const PACK_MAGIC: &[u8; 8] = b"HSTPACK1";

/// What a pack leaves out: the FMV movies (217 MB), the IOP driver modules and the movie overlay — nothing the
/// remaster reads. Everything else on the disc (≈ 170 MB) goes in, so a later port that reads a new file still finds it.
fn packed(path: &str) -> bool {
    !(path.starts_with("A/") || path.starts_with("MODULES/") || path.starts_with("IRX/") || path == "ZZBIN/MOVIE.BIN")
}

impl Iso {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut iso = Self { file: File::open(path)?, files: BTreeMap::new(), pack_hash: None };
        if iso.read_at(0, 8).ok().as_deref() == Some(PACK_MAGIC) {
            iso.read_index()?;
            return Ok(iso);
        }
        let pvd = iso.read_at(16 * SECTOR, 2048)?;
        if &pvd[1..6] != b"CD001" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not an ISO 9660 image"));
        }
        let (lba, size) = extent(&pvd[156..]);
        iso.walk(String::new(), lba, size, 0)?;
        Ok(iso)
    }

    fn read_index(&mut self) -> io::Result<()> {
        let bad = || io::Error::new(io::ErrorKind::InvalidData, "bad disc pack index");
        let mut head = [0; 12];
        self.file.seek(SeekFrom::Start(8))?;
        self.file.read_exact(&mut head)?;
        self.pack_hash = Some(u64::from_le_bytes(head[..8].try_into().unwrap()));
        let mut r = io::BufReader::new(&mut self.file);
        for _ in 0..u32::from_le_bytes(head[8..].try_into().unwrap()) {
            let mut n = [0; 2];
            r.read_exact(&mut n)?;
            let mut name = vec![0; u16::from_le_bytes(n) as usize];
            r.read_exact(&mut name)?;
            let mut e = [0; 12];
            r.read_exact(&mut e)?;
            let name = String::from_utf8(name).map_err(|_| bad())?;
            self.files.insert(name, (u64::from_le_bytes(e[..8].try_into().unwrap()), u32::from_le_bytes(e[8..].try_into().unwrap())));
        }
        Ok(())
    }

    /// Copy every file the remaster can read (`packed`) into a pack at `out`, after checking this is the supported
    /// disc (SCUS-97610, US 1.00). Written to `out` + `.part` and renamed when complete. Returns the pack's hash.
    pub fn write_pack(&mut self, out: &Path, mut progress: impl FnMut(usize, usize)) -> io::Result<u64> {
        let invalid = |e: crate::xb::Error| io::Error::new(io::ErrorKind::InvalidData, e.0);
        let (cnf, bin) = (self.read("SYSTEM.CNF")?, self.read("ZZBIN/GAME.BIN")?);
        crate::exe::Game::new(&cnf, &bin).map_err(invalid)?;
        let names: Vec<String> = self.files.keys().filter(|p| packed(p)).cloned().collect();
        let mut index = (names.len() as u32).to_le_bytes().to_vec();
        let index_len = 20 + names.iter().map(|n| 14 + n.len() as u64).sum::<u64>();
        let mut off = index_len;
        for n in &names {
            let size = self.files[n].1;
            index.extend((n.len() as u16).to_le_bytes());
            index.extend(n.as_bytes());
            index.extend(off.to_le_bytes());
            index.extend(size.to_le_bytes());
            off += size as u64;
        }
        let part = out.with_extension("part");
        let mut w = BufWriter::new(File::create(&part)?);
        let mut h = XxHash3_64::new();
        w.write_all(PACK_MAGIC)?;
        w.write_all(&[0; 8])?;
        w.write_all(&index)?;
        h.write(&index);
        for (i, n) in names.iter().enumerate() {
            let data = self.read(n)?;
            w.write_all(&data)?;
            h.write(&data);
            progress(i + 1, names.len());
        }
        let hash = h.finish();
        let mut f = w.into_inner().map_err(|e| e.into_error())?;
        f.seek(SeekFrom::Start(8))?;
        f.write_all(&hash.to_le_bytes())?;
        f.sync_all()?;
        std::fs::rename(&part, out)?;
        Ok(hash)
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
