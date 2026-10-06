//! Court layout: which models a court uses (`entry_cNN.txt`) and where props stand (`plant_cNN_hNN_N.dat`).
//!
//! The entry list is the editor's list of material files, grouped by directory. A plant record is 0x40 bytes:
//! `i16 index, u8 category-0x11, u8 sub, f32 pos[3], f32 extra[4], f32 yaw, f32 scale, ...`. For placed
//! categories the index counts entries of that category's directory, in list order.

use crate::xb::Error;

/// Directory of each category id; categories without one are special objects (lights, markers, …).
pub const CATEGORY_DIRS: [Option<&str>; 24] = {
    let mut t = [None; 24];
    t[0] = Some("hole");
    t[1] = Some("holeend");
    t[2] = Some("bg");
    t[3] = Some("bg");
    t[4] = Some("bg");
    t[5] = Some("bg");
    t[6] = Some("cloud");
    t[17] = Some("tree");
    t[18] = Some("prop");
    t[19] = Some("structure");
    t[20] = Some("billboard");
    t[21] = Some("gallery");
    t[22] = Some("dmy");
    t[23] = Some("creature");
    t
};

/// One line of the entry list: model stem (lower-case, no extension) and its directory.
#[derive(Clone, Debug)]
pub struct Entry {
    pub dir: String,
    pub stem: String,
}

pub fn entries(text: &str) -> Vec<Entry> {
    text.lines()
        .filter_map(|l| {
            let mut parts = l.trim().rsplit(['\\', '/']);
            let file = parts.next()?;
            let dir = parts.next()?;
            let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
            (!stem.is_empty()).then(|| Entry { dir: dir.to_ascii_lowercase(), stem: stem.to_ascii_lowercase() })
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Placement {
    pub category: u8,
    /// Index within the category's directory (see [`resolve`]).
    pub index: i16,
    pub sub: u8,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub scale: f32,
}

pub fn plants(d: &[u8]) -> Result<Vec<Placement>, Error> {
    if d.len() % 0x40 != 0 {
        return Err(Error(format!("plant file size {:#x} is not a multiple of 0x40", d.len())));
    }
    let f = |r: &[u8], o: usize| f32::from_le_bytes(r[o..o + 4].try_into().unwrap());
    Ok(d.chunks_exact(0x40)
        .map(|r| Placement {
            index: i16::from_le_bytes([r[0], r[1]]),
            category: r[2].wrapping_add(0x11),
            sub: r[3],
            pos: [f(r, 4), f(r, 8), f(r, 12)],
            yaw: f(r, 0x20),
            scale: f(r, 0x24),
        })
        .collect())
}

/// The entry a placement refers to, if its category has a directory and the index is in range.
pub fn resolve<'a>(entries: &'a [Entry], p: &Placement) -> Option<&'a Entry> {
    let dir = (*CATEGORY_DIRS.get(p.category as usize)?)?;
    entries.iter().filter(|e| e.dir == dir).nth(usize::try_from(p.index).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_by_directory_order() {
        let list = entries("c:\\x\\tree\\a_s1111.mtl\r\nc:\\x\\structure\\brush.mtl\nc:\\x\\tree\\b_s1111.mtl\n");
        let mut rec = vec![0u8; 0x40];
        rec[0] = 1;
        rec[2] = 17u8.wrapping_sub(0x11);
        let p = &plants(&rec).unwrap()[0];
        assert_eq!(p.category, 17);
        assert_eq!(resolve(&list, p).unwrap().stem, "b_s1111");
    }
}
