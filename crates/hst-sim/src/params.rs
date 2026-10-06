//! Per-character shot parameters, built exactly as the game builds its runtime table at start-up.
//!
//! Index: shot class (0 serve, 1 stroke, 2 volley, 3 smash) × kind × record. Records 0–2 are three base
//! archetypes stored on disc; record `r` = character id + 3 is that character's blend of them, with weights
//! from a per-record row. A record is 13 f32: [0..3) aim/launch, [3..7) middle group, [7..13) spin (deg),
//! first-bounce spin (deg), curve terms, first-bounce restitution pair.

use crate::ps2;

/// Stride of one shot class, one kind and one record, in floats (as in the game's table).
const CLASS: usize = 0x451;
const KIND: usize = 0xdd;
const REC: usize = 0xd;
/// Records per kind: 3 archetypes + 14 characters.
pub const RECORDS: usize = 17;

pub struct ShotParams {
    table: Vec<f32>,
}

/// Weight row of record `r` (0x7c bytes = 31 floats).
fn w(weights: &[f32], r: usize, byte_off: usize) -> f32 {
    weights[r * 0x7c / 4 + byte_off / 4]
}

impl ShotParams {
    /// `base`: on-disc table (4 classes × 0x451 floats); `kinds`: kinds per class; `weights`: 17 rows of
    /// 0x7c bytes; `mid`: mix between the two weights for the middle group.
    pub fn build(base: &[f32], kinds: &[i32], weights: &[f32], mid: f32) -> Self {
        let mut t = vec![0.0f32; 4 * CLASS];
        for class in 0..4 {
            for kind in 0..kinds[class].max(0) as usize {
                let k0 = class * CLASS + kind * KIND;
                for r in 0..RECORDS {
                    let o = k0 + r * REC;
                    if r < 3 {
                        t[o..o + REC].copy_from_slice(&base[o..o + REC]);
                        continue;
                    }
                    // the three archetypes this record blends (taken from the table being built, like the game)
                    let a = |i: usize| t[k0 + i];
                    // first weight: by class, with an override for kind 3 of classes 1–2
                    let mut wa = w(weights, r, 0x4);
                    if (1..=2).contains(&class) && kind == 3 {
                        wa = w(weights, r, 0xc);
                    } else if class == 0 || class == 3 {
                        wa = w(weights, r, 0x0);
                    } else if class == 2 {
                        wa = w(weights, r, 0x8);
                    }
                    let mut out = [0.0f32; REC];
                    // weight ≥ 0 moves from archetype 1 toward 0, < 0 from archetype 1 toward 2
                    let blend = |out: &mut [f32; REC], wt: f32, fields: std::ops::Range<usize>| {
                        for f in fields {
                            let (from, to) = if wt >= 0.0 { (a(REC + f), a(f)) } else { (a(REC + f), a(2 * REC + f)) };
                            out[f] = ps2::lerp(from, to, wt.abs());
                        }
                    };
                    blend(&mut out, wa, 0..3);
                    let mut wb = w(weights, r, 0x14);
                    if (1..=2).contains(&class) && kind == 3 {
                        wb = w(weights, r, 0xc);
                    } else if class < 3 && kind == 1 {
                        wb = w(weights, r, 0x18);
                    } else if class < 3 && kind == 4 {
                        wb = w(weights, r, 0x1c);
                    }
                    blend(&mut out, wb, 7..13);
                    let wm = ps2::lerp(wa, wb, mid);
                    blend(&mut out, wm, 3..7);
                    t[o..o + REC].copy_from_slice(&out);
                }
            }
        }
        Self { table: t }
    }

    pub fn record(&self, class: usize, kind: usize, r: usize) -> &[f32] {
        let o = class * CLASS + kind * KIND + r * REC;
        &self.table[o..o + REC]
    }

    /// Whole table in the game's layout (for verification).
    pub fn raw(&self) -> &[f32] {
        &self.table
    }
}

/// Record index of a character (pc00 = 0 … pc13 = 13).
pub fn record_of(character: usize) -> usize {
    character + 3
}
