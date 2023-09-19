use example::*;
use quickcheck::Arbitrary as _;

pub mod example {
    use derive_quickcheck_arbitrary::Arbitrary;

    pub struct StateMachine {
        /// Must always be [`Some`] when observable by a user
        inner: Option<State>,
    }

    impl StateMachine {
        pub fn new(initial: State) -> Self {
            Self {
                inner: Some(initial),
            }
        }
        pub fn state(&self) -> &State {
            self.inner.as_ref().unwrap()
        }
    }

    #[derive(Arbitrary, Clone)]
    pub enum State {
        /// An isolated vertex with data
        PopulatedIsland(PopulatedIslandData),
        /// An isolated vertex with no data
        DesertIsland,

        /// A vertex with nonzero indegree and outdegree, with data
        BeautifulBridge(BeautifulBridgeData),
        /// A vertex with nonzero indegree and outdegree, with no data
        Plank,

        /// A sink vertex with data
        Tombstone(TombstoneData),
        /// A sink vertex with no data
        UnmarkedGrave,

        /// A source vertex with data
        Fountain(FountainData),
        /// A source vertex with no data,
        Stream,
    }

    #[derive(Arbitrary, Clone)]
    pub struct PopulatedIslandData {}
    #[derive(Arbitrary, Clone)]
    pub struct BeautifulBridgeData {}
    #[derive(Arbitrary, Clone)]
    pub struct TombstoneData {}
    #[derive(Arbitrary, Clone)]
    pub struct FountainData {}

    pub enum Entry<'a> {
        PopulatedIsland(&'a PopulatedIslandData),
        DesertIsland,
        BeautifulBridge(BeautifulBridgeTransition<'a>),
        Plank(PlankTransition<'a>),
        Tombstone(&'a TombstoneData),
        UnmarkedGrave,
        Fountain(FountainTransition<'a>),
        Stream(StreamTransition<'a>),
    }

    //////////////////////////////
    // Entries with transitions //
    //////////////////////////////

    pub struct BeautifulBridgeTransition<'a> {
        inner: &'a mut Option<State>,
    }

    pub struct PlankTransition<'a> {
        inner: &'a mut Option<State>,
    }

    pub struct FountainTransition<'a> {
        inner: &'a mut Option<State>,
    }

    pub struct StreamTransition<'a> {
        inner: &'a mut Option<State>,
    }

    ///////////////////////////////////////
    // Entries with transitions and data //
    ///////////////////////////////////////

    impl BeautifulBridgeTransition<'_> {
        pub fn data(&mut self) -> &BeautifulBridgeData {
            let Some(State::BeautifulBridge(data)) = self.inner else {
                unreachable!()
            };
            data
        }
    }

