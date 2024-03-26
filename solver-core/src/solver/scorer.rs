use approx::assert_relative_eq;

use crate::hand;
use crate::solver::cards::{CardView, Hand, HandKind};

pub struct Scorer<'a> {
    kind: HandKind,
    hand: &'a Hand,
}

impl<'a> Scorer<'a> {
    fn new(kind: HandKind, hand: &'a Hand) -> Self {
        Self { kind, hand }
    }

    fn score(&self) -> f32 {
        const HAND_BASE_CHIPS: [f32; 12] = [
            5.0, 10.0, 20.0, 30.0, 30.0, 35.0, 40.0, 60.0, 100.0, 120.0, 140.0, 160.0,
        ];
        const HAND_BASE_MULT: [f32; 12] = [
            1.0, 2.0, 2.0, 3.0, 4.0, 4.0, 4.0, 7.0, 8.0, 12.0, 14.0, 16.0,
        ];
        const RANK_CHIPS: [f32; 13] = [
            2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 10.0, 10.0, 10.0, 11.0,
        ];

        let mut chips = HAND_BASE_CHIPS[self.kind as usize];
        let mult = HAND_BASE_MULT[self.kind as usize];
        for card in self.hand.view() {
            chips += RANK_CHIPS[card.rank as usize];
        }

        chips * mult
    }

    pub fn score_hand(kind: HandKind, hand: &'a Hand) -> f32 {
        let scorer = Self::new(kind, hand);
        scorer.score()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expect_score(hand: &Hand, kind: HandKind, expected_score: f32) {
        let score = Scorer::score_hand(kind, hand);
        assert_relative_eq!(score, expected_score);
    }

    #[test]
    fn scoring_test() {
        expect_score(&hand!("2H 3H 4H 5H 6C"), HandKind::Straight, 200.0);
        expect_score(&hand!("3D 3D 2C 2C"), HandKind::TwoPair, 60.0);
        expect_score(&hand!("AS KS QS JS TS"), HandKind::StraightFlush, 1208.0);
    }
}
