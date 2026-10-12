//! `.ANI2` skeletal animations (`PCANI/PCnnANI.XB`, exported from 3ds Max). 16-byte aligned:
//!
//! ```text
//! u32 ticks per game frame (80: 4800 ticks/s at 60 Hz), u32 root track count, 8 bytes pad
//! per track: u32 name length, f32 bone length, 8 pad; name (padded)
//!            2 key lists (rotation, position): u32 count (padded); count × u32 tick (padded);
//!            count × 16 bytes (rotation: quaternion x y z w; position: x y z 1)
//!            u32 child count (padded), then the children, depth first
//! ```
//! Character files are flat (no children); effect files (`AZUMA/C_EFF`) hang their parts under a scene root.
//! Tracks are named after the skeleton's nodes (3ds Max Biped: `Bip01`, `Bip01Pelvis`, …). A rotation key is
//! the conjugate of the node's local rotation in the usual (column-vector) sense; position keys are the local
//! translation. `*_dummy` / `*_ball` files hold one track: the ball's path during that motion.

use crate::xb::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub name: String,
    pub length: f32,
    pub rotation: Vec<(i32, [f32; 4])>,
    pub position: Vec<(i32, [f32; 4])>,
    /// Index of the parent track (tracks are stored depth first, so the parent comes earlier).
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Anim {
    pub ticks_per_frame: i32,
    pub tracks: Vec<Track>,
}

impl Anim {
    /// Last key time over all tracks (ticks).
    pub fn end_tick(&self) -> i32 {
        self.tracks.iter().flat_map(|t| [&t.rotation, &t.position]).filter_map(|k| k.last().map(|k| k.0)).max().unwrap_or(0)
    }
}

pub fn parse(d: &[u8]) -> Result<Anim, Error> {
    let align = |x: usize| (x + 15) & !15;
    let bad = |what: &str| Error(format!("ani2: bad {what}"));
    let rd_i32 = |o: usize| d.get(o..o + 4).map(|b| i32::from_le_bytes(b.try_into().unwrap())).ok_or_else(|| Error(format!("ani2: truncated at {o:#x}")));
    let rd_f32 = |o: usize| rd_i32(o).map(|v| f32::from_bits(v as u32));
    let ticks_per_frame = rd_i32(0)?;
    let count = usize::try_from(rd_i32(4)?).map_err(|_| bad("track count"))?;
    let mut p = 0x10usize;
    let mut tracks = Vec::new();
    // (parent, children still to read) for each open level of the tree
    let mut open = vec![(None, count)];
    while let Some(top) = open.last_mut() {
        if top.1 == 0 {
            open.pop();
            continue;
        }
        top.1 -= 1;
        let parent = top.0;
        let name_len = usize::try_from(rd_i32(p)?).map_err(|_| bad("name length"))?;
        let length = rd_f32(p + 4)?;
        p += 16;
        let raw = d.get(p..p + name_len).ok_or_else(|| bad("name"))?;
        let name = String::from_utf8_lossy(raw.split(|&b| b == 0).next().unwrap_or_default()).into_owned();
        p = align(p + name_len);
        let mut lists: [Vec<(i32, [f32; 4])>; 2] = Default::default();
        for list in &mut lists {
            let n = usize::try_from(rd_i32(p)?).map_err(|_| bad("key count"))?;
            p = align(p + 4);
            let times: Vec<i32> = (0..n).map(|k| rd_i32(p + 4 * k)).collect::<Result<_, _>>()?;
            p = align(p + 4 * n);
            for (k, t) in times.into_iter().enumerate() {
                let o = p + 16 * k;
                list.push((t, [rd_f32(o)?, rd_f32(o + 4)?, rd_f32(o + 8)?, rd_f32(o + 12)?]));
            }
            p += 16 * n;
        }
        let children = usize::try_from(rd_i32(p)?).map_err(|_| bad("child count"))?;
        p = align(p + 4);
        let [rotation, position] = lists;
        tracks.push(Track { name, length, rotation, position, parent });
        open.push((Some(tracks.len() - 1), children));
    }
    if p != d.len() {
        return Err(Error(format!("ani2: {} bytes left after the tracks", d.len() as isize - p as isize)));
    }
    Ok(Anim { ticks_per_frame, tracks })
}

