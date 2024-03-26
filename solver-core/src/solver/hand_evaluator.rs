use std::cmp::Ordering;
use std::thread::current;

use bitflags::bitflags;
use heapless;
use itertools::Itertools;
use slab::Slab;
use strum::IntoEnumIterator;

use crate::solver::cards::{Card, CardView, Hand, HandKind, Rank, Suit};
use crate::solver::cardset::CardSet;
use crate::solver::error::{Error, Result};
use crate::{card, cards, hand};

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct Options: u32 {
        const GappedStraights             = 0b0001;
        const FourCardStraightsAndFlushes = 0b0010;
    }
}

#[derive(Debug)]
pub struct HandEvaluator {
    len: usize,
    cards: Hand,
    sorted: Hand,
    cardset: CardSet,
    options: Options,
}

impl HandEvaluator {
    fn new(card_view: impl CardView, options: Options) -> Self {
        let card_slice = card_view.view();
        let len = card_slice.len();

        // For now, we're restricting ourselves to scoring 5-card hands, because that eliminates
        // any "tiebreaking" that we'd have to do when scoring more than 5 cards at once.
        // This doesn't directly affect Balatro, because you can only play 5 cards anyways, but it's
        // probably still worth extending to this functionality at some point.
        assert!(len <= 5);

        let cards = Hand::from_slice(card_slice).unwrap();

        let mut sorted = cards.clone();
        sorted.cards.sort_by(|a, b| b.cmp(a));

        let cardset = CardSet::from(card_view);

        Self {
            len,
            cards,
            sorted,
            cardset,
            options,
        }
    }

    fn evaluate_high_card(&self) -> Option<Hand> {
        Some(Hand::from_slice(&[*self.sorted.cards.first()?]).unwrap())
    }

    fn evaluate_suit_matches(&self, four_card: bool) -> Option<Hand> {
        let length = if four_card { 4 } else { 5 };

        if self.len < length {
            return None;
        }

        if self.cardset.count() < self.len {
            let mut seen: [usize; 4] = [0, 0, 0, 0];

            for card in self.cards.view() {
                seen[card.suit as usize] += 1;
            }

            for count in seen {
                if count >= length {
                    return Some(self.cards.clone());
                }
            }

            None
        } else {
            Suit::iter()
                .find(|suit| self.cardset.count_in_suit(*suit) == 5)
                .map(|_| self.cards.clone())
        }
    }

    fn evaluate_run(&self) -> Option<Hand> {
        #[inline]
        fn is_consecutive(left: Rank, right: Rank) -> bool {
            if left == Rank::Deuce && right == Rank::Ace {
                true
            } else {
                (left as i32 - 1) == (right as i32)
            }
        }

        #[inline]
        fn has_gap(left: Rank, right: Rank) -> bool {
            if left == Rank::Three && right == Rank::Ace {
                true
            } else {
                (left as i32 - 2) == (right as i32)
            }
        }

        let four_card = self.options.contains(Options::FourCardStraightsAndFlushes);

        let min_length = if four_card { 4 } else { 5 };
        if self.len < min_length {
            return None;
        }

        let mut can_gap = self.options.contains(Options::GappedStraights);
        let mut can_fail = self.options.contains(Options::FourCardStraightsAndFlushes);
        let mut straight_length = 1;

        for (i, (left, right)) in self.sorted.view().iter().tuple_windows().enumerate() {
            let consecutive = is_consecutive(left.rank, right.rank);
            let gapped = has_gap(left.rank, right.rank);

            if consecutive {
                straight_length += 1;
            } else if gapped && can_gap {
                straight_length += 1;
                can_gap = false;
            } else if four_card && i == 3 {
                break;
            } else if can_fail {
                straight_length = 1;
                can_fail = false;
            } else {
                break;
            }
        }

        if straight_length == 5 || (self.len == 4 && four_card && straight_length == 4) {
            Some(self.cards.clone())
        } else if four_card && straight_length == 4 {
            let except_card = if can_fail { 4 } else { 0 };

            let mut ditched_card = false;
            let mut vec = heapless::Vec::<_, 5>::new();

            for card in self.cards.view() {
                if !ditched_card && card == &self.sorted.view()[except_card] {
                    ditched_card = true;
                    continue;
                }
                vec.push(*card).unwrap();
            }

            Some(Hand::from(vec))
        } else {
            None
        }
    }

    fn evaluate_rank_matches(&self, match_size: usize, match_count: usize) -> Option<Hand> {
        let min_length = match_size * match_count;
        if self.len < min_length {
            return None;
        }

        let mut ranks: [u8; 13] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        for card in self.sorted.view() {
            ranks[card.rank as usize] += 1;
        }

        let matched_ranks: Vec<_> = ranks
            .iter()
            .enumerate()
            .filter(|(_, count)| (**count as usize) == match_size)
            .map(|(index, _)| index)
            .collect();

        (matched_ranks.len() == match_count).then(|| {
            self.cards
                .view()
                .iter()
                .copied()
                .filter(|card| matched_ranks.contains(&(card.rank as usize)))
                .collect()
        })
    }

