use std::cell::RefCell;
use std::collections::HashMap;

use anyhow::Result;
use clap::Subcommand;
use itertools::Itertools;
use rand::prelude::*;
use rayon::prelude::*;
use strum::IntoEnumIterator;

use solver_core::prelude::{
    CardCollection, CardView, Deck, Hand, HandEvaluator, HandKind, Options, Scorer,
};

#[derive(Debug, Subcommand)]
pub enum CliCommands {
    /// Generate statistics for the 12 different types of Balatro hands
    HandStats {
        /// Run on a single thread (for profiling)
        #[arg(long = "single-threaded", default_value = "false")]
        single_threaded: bool,

        /// Perform this many iterations, in tens of thousands
        #[arg(short = 'i', long = "iterations", default_value = "100")]
        iterations: usize,

        /// Whether the "Shortcut" joker is enabled, allowing straights to have a gap
        #[arg(long = "shortcut", default_value = "false")]
        shortcut: bool,

        /// Whether the "Four Fingers" joker is enabled, allowing straights/flushes to consist of 4 cards
        #[arg(long = "four-fingers", default_value = "false")]
        four_fingers: bool,
    },
}

struct HandStats {
    frequency: f32,
    average_score: f32,
}

#[allow(clippy::cast_precision_loss)]
fn generate_hand_stats<G>(
    single_threaded: bool,
    iterations: usize,
    generate_hand: G,
) -> HashMap<HandKind, HandStats>
where
    G: Fn() -> (HandKind, Hand) + std::marker::Sync,
{
    let hand_map: HashMap<HandKind, (usize, f32)> = if single_threaded {
        (0..iterations)
            .map(|_| generate_hand())
            .fold(HashMap::new(), |mut map, (kind, hand)| {
                let entry = map.entry(kind).or_insert((0, 0.0));
                entry.0 += 1;
                entry.1 += Scorer::score_hand(kind, &hand);
                map
            })
    } else {
        (0..iterations)
            .into_par_iter()
            .map(|_| generate_hand())
            .fold(HashMap::new, |mut map, (kind, hand)| {
                let entry = map.entry(kind).or_insert((0, 0.0));
                entry.0 += 1;
                entry.1 += Scorer::score_hand(kind, &hand);
                map
            })
            .reduce(HashMap::new, |mut left, right| {
                for (hand, (count, score)) in right {
                    let entry = left.entry(hand).or_insert((0, 0.0));
                    entry.0 += count;
                    entry.1 += score;
                }

                left
            })
    };

    let total = hand_map.values().map(|(count, _)| count).sum::<usize>() as f32;

    let frequencies: HashMap<_, _> = hand_map
        .into_iter()
        .map(|(hand, (count, score))| {
            (
                hand,
                HandStats {
                    frequency: (count as f32) / total,
                    average_score: score / (count as f32),
                },
            )
        })
        .collect();

    frequencies
}

fn print_card_stats(stats: HashMap<HandKind, HandStats>) {
    let hand_name_columns = HandKind::iter()
        .map(|h| format!("{h:?}").len())
        .max()
        .unwrap();
    for (
        hand,
        HandStats {
            frequency,
            average_score,
        },
    ) in stats.into_iter().sorted_by_key(|(hand, _)| *hand)
    {
        println!(
            " - {:hand_width$} {:>6.3}% (avg: {average_score:>6.1}, ev: {:>6.1})",
            format!("{:?}", hand),
            frequency * 100.0,
            average_score * frequency,
            hand_width = hand_name_columns
        );
    }
}

fn fresh_draw_stats(single_threaded: bool, iterations: usize, options: Options) {
    thread_local! {
        static RNG: RefCell<SmallRng> = RefCell::new(rand::rngs::SmallRng::from_entropy());
    }

    let generate_hand = || {
        let mut deck = RNG.with_borrow_mut(Deck::shuffled);
        let hand = deck.draw_hand().unwrap();

        HandEvaluator::evaluate_poker_hand(hand, options).unwrap()
    };

    let stats = generate_hand_stats(single_threaded, iterations, generate_hand);

    println!("When drawing 5 cards from a shuffled 52-card standard deck, the frequencies of each hand are:");
    print_card_stats(stats);
}

fn eight_card_draw_stats(single_threaded: bool, iterations: usize, options: Options) {
    thread_local! {
        static RNG: RefCell<SmallRng> = RefCell::new(rand::rngs::SmallRng::from_entropy());
    }

    let generate_hand = || {
        let mut deck = RNG.with_borrow_mut(Deck::shuffled);
        let cards = deck.draw_n(8).unwrap();

        let mut best_hand: Option<(HandKind, Hand)> = None;
        for hand in cards.view().iter().copied().combinations(5) {
            let (kind, hand) =
                HandEvaluator::evaluate_poker_hand(Hand::from_slice(&hand).unwrap(), options)
                    .unwrap();

            if best_hand.is_none() || kind > best_hand.as_ref().unwrap().0 {
                best_hand = Some((kind, hand));
            }
        }

        best_hand.unwrap()
    };

    let stats = generate_hand_stats(single_threaded, iterations, generate_hand);

    println!("When drawing 8 cards from a shuffled 52-card standard deck, the frequencies of each best hand are:");
    print_card_stats(stats);
}

#[allow(clippy::unnecessary_wraps)]
fn hand_stats(single_threaded: bool, iterations: usize, options: Options) -> Result<()> {
    fresh_draw_stats(single_threaded, iterations, options);
    eight_card_draw_stats(single_threaded, iterations, options);

    Ok(())
}

pub fn run(command: &CliCommands) -> Result<()> {
    match command {
        CliCommands::HandStats {
            single_threaded,
            iterations,
            shortcut,
            four_fingers,
        } => hand_stats(*single_threaded, *iterations * 10_000, {
            let mut options = Options::empty();
            if *shortcut {
                options |= Options::GappedStraights;
            }
            if *four_fingers {
                options |= Options::FourCardStraightsAndFlushes;
            }
            options
        }),
    }
}
