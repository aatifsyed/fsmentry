const _: &str = stringify! {
    #[derive(Debug)]
    #[fsmentry::fsmentry]
    enum State<'a, T>
    where
        T: MyTrait
    {
        // Isolated vertices
        PopulatedIsland(String),
        DesertIsland,

        Fountain(&'a mut T) -> BeautifulBridge(Vec<u8>) -> Tombstone(char),
        Fountain -> Plank -> UnmarkedGrave,

        Stream -> BeautifulBridge,
        Stream -> Plank,
    }
};

trait MyTrait {}

pub(crate) mod mymachine {
    use super::*;

    #[derive(Debug)]
    pub(super) enum State<'a, T>
    where
        T: MyTrait,
    {
        PopulatedIsland(String),
        DesertIsland,

        Fountain(&'a mut T),
        Stream,

        BeautifulBridge(Vec<u8>),
        Plank,

        Tombstone(char),
        UnmarkedGrave,
    }
    impl<'a, T> State<'a, T>
    where
        T: MyTrait,
    {
        pub fn entry<'state>(&'state mut self) -> Entry<'state, 'a, T> {
            match self {
                State::PopulatedIsland(it) => Entry::PopulatedIsland(it),
                State::DesertIsland => Entry::DesertIsland,
                State::Fountain(_) => Entry::Fountain(entry::Fountain(self)),
                State::Stream => Entry::Stream(entry::Stream(self)),
                State::BeautifulBridge(_) => Entry::BeautifulBridge(entry::BeautifulBridge(self)),
                State::Plank => Entry::Plank(entry::Plank(self)),
                State::Tombstone(it) => Entry::Tombstone(it),
                State::UnmarkedGrave => Entry::UnmarkedGrave,
            }
        }
    }
    /// Interactively advance the state of [`State`].
    ///
    /// Created by [`State::entry`].
    pub(super) enum Entry<'state, 'a, T>
    where
        T: MyTrait,
    {
        PopulatedIsland(&'state mut String),
        DesertIsland,
        Fountain(entry::Fountain<'state, 'a, T>),
        Stream(entry::Stream<'state, 'a, T>),
        BeautifulBridge(entry::BeautifulBridge<'state, 'a, T>),
        Plank(entry::Plank<'state, 'a, T>),
        Tombstone(&'state mut char),
        UnmarkedGrave,
    }
    pub(super) mod entry {
        use super::*;
        pub struct Fountain<'state, 'a, T>(pub(super) &'state mut super::State<'a, T>)
        where
            T: MyTrait;
        impl<'state, 'a, T> ::core::convert::AsRef<&'a mut T> for Fountain<'state, 'a, T>
        where
            T: MyTrait,
        {
            fn as_ref(&self) -> &&'a mut T {
                match &self.0 {
                    super::State::Fountain(it) => it,
                    _ if ::core::cfg!(debug_assertions) => ::core::unreachable!(),
                    _ => unsafe { ::core::hint::unreachable_unchecked() },
                }
            }
        }
        impl<'state, 'a, T> ::core::convert::AsMut<&'a mut T> for Fountain<'state, 'a, T>
        where
            T: MyTrait,
        {
            fn as_mut(&mut self) -> &mut &'a mut T {
                match &mut self.0 {
                    super::State::Fountain(it) => it,
                    _ if ::core::cfg!(debug_assertions) => ::core::unreachable!(),
                    _ => unsafe { ::core::hint::unreachable_unchecked() },
                }
            }
        }
        impl<'state, 'a, T> Fountain<'state, 'a, T>
        where
            T: MyTrait,
        {
            pub fn to_beautiful_bridge(self, next: Vec<u8>) -> &'a mut T {
                match ::core::mem::replace(self.0, super::State::BeautifulBridge(next)) {
                    super::State::Fountain(prev) => prev,
                    _ if ::core::cfg!(debug_assertions) => ::core::unreachable!(),
                    _ => unsafe { ::core::hint::unreachable_unchecked() },
                }
            }
            pub fn to_plank(self) -> &'a mut T {
                match ::core::mem::replace(self.0, super::State::Plank) {
                    super::State::Fountain(prev) => prev,
                    _ if ::core::cfg!(debug_assertions) => ::core::unreachable!(),
                    _ => unsafe { ::core::hint::unreachable_unchecked() },
                }
            }
        }

        pub struct Stream<'state, 'a, T>(pub(super) &'state mut super::State<'a, T>)
        where
            T: MyTrait;
        pub struct BeautifulBridge<'state, 'a, T>(pub(super) &'state mut super::State<'a, T>)
        where
            T: MyTrait;
        pub struct Plank<'state, 'a, T>(pub(super) &'state mut super::State<'a, T>)
        where
            T: MyTrait;
    }
}
