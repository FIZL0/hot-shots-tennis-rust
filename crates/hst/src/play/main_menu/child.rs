//! B202: a child-button submenu. Choosing a row that needs a second word (Remove, Upload, Sign out...) opens a short
//! column of buttons to the right of that row, over the list, with Cancel last; the rest of the screen dims and takes
//! no input. Up/down move between the buttons, ✕ acts (Cancel closes), ○ / Back closes. Mouse and touch hit only the
//! buttons while it's open; a tap anywhere else is Back (`Draw::popup`). Our own design (menus aren't 1:1).

use super::{Draw, Effect, Press, Tex, INK, WHITE, pointer};

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

/// The buttons' size, gap and their column's gap from the row.
const W: f32 = 170.0;
const H: f32 = 34.0;
const PITCH: f32 = 40.0;
const GAP: f32 = 14.0;

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

    /// The buttons' rects (640×448, centre-anchored) for the row at `row` ([x, y, w, h]): right of it, kept on the
    /// 4:3 screen and above the description bar.
    pub fn rects(&self, row: [f32; 4]) -> Vec<[f32; 4]> {
        let n = self.opts.len() as f32;
        let x = (row[0] + row[2] + GAP).min(640.0 - 8.0 - W);
        let y = row[1].min(400.0 - PITCH * n);
        (0..self.opts.len()).map(|i| [x, y + PITCH * i as f32, W, H]).collect()
    }
}

/// Draws `c` over the screen drawn so far: the screen dims (the description bar stays lit), its row (`row`, labelled
/// `label`) is drawn again chosen, its buttons beside it with the hand on the chosen one; the screen's own hand goes.
pub(super) fn draw(d: &mut Draw, c: &Child, row: [f32; 4], label: &str, hand_x: f32) {
    use super::super::menu::{CHOSEN, UNCHOSEN};
    if let Some(i) = d.out.iter().rposition(|q| q.tex == Tex::Cursor) {
        d.out.remove(i);
    }
    d.popup(Some(pointer::press(|k| k.back = true)));
    let was = d.anchor;
    d.anchor = super::Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 404.0], [0.0; 3], 70.0);
    d.anchor = super::Anchor::Centre;
    d.pill(row[0], row[1], row[2], [128.0, 110.0, 40.0]);
    d.centred(label, row[0] + row[2] / 2.0, row[1] + 7.0, 24.0, WHITE);
    for (i, (r, word)) in c.rects(row).into_iter().zip(&c.opts).enumerate() {
        let on = i == c.at;
        d.pill_at(r, [INK, WHITE, if on { CHOSEN } else { UNCHOSEN }], 128.0);
        d.hit(r, pointer::Act::Row(i, pointer::OK));
        d.label(word, r[0] + r[2] / 2.0, r[1] + 7.0, 20.0, r[2] - 24.0, if on { WHITE } else { UNCHOSEN.map(|c| c * 1.4) });
    }
    let r = c.rects(row)[c.at];
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [r[0] - 60.0 + hand_x, r[1] + 1.0, 64.0, 32.0], WHITE, 128.0);
    d.anchor = was;
}
