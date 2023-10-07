use quickcheck::{Arbitrary as _, Gen, TestResult, Testable};

use example::{Entry, Example};
fsmentry::dsl! {
    /// This is an example state machine.
    ///
    /// This documentation and derives will be on the top-level Example and ExampleState structs
    #[derive(Clone, derive_quickcheck_arbitrary::Arbitrary, Debug)]
    pub Example {
        /// An isolated vertex with associated data
        PopulatedIsland: String;
        /// An isolated vertex with no data
        DesertIsland;
        /// A vertex with nonzero indegree and outdegree, with associated data
        BeautifulBridge: Vec<u8>;
        /// A vertex with nonzero indegree and outdegree, with no data
        Plank;
        /// A sink with data
        Tombstone: char;
        /// A sink with no data
        UnmarkedGrave;
        /// A source with data
        Fountain: std::net::IpAddr;
        /// A source with no data
        Stream;

        /// An edge
        BeautifulBridge -> UnmarkedGrave;

        Plank -"inline documentation"-> Tombstone;

        /// This documentation is shared among all transitions
        Fountain -> BeautifulBridge -> Tombstone;

        /// This is also shared among all transitions
        Fountain -"with inline too"-> Plank -> UnmarkedGrave;

        Stream --> BeautifulBridge;
        Stream --"different arrow lengths are ok"-> Plank;
    }
}

fn main() {
    quickcheck::quickcheck(RandomWalk)
}

struct RandomWalk;

impl Testable for RandomWalk {
    fn result(&self, g: &mut Gen) -> TestResult {
        let mut machine = Example::arbitrary(g);
        println!("initial state: {:?}", machine);
        loop {
            match machine.entry() {
                Entry::DesertIsland => break,
                Entry::Plank(it) => match CoinFlip::arbitrary(g) {
                    Heads => it.tombstone('p'),
                    Tails => it.unmarked_grave(),
                },
                Entry::UnmarkedGrave => break,
                Entry::BeautifulBridge(it) => {
                    let _current_data: &Vec<u8> = it.get();
                    let _old_data: Vec<u8> = match CoinFlip::arbitrary(g) {
                        Heads => it.tombstone('b'),
                        Tails => it.unmarked_grave(),
                    };
                }
                Entry::PopulatedIsland(data) => {
                    let _: String = data;
                    break;
                }
                Entry::Fountain(it) => {
                    let _current_data: &std::net::IpAddr = it.get();
                    let _old_data: std::net::IpAddr = match CoinFlip::arbitrary(g) {
                        Heads => it.beautiful_bridge(Vec::from_iter(*b"from fountain")),
                        Tails => it.plank(),
                    };
                }
                Entry::Tombstone(data) => {
                    let _: char = data;
                    break;
                }
                Entry::Stream(it) => match CoinFlip::arbitrary(g) {
                    Heads => it.beautiful_bridge(Vec::from_iter(*b"from stream")),
                    Tails => it.plank(),
                },
            }
            println!("\tnew state: {:?}", machine);
        }
        println!("final state: {:?}", machine);
        TestResult::passed()
    }
}

use CoinFlip::{Heads, Tails};

#[derive(derive_quickcheck_arbitrary::Arbitrary, Clone)]
enum CoinFlip {
    Heads,
    Tails,
}
