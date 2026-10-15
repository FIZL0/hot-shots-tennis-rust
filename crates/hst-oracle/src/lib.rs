//! Function oracle: calls one of the original's functions with chosen arguments on an R5900 interpreter and stops
//! at its return, so tests can compare a port against the original on many inputs with no emulator running.
//!
//! The interpreter and ELF loader are our game-agnostic `ps2-oracle` crate (FIZL0/ps2-oracle, re-exported here);
//! [`hst`] is the Hot Shots Tennis side: the code from the user's own disc at test time (the boot ELF and the GAME
//! overlay, disc checked by `hst_data::exe::Game`), optionally a RAM dump from `context/ram/` for functions that
//! read game state, and function addresses from `research/oracle.txt` (research notes), never from this crate.

pub mod hst;

pub use hst::{HstEe, addr, root};
pub use ps2_oracle::{DONE, Ee, Hook};

/// The EE's main RAM: 32 MB.
pub const RAM: usize = 32 << 20;