    fn evaluate_full_house(&self) -> Option<Hand> {
        if self.len < 5 {
            return None;
        }

        let sorted_cards = self.sorted.view();

        let first_rank = sorted_cards[0].rank;

        if sorted_cards[1].rank != first_rank {
            return None;
        }

        if sorted_cards[2].rank == first_rank {
            let second_rank = sorted_cards[3].rank;

            if second_rank == first_rank {
                return None;
            }

            if sorted_cards[4].rank != second_rank {
                return None;
            }
        } else {
            let second_rank = sorted_cards[2].rank;

            if sorted_cards[3].rank != second_rank {
                return None;
            }

            if sorted_cards[4].rank != second_rank {
                return None;
            }
        }

        Some(self.cards.clone())
    }

    fn evaluate(&self) -> Option<(HandKind, Hand)> {
        if self.cards.view().is_empty() {
            return None;
        }

        let five_card_flush = self.evaluate_suit_matches(false).is_some();

        // 1. FLUSH FIVE
        if five_card_flush {
            if let Some(hand) = self.evaluate_rank_matches(5, 1) {
                return Some((HandKind::FlushFive, hand));
            }
        }

        let full_house = self.evaluate_full_house();

        // 2. FLUSH HOUSE
        if five_card_flush {
            if let Some(hand) = full_house {
                return Some((HandKind::FlushHouse, hand));
            }
        }

        // 3. FIVE OF A KIND
        if let Some(hand) = self.evaluate_rank_matches(5, 1) {
            return Some((HandKind::FiveOfAKind, hand));
        }

        let straight = self.evaluate_run();

        // 4. STRAIGHT FLUSH
        if let Some(straight) = straight.clone() {
            if straight
                .view()
                .iter()
                .all(|card| card.suit == straight.view()[0].suit)
            {
                return Some((HandKind::StraightFlush, straight));
            }
        }

        // 5. FOUR OF A KIND
        if let Some(hand) = self.evaluate_rank_matches(4, 1) {
            return Some((HandKind::FourOfAKind, hand));
        }

        // 6. FULL HOUSE
        if let Some(hand) = full_house {
            return Some((HandKind::FullHouse, hand));
        }

        // 7. FLUSH
        if let Some(hand) =
            self.evaluate_suit_matches(self.options.contains(Options::FourCardStraightsAndFlushes))
        {
            return Some((HandKind::Flush, hand));
        }

        // 8. STRAIGHT
        if let Some(hand) = straight {
            return Some((HandKind::Straight, hand));
        }

        // 9. THREE OF A KIND
        if let Some(hand) = self.evaluate_rank_matches(3, 1) {
            return Some((HandKind::ThreeOfAKind, hand));
        }

        // 10. TWO PAIR
        if let Some(hand) = self.evaluate_rank_matches(2, 2) {
            return Some((HandKind::TwoPair, hand));
        }

        // 11. PAIR
        if let Some(hand) = self.evaluate_rank_matches(2, 1) {
            return Some((HandKind::Pair, hand));
        }

        // 12. HIGH CARD
        if let Some(hand) = self.evaluate_high_card() {
            return Some((HandKind::HighCard, hand));
        }

        unreachable!()
    }

    pub fn evaluate_poker_hand(
        card_view: impl CardView,
        options: Options,
    ) -> Option<(HandKind, Hand)> {
        let evaluator = Self::new(card_view, options);
        evaluator.evaluate()
    }

    fn find_best(&self) -> Option<(HandKind, Hand)> {
        todo!()
    }

    pub fn find_best_poker_hand(
        card_view: impl CardView,
        options: Options,
    ) -> Option<(HandKind, Hand)> {
        let evaluator = Self::new(card_view, options);
        evaluator.find_best()
    }
}

#[cfg(test)]
mod tests {
    use crate::solver::cards::CardCollection;

    use super::*;

    fn expect(
        cards: impl CardView,
        expected_kind: HandKind,
        expected_hand: Hand,
        options: Options,
    ) {
        let hand = HandEvaluator::evaluate_poker_hand(cards, options);
        assert_eq!(hand, Some((expected_kind, expected_hand)));
    }

    #[test]
    fn empty_hand_test() {
        let cards = CardCollection::empty();
        assert_eq!(
            HandEvaluator::evaluate_poker_hand(cards, Options::empty()),
            None
        );
    }

    #[test]
    fn high_card_test() {
        expect(
            cards!("AS 9C 6C KH TS"),
            HandKind::HighCard,
            hand!("AS"),
            Options::empty(),
        );
    }

    #[test]
    fn pair_test() {
        expect(
            cards!("9S 2S 3C 9D AS"),
            HandKind::Pair,
            hand!("9S 9D"),
            Options::empty(),
        );
    }

    #[test]
    fn two_pair_test() {
        expect(
            cards!("9S 2S 3C 9D 2S"),
            HandKind::TwoPair,
            hand!("9S 2S 9D 2S"),
            Options::empty(),
        );
    }

