Parse a state machine from the [The `DOT` graph description language](https://en.wikipedia.org/wiki/DOT_%28graph_description_language%29):
```rust,ignore
digraph my_state_machine {
    // declaring a node.
    shaving_yaks;

    // declaring some edges, with implicit nodes.
    shaving_yaks -> sweeping_hair -> resting;
}
```
