//! Per-character shot parameters, built exactly as the game builds its runtime table at start-up.
//!
//! Index: shot class (`shot::class`: serve, stroke, volley, smash) × kind × record. Records 0–2 are three base
//! archetypes stored on disc; record `r` = character id + 3 is that character's blend of them, with weights
//! from a per-record row. A record is 13 f32 (`field`): [0..3) aim/launch, [3..7) middle group, [7..13) spin
//! (deg), first-bounce spin (deg), curve terms, first-bounce restitution pair.
#![deny(clippy::float_arithmetic)]

use crate::lines::{BASELINE, DEG};
use crate::ps2::F;
use crate::shot::{class, kind};
use std::ops::Range;

/// Shot classes in the table, and room for kinds per class.
const CLASSES: usize = 4;
const KINDS: usize = 5;
/// Records per kind: 3 archetypes + 14 characters.
pub const RECORDS: usize = 17;
/// Floats in one record, and the strides of one kind and one class (as in the game's table).
const REC: usize = 13;
const KIND: usize = RECORDS * REC;
const CLASS: usize = KINDS * KIND;
/// The archetypes every character's record blends: the weight moves from `MIDDLE` toward `HIGH` (≥ 0) or `LOW`.
const HIGH: usize = 0;
const MIDDLE: usize = 1;
const LOW: usize = 2;
const ARCHETYPES: usize = 3;

/// A record's fields (indices into its 13 floats).
pub mod field {
    use std::ops::Range;
    /// Aim and launch terms, blended by the record's first weight.
    pub const LAUNCH: Range<usize> = 0..3;
    /// The middle group, blended by a mix of the first and second weights.
    pub const MIDDLE: Range<usize> = 3..7;
    /// Spin and bounce terms, blended by the second weight.
    pub const SPIN_GROUP: Range<usize> = 7..13;
    /// Launch spin (degrees).
    pub const SPIN: usize = 7;
    /// First-bounce spin (degrees).
    pub const FIRST_SPIN: usize = 9;
    /// Side angle (degrees; nonzero only on slice serves).
    pub const SIDE: usize = 10;
    /// First-bounce restitution, and the value a low deep hit slides it toward (0: none).
    pub const RESTITUTION: usize = 11;
    pub const RESTITUTION_LOW: usize = 12;
}

/// A character's weight row (31 floats): the slots `build` reads.
mod weight {
    pub const ROW: usize = 31;
    /// First weight: serves and smashes, strokes, volleys, and kind 3 of strokes and volleys (also its second).
    pub const SERVE_SMASH: usize = 0;
    pub const STROKE: usize = 1;
    pub const VOLLEY: usize = 2;
    pub const LOB: usize = 3;
    /// Second weight: the default, then kinds 1 and 4 of serves, strokes and volleys.
    pub const SPIN: usize = 5;
    pub const SPIN_KIND1: usize = 6;
    pub const SPIN_KIND4: usize = 7;
}

pub struct ShotParams {
    table: Vec<f32>,
}

/// Slot `slot` of record `r`'s weight row.
fn w(weights: &[f32], r: usize, slot: usize) -> f32 {
    weights[r * weight::ROW + slot]
}

