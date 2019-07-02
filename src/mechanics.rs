use smallvec::SmallVec;

use rand::seq::SliceRandom;
use rand::Rng;

type CardVal = u8;

/// Not clone: cards are a physical resource!
#[derive(PartialOrd, Ord, PartialEq, Eq)]
pub struct Card(pub CardVal);

// there's never more than 7 cards in one place.
pub type Cards = SmallVec<[Card; 7]>;

#[derive(Copy, Clone)]
pub enum PlayerId {
    First,
    Second,
}

impl PlayerId {
    fn index(self) -> usize {
        match self {
            PlayerId::First => 0,
            PlayerId::Second => 1,
        }
    }

    fn other(self) -> PlayerId {
        match self {
            PlayerId::First => PlayerId::Second,
            PlayerId::Second => PlayerId::First,
        }
    }

    fn swap(&mut self) {
        *self = self.other();
    }

    fn of_index(idx: usize) -> PlayerId {
        match idx {
            0 => PlayerId::First,
            1 => PlayerId::Second,
            _ => panic!("PlayerId::of_index({}): this is a two player game", idx),
        }
    }
}

pub struct Place {
    pub control: Option<PlayerId>,
    pub scores: [u8; 2],
    pub value: CardVal,
}

impl Place {
    fn new(val: CardVal) -> Place {
        Place {
            control: None,
            scores: [0, 0],
            value: val,
        }
    }
}

pub struct Player {
    // Actions this player has remaining
    pub remaining_actions: SmallVec<[Action; 4]>,
    // Cards in their hand
    pub hand: Cards,
    // Card they secreted for this round
    pub secret: Option<Card>,
    // Their id (FIXME unused: in case we need a reverse lookup?)
    pub id: PlayerId,
}

impl Player {
    pub fn new(id: PlayerId) -> Player {
        Player {
            remaining_actions: smallvec![],
            hand: smallvec![],
            secret: None,
            id: id,
        }
    }
}

pub struct GameState {
    /// Deck of resources
    pub deck: Vec<Card>,
    /// Player state
    pub players: [Player; 2],
    /// Places the players are fighting for control over
    pub places: [Place; 7],
    /// Card that was discarded at the start of the game
    pub discarded: Option<Card>,
    /// Randomness
    pub rng: Box<dyn rand::RngCore>,
}

#[derive(PartialEq, Eq)]
pub enum Action {
    Secret,
    Discard,
    Gift,
    Competition,
}

pub trait Behavior {
    fn remove_n_cards(&mut self, id: PlayerId, n: usize, hand: &mut Cards) -> Cards;
    fn request_action(&mut self, gs: &GameState, id: PlayerId) -> Action;
    fn assign(&mut self, id: PlayerId, cards: [Cards; 2]) -> [(Cards, PlayerId); 2];
}

static PLACE_VAL_COUNT: [(u8, u8); 4] = [(2, 3), (3, 2), (4, 1), (5, 1)];

impl GameState {
    // A new game state, already reset and ready to play.
    pub fn new(rng: Box<dyn rand::RngCore>) -> GameState {
        let mut gs = GameState {
            deck: vec![],
            places: [
                Place::new(2),
                Place::new(2),
                Place::new(2),
                Place::new(3),
                Place::new(3),
                Place::new(4),
                Place::new(5),
            ],
            players: [Player::new(PlayerId::First), Player::new(PlayerId::Second)],
            discarded: None,
            rng: rng,
        };
        gs.reset();
        gs
    }

    // Rebuild the deck, reset the place scores, discard one, deal 6 cards to each player, and set all actions as available
    fn reset(&mut self) {
        self.deck.clear();
        for (place_id, &(val, how_many)) in PLACE_VAL_COUNT.iter().enumerate() {
            for _ in 0..how_many {
                for _ in 0..(val-1) {
                    self.deck.push(Card(place_id as u8));
                }
                self.places[place_id].scores = [0, 0];
            }
        }
        self.deck.shuffle(&mut self.rng);
        self.discarded = self.deck.pop(); // Discard one
        for _ in 0..6 {
            self.players[0].hand.push(self.deck.pop().unwrap());
        }
        for _ in 0..6 {
            self.players[1].hand.push(self.deck.pop().unwrap());
        }
        self.players[0].remaining_actions = smallvec![
            Action::Secret,
            Action::Discard,
            Action::Gift,
            Action::Competition
        ];
        self.players[1].remaining_actions = smallvec![
            Action::Secret,
            Action::Discard,
            Action::Gift,
            Action::Competition
        ];
    }

