<!-- cargo-rdme start -->

# `fsmentry`

A code generator for finite state machines (FSMs) with the following features:
- Define your machine as a graph in e.g [`DOT`](https://en.wikipedia.org/wiki/DOT_%28graph_description_language%29).
- An `entry` api to transition the state machine.
- Illegal states and transitions are unrepresentable.
- States can contain data.
- Custom `#[derive(..)]` support.
- `#![no_std]` support.
- Inline SVG diagrams of the state machine in docs.

```rust
// define the machine.
// you can also use the DOT language if you prefer.
fsmentry::dsl! {
    /// This is a state machine for a traffic light
    // Documentation on nodes and states will appear in the generated code
    pub TrafficLight {
        Red; // this is a state
        Green: String; // this state has data inside it.

        /// Cars speed up
        // this documentation is shared among all the edges
        Red -> RedAmber -> Green;
        //     ^ states are implicitly created

        /// Cars slow down
        Green -> Amber -"make sure you stop!"-> Red;
        //             ^ this documentation is for this edge only
    }
}

use traffic_light::{TrafficLight, Entry};

// instantiate the machine
let mut machine = TrafficLight::new(traffic_light::State::Red);
loop {
    match machine.entry() {
        Entry::Red(it) => it.red_amber(), // transition the state machine
        // when you transition to a state with data,
        // you must provide the data
        Entry::RedAmber(it) => it.green(String::from("this is some data")),
        Entry::Green(mut it) => {
            // you can inspect or mutate the data in a state...
            let data: &String = it.get();
            let data: &mut String = it.get_mut();
            // ...and you get it back when you transition out of a state
            let data: String = it.amber();
        },
        Entry::Amber(it) => break,
    }
}
```

# Cargo features

- `macros` (default): Include the [`dot`] and [`dsl`] macros.
- `svg` (default): The macros will shell out to `dot`, if available, and
  generate a diagram of the state machine for documentation.
- `std` (default): Includes the [`FSMGenerator`], for custom codegen tools.
- `cli`: This does not affect the library, but if you
  ```console
  cargo install fsmentry --features=cli
  ```
  You will get an `fsmentry` binary that you can use to generate code.

# Advanced usage

```rust
fsmentry::dsl! {
    #[derive(Clone, Debug, derive_quickcheck_arbitrary::Arbitrary)] // attach `#[derive(..)]`s here
    pub MyStateMachine { .. }
}
use my_state_machine::{MyStateMachine, State, Entry};
// ^ A module with matching publicity is generated for the state machine.
//   The `#[derive(..)]`s apply to the `State` and the `MyStateMachine` items.

let mut machine = MyStateMachine::arbitrary(g); // we can use derived traits!

// you can also inspect and mutate the state yourself.
let state: &State = machine.state();
let state: &mut State = machine.state_mut();

match machine.entry() {
    // states with no transitions and no data are empty entries
    Entry::DeadEnd => {},
    // states with no transitions give you the data
    Entry::DeadEndWithData(data) => {
        let _: &mut String = data;
    },
    Entry::WithTransitions(handle) => {
        // otherwise, you get a struct which allows you to transition the machine.
        // (It will have getters for data as appropriate).
        handle.dead_end();
    }
    // ...
}
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
fsmentry::dsl! { // the outer state machine
    pub Webcam {
        NotBlinking -> Blinking -> NotBlinking;
        Blinking: super::led::Led; // The `Blinking` state contains a state machine
    }
}

fsmentry::dsl! { // the inner state machine
    pub Led {
        LedOn -> LedOff -> LedOn;
    }
}

let mut machine = webcam::Webcam::new(webcam::State::NotBlinking);
loop {
    match machine.entry() { // transition the outer machine
        webcam::Entry::Blinking(mut webcam) => match webcam.get_mut().entry() { // transition the inner machine
            led::Entry::LedOff(it) => it.led_on(),
            led::Entry::LedOn(it) => {
                it.led_off();
                webcam.not_blinking();
            }
        },
        webcam::Entry::NotBlinking(webcam) => {
            webcam.blinking(led::Led::new(led::State::LedOff))
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
