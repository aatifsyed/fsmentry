/// This machine exercises all vertex types, with and without data.
#[derive(Clone, Debug)]
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
    
    BeautifulBridge -> UnmarkedGrave;

    Plank -"plank transitions to tombstone"-> Tombstone;
    
    /// This documentation is shared from `Fountain` to `BeautifulBridge` to `Tombstone`
    Fountain -> BeautifulBridge -> Tombstone;

    /// This is also shared among a few transitions.
    Fountain -"just on fountain to plank"-> Plank -> UnmarkedGrave;
    
    Stream --> BeautifulBridge;
    Stream --"different arrow lengths are ok"-> Plank;
}
