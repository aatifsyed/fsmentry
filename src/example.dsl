/// This explores all vertex types.
#[derive(Debug)]
#[fsmentry(
    mermaid(true),
)]
pub enum State<'a, T>
where
    T: Ord
{
    /// Isolated vertex, with data.
    IsolatedWithData(String),
    /// Isolated vertex, without data.
    IsolatedEmpty,

    /// Source vertex, with data.
    SourceWithData(&'a mut T)
        /// Method documentation on renamed method.
        -to_non_terminal_with_data->
        /// Non-terminal vertex, with data.
        NonTerminalWithData(Vec<u8>)
        /// Method documentation on a non-renamed method.
        ->
        /// Sink vertex, with data.
        SinkWithData(char),

    SourceWithData -> NonTerminalEmpty -> SinkEmpty,

    SourceEmpty -> NonTerminalWithData,
    SourceEmpty -> NonTerminalEmpty,
}
