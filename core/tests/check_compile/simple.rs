#![cfg_attr(rustfmt, rustfmt_skip)]
enum Road {
    End,
    Fork,
    Start,
}
/// Progress through variants of [`Road`], created by its [`entry`](Road::entry) method.
enum RoadEntry<'state> {
    /// Represents [`Road::End`]
    ///
    /// This state is reachable from the following:
    /// - [`Fork`](Road::Fork) via [`end`](Fork::end)
    End,
    /// Represents [`Road::Fork`]
    ///
    /// This state is reachable from the following:
    /// - [`Start`](Road::Start) via [`fork`](Start::fork)
    ///
    /// This state can transition to the following:
    /// - [`End`](Road::End) via [`end`](Fork::end)
    /// - [`Start`](Road::Start) via [`start`](Fork::start)
    Fork(Fork<'state>),
    /// Represents [`Road::Start`]
    ///
    /// This state is reachable from the following:
    /// - [`Fork`](Road::Fork) via [`start`](Fork::start)
    ///
    /// This state can transition to the following:
    /// - [`Fork`](Road::Fork) via [`fork`](Start::fork)
    Start(Start<'state>),
}
impl<'state> ::core::convert::From<&'state mut Road> for RoadEntry<'state> {
    fn from(value: &'state mut Road) -> Self {
        match value {
            Road::End => RoadEntry::End,
            Road::Fork => RoadEntry::Fork(Fork(value)),
            Road::Start => RoadEntry::Start(Start(value)),
        }
    }
}
impl Road {
    #[allow(clippy::needless_lifetimes)]
    fn entry<'state>(&'state mut self) -> RoadEntry<'state> {
        self.into()
    }
}
/// See [`RoadEntry::Fork`]
struct Fork<'state>(
    /// MUST match [`RoadEntry::Fork`]
    &'state mut Road,
);
/// See [`RoadEntry::Start`]
struct Start<'state>(
    /// MUST match [`RoadEntry::Start`]
    &'state mut Road,
);
#[allow(clippy::needless_lifetimes)]
impl<'state> Fork<'state> {
    /// Transition to [`Road::End`]
    pub fn end(self) {
        match ::core::mem::replace(self.0, Road::End) {
            Road::Fork => {}
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state> Fork<'state> {
    /// Transition to [`Road::Start`]
    pub fn start(self) {
        match ::core::mem::replace(self.0, Road::Start) {
            Road::Fork => {}
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state> Start<'state> {
    /// Transition to [`Road::Fork`]
    pub fn fork(self) {
        match ::core::mem::replace(self.0, Road::Fork) {
            Road::Start => {}
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