/// A camera clip (`AZUMA/C_EFF/EFFCT.XB0` `prize/CAM/*.CAM`): the ANI2 header, then the root tracks one after the
/// other (no child counts), each with 4 key lists: rotation and position (16-byte values, as ANI2), a 16-byte list
/// the game doesn't read, and the full field of view in radians (4-byte values). Returns the clip's rotation and
/// position, and an anim whose tracks hold the FOV keys (in x) as their position list.
pub fn parse_cam(d: &[u8]) -> Result<(Anim, Anim), Error> {
    let align = |x: usize| (x + 15) & !15;
    let rd = |o: usize| d.get(o..o + 4).map(|b| <[u8; 4]>::try_from(b).unwrap()).ok_or_else(|| Error(format!("cam: truncated at {o:#x}")));
    let int = |o: usize| rd(o).map(i32::from_le_bytes);
    let float = |o: usize| rd(o).map(f32::from_le_bytes);
    let count = |o: usize| int(o).and_then(|n| usize::try_from(n).map_err(|_| Error(format!("cam: bad count at {o:#x}"))));
    let (ticks_per_frame, n) = (int(0)?, count(4)?);
    let (mut cam, mut fov) = (Anim { ticks_per_frame, tracks: vec![] }, Anim { ticks_per_frame, tracks: vec![] });
    let mut p = 0x10;
    for _ in 0..n {
        let (name_len, length) = (count(p)?, float(p + 4)?);
        p += 16;
        let raw = d.get(p..p + name_len).ok_or_else(|| Error("cam: bad name".into()))?;
        let name = String::from_utf8_lossy(raw.split(|&b| b == 0).next().unwrap_or_default()).into_owned();
        p = align(p + name_len);
        let mut lists: Vec<Vec<(i32, [f32; 4])>> = Vec::new();
        for size in [16, 16, 16, 4] {
            let k = count(p)?;
            p = align(p + 4);
            let ticks = (0..k).map(|i| int(p + 4 * i)).collect::<Result<Vec<_>, _>>()?;
            p = align(p + 4 * k);
            let mut keys = Vec::new();
            for (i, t) in ticks.into_iter().enumerate() {
                let o = p + size * i;
                let v = if size == 4 { [float(o)?, 0.0, 0.0, 1.0] } else { [float(o)?, float(o + 4)?, float(o + 8)?, float(o + 12)?] };
                keys.push((t, v));
            }
            p = align(p + size * k);
            lists.push(keys);
        }
        let track = |rotation, position| Track { name: name.clone(), length, rotation, position, parent: None };
        fov.tracks.push(track(vec![], lists.pop().unwrap()));
        lists.pop();
        let position = lists.pop().unwrap();
        cam.tracks.push(track(lists.pop().unwrap(), position));
    }
    if p != d.len() {
        return Err(Error(format!("cam: {} bytes left after the tracks", d.len() as isize - p as isize)));
    }
    Ok((cam, fov))
}

/// The game's animation table (program data, in this order): a motion's number is its index here; the file is
/// `<name with %02d = character number>.ANI2` in `PCANI/PCnnANI.XB`.
pub const MOTIONS: [&str; 55] = [
    "mo_pc%02d_ad00", "mo_pc%02d_tb_f_ad00", "mo_pc%02d_tb_b_ad00", "mo_pc%02d_run_f", "mo_pc%02d_run_b",
    "mo_pc%02d_run_l", "mo_pc%02d_run_r", "mo_pc%02d_dush_f", "mo_pc%02d_ad01", "mo_pc%02d_tb_f_ad01",
    "mo_pc%02d_tb_b_ad01", "mo_pc%02d_run_f_ti", "mo_pc%02d_run_b_ti", "mo_pc%02d_run_l_ti", "mo_pc%02d_run_r_ti",
    "mo_pc%02d_dush_f_ti", "sh_pc%02d_f_t", "sh_pc%02d_b_t", "sh_pc%02d_f_s", "sh_pc%02d_b_s", "sh_pc%02d_f_l",
    "sh_pc%02d_b_l", "sh_pc%02d_f_v", "sh_pc%02d_b_v", "sh_pc%02d_f_v_l", "sh_pc%02d_b_v_l", "sh_pc%02d_f_t_c",
    "sh_pc%02d_b_t_c", "sh_pc%02d_f_w", "sh_pc%02d_b_w", "mo_pc%02d_receive_f", "sh_pc%02d_s_n",
    "sh_pc%02d_serve_ad00", "mo_pc%02d_serve_l", "mo_pc%02d_serve_r", "sh_pc%02d_serve_up00",
    "sh_pc%02d_serve_up01", "sh_pc%02d_serve_t", "sh_pc%02d_serve_u", "sh_pc%02d_f_k01", "sh_pc%02d_b_k01",
    "sh_pc%02d_s_k01", "sh_pc%02d_s_u_k01", "re_pc%02d_ball", "re_pc%02d_gu", "re_pc%02d_di", "re_pc%02d_gu_set",
    "re_pc%02d_di_set", "sh_pc%02d_s_n_dummy", "sh_pc%02d_serve_t_dummy", "re_pc%02d_ball_dummy",
    "mo_pc%02d_receive_f_dummy", "sh_pc%02d_serve_ad00_ball", "re_pc%02d_gu_set_dummy", "re_pc%02d_co01_f",
];

/// A motion's file stem for character `character`.
pub fn motion_name(motion: usize, character: usize) -> Option<String> {
    MOTIONS.get(motion).map(|m| m.replace("%02d", &format!("{character:02}")))
}