    impl FountainTransition<'_> {
        pub fn data(&mut self) -> &FountainData {
            let Some(State::Fountain(data)) = self.inner else {
                unreachable!()
            };
            data
        }
    }

    /////////////////
    // Transitions //
    /////////////////

    impl BeautifulBridgeTransition<'_> {
        // data -> data
        pub fn tombstone(self, next: TombstoneData) -> BeautifulBridgeData {
            let Some(State::BeautifulBridge(prev)) = self.inner.take() else {
                unreachable!()
            };
            *self.inner = Some(State::Tombstone(next));
            prev
        }
        // data -> no data
        pub fn unmarked_grave(self) -> BeautifulBridgeData {
            let Some(State::BeautifulBridge(prev)) = self.inner.take() else {
                unreachable!()
            };
            *self.inner = Some(State::UnmarkedGrave);
            prev
        }
    }

    impl PlankTransition<'_> {
        // no data -> data
        pub fn tombstone(self, next: TombstoneData) {
            assert!(matches!(self.inner, Some(State::Plank)));
            *self.inner = Some(State::Tombstone(next));
        }
        // no data -> no data
        pub fn unmarked_grave(self) {
            assert!(matches!(self.inner, Some(State::Plank)));
            *self.inner = Some(State::UnmarkedGrave);
        }
    }

    // Included for completeness
    impl FountainTransition<'_> {
        pub fn beautiful_bridge(self, next: BeautifulBridgeData) -> FountainData {
            let Some(State::Fountain(prev)) = self.inner.take() else {
                unreachable!()
            };
            *self.inner = Some(State::BeautifulBridge(next));
            prev
        }
        pub fn plank(self) -> FountainData {
            let Some(State::Fountain(prev)) = self.inner.take() else {
                unreachable!()
            };
            *self.inner = Some(State::Plank);
            prev
        }
    }

    impl StreamTransition<'_> {
        pub fn beautiful_bridge(self, next: BeautifulBridgeData) {
            assert!(matches!(self.inner, Some(State::Stream)));
            *self.inner = Some(State::BeautifulBridge(next));
        }
        pub fn plank(self) {
            assert!(matches!(self.inner, Some(State::Stream)));
            *self.inner = Some(State::Plank);
        }
    }

    ///////////
    // Entry //
    ///////////
    impl StateMachine {
        pub fn entry(&mut self) -> Entry<'_> {
            // mut - must go first for borrow-checking
            if let State::BeautifulBridge(_) = self.inner.as_ref().unwrap() {
                return Entry::BeautifulBridge(BeautifulBridgeTransition {
                    inner: &mut self.inner,
                });
            }
            if let State::Fountain(_) = self.inner.as_ref().unwrap() {
                return Entry::Fountain(FountainTransition {
                    inner: &mut self.inner,
                });
            }
            if let State::Plank = self.inner.as_ref().unwrap() {
                return Entry::Plank(PlankTransition {
                    inner: &mut self.inner,
                });
            }
            if let State::Stream = self.inner.as_ref().unwrap() {
                return Entry::Stream(StreamTransition {
                    inner: &mut self.inner,
                });
            }

            // ref
            if let State::PopulatedIsland(data) = self.inner.as_ref().unwrap() {
                return Entry::PopulatedIsland(data);
            }
            if let State::Tombstone(data) = self.inner.as_ref().unwrap() {
                return Entry::Tombstone(data);
            }

            // empty
            if let State::DesertIsland = self.inner.as_ref().unwrap() {
                return Entry::DesertIsland;
            }
            if let State::UnmarkedGrave = self.inner.as_ref().unwrap() {
                return Entry::UnmarkedGrave;
            }

            unreachable!()
        }
    }
}

fn main() {
    loop {
        quickcheck::quickcheck(Test);
    }
}

struct Test;

impl quickcheck::Testable for Test {
    fn result(&self, g: &mut quickcheck::Gen) -> quickcheck::TestResult {
        let mut state_machine = StateMachine::new(State::arbitrary(g));
        println!("START {}", state_str(&state_machine));
        loop {
            match state_machine.entry() {
                Entry::PopulatedIsland(PopulatedIslandData {}) => break,
                Entry::DesertIsland => break,
                Entry::BeautifulBridge(mut tsn) => {
                    let _data = tsn.data();
                    match bool::arbitrary(g) {
                        true => tsn.tombstone(TombstoneData {}),
                        false => tsn.unmarked_grave(),
                    };
                }
                Entry::Plank(tsn) => {
                    match bool::arbitrary(g) {
                        true => tsn.tombstone(TombstoneData {}),
                        false => tsn.unmarked_grave(),
                    };
                }
                Entry::Tombstone(TombstoneData {}) => break,
                Entry::UnmarkedGrave => break,
                Entry::Fountain(mut tsn) => {
                    let _data = tsn.data();
                    match bool::arbitrary(g) {
                        true => tsn.beautiful_bridge(BeautifulBridgeData {}),
                        false => tsn.plank(),
                    };
                }
                Entry::Stream(tsn) => {
                    match bool::arbitrary(g) {
                        true => tsn.beautiful_bridge(BeautifulBridgeData {}),
                        false => tsn.plank(),
                    };
                }
            }
            println!("-> {}", state_str(&state_machine));
        }
        println!("END {}", state_str(&state_machine));
        quickcheck::TestResult::passed()
    }
}

fn state_str(machine: &StateMachine) -> &'static str {
    match machine.state() {
        State::PopulatedIsland(_) => "populated_island",
        State::DesertIsland => "desert_island",
        State::BeautifulBridge(_) => "beautiful_bridge",
        State::Plank => "plank",
        State::Tombstone(_) => "tombstone",
        State::UnmarkedGrave => "unmarked_grave",
        State::Fountain(_) => "fountain",
        State::Stream => "stream",
    }
}
