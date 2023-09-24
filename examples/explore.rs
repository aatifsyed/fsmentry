use example::*;
use quickcheck::Arbitrary as _;

pub mod example {
    use derive_quickcheck_arbitrary::Arbitrary;

    pub struct StateMachine {
        state: State,
    }

    impl StateMachine {
        pub fn new(initial: State) -> Self {
            Self { state: initial }
        }
        pub fn state(&self) -> &State {
            &self.state
        }
        pub fn state_mut(&mut self) -> &mut State {
            &mut self.state
        }
        pub fn entry(&mut self) -> Entry {
            match &mut self.state {
                State::PopulatedIsland(_) => {
                    if let State::PopulatedIsland(data) = &mut self.state {
                        // reborrow to get the data
                        Entry::PopulatedIsland(data)
                    } else {
                        unreachable!("we've held an immutable reference to state, so state cannot have changed")
                    }
                }
                State::DesertIsland => Entry::DesertIsland,
                State::BeautifulBridge(_) => Entry::BeautifulBridge(BeautifulBridgeTransition {
                    inner: &mut self.state,
                }),
                State::Plank => Entry::Plank(PlankTransition {
                    inner: &mut self.state,
                }),
                State::Tombstone(_) => {
                    if let State::Tombstone(data) = &mut self.state {
                        // reborrow to get the data
                        Entry::Tombstone(data)
                    } else {
                        unreachable!("we've held an immutable reference to state, so state cannot have changed")
                    }
                }
                State::UnmarkedGrave => Entry::UnmarkedGrave,
                State::Fountain(_) => Entry::Fountain(FountainTransition {
                    inner: &mut self.state,
                }),
                State::Stream => Entry::Stream(StreamTransition {
                    inner: &mut self.state,
                }),
            }
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
        PopulatedIsland(&'a mut PopulatedIslandData),
        DesertIsland,
        BeautifulBridge(BeautifulBridgeTransition<'a>),
        Plank(PlankTransition<'a>),
        Tombstone(&'a mut TombstoneData),
        UnmarkedGrave,
        Fountain(FountainTransition<'a>),
        Stream(StreamTransition<'a>),
    }

    //////////////////////////////
    // Entries with transitions //
    //////////////////////////////

    pub struct BeautifulBridgeTransition<'a> {
        inner: &'a mut State,
    }

    pub struct PlankTransition<'a> {
        inner: &'a mut State,
    }

    pub struct FountainTransition<'a> {
        inner: &'a mut State,
    }

    pub struct StreamTransition<'a> {
        inner: &'a mut State,
    }

    ///////////////////////////////////////
    // Entries with transitions and data //
    ///////////////////////////////////////

    impl BeautifulBridgeTransition<'_> {
        pub fn get(&self) -> &BeautifulBridgeData {
            let State::BeautifulBridge(data) = &self.inner else {
                unreachable!()
            };
            data
        }
        pub fn get_mut(&mut self) -> &mut BeautifulBridgeData {
            let State::BeautifulBridge(data) = self.inner else {
                unreachable!()
            };
            data
        }
    }

    impl FountainTransition<'_> {
        pub fn get(&self) -> &FountainData {
            let State::Fountain(data) = &self.inner else {
                unreachable!()
            };
            data
        }
        pub fn get_mut(&mut self) -> &mut FountainData {
            let State::Fountain(data) = self.inner else {
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
            let prev = std::mem::replace(self.inner, State::Tombstone(next));
            let State::BeautifulBridge(prev) = prev else {
                unreachable!()
            };
            prev
        }
        // data -> no data
        pub fn unmarked_grave(self) -> BeautifulBridgeData {
            let prev = std::mem::replace(self.inner, State::UnmarkedGrave);
            let State::BeautifulBridge(prev) = prev else {
                unreachable!()
            };
            prev
        }
    }

    impl PlankTransition<'_> {
        // no data -> data
        pub fn tombstone(self, next: TombstoneData) {
            let prev = std::mem::replace(self.inner, State::Tombstone(next));
            assert!(matches!(prev, State::Plank));
        }
        // no data -> no data
        pub fn unmarked_grave(self) {
            let prev = std::mem::replace(self.inner, State::UnmarkedGrave);
            assert!(matches!(prev, State::Plank));
        }
    }

    // Included for completeness
    impl FountainTransition<'_> {
        pub fn beautiful_bridge(self, next: BeautifulBridgeData) -> FountainData {
            let prev = std::mem::replace(self.inner, State::BeautifulBridge(next));
            let State::Fountain(prev) = prev else {
                unreachable!()
            };
            prev
        }
        pub fn plank(self) -> FountainData {
            let prev = std::mem::replace(self.inner, State::Plank);
            let State::Fountain(prev) = prev else {
                unreachable!()
            };
            prev
        }
    }

    impl StreamTransition<'_> {
        pub fn beautiful_bridge(self, next: BeautifulBridgeData) {
            let prev = std::mem::replace(self.inner, State::BeautifulBridge(next));
            assert!(matches!(prev, State::Stream));
        }
        pub fn plank(self) {
            let prev = std::mem::replace(self.inner, State::Plank);
            assert!(matches!(prev, State::Stream));
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
                    let _data = tsn.get();
                    let _data = tsn.get_mut();
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
                    let _data = tsn.get();
                    let _data = tsn.get_mut();
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