impl ShotParams {
    /// `base`: on-disc table (`CLASSES` × `CLASS` floats); `kinds`: kinds per class; `weights`: `RECORDS` rows of
    /// 31 floats (`weight::ROW`); `mid`: mix between the two weights for the middle group.
    pub fn build(base: &[f32], kinds: &[i32], weights: &[f32], mid: f32) -> Self {
        let mut t = vec![0.0f32; CLASSES * CLASS];
        for class in 0..CLASSES {
            for kind in 0..kinds[class].max(0) as usize {
                let k0 = class * CLASS + kind * KIND;
                for r in 0..RECORDS {
                    let o = k0 + r * REC;
                    if r < ARCHETYPES {
                        t[o..o + REC].copy_from_slice(&base[o..o + REC]);
                        continue;
                    }
                    // the three archetypes this record blends (taken from the table being built, like the game)
                    let a = |i: usize| t[k0 + i];
                    // first weight: by class, with an override for the lob (kind 3) of strokes and volleys
                    let rally = class == class::STROKE as usize || class == class::VOLLEY as usize;
                    let lob = rally && kind == kind::LOB as usize;
                    let mut wa = w(weights, r, weight::STROKE);
                    if lob {
                        wa = w(weights, r, weight::LOB);
                    } else if class == class::SERVE as usize || class == class::SMASH as usize {
                        wa = w(weights, r, weight::SERVE_SMASH);
                    } else if class == class::VOLLEY as usize {
                        wa = w(weights, r, weight::VOLLEY);
                    }
                    let mut out = [0.0f32; REC];
                    // weight ≥ 0 moves from the middle archetype toward the high one, < 0 toward the low one
                    let blend = |out: &mut [f32; REC], wt: f32, fields: Range<usize>| {
                        for f in fields {
                            let (from, to) = if wt >= 0.0 { (a(MIDDLE * REC + f), a(HIGH * REC + f)) } else { (a(MIDDLE * REC + f), a(LOW * REC + f)) };
                            out[f] = F(from).lerp(to, wt.abs()).0;
                        }
                    };
                    blend(&mut out, wa, field::LAUNCH);
                    let not_smash = class < class::SMASH as usize;
                    let mut wb = w(weights, r, weight::SPIN);
                    if lob {
                        wb = w(weights, r, weight::LOB);
                    } else if not_smash && kind == kind::SLICE as usize {
                        wb = w(weights, r, weight::SPIN_KIND1);
                    } else if not_smash && kind == kind::DROP as usize {
                        wb = w(weights, r, weight::SPIN_KIND4);
                    }
                    blend(&mut out, wb, field::SPIN_GROUP);
                    let wm = F(wa).lerp(wb, mid).0;
                    blend(&mut out, wm, field::MIDDLE);
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

    /// A variant record (see `hst_data::exe::ShotVariant`): record `r` moved by |weight| toward the high
    /// archetype (weight > 0) or the low one (weight < 0).
    pub fn variant(&self, class: usize, kind: usize, r: usize, weight: f32) -> [f32; REC] {
        let (rec, to) = (self.record(class, kind, r), self.record(class, kind, if weight > 0.0 { HIGH } else { LOW }));
        std::array::from_fn(|f| F(rec[f]).lerp(to[f], weight.abs()).0)
    }

    /// Whole table in the game's layout (for verification).
    pub fn raw(&self) -> &[f32] {
        &self.table
    }
}

/// A record's launch spin (`field::SPIN`, degrees) in the ball's radians, rounded as the game's FPU does.
pub fn spin(record: &[f32]) -> f32 {
    (F(record[field::SPIN]) * DEG).0
}

/// Record index of a character (pc00 = 0 … pc13 = 13).
pub fn record_of(character: usize) -> usize {
    character + ARCHETYPES
}

/// Spin, first-bounce spin (radians) and first-bounce restitution of a stroke or volley launched from `hit`
/// at `target` with `record`, as the game's launch sets them. Strokes (class 1) shape them by kind: a flat
/// shot spins more when hit low and near the net, a short lob gets extra topspin, a drop shot near the net
/// bites less on its first bounce. A drop's restitution slides toward its second value when hit low and deep.
/// The records' side angle (`field::SIDE`) is zero for every stroke and volley, so the side rotation and the
/// left-hander spin flip that hang on it never apply here.
pub fn rally_spin(record: &[f32], class: usize, kind: usize, hit: [f32; 3], target: [f32; 3]) -> (f32, f32, f32) {
    let clamp01 = |x: F| x.clamp(0.0, 1.0);
    let near_net = |over: f32| clamp01((F(BASELINE) - hit[2].abs()) / over);
    let (mut spin, mut first, mut rest) = (F(spin(record)), F(record[field::FIRST_SPIN]) * DEG, F(record[field::RESTITUTION]));
    if class == class::STROKE as usize {
        match kind as i32 {
            kind::FLAT => {
                let mut low = 1.0 - (-F(hit[1]) / 2.0).min(1.0);
                let mut net = near_net(8.0);
                net = net * net; // powf(net, 2), which fdlibm answers as net·net
                if 1.0 < low + net {
                    low = low / (low + net);
                    net = net / (low + net);
                }
                let lift = |w: F, top: f32| (F::ZERO + 1.0).madd(w, F(top) - 1.0);
                spin = spin * lift(low, 2.5) * lift(net, 3.0);
            }
            kind::LOB => {
                let long = clamp01(((F(target[2]) - hit[2]).abs() - 4.5) / (F(10.0) - 4.5));
                spin = (F::ZERO + spin).madd(F(200.0) * DEG, 1.0 - long);
            }
            kind::DROP => first *= (F::ZERO + 1.0).madd(near_net(8.0), F(0.4) - 1.0),
            _ => {}
        }
    }
    if record[field::RESTITUTION_LOW] != 0.0 {
        let low = clamp01(-F(hit[1]) / 1.5);
        rest = (F::ZERO + rest).madd(low * (1.0 - near_net(8.0)), F(record[field::RESTITUTION_LOW]) - rest);
    }
    (spin.0, first.0, rest.0)
}
