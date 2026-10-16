//! The character select's hand toggle (Select / −): a player whose toggle is on plays their character with the other
//! hand. The original keeps it as the model's left-hand flag = (TParam's 利き腕 is left) XOR the toggle, and the
//! player's hand (±1) follows the flag: mirrored model and racket hand, forehand/backhand sides, reach,
//! serve walk and toss. The AI's strong side stays TParam's (`Game::strong_hand`).

use super::Game;
use bevy::prelude::*;

/// `--swap-hands 1,0,…` by match slot: whose toggle was on at the select.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub struct SwapHands(pub [bool; 4]);

impl SwapHands {
    pub fn parse(s: &str) -> Self {
        let mut out = [false; 4];
        for (o, v) in out.iter_mut().zip(s.split(',')) {
            *o = v.trim() == "1";
        }
        SwapHands(out)
    }
}

/// Player `i`, set up with its character's (or mod's) hand: keeps that as the strong side, then flips the hand the
/// match plays with if the toggle is on.
pub(super) fn apply(g: &mut Game, i: usize, swap: Option<&SwapHands>) {
    g.strong_hand[i] = g.players[i].hand;
    if swap.is_some_and(|s| s.0[i]) {
        g.players[i].hand = -g.players[i].hand;
        g.reaches[i].hand = g.players[i].hand;
    }
}
