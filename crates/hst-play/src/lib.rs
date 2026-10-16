//! P61: the match (`play`) and the mode switching it runs under (`mode`, `preload`), on `hst-base`; the front-end
//! (`hst`'s `main_menu/`, the pause screen `play/menu.rs`, `main.rs`) sits on top. It never names `hst`: what the
//! match asks of the menus goes through `play::front_end::FrontEnd`.
//! Re-exports `hst_base`, so `crate::audio` etc. resolve here as in `hst`.

pub use hst_base::*;

pub mod mode;
pub mod play;
pub mod preload;
mod stage;
pub use stage::*;
