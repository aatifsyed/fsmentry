/// This explores all vertex types.
#[derive(Debug)]
pub enum State<'a, T>
where
    T: Ord,
{
    /// Isolated vertex, without data.
    IsolatedEmpty,
    /// Isolated vertex, with data.
    IsolatedWithData(String),
    NonTerminalEmpty,
    /// Non-terminal vertex, with data.
    NonTerminalWithData(Vec<u8>),
    SinkEmpty,
    /// Sink vertex, with data.
    SinkWithData(char),
    SourceEmpty,
    /// Source vertex, with data.
    SourceWithData(&'a mut T),
}
/// Progress through variants of [`State`], created by its [`entry`](State::entry) method.
///
/**<pre class="mermaid">
graph LR
  NonTerminalEmpty --> SinkEmpty;
  NonTerminalWithData --> SinkWithData;
  SourceEmpty --> NonTerminalEmpty;
  SourceEmpty --> NonTerminalWithData;
  SourceWithData --> NonTerminalEmpty;
  SourceWithData --> NonTerminalWithData;
  IsolatedEmpty;
  IsolatedWithData;

</pre>
<script type="module">
  import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
  var doc_theme = localStorage.getItem("rustdoc-theme");
  if (doc_theme === "dark" || doc_theme === "ayu") mermaid.initialize({theme: "dark"});
</script>*/
pub enum StateEntry<'state, 'a, T>
where
    T: Ord,
{
    /// Represents [`State::IsolatedEmpty`]
    IsolatedEmpty,
    /// Represents [`State::IsolatedWithData`]
    IsolatedWithData(&'state mut String),
    /// Represents [`State::NonTerminalEmpty`]
    ///
    /// This state is reachable from the following:
    /// - [`SourceEmpty`](State::SourceEmpty) via [`non_terminal_empty`](SourceEmpty::non_terminal_empty)
    /// - [`SourceWithData`](State::SourceWithData) via [`non_terminal_empty`](SourceWithData::non_terminal_empty)
    ///
    /// This state can transition to the following:
    /// - [`SinkEmpty`](State::SinkEmpty) via [`sink_empty`](NonTerminalEmpty::sink_empty)
    NonTerminalEmpty(NonTerminalEmpty<'state, 'a, T>),
    /// Represents [`State::NonTerminalWithData`]
    ///
    /// This state is reachable from the following:
    /// - [`SourceEmpty`](State::SourceEmpty) via [`non_terminal_with_data`](SourceEmpty::non_terminal_with_data)
    /// - [`SourceWithData`](State::SourceWithData) via [`to_non_terminal_with_data`](SourceWithData::to_non_terminal_with_data)
    ///
    /// This state can transition to the following:
    /// - [`SinkWithData`](State::SinkWithData) via [`sink_with_data`](NonTerminalWithData::sink_with_data)
    NonTerminalWithData(NonTerminalWithData<'state, 'a, T>),
    /// Represents [`State::SinkEmpty`]
    ///
    /// This state is reachable from the following:
    /// - [`NonTerminalEmpty`](State::NonTerminalEmpty) via [`sink_empty`](NonTerminalEmpty::sink_empty)
    SinkEmpty,
    /// Represents [`State::SinkWithData`]
    ///
    /// This state is reachable from the following:
    /// - [`NonTerminalWithData`](State::NonTerminalWithData) via [`sink_with_data`](NonTerminalWithData::sink_with_data)
    SinkWithData(&'state mut char),
    /// Represents [`State::SourceEmpty`]
    ///
    /// This state can transition to the following:
    /// - [`NonTerminalEmpty`](State::NonTerminalEmpty) via [`non_terminal_empty`](SourceEmpty::non_terminal_empty)
    /// - [`NonTerminalWithData`](State::NonTerminalWithData) via [`non_terminal_with_data`](SourceEmpty::non_terminal_with_data)
    SourceEmpty(SourceEmpty<'state, 'a, T>),
    /// Represents [`State::SourceWithData`]
    ///
    /// This state can transition to the following:
    /// - [`NonTerminalEmpty`](State::NonTerminalEmpty) via [`non_terminal_empty`](SourceWithData::non_terminal_empty)
    /// - [`NonTerminalWithData`](State::NonTerminalWithData) via [`to_non_terminal_with_data`](SourceWithData::to_non_terminal_with_data)
    SourceWithData(SourceWithData<'state, 'a, T>),
}
impl<'state, 'a, T> ::core::convert::From<&'state mut State<'a, T>>
for StateEntry<'state, 'a, T>
where
    T: Ord,
{
    fn from(value: &'state mut State<'a, T>) -> Self {
        match value {
            State::IsolatedEmpty => StateEntry::IsolatedEmpty,
            State::IsolatedWithData(it) => StateEntry::IsolatedWithData(it),
            State::NonTerminalEmpty => {
                StateEntry::NonTerminalEmpty(NonTerminalEmpty(value))
            }
            State::NonTerminalWithData(_) => {
                StateEntry::NonTerminalWithData(NonTerminalWithData(value))
            }
            State::SinkEmpty => StateEntry::SinkEmpty,
            State::SinkWithData(it) => StateEntry::SinkWithData(it),
            State::SourceEmpty => StateEntry::SourceEmpty(SourceEmpty(value)),
            State::SourceWithData(_) => StateEntry::SourceWithData(SourceWithData(value)),
        }
    }
}
impl<'a, T> State<'a, T>
where
    T: Ord,
{
    #[allow(clippy::needless_lifetimes)]
    pub fn entry<'state>(&'state mut self) -> StateEntry<'state, 'a, T> {
        self.into()
    }
}
/// See [`StateEntry::NonTerminalEmpty`]
pub struct NonTerminalEmpty<'state, 'a, T>(
    /// MUST match [`StateEntry::NonTerminalEmpty`]
    &'state mut State<'a, T>,
)
where
    T: Ord;
/// See [`StateEntry::NonTerminalWithData`]
pub struct NonTerminalWithData<'state, 'a, T>(
    /// MUST match [`StateEntry::NonTerminalWithData`]
    &'state mut State<'a, T>,
)
where
    T: Ord;
/// See [`StateEntry::SourceEmpty`]
pub struct SourceEmpty<'state, 'a, T>(
    /// MUST match [`StateEntry::SourceEmpty`]
    &'state mut State<'a, T>,
)
where
    T: Ord;
/// See [`StateEntry::SourceWithData`]
pub struct SourceWithData<'state, 'a, T>(
    /// MUST match [`StateEntry::SourceWithData`]
    &'state mut State<'a, T>,
)
where
    T: Ord;
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> ::core::convert::AsRef<Vec<u8>>
for NonTerminalWithData<'state, 'a, T>
where
    T: Ord,
{
    fn as_ref(&self) -> &Vec<u8> {
        match &self.0 {
            State::NonTerminalWithData(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> ::core::convert::AsMut<Vec<u8>>
for NonTerminalWithData<'state, 'a, T>
where
    T: Ord,
{
    fn as_mut(&mut self) -> &mut Vec<u8> {
        match &mut self.0 {
            State::NonTerminalWithData(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> ::core::convert::AsRef<&'a mut T> for SourceWithData<'state, 'a, T>
where
    T: Ord,
{
    fn as_ref(&self) -> &&'a mut T {
        match &self.0 {
            State::SourceWithData(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> ::core::convert::AsMut<&'a mut T> for SourceWithData<'state, 'a, T>
where
    T: Ord,
{
    fn as_mut(&mut self) -> &mut &'a mut T {
        match &mut self.0 {
            State::SourceWithData(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> NonTerminalEmpty<'state, 'a, T>
where
    T: Ord,
{
    /// Transition to [`State::SinkEmpty`]
    pub fn sink_empty(self) {
        match ::core::mem::replace(self.0, State::SinkEmpty) {
            State::NonTerminalEmpty => {}
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> NonTerminalWithData<'state, 'a, T>
where
    T: Ord,
{
    /// Method documentation on a non-renamed method.
    ///
    /// Transition to [`State::SinkWithData`]
    pub fn sink_with_data(self, next: char) -> Vec<u8> {
        match ::core::mem::replace(self.0, State::SinkWithData(next)) {
            State::NonTerminalWithData(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> SourceEmpty<'state, 'a, T>
where
    T: Ord,
{
    /// Transition to [`State::NonTerminalEmpty`]
    pub fn non_terminal_empty(self) {
        match ::core::mem::replace(self.0, State::NonTerminalEmpty) {
            State::SourceEmpty => {}
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> SourceEmpty<'state, 'a, T>
where
    T: Ord,
{
    /// Transition to [`State::NonTerminalWithData`]
    pub fn non_terminal_with_data(self, next: Vec<u8>) {
        match ::core::mem::replace(self.0, State::NonTerminalWithData(next)) {
            State::SourceEmpty => {}
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> SourceWithData<'state, 'a, T>
where
    T: Ord,
{
    /// Transition to [`State::NonTerminalEmpty`]
    pub fn non_terminal_empty(self) -> &'a mut T {
        match ::core::mem::replace(self.0, State::NonTerminalEmpty) {
            State::SourceWithData(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
#[allow(clippy::needless_lifetimes)]
impl<'state, 'a, T> SourceWithData<'state, 'a, T>
where
    T: Ord,
{
    /// Method documentation on renamed method.
    ///
    /// Transition to [`State::NonTerminalWithData`]
    pub fn to_non_terminal_with_data(self, next: Vec<u8>) -> &'a mut T {
        match ::core::mem::replace(self.0, State::NonTerminalWithData(next)) {
            State::SourceWithData(it) => it,
            _ => ::core::panic!("entry struct was instantiated with a mismatched state"),
        }
    }
}
