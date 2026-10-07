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

    /// A variant record (see `hst_data::exe::ShotVariant`): record `r` moved by |weight| toward archetype 0
    /// (weight > 0) or 2 (weight < 0).
    pub fn variant(&self, class: usize, kind: usize, r: usize, weight: f32) -> [f32; REC] {
        let (rec, to) = (self.record(class, kind, r), self.record(class, kind, if weight > 0.0 { 0 } else { 2 }));
        std::array::from_fn(|f| ps2::lerp(rec[f], to[f], weight.abs()))
    }

    /// Whole table in the game's layout (for verification).
    pub fn raw(&self) -> &[f32] {
        &self.table
    }
}

/// A record's launch spin (field 7, degrees) in the ball's radians, rounded as the game's FPU does.
pub fn spin(record: &[f32]) -> f32 {
    ps2::mul(record[7], 0.017453292)
}

/// Record index of a character (pc00 = 0 … pc13 = 13).
pub fn record_of(character: usize) -> usize {
    character + 3
}

/// Spin, first-bounce spin (radians) and first-bounce restitution of a stroke or volley launched from `hit`
/// at `target` with `record`, as the game's launch sets them. Strokes (class 1) shape them by kind: a flat
/// shot spins more when hit low and near the net, a short lob gets extra topspin, a drop shot near the net
/// bites less on its first bounce. A drop's restitution slides toward its second value when hit low and deep.
/// The records' side angle (field 10) is zero for every stroke and volley, so the side rotation and the
/// left-hander spin flip that hang on it never apply here.
pub fn rally_spin(record: &[f32], class: usize, kind: usize, hit: [f32; 3], target: [f32; 3]) -> (f32, f32, f32) {
    let clamp01 = |x: f32| x.clamp(0.0, 1.0);
    let near_net = |over: f32| clamp01(ps2::div(ps2::sub(11.885, hit[2].abs()), over));
    let (mut spin, mut first, mut rest) = (spin(record), ps2::mul(record[9], 0.017453292), record[11]);
    if class == 1 {
        match kind {
            2 => {
                let mut low = ps2::sub(1.0, ps2::div(-hit[1], 2.0).min(1.0));
                let mut net = near_net(8.0);
                net = ps2::mul(net, net); // powf(net, 2), which fdlibm answers as net·net
                if 1.0 < ps2::add(low, net) {
                    low = ps2::div(low, ps2::add(low, net));
                    net = ps2::div(net, ps2::add(low, net));
                }
                let lift = |w: f32, top: f32| ps2::madd(ps2::add(0.0, 1.0), w, ps2::sub(top, 1.0));
                spin = ps2::mul(ps2::mul(spin, lift(low, 2.5)), lift(net, 3.0));
            }
            3 => {
                let long = clamp01(ps2::div(ps2::sub(ps2::sub(target[2], hit[2]).abs(), 4.5), ps2::sub(10.0, 4.5)));
                spin = ps2::madd(ps2::add(0.0, spin), ps2::mul(200.0, 0.017453292), ps2::sub(1.0, long));
            }
            4 => first = ps2::mul(first, ps2::madd(ps2::add(0.0, 1.0), near_net(8.0), ps2::sub(0.4, 1.0))),
            _ => {}
        }
    }
    if record[12] != 0.0 {
        let low = clamp01(ps2::div(-hit[1], 1.5));
        rest = ps2::madd(ps2::add(0.0, rest), ps2::mul(low, ps2::sub(1.0, near_net(8.0))), ps2::sub(record[12], rest));
    }
    (spin, first, rest)
}