    // Score the places, determining if there's a winner.
    fn update_control_and_score(&mut self) -> Option<PlayerId> {
        for place in &mut self.places {
            if place.scores[0] > place.scores[1] {
                place.control = Some(PlayerId::First);
            } else if place.scores[1] > place.scores[0] {
                place.control = Some(PlayerId::Second)
            }
        }

        let (score, places_controlled) =
            self.places
                .iter()
                .fold(([0, 0], [0, 0]), |(mut score, mut places), place| {
                    match place.control {
                        Some(pid) => {
                            score[pid.index()] += place.value;
                            places[pid.index()] += 1;
                        }
                        None => {}
                    }
                    (score, places)
                });

        // if anyone has >= 11 points, they win
        match score.iter().enumerate().max_by_key(|(_, &s)| s) {
            Some((player_index, &m)) if m >= 11 => return Some(PlayerId::of_index(player_index)),
            Some(_) | None => {}
        }

        // otherwise, if anyone has >= 4 places, they win.
        match places_controlled.iter().enumerate().max_by_key(|(_, &s)| s) {
            Some((player_index, &m)) if m >= 4 => return Some(PlayerId::of_index(player_index)),
            Some(_) | None => {}
        }

        // nobody won.
        None
    }

    fn place_cards(&mut self, pid: PlayerId, cards: &Cards) {
        for card in cards {
            self.places[card.0 as usize].scores[pid.index()] += 1;
        }
    }

    pub fn play<B: Behavior>(mut self, b: &mut B) -> PlayerId {
        let mut current_player = if self.rng.gen::<bool>() {
            PlayerId::First
        } else {
            PlayerId::Second
        };
        loop {
            let action = b.request_action(&self, current_player);
            match action {
                Action::Secret => {
                    assert!(self.players[current_player.index()].secret.is_none());
                    let card = b.remove_n_cards(
                        current_player,
                        1,
                        &mut self.players[current_player.index()].hand,
                    );
                    self.players[current_player.index()].secret =
                        Some(card.into_iter().next().unwrap());
                }
                Action::Discard => {
                    let _ = b.remove_n_cards(
                        current_player,
                        2,
                        &mut self.players[current_player.index()].hand,
                    );
                }
                Action::Gift => {
                    let mut cards = b.remove_n_cards(
                        current_player,
                        3,
                        &mut self.players[current_player.index()].hand,
                    );
                    let for_other = b.remove_n_cards(current_player.other(), 1, &mut cards);
                    self.place_cards(current_player, &cards);
                    self.place_cards(current_player.other(), &for_other);
                }
                Action::Competition => {
                    let first_set = b.remove_n_cards(
                        current_player,
                        2,
                        &mut self.players[current_player.index()].hand,
                    );
                    let second_set = b.remove_n_cards(
                        current_player,
                        2,
                        &mut self.players[current_player.index()].hand,
                    );
                    for (hand, player) in b
                        .assign(current_player.other(), [first_set, second_set])
                        .iter()
                    {
                        self.place_cards(*player, hand);
                    }
                }
            }
            let rem = &mut self.players[current_player.index()].remaining_actions;
            rem.swap_remove(
                rem.iter()
                    .position(|a| a == &action)
                    .expect("request_action gave unavailable action"),
            );
            current_player.swap();
            if self.players.iter().all(|p| p.remaining_actions.len() == 0) {
                match self.update_control_and_score() {
                    Some(player_id) => return player_id,
                    None => continue,
                }
            }
            self.reset();
        }
    }
}
