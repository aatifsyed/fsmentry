#![cfg_attr(rustfmt, rustfmt_skip)]
/// This is a state machine that explores all vertex types
#[derive(Debug)]
pub enum State<'a, T>
where
    T: Ord,
{
    /// A non-terminal vertex with data
    BeautifulBridge(Vec<u8>),
    /// An isolated vertex without data.
    DesertIsland,
    /// A source vertex with data.
    Fountain(&'a mut T),
    Plank,
    /// An isolated vertex with data.
    PopulatedIsland(String),
    Stream,
    /// A sink vertex with data
    Tombstone(char),
    UnmarkedGrave,
}
/// Progress through variants of [`State`], created by its [`entry`](State::entry) method.
pub enum StateEntry<'state, 'a, T>
where
    T: Ord,
{
    /// Represents [`State::BeautifulBridge`]
    ///
    /// This state is reachable from the following:
    /// - [`Fountain`](State::Fountain) via [`fountain2bridge`](Fountain::fountain2bridge)
    /// - [`Stream`](State::Stream) via [`beautiful_bridge`](Stream::beautiful_bridge)
    ///
    /// This state can transition to the following:
    /// - [`Tombstone`](State::Tombstone) via [`bridge2tombstone`](BeautifulBridge::bridge2tombstone)
    BeautifulBridge(BeautifulBridge<'state, 'a, T>),
    /// Represents [`State::DesertIsland`]
    DesertIsland,
    /// Represents [`State::Fountain`]
    ///
    /// This state can transition to the following:
    /// - [`BeautifulBridge`](State::BeautifulBridge) via [`fountain2bridge`](Fountain::fountain2bridge)
    /// - [`Plank`](State::Plank) via [`plank`](Fountain::plank)
    Fountain(Fountain<'state, 'a, T>),
    /// Represents [`State::Plank`]
    ///
    /// This state is reachable from the following:
    /// - [`Fountain`](State::Fountain) via [`plank`](Fountain::plank)
    /// - [`Stream`](State::Stream) via [`plank`](Stream::plank)
    ///
    /// This state can transition to the following:
    /// - [`UnmarkedGrave`](State::UnmarkedGrave) via [`unmarked_grave`](Plank::unmarked_grave)
    Plank(Plank<'state, 'a, T>),
    /// Represents [`State::PopulatedIsland`]
    PopulatedIsland(&'state mut String),
    /// Represents [`State::Stream`]
    ///
    /// This state can transition to the following:
    /// - [`BeautifulBridge`](State::BeautifulBridge) via [`beautiful_bridge`](Stream::beautiful_bridge)
    /// - [`Plank`](State::Plank) via [`plank`](Stream::plank)
    Stream(Stream<'state, 'a, T>),
    /// Represents [`State::Tombstone`]
    ///
    /// This state is reachable from the following:
    /// - [`BeautifulBridge`](State::BeautifulBridge) via [`bridge2tombstone`](BeautifulBridge::bridge2tombstone)
    Tombstone(&'state mut char),
    /// Represents [`State::UnmarkedGrave`]
    ///
    /// This state is reachable from the following:
    /// - [`Plank`](State::Plank) via [`unmarked_grave`](Plank::unmarked_grave)
    UnmarkedGrave,
}
impl<'state, 'a, T> ::core::convert::From<&'state mut State<'a, T>>
for StateEntry<'state, 'a, T>
where
    T: Ord,
{
    fn from(value: &'state mut State<'a, T>) -> Self {
        match value {
            State::BeautifulBridge(_) => {
                StateEntry::BeautifulBridge(BeautifulBridge(value))
            }
            State::DesertIsland => StateEntry::DesertIsland,
            State::Fountain(_) => StateEntry::Fountain(Fountain(value)),
            State::Plank => StateEntry::Plank(Plank(value)),
            State::PopulatedIsland(it) => StateEntry::PopulatedIsland(it),
            State::Stream => StateEntry::Stream(Stream(value)),
            State::Tombstone(it) => StateEntry::Tombstone(it),
            State::UnmarkedGrave => StateEntry::UnmarkedGrave,
        }
    }
}
impl<'a, T> State<'a, T>
where
    T: Ord,
{
    pub fn entry<'state>(&'state mut self) -> StateEntry<'state, 'a, T> {
        self.into()
    }
}
pub struct BeautifulBridge<'state, 'a, T>(
    &'state mut State<'a, T>,
)
where
    T: Ord;
pub struct Fountain<'state, 'a, T>(
    &'state mut State<'a, T>,
)
where
    T: Ord;
pub struct Plank<'state, 'a, T>(
    &'state mut State<'a, T>,
)
where
    T: Ord;
pub struct Stream<'state, 'a, T>(
    &'state mut State<'a, T>,
)
where
    T: Ord;
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> ::core::convert::AsRef<Vec<u8>> for BeautifulBridge<'state, 'a, T>
where
    T: Ord,
{
    fn as_ref(&self) -> &Vec<u8> {
        match &self.0 {
            State::BeautifulBridge(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> ::core::convert::AsMut<Vec<u8>> for BeautifulBridge<'state, 'a, T>
where
    T: Ord,
{
    fn as_mut(&mut self) -> &mut Vec<u8> {
        match &mut self.0 {
            State::BeautifulBridge(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> ::core::convert::AsRef<&'a mut T> for Fountain<'state, 'a, T>
where
    T: Ord,
{
    fn as_ref(&self) -> &&'a mut T {
        match &self.0 {
            State::Fountain(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> ::core::convert::AsMut<&'a mut T> for Fountain<'state, 'a, T>
where
    T: Ord,
{
    fn as_mut(&mut self) -> &mut &'a mut T {
        match &mut self.0 {
            State::Fountain(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> BeautifulBridge<'state, 'a, T>
where
    T: Ord,
{
    /// Transition to [`State::Tombstone`]
    pub fn bridge2tombstone(self, next: char) -> Vec<u8> {
        match ::core::mem::replace(self.0, State::Tombstone(next)) {
            State::BeautifulBridge(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> Fountain<'state, 'a, T>
where
    T: Ord,
{
    /// I've overridden transition method name
    ///
    /// Transition to [`State::BeautifulBridge`]
    pub fn fountain2bridge(self, next: Vec<u8>) -> &'a mut T {
        match ::core::mem::replace(self.0, State::BeautifulBridge(next)) {
            State::Fountain(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> Fountain<'state, 'a, T>
where
    T: Ord,
{
    /// Transition to [`State::Plank`]
    pub fn plank(self) -> &'a mut T {
        match ::core::mem::replace(self.0, State::Plank) {
            State::Fountain(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> Plank<'state, 'a, T>
where
    T: Ord,
{
    /// Transition to [`State::UnmarkedGrave`]
    pub fn unmarked_grave(self) {
        match ::core::mem::replace(self.0, State::UnmarkedGrave) {
            State::Plank => {}
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> Stream<'state, 'a, T>
where
    T: Ord,
{
    /// Transition to [`State::BeautifulBridge`]
    pub fn beautiful_bridge(self, next: Vec<u8>) {
        match ::core::mem::replace(self.0, State::BeautifulBridge(next)) {
            State::Stream => {}
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> Stream<'state, 'a, T>
where
    T: Ord,
{
    /// Transition to [`State::Plank`]
    pub fn plank(self) {
        match ::core::mem::replace(self.0, State::Plank) {
            State::Stream => {}
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
