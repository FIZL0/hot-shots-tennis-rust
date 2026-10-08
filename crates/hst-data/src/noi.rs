//! `.NOI` costume noise deformers: a count, then from 0x10 one 0x30-byte entry each.

/// One deformer: the node whose packets it moves, its noise period and rate, and its amplitudes.
#[derive(Clone, Debug, PartialEq)]
pub struct Deformer {
    pub node: usize,
    pub period: f32,
    pub rate: f32,
    pub amp: [f32; 3],
    pub name: String,
}

pub fn parse(d: &[u8]) -> Option<Vec<Deformer>> {
    let f = |o: usize| Some(f32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?));
    let n = u32::from_le_bytes(d.get(..4)?.try_into().ok()?) as usize;
    (0..n)
        .map(|i| {
            let o = 0x10 + i * 0x30;
            let name = d.get(o + 0x20..o + 0x30)?;
            Some(Deformer {
                node: u16::from_le_bytes([*d.get(o + 2)?, *d.get(o + 3)?]) as usize,
                period: f(o + 8)?,
                rate: f(o + 0xc)?,
                amp: [f(o + 0x10)?, f(o + 0x14)?, f(o + 0x18)?],
                name: String::from_utf8_lossy(name.split(|&b| b == 0).next()?).into_owned(),
            })
        })
        .collect()
}
