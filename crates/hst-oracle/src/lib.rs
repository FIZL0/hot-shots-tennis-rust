//! Function oracle: a small R5900 (PS2 EE) interpreter that calls one of the original's functions with chosen
//! arguments and stops at its return, so tests can compare a port against the original on many inputs with
//! no emulator running.
//!
//! Laid out like the decomp kit's `ps2-oracle`: [`ee`] is the interpreter and [`disc`] loads an ELF plus overlays,
//! both game-agnostic (they take bytes and load addresses); [`hst`] is the Hot Shots Tennis side: the code from the
//! user's own disc at test time (the boot ELF and the GAME overlay, disc checked by `hst_data::exe::Game`),
//! optionally a RAM dump from `context/ram/` for functions that read game state, and function addresses from
//! `research/oracle.txt` (research notes), never from this crate.

pub mod disc;
pub mod ee;
pub mod hst;

pub use ee::{DONE, Ee, RAM};
pub use hst::{addr, root};
