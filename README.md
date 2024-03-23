### balatro-solver

This is an early-stages idea I'm experimenting with for trying to weakly solve Balatro through
an extensive monte carlo simulator.

---

#### Short-term goals

 - [ ] v0.1: Get the core simulation/prediction engine working for ante 1, round 1, meaning nothing but the basic cards.

---

#### Large-scale thinking

Two pieces:
 - A Rust library that powers a fast monte carlo simulation of Balatro game logic
 - A Lua mod that displays the results of that simulation on the interface in the shop

Broad steps:
 1. In the shop, each card that you can buy is sent to the backend, along with the rest of
    your current build and the cost of the next blind.
 2. Then, some N rounds are simulated on that ante (where N is very large), and the average
    ending score is kept track of.
 3. That average ending score is divided by the size of the blind, and passed back to the
    frontend, which displays it color-coded beside the card in question.
 4. The same simulations are used to recommend moves during a round.

How does the simulation work?
 - Detailed scoring system including jokers, enhancements, cards-in-hand, etc.
 - Simple low-depth AI for making play/discard decisions.
   - NOTE: Actually, it will also have to make hand ordering decisions, which is interesting.
           I wonder if there's a good way to make a greedy heuristic for that, maybe by having a way
           to sort cards that cause Xmult to the end. This would require a system though for "marking"
           cards with Xmult that are given it by jokers -- and still, it might not be able to be completely
           greedy because you also have to account for those Xmult cards having varying chip and flat mult amounts.
 - To simulate a round, iterate:
   1. Have we surpassed blind score? If yes, exit with total score.
   2. Score all possible hands from current cards.
   3. Use a simple heuristic to judge which hands are possible to draw for.
   4. Score all those possible draw hands, and multiply by their likelihood of being drawn
      to get an expected value.
   5. Compare the immediate hand values to the possible expected values, and choose the
      action with the highest value.

Systems that affect score:
 - Hand levels (planet cards, space joker, etc)
 - Boss blinds
 - Card scoring
   - Card-scoring jokers (scholar, ...)

---

#### Implementation details/thinking

Likely want to have some `Clone`able `World` struct that holds all game state -- hand levels, jokers, current deck shuffle order, etc.

How to represent a deck? Considering the extremely tight loops that deck shuffling/drawing/etc will be within, it's worth considering something
other than, or at least in addition to, the obvious `Vec<Card>` implementation.

```Rust
// Maybe something like this?

#[derive(Copy, Clone, Debug, Hash)]
struct DeckEntry {
    card: Card,
    drawn: bool
};

struct Deck {
    cards: HashSet<DeckEntry>,
    ordering: Vec<usize>,
};
```

This helps bring down complexity, although it means that we need to have *two* dynamically allocated data structures for every deck.

Also want to be wary of premature optimization, who knows -- deck manipulation might not end up being even close to a performance bottleneck.
