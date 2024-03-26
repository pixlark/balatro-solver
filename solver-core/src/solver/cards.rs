use heapless;
use itertools::Itertools;
use lazy_static::lazy_static;
use rand::{
    prelude::{Rng, SeedableRng, SliceRandom},
    rngs::SmallRng,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::solver::{
    cardset::CardSet,
    error::{Error, Result},
};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, EnumIter)]
#[repr(u8)]
pub enum Suit {
    Spades = 0,
    Clubs = 1,
    Hearts = 2,
    Diamonds = 3,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, EnumIter)]
#[repr(u8)]
pub enum Rank {
    Deuce = 0,
    Three = 1,
    Four = 2,
    Five = 3,
    Six = 4,
    Seven = 5,
    Eight = 6,
    Nine = 7,
    Ten = 8,
    Jack = 9,
    Queen = 10,
    King = 11,
    Ace = 12,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
    // TODO(pixlark): Card modifiers
}

impl Card {
    /// Convert a shorthand identifier into a card. Panics if the identifier
    /// is incorrect. This exists only for test-writing.
    ///
    /// ```
    /// # use solver_core::prelude::{Card, Suit, Rank};
    /// let a = Card::from_ident("KH");
    /// let b = Card {
    ///     rank: Rank::King,
    ///     suit: Suit::Hearts,
    /// };
    /// assert_eq!(a, b);
    /// ```
    pub fn from_ident(ident: &str) -> Self {
        assert!(ident.chars().count() == 2);

        let rank = ident.chars().nth(0).unwrap();
        let rank = match rank.to_ascii_uppercase() {
            '2' => Rank::Deuce,
            '3' => Rank::Three,
            '4' => Rank::Four,
            '5' => Rank::Five,
            '6' => Rank::Six,
            '7' => Rank::Seven,
            '8' => Rank::Eight,
            '9' => Rank::Nine,
            'T' => Rank::Ten,
            'J' => Rank::Jack,
            'Q' => Rank::Queen,
            'K' => Rank::King,
            'A' => Rank::Ace,
            _ => panic!(),
        };

        let suit = ident.chars().nth(1).unwrap();
        let suit = match suit.to_ascii_uppercase() {
            'S' => Suit::Spades,
            'C' => Suit::Clubs,
            'H' => Suit::Hearts,
            'D' => Suit::Diamonds,
            _ => panic!(),
        };

        Self { rank, suit }
    }
}

#[macro_export]
macro_rules! card {
    ($ident:literal) => {
        $crate::solver::cards::Card::from_ident($ident)
    };
}

pub trait CardView {
    fn view(&self) -> &[Card];
}

#[derive(Clone, Debug)]
pub struct Deck {
    cards: Vec<Card>,
}

lazy_static! {
    static ref BASE_DECK_CARDS: Vec<Card> = {
        let mut cards = Vec::with_capacity(52);
        for suit in Suit::iter() {
            for rank in Rank::iter() {
                cards.push(Card { rank, suit });
            }
        }
        cards
    };
}

impl Deck {
    pub fn base_deck() -> Self {
        Self {
            cards: BASE_DECK_CARDS.clone(),
        }
    }

    pub fn shuffle(&mut self, rng: &mut impl Rng) {
        self.cards.shuffle(rng);
    }

    pub fn shuffled(rng: &mut impl Rng) -> Self {
        let mut deck = Self::base_deck();
        deck.shuffle(rng);
        deck
    }

    pub fn peek_top_card(&self) -> Option<Card> {
        self.cards.last().copied()
    }

    pub fn draw(&mut self) -> Option<Card> {
        if self.count() == 0 {
            None
        } else {
            Some(self.cards.remove(self.count() - 1))
        }
    }

    pub fn draw_hand(&mut self) -> Option<Hand> {
        if self.count() < 5 {
            None
        } else {
            let mut hand = Hand::empty();
            for _ in 0..5 {
                hand.cards.push(self.draw().unwrap()).unwrap();
            }
            Some(hand)
        }
    }

    pub fn draw_n(&mut self, n: usize) -> Option<CardCollection> {
        if self.count() < n {
            None
        } else {
            let mut cards = Vec::with_capacity(n);
            for _ in 0..n {
                cards.push(self.draw().unwrap());
            }
            Some(CardCollection { cards })
        }
    }

    pub fn count(&self) -> usize {
        self.cards.len()
    }
}

impl CardView for Deck {
    fn view(&self) -> &[Card] {
        &self.cards
    }
}

