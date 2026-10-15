//! Sim code does its float math through `ps2::F` / `vu0::Vf` (C2): the operators are the EE FPU's / VU0's in the
//! written order, so a converted module reads as plain math. Each module of `src/` must carry
//! `#![deny(clippy::float_arithmetic)]` (no plain `f32` arithmetic: `tools/done.sh` runs clippy, which enforces
//! it) and must not call the `ps2` / `vu0` arithmetic functions by hand, unless it is still listed below. The lists
//! only shrink: a listed module that is already clean fails too, so converting one means deleting its name.

use std::path::Path;

/// Modules with plain `f32` arithmetic left (no deny attribute yet).
const PLAIN: &[&str] = &[
    "aim", "camera", "cutaway", "effect", "flow", "foot", "instant", "judge", "motion", "npc", "params",
    "pose", "position", "rally", "serve", "shade", "sound", "stats", "swing", "umpire",
];

/// Modules still calling `ps2::add(…)`, `vu0::mul(…)` … instead of the operators.
const CALLS: &[&str] = &[
    "aim", "effect", "face", "foot", "instant", "judge", "motion", "npc", "params", "pose", "position",
    "rally", "serve", "shade", "sound", "swing",
];

const OPS: [&str; 8] = ["add", "sub", "mul", "div", "madd", "msub", "sqrt", "lerp"];

/// Whether `src` calls a `ps2`/`vu0` arithmetic function: a qualified call, or one imported by name.
/// `vu0::lerp` is the four-lane blend (a vector helper like `vu0::dot3`), not a lane op.
fn calls_by_hand(src: &str) -> bool {
    let qualified = ["ps2::", "vu0::"].iter().any(|m| {
        OPS.iter().filter(|&&op| !(*m == "vu0::" && op == "lerp")).any(|op| src.contains(&format!("{m}{op}(")) || src.contains(&format!("{m}{op},")))
    });
    let imported = src.lines().filter(|l| l.trim_start().starts_with("use crate::ps2") || l.trim_start().starts_with("use crate::vu0")).any(|l| {
        l.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| OPS.contains(&w))
    });
    qualified || imported
}

#[test]
fn sim_modules_use_the_fpu_types() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut bad = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_stem().unwrap().to_str().unwrap().to_owned();
        if p.extension().is_none_or(|x| x != "rs") || name == "lib" {
            continue;
        }
        let src = std::fs::read_to_string(&p).unwrap();
        let denied = src.contains("#![deny(clippy::float_arithmetic)]");
        if denied == PLAIN.contains(&name.as_str()) {
            bad.push(if denied { format!("{name}: has the deny attribute, drop it from PLAIN") } else { format!("{name}: missing #![deny(clippy::float_arithmetic)]") });
        }
        // the two modules defining the arithmetic call their own functions
        let hand = calls_by_hand(&src) && name != "ps2" && name != "vu0";
        if hand != CALLS.contains(&name.as_str()) {
            bad.push(if hand { format!("{name}: calls ps2/vu0 arithmetic by hand, use F / Vf operators") } else { format!("{name}: no hand calls left, drop it from CALLS") });
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
