use static_assertions::const_assert;

use super::cards::{Card, CardView, Rank, Suit};
use crate::{card, cards};

#[derive(Copy, Clone, Debug)]
pub(crate) struct CardSet(u64);

#[rustfmt::skip]
impl CardSet {
    const SPADES_MASK:   u64 = 0x0000_0000_0000_1fff;
    const CLUBS_MASK:    u64 = 0x0000_0000_1fff_0000;
    const HEARTS_MASK:   u64 = 0x0000_1fff_0000_0000;
    const DIAMONDS_MASK: u64 = 0x1fff_0000_0000_0000;

    const ALL_CARDS_MASK: u64 = Self::SPADES_MASK | Self::CLUBS_MASK | Self::HEARTS_MASK | Self::DIAMONDS_MASK;

    const MASK_TABLE: [u64; 4] = [
        Self::SPADES_MASK,
        Self::CLUBS_MASK,
        Self::HEARTS_MASK,
        Self::DIAMONDS_MASK,
    ];
}

const_assert!(CardSet::SPADES_MASK.count_ones() == 13);
const_assert!(CardSet::CLUBS_MASK.count_ones() == 13);
const_assert!(CardSet::HEARTS_MASK.count_ones() == 13);
const_assert!(CardSet::DIAMONDS_MASK.count_ones() == 13);
const_assert!(CardSet::ALL_CARDS_MASK.count_ones() == 52);

#[allow(clippy::multiple_inherent_impl)]
impl CardSet {
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn full() -> Self {
        Self(Self::ALL_CARDS_MASK)
    }

    pub fn count(self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn count_in_suit(self, suit: Suit) -> usize {
        (self.0 & Self::MASK_TABLE[suit as usize]).count_ones() as usize
    }

    pub fn insert(&mut self, card: Card) {
        self.0 |= Self::get_mask(card);
    }

    pub fn remove(&mut self, card: Card) {
        self.0 &= Self::ALL_CARDS_MASK & !Self::get_mask(card);
    }

    pub fn contains(self, card: Card) -> bool {
        (self.0 & Self::get_mask(card)) > 0
    }

    #[inline]
    fn get_index(card: Card) -> usize {
        ((card.suit as u8 as usize) << 4) | (card.rank as u8 as usize)
    }

    #[inline]
    fn get_mask(card: Card) -> u64 {
        1_u64 << Self::get_index(card)
    }
}

impl<V: CardView> From<V> for CardSet {
    fn from(value: V) -> Self {
        let mut cardset = Self::empty();
        for card in value.view() {
            cardset.insert(*card);
        }
        cardset
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn counting_test() {
        let mut cardset = CardSet::full();

        assert_eq!(cardset.count(), 52);
        assert_eq!(cardset.count_in_suit(Suit::Spades), 13);
        assert_eq!(cardset.count_in_suit(Suit::Clubs), 13);
        assert_eq!(cardset.count_in_suit(Suit::Hearts), 13);
        assert_eq!(cardset.count_in_suit(Suit::Diamonds), 13);

        assert!(cardset.contains(card!("KH")));
        cardset.remove(card!("KH"));
        assert!(!cardset.contains(card!("KH")));
        assert_eq!(cardset.count(), 51);
        assert_eq!(cardset.count_in_suit(Suit::Hearts), 12);
    }

    #[test]
    fn from_cardview_test() {
        let cardset = CardSet::from(cards!("KH TS 9D 8C 8C 8C TS KS KD"));
        assert_eq!(cardset.count(), 6);
    }
}
