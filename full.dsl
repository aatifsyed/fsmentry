/// This is an example state machine.
/// It exercises all vertex types, with and without data.
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
