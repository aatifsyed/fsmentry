<!-- cargo-rdme start -->

# `fsmentry`

```rust
fsmentry::dsl! {
    enum TrafficLight {
        Red -> RedAmber -> Green -> Amber -> Red
    }
}
```

A code generator for finite state machines (FSMs) with the following features:
- An `entry` api to transition the state machine.
- Illegal states and transitions can be made unrepresentable.
- States can contain data.
- Generic over user types.
- Custom `#[derive(..)]` support.
- Inline SVG diagrams in docs.
- Generated code is `#[no_std]` compatible.

```rust
// define the machine.
fsmentry! {
    /// This is a state machine for a traffic light
    // Documentation on nodes and states will appear in the generated code
    pub enum TrafficLight {
        /// Documentation for the [`Red`] state.
        Red, // this is a state
        Green(String), // this state has data inside it.

        Red -> RedAmber -> Green,
        //     ^ states can be defined inline.

        Green -custom_method_name-> Amber
            /// Custom method documentation
            -> Red,
    }
}

// instantiate the machine
let mut state = TrafficLight::Red;
loop {
    match state.entry() {
        TrafficLightEntry::Red(to) => to.red_amber(), // transition the state machine
        // when you transition to a state with data,
        // you must provide the data
        TrafficLightEntry::RedAmber(to) => to.green(String::from("this is some data")),
        TrafficLightEntry::Green(mut to) => {
            // you can inspect or mutate the data in a state...
            let data: &String = to.as_ref();
            let data: &mut String = to.as_mut();
            // ...and you get it back when you transition out of a state
            let data: String = to.custom_method_name();
        },
        TrafficLightEntry::Amber(_) => break,
    }
}
```

# About the generated code.

This macro has three main outputs:
- A "state" enum, which reflects the enum you pass in.
- An "entry" enum, with variants that reflect.
  - Data contained in the state (if any).
  - Transitions to a different state variant (if any) - see below.
- "transition" structs, which access the data in a variant and allow only legal transitions via methods.
  - Transition structs expose their mutable reference to the "state" above,
    to allow you to write e.g your own pinning logic.
    It is recommended that you wrap each machine in its own module to keep
    this reference private, lest you seed panics by manually creating a
    transition struct with the wrong underlying state.

```rust
mod my_state { // recommended to create a module per machine.
fsmentry::fsmentry! {
    /// These attributes are passed through to the state enum.
    #[derive(Debug)]
    #[fsmentry(
        mermaid(true), // Embed mermaid-js into the rustdoc to render a diagram.
        entry(pub(crate) MyEntry), // Override the default visibility and name
        unsafe(false), // By default, transition structs will panic if constructed incorrectly.
                       // If you promise to only create valid transition structs,
                       // or hide the transition structs in their own module,
                       // you can make these panics unreachable_unchecked instead.
        rename_methods(false), // By default, non-overridden methods are given
                               // snake_case names according to their destination
                               // but you can turn this off.
    )]
    pub enum MyState<'a, T> {
        Start -> GenericData(&'a mut T) -> Stop,
    }
}}

assert_impl_debug::<my_state::MyState<u8>>();
```

## Hierarchical state machines

`fsmentry` needs no special considerations for sub-state machines - simply store one
on the relevant node!
Here is the example from the [`statig`](https://crates.io/crates/statig) crate:
```text
┌─────────────────────────┐
│         Blinking        │🞀─────────┐
│    ┌───────────────┐    │          │
│ ┌─🞂│     LedOn     │──┐ │  ┌───────────────┐
│ │  └───────────────┘  │ │  │  NotBlinking  │
│ │  ┌───────────────┐  │ │  └───────────────┘
│ └──│     LedOff    │🞀─┘ │          🞁
│    └───────────────┘    │──────────┘
└─────────────────────────┘
```

```rust
fsmentry! {
    enum Webcam {
        NotBlinking -> Blinking(Led) -> NotBlinking
    }
}
fsmentry! {
    enum Led {
        On -> Off -> On,
    }
}

let mut webcam = Webcam::NotBlinking;
loop {
    match webcam.entry() { // transition the outer machine
        WebcamEntry::Blinking(mut webcam) => match webcam.as_mut().entry() { // transition the inner machine
            LedEntry::Off(led) => led.on(),
            LedEntry::On(led) => {
                led.off();
                webcam.not_blinking();
            }
        },
        WebcamEntry::NotBlinking(webcam) => {
            webcam.blinking(Led::On)
        }
    }
}
```

# Comparison with other state machine libraries

| Crate                                                 | Illegal states/transitions unrepresentable | States contain data | State machine definition    | Comments         |
| ----------------------------------------------------- | ------------------------------------------ | ------------------- | --------------------------- | ---------------- |
| [`fsmentry`](https://crates.io/crates/fsmentry)       | Yes                                        | Yes                 | Graph                       |                  |
| [`sm`](https://crates.io/crates/sm)                   | Yes                                        | No                  | States, events, transitions |                  |
| [`rust-fsm`](https://crates.io/crates/rust-fsm)       | No                                         | Yes (manually)      | States, events, transitions |                  |
| [`finny`](https://crates.io/crates/finny)             | No                                         | Yes                 | Builder                     |                  |
| [`sfsm`](https://crates.io/crates/sfsm)               | No                                         | No                  | States and transitions      |                  |
| [`statig`](https://crates.io/crates/statig)           | ?                                          | ?                   | ?                           | Complicated API! |
| [`sad_machine`](https://crates.io/crates/sad_machine) | Yes                                        | No                  | States, events, transitions |                  |
| [`machine`](https://crates.io/crates/machine)         | No                                         | Yes                 | States, events, transitions |                  |

<!-- cargo-rdme end -->
