//! Court layout: which models a court uses (`entry_cNN.txt`) and where props stand (`plant_cNN_hNN_N.dat`).
//!
//! The entry list is the editor's list of material files, grouped by directory. A plant record is 0x40 bytes:
//! `i16 index, u8 category-0x11, u8 sub, f32 pos[3], f32 extra[4], f32 yaw, f32 scale, u8 code[4], ...`. For placed
//! categories the index counts entries of that category's directory, in list order. Creatures also carry a link at
//! 0x2c: `i16` (bit 15 set = linked, low bits the anchor's index) and `u8` anchor category−0x11 at 0x2e.

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
    /// Placement code characters (byte 0: extra turn, byte 1: tilt, in 36ths of a circle).
    pub code: [u8; 4],
    /// Creatures (category 23) only: the record whose position this one hangs off, as (category, index among that
    /// category's records in file order); `pos` is then an offset subtracted from the anchor's position.
    pub link: Option<(u8, u16)>,
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
            code: r[0x28..0x2c].try_into().unwrap(),
            link: (r[2].wrapping_add(0x11) == 23 && r[0x2d] & 0x80 != 0)
                .then(|| (r[0x2e].wrapping_add(0x11), u16::from_le_bytes([r[0x2c], r[0x2d]]) & 0x7fff)),
        })
        .collect())
}

/// Split a `name_sXXXX[_rest]` stem into its base name and four-season mask (`_s1010` = seasons 0 and 2).
fn season_split(stem: &str) -> (String, [bool; 4]) {
    if let Some(i) = stem.rfind("_s") {
        let m = stem.as_bytes().get(i + 2..i + 6).unwrap_or(&[]);
        if m.len() == 4 && m.iter().all(|c| matches!(c, b'0' | b'1')) {
            return (format!("{}{}", &stem[..i], &stem[i + 6..]), std::array::from_fn(|k| m[k] == b'1'));
        }
    }
    (stem.to_string(), [true; 4])
}

/// Whether the game loads this model in `season`: its `_sXXXX` mask has the season (no mask: always), and
/// no `_p01` in the name (the game never loads those).
pub fn in_season(stem: &str, season: usize) -> bool {
    season_split(stem).1[season.min(3)] && !stem.contains("_p01")
}

/// The entry a placement refers to for `season` (0..3). Season variants of one model share an index:
/// the index counts distinct base names in the category's directory, then the variant whose season
/// mask includes `season` is picked (any variant if none does).
pub fn resolve<'a>(entries: &'a [Entry], p: &Placement, season: usize) -> Option<&'a Entry> {
    let dir = (*CATEGORY_DIRS.get(p.category as usize)?)?;
    let mut bases: Vec<String> = Vec::new();
    for e in entries.iter().filter(|e| e.dir == dir) {
        let (b, _) = season_split(&e.stem);
        if !bases.contains(&b) {
            bases.push(b);
        }
    }
    let base = bases.get(usize::try_from(p.index).ok()?)?;
    let variants: Vec<&Entry> = entries.iter().filter(|e| e.dir == dir && &season_split(&e.stem).0 == base).collect();
    variants.iter().find(|e| season_split(&e.stem).1[season.min(3)]).or(variants.first()).copied()
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
        assert_eq!(resolve(&list, p, 0).unwrap().stem, "b_s1111");
    }

    #[test]
    fn season_variants_share_an_index() {
        let list = entries("c:\\x\\tree\\a_s1111.mtl\nc:\\x\\tree\\net_s0100.mtl\nc:\\x\\tree\\net_s1000.mtl\n");
        let mut rec = vec![0u8; 0x40];
        rec[0] = 1;
        rec[2] = 17u8.wrapping_sub(0x11);
        let p = &plants(&rec).unwrap()[0];
        assert_eq!(resolve(&list, p, 0).unwrap().stem, "net_s1000");
        assert_eq!(resolve(&list, p, 1).unwrap().stem, "net_s0100");
        assert!(in_season("greece00_sky_s1100", 1) && !in_season("greece00_sky_s1100", 2) && !in_season("resort_bg_s1111_p01", 0));
    }
}

/// How many clouds a court makes: `envir_cNN.dat`'s i16 per environment at 0x4d0 scaled by the hole file's
/// `envir_cNN_h01.dat` percent at 0x50; 0 means 20. Environment 1 is singles, 0 doubles.
pub fn cloud_count(envir: &[u8], hole_envir: &[u8], env: usize) -> Option<usize> {
    let base = i16::from_le_bytes(envir.get(0x4d0 + 2 * env..0x4d2 + 2 * env)?.try_into().ok()?) as i32;
    let n = base * *hole_envir.get(0x50 + env)? as i32 / 100;
    Some(if n == 0 { 20 } else { n.max(0) as usize })
}