#[derive(Clone, Debug)]
pub struct Hand {
    pub(crate) cards: heapless::Vec<Card, 5>,
}

impl Hand {
    pub fn empty() -> Self {
        Self {
            cards: heapless::Vec::new(),
        }
    }
    pub fn from_slice(cards: &[Card]) -> Result<Self> {
        Ok(Self {
            cards: heapless::Vec::from_slice(cards).map_err(|()| Error::OverfullHand)?,
        })
    }
    /// Convert a series of shorthand identifiers into a `Hand`.
    /// Panics if the input is incorrect. This exists only for test-writing.
    pub fn from_idents(idents: &str) -> Self {
        Self::from_slice(CardCollection::from_idents(idents).view()).unwrap()
    }
}

#[macro_export]
macro_rules! hand {
    ($ident:literal) => {
        $crate::solver::cards::Hand::from_idents($ident)
    };
}

impl PartialEq for Hand {
    fn eq(&self, other: &Self) -> bool {
        self.cards == other.cards
        // let mut a = self.cards.clone();
        // a.sort();
        // let mut b = other.cards.clone();
        // b.sort();
        // a == b
    }
}

impl From<heapless::Vec<Card, 5>> for Hand {
    fn from(value: heapless::Vec<Card, 5>) -> Self {
        Self { cards: value }
    }
}

impl std::iter::FromIterator<Card> for Hand {
    fn from_iter<T: IntoIterator<Item = Card>>(iter: T) -> Self {
        Self {
            cards: iter.into_iter().take(5).collect::<heapless::Vec<_, 5>>(),
        }
    }
}

impl CardView for Hand {
    fn view(&self) -> &[Card] {
        &self.cards
    }
}

pub struct CardCollection {
    cards: Vec<Card>,
}

impl CardCollection {
    pub fn empty() -> Self {
        Self { cards: Vec::new() }
    }
    /// Convert a series of shorthand identifiers into a `CardCollection`.
    /// Panics if the input is incorrect. This exists only for test-writing.
    ///
    /// ```
    /// # use solver_core::prelude::{Suit, Rank, Card, CardCollection};
    /// let cards = CardCollection::from_idents("KH TD JS 2C");
    /// assert_eq!(cards.nth(2), Some(Card {
    ///     rank: Rank::Jack,
    ///     suit: Suit::Spades,
    /// }));
    /// ```
    pub fn from_idents(idents: &str) -> Self {
        let idents = idents.split_ascii_whitespace();
        Self::from(idents.map(Card::from_ident).collect::<Vec<_>>().as_slice())
    }
    pub fn nth(&self, n: usize) -> Option<Card> {
        if n < self.cards.len() {
            Some(self.cards[n])
        } else {
            None
        }
    }
}

#[macro_export]
macro_rules! cards {
    ($ident:literal) => {
        $crate::solver::cards::CardCollection::from_idents($ident)
    };
}

impl From<&[Card]> for CardCollection {
    fn from(value: &[Card]) -> Self {
        Self {
            cards: Vec::from(value),
        }
    }
}

impl CardView for CardCollection {
    fn view(&self) -> &[Card] {
        &self.cards
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, EnumIter)]
#[repr(u8)]
pub enum HandKind {
    HighCard = 0,
    Pair = 1,
    TwoPair = 2,
    ThreeOfAKind = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    FourOfAKind = 7,
    StraightFlush = 8,
    FiveOfAKind = 9,
    FlushHouse = 10,
    FlushFive = 11,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn base_deck_test() {
        let mut seen = HashSet::new();
        let deck = Deck::base_deck();
        for card in deck.cards {
            seen.insert(card);
        }
        assert_eq!(seen.len(), 52);
    }

    #[test]
    fn base_shuffle_test() {
        let mut seen = HashSet::new();
        let mut rng = SmallRng::from_entropy();
        let deck = Deck::shuffled(&mut rng);
        for card in deck.cards {
            seen.insert(card);
        }
        assert_eq!(seen.len(), 52);
    }

    #[test]
    fn deck_draw_test() {
        let mut seen = HashSet::new();
        let mut rng = SmallRng::from_entropy();
        let mut deck = Deck::shuffled(&mut rng);
        for _ in 0..52 {
            seen.insert(deck.draw());
        }
        assert_eq!(seen.len(), 52);
        assert_eq!(deck.count(), 0);
        assert_eq!(deck.peek_top_card(), None);
        assert_eq!(deck.draw(), None);
    }
}
