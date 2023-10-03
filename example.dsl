#[derive(Clone, derive_quickcheck_arbitrary::Arbitrary)]
pub Example {
    PopulatedIsland: String;
    DesertIsland;
    BeautifulBridge: Vec<u8>;
    Plank;
    Tombstone: char;
    UnmarkedGrave;
    Fountain: std::net::IpAddr;
    Stream;
    
    BeautifulBridge -> Tombstone;
    BeautifulBridge -> UnmarkedGrave;
    
    Plank -> Tombstone;
    Plank -> UnmarkedGrave;
    
    Fountain -> BeautifulBridge;
    Fountain -> Plank;
    
    Stream -> BeautifulBridge;
    Stream -> Plank;
}