    #[test]
    fn three_of_a_kind_test() {
        expect(
            cards!("9S 2S 3C 9D 9S"),
            HandKind::ThreeOfAKind,
            hand!("9S 9D 9S"),
            Options::empty(),
        );
    }

    #[test]
    fn straight_test() {
        expect(
            cards!("5S 8D 7S 6C 9S"),
            HandKind::Straight,
            hand!("5S 8D 7S 6C 9S"),
            Options::empty(),
        );
        // Gapped straights
        expect(
            cards!("5S 8D 7S 6C TS"),
            HandKind::HighCard,
            hand!("TS"),
            Options::empty(),
        );
        expect(
            cards!("5S 8D 7S 6C TS"),
            HandKind::Straight,
            hand!("5S 8D 7S 6C TS"),
            Options::GappedStraights,
        );
        expect(
            cards!("8D 2S 6D 4S 7D"),
            HandKind::HighCard,
            hand!("8D"),
            Options::GappedStraights,
        );
        // Four-card straights
        expect(
            cards!("5S AS 8D 7S 6C"),
            HandKind::HighCard,
            hand!("AS"),
            Options::empty(),
        );
        expect(
            cards!("5S 8D 7S 6C"),
            HandKind::Straight,
            hand!("5S 8D 7S 6C"),
            Options::FourCardStraightsAndFlushes,
        );
        expect(
            cards!("5S AS 8D 7S 6C"),
            HandKind::Straight,
            hand!("5S 8D 7S 6C"),
            Options::FourCardStraightsAndFlushes,
        );
        expect(
            cards!("5S 2S 8D 7S 6C"),
            HandKind::Straight,
            hand!("5S 8D 7S 6C"),
            Options::FourCardStraightsAndFlushes,
        );
        // Gapped four-card straights
        expect(
            cards!("8D 2S 6D 4S 7D"),
            HandKind::Straight,
            hand!("8D 6D 4S 7D"),
            Options::FourCardStraightsAndFlushes | Options::GappedStraights,
        );
    }

    #[test]
    fn flush_test() {
        expect(
            cards!("AS TS 9S 2S 5S"),
            HandKind::Flush,
            hand!("AS TS 9S 2S 5S"),
            Options::empty(),
        );
        expect(
            cards!("AS AS AS TS 2S"),
            HandKind::Flush,
            hand!("AS AS AS TS 2S"),
            Options::empty(),
        );
    }

    #[test]
    fn full_house_test() {
        expect(
            cards!("9S 2D 2S 9D 9C"),
            HandKind::FullHouse,
            hand!("9S 2D 2S 9D 9C"),
            Options::empty(),
        );
        expect(
            cards!("2S 2D 2S 9D 9C"),
            HandKind::FullHouse,
            hand!("2S 2D 2S 9D 9C"),
            Options::empty(),
        );
    }

    #[test]
    fn four_of_a_kind_test() {
        expect(
            cards!("9S 9D 2S 9D 9C"),
            HandKind::FourOfAKind,
            hand!("9S 9D 9D 9C"),
            Options::empty(),
        );
    }

    #[test]
    fn straight_flush() {
        expect(
            cards!("5S 8S 7S 6S 9S"),
            HandKind::StraightFlush,
            hand!("5S 8S 7S 6S 9S"),
            Options::empty(),
        );
        // Gapped straights
        expect(
            cards!("5D 8D 7D 6D TD"),
            HandKind::StraightFlush,
            hand!("5D 8D 7D 6D TD"),
            Options::GappedStraights,
        );
        // Four-card straights
        expect(
            cards!("AD 5S 8S 7S 6S"),
            HandKind::StraightFlush,
            hand!("5S 8S 7S 6S"),
            Options::FourCardStraightsAndFlushes,
        );
        expect(
            cards!("AS 5D 8S 7S 6S"),
            HandKind::Straight,
            hand!("5D 8S 7S 6S"),
            Options::FourCardStraightsAndFlushes,
        );
        // Gapped four-card straights
        expect(
            cards!("8H 2S 6H 4H 7H"),
            HandKind::StraightFlush,
            hand!("8H 6H 4H 7H"),
            Options::FourCardStraightsAndFlushes | Options::GappedStraights,
        );
    }

    #[test]
    fn five_of_a_kind_test() {
        expect(
            cards!("9S 9D 9S 9D 9C"),
            HandKind::FiveOfAKind,
            hand!("9S 9D 9S 9D 9C"),
            Options::empty(),
        );
    }

    #[test]
    fn flush_house_test() {
        expect(
            cards!("9S AS 9S AS 9S"),
            HandKind::FlushHouse,
            hand!("9S AS 9S AS 9S"),
            Options::empty(),
        );
    }

    #[test]
    fn flush_five_test() {
        expect(
            cards!("9S 9S 9S 9S 9S"),
            HandKind::FlushFive,
            hand!("9S 9S 9S 9S 9S"),
            Options::empty(),
        );
    }
}
