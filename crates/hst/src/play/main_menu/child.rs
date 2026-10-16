//! B202: a child-button submenu. Choosing a row that needs a second word (Remove, Upload, Sign out...) opens a short
//! column of buttons to the right of that row, over the list, with Cancel last; the rest of the screen dims and takes
//! no input. Up/down move between the buttons, ✕ acts (Cancel closes), ○ / Back closes. Mouse and touch hit only the
//! buttons while it's open; a tap anywhere else is Back (`Draw::popup`). Our own design (menus aren't 1:1).

use super::{Effect, Press};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Child {
    /// The row it hangs off, and the button under the cursor.
    pub row: usize,
    pub at: usize,
    /// Its buttons, Cancel last.
    pub opts: Vec<&'static str>,
}

/// What a press on an open child did.
#[derive(Debug, PartialEq)]
pub(super) enum Got {
    Open,
    Close,
    /// Button `i` (not Cancel) was chosen; the caller acts and plays the sound.
    Pick(usize),
}

impl Child {
    pub fn new(row: usize, opts: &[&'static str]) -> Self {
        let mut opts = opts.to_vec();
        opts.push("Cancel");
        Self { row, at: 0, opts }
    }

    pub fn step(&mut self, k: Press, out: &mut Vec<Effect>) -> Got {
        let n = self.opts.len();
        if k.up || k.down {
            self.at = if k.down { (self.at + 1) % n } else { (self.at + n - 1) % n };
            out.push(Effect::Sound(0));
            Got::Open
        } else if k.back || (k.ok && self.at == n - 1) {
            out.push(Effect::Sound(1));
            Got::Close
        } else if k.ok {
            Got::Pick(self.at)
        } else {
            Got::Open
        }
    }
}
