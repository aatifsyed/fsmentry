fsmentry::dot! {
    digraph Example {
        PopulatedIsland;
        DesertIsland;
        BeautifulBridge;
        Plank;
        Tombstone;
        UnmarkedGrave;
        Fountain;
        Stream;

        BeautifulBridge -> UnmarkedGrave;

        Plank -> Tombstone;

        Fountain -> BeautifulBridge -> Tombstone;
        Fountain -> Plank -> UnmarkedGrave;

        Stream -> BeautifulBridge;
        Stream -> Plank;
    }
}

fn main() {}
