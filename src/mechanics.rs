use smallvec::SmallVec;

use rand::seq::SliceRandom;
use rand::Rng;

type CardVal = u8;

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct Card(pub CardVal);

impl Drop for Card {
    fn drop(&mut self) {
        unreachable!()
    }
}

pub type Cards = SmallVec<[Card; 7]>;

#[derive(Copy, Clone, Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct Place {
    pub control: Option<PlayerId>,
    pub scores: [u8; 2],
    pub face_value: CardVal,
}

impl Place {
    fn new(val: CardVal) -> Place {
        Place {
            control: None,
            scores: [0, 0],
            face_value: val,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Player {
    // Cards in their hand
    #[serde(skip)]
    pub hand: Cards,
    // Card they secreted
    pub secret: Option<Card>,
    // Cards they discarded
    pub discard: Option<[Card; 2]>,
    // Card they gifted and the cards they kept.
    pub gift: Option<(Card, [Card; 2])>,
    // Cards they gave in competition. Second is the ones they kept.
    pub competition: Option<([Card; 2], [Card; 2])>,
    // Their id (FIXME unused: in case we need a reverse lookup?)
    pub id: PlayerId,
}

impl Player {
    pub fn new(id: PlayerId) -> Player {
        Player {
            hand: smallvec![],
            secret: None,
            discard: None,
            gift: None,
            competition: None,
            id: id,
        }
    }
}

#[derive(Serialize, Deserialize)]
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
    pub rng: rand_pcg::Pcg64,
    pub current_player: PlayerId,
}

#[derive(PartialEq, Eq)]
pub enum Action {
    Secret {
        card: CardVal,
    },
    Discard {
        cards: [CardVal; 2],
    },
    Gift {
        for_player: [CardVal; 2],
        for_other: CardVal,
    },
    Competition {
        for_player: [CardVal; 2],
        for_other: [CardVal; 2],
    },
}

impl GameState {
    pub fn clone(&self) -> GameState {
        let mut buf = [0u8; 512];
        let mut cursor = std::io::Cursor::new(&mut buf[..]);
        serde_cbor::to_writer(&mut cursor, self);
        let len = cursor.position() as usize;
        let almost_new: GameState = serde_cbor::from_reader(std::io::Cursor::new(&buf[..len]))
            .expect("can't roundtrip? really?");
        almost_new
    }

    // A new game state, already reset and ready to play.
    pub fn new(mut rng: rand_pcg::Pcg64) -> GameState {
        let mut deck = vec![];
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
            current_player: PlayerId::of_index(rng.gen_range(0, 2)),
            rng: rng,
        };
        for (place_idx, place) in gs.places.iter().enumerate() {
            for _ in 0..place.face_value {
                deck.push(Card(place_idx as u8))
            }
        }
        gs.deck = deck;
        gs.deck.shuffle(&mut gs.rng);
        gs.discarded = gs.deck.pop(); // Discard one
        gs
    }

    // Rebuild the deck, reset the place scores, discard one, deal 6 cards to each player, and set all actions as available
    // Panics if the game isn't in a final state.
    fn add_two(&mut self, [a, b]: [Card; 2]) {
        self.deck.push(a);
        self.deck.push(b);
    }

    fn drain_player(&mut self, pid: PlayerId) {
        self.deck
            .push(self.players[pid.index()].secret.take().unwrap());
        let discarded = self.players[pid.index()].discard.take().unwrap();
        self.add_two(discarded);
        let (their_card, our_cards) = self.players[pid.index()].gift.take().unwrap();
        self.deck.push(their_card);
        self.add_two(our_cards);
        let (their_cards, our_cards) = self.players[pid.index()].competition.take().unwrap();
        self.add_two(their_cards);
        self.add_two(our_cards);
    }

    fn reset(&mut self, for_new: bool) {
        assert!(self.deck.len() == 0);
        self.deck.push(self.discarded.take().unwrap());
        self.drain_player(self.current_player);
        self.drain_player(self.current_player.other());
        for place in &mut self.places {
            place.scores = [0, 0];
        }
        self.deck.shuffle(&mut self.rng);
        self.discarded = self.deck.pop(); // Discard one
        for _ in 0..6 {
            self.players[0].hand.push(self.deck.pop().unwrap());
        }
        for _ in 0..6 {
            self.players[1].hand.push(self.deck.pop().unwrap());
        }
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
                            score[pid.index()] += place.face_value;
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

    fn place_card(&mut self, pid: PlayerId, card: CardVal) {
        self.places[card as usize].scores[pid.index()] += 1;
    }

    fn us(&mut self) -> &mut Player {
        &mut self.players[self.current_player.index()]
    }
    fn them(&mut self) -> &mut Player {
        &mut self.players[self.current_player.other().index()]
    }

    pub fn apply_action(&mut self, action: Action) {
        let us_id = self.current_player;
        let them_id = self.current_player.other();
        let remove = |hand: &mut Cards, cv: CardVal| {
            let pos = hand
                .iter()
                .position(|x| x.0 == cv)
                .expect("action mentioned card not in hand");
            hand.remove(pos)
        };
        match action {
            Action::Secret { card } => {
                let card = remove(&mut self.us().hand, card);
                self.us().secret = Some(card);
            }
            Action::Discard { cards } => {
                let cards = [
                    remove(&mut self.us().hand, cards[0]),
                    remove(&mut self.us().hand, cards[1]),
                ];
                self.us().discard = Some(cards);
            }
            Action::Gift {
                for_player,
                for_other,
            } => {
                let for_player = [
                    remove(&mut self.us().hand, for_player[0]),
                    remove(&mut self.us().hand, for_player[1]),
                ];
                let for_other = remove(&mut self.us().hand, for_other);
                self.place_card(us_id, for_player[0].0);
                self.place_card(us_id, for_player[1].0);
                self.place_card(them_id, for_other.0);
                self.us().gift = Some((for_other, for_player));
            }
            Action::Competition {
                for_player,
                for_other,
            } => {
                let for_player = [
                    remove(&mut self.us().hand, for_player[0]),
                    remove(&mut self.us().hand, for_player[1]),
                ];
                let for_other = [
                    remove(&mut self.them().hand, for_other[0]),
                    remove(&mut self.them().hand, for_other[1]),
                ];
                self.place_card(us_id, for_player[0].0);
                self.place_card(us_id, for_player[1].0);
                self.place_card(them_id, for_other[0].0);
                self.place_card(them_id, for_other[1].0);
                self.us().competition = Some((for_other, for_player));
            }
        }
        self.current_player.swap();
    }
}
