//! P4: the contact search's skeleton-derived reach, per player, as the game measures it on the character's own
//! motions at load: the stroke contact point ahead and the body-shot widths (the arm table, strokes at frame 8),
//! the smash's contact point ahead (the smash motion's racket point at frame 8) and the dive arm (the receive
//! motion at time 5). A mod character's come from its own motions.

use hst_sim::swing::Reach;
use hst_sim::motion::id;

use crate::character::CharacterData;

/// Fill `r` from the character's skeleton and motions; a value whose motions it lacks stays as it was.
pub(super) fn apply(r: &mut Reach, data: &CharacterData) {
    if let Some(arm) = &data.arm {
        (r.ahead, r.body_low, r.body_high) = (arm.reach, arm.side, arm.hand_x);
    }
    if let Some(clip) = data.motions.get(&(id::SMASH as usize)) {
        r.smash_ahead = hst_sim::serve::serve_arm(&data.skeleton, clip, None, false).point[2];
    }
    if let Some(clip) = data.motions.get(&(id::RECEIVE as usize)) {
        r.dive_arm = hst_sim::pose::dive_arm(&data.skeleton, clip);
    }
}
