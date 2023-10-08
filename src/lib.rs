//! A code generator for finite state machines with the following features:
//! - An `entry` api to transition the state machine.
//! - Illegal states and transitions are unrepresentable.
//! - States can contain data.
//! - Custom `#[derive(..)]` support.
//! - `#![no_std]` support.
//! - Inline SVG diagrams of the state machine in docs.
//!
//! ```
//! // define the machine.
//! // you can also use the DOT language if you prefer.
//! fsmentry::dsl! {
//!     /// This is a state machine for a traffic light
//!     // Documentation on nodes and states will appear in the generated code
//!     pub TrafficLight {
//!         Red; // this is a state
//!         Green: String; // this state has data inside it.
//!
//!         /// Cars speed up
//!         // this documentation is shared among all the edges
//!         Red -> RedAmber -> Green;
//!         //     ^ states are implicitly created
//!
//!         /// Cars slow down
//!         Green -> Amber -"make sure you stop!"-> Red;
//!         //             ^ this documentation is for this edge only
//!     }
//! }
//!
//! use traffic_light::{TrafficLight, Entry};
//!
//! // instantiate the machine
//! let mut machine = TrafficLight::new(traffic_light::State::Red);
//! loop {
//!     match machine.entry() {
//!         Entry::Red(it) => it.red_amber(), // transition the state machine
//!         // when you transition to a state with data,
//!         // you must provide the data
//!         Entry::RedAmber(it) => it.green(String::from("this is some data")),
//!         Entry::Green(mut it) => {
//!             // you can inspect or mutate the data in a state...
//!             let data: &String = it.get();
//!             let data: &mut String = it.get_mut();
//!             // ...and you get it back when you transition out of a state
//!             let data: String = it.amber();
//!         },
//!         Entry::Amber(it) => break,
//!     }
//! }
//! ```
//!
//! # Features
//! - `macros` (default): Include the [`dot`] and [`dsl`] macros.
//! - `svg` (default): The macros will shell out to `dot`, if available, and
//!   generate a diagram of the state machine for documentation.
//! - `std` (default): Includes the [`FSMGenerator`], for custom codegen tools.
//! - `cli`: This does not affect the library, but if you
//!   ```console
//!   cargo install fsmentry --features=cli
//!   ```
//!   You will get an `fsmentry` binary that you can use to generate code.
//!
//! Here are more details about the generated code.
//! ```
//! fsmentry::dsl! {
//!     #[derive(Clone, Debug, derive_quickcheck_arbitrary::Arbitrary)]
//! # pub MyStateMachine { DeadEnd; DeadEndWithData: String; WithTransitions -> DeadEnd; }
//! # }
//! # const _: &str = stringify! {
//!     pub MyStateMachine { .. }
//! # };
//! # {
//! }
//! use my_state_machine::{MyStateMachine, State, Entry};
//! // ^ A module with matching publicity is generated for the state machine.
//! //   The `#[derive(..)]`s apply to the `State` and the `MyStateMachine` items.
//!
//! # fn _doc(g: &mut quickcheck::Gen) {
//! # use quickcheck::Arbitrary as _;
//! let mut machine = MyStateMachine::arbitrary(g);
//!
//! // you can also inspect and mutate the state yourself.
//! let state: &State = machine.state();
//! let state: &mut State = machine.state_mut();
//!
//! match machine.entry() {
//!     // states with no transitions and no data are empty entries
//!     Entry::DeadEnd => {},
//!     // states with no transitions give you the data
//!     Entry::DeadEndWithData(data) => {
//!         let _: &mut String = data;
//!     },
//!     Entry::WithTransitions(handle) => {
//!         // otherwise, you get a struct which allows you to transition the machine.
//!         // (It will have getters for data as appropriate).
//!         handle.dead_end();
//!     }
//!     // ...
//! }
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

/// This is an example state machine that is only included on [`docs.rs`](https://docs.rs).
/// It is generated from the following definition:
/// ```rust,ignore
#[doc = include_str!("full.dsl")]
/// ```
#[cfg(docsrs)]
pub mod example;

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[doc(inline)]
pub use fsmentry_core::FSMGenerator;

#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
#[doc(inline)]
pub use fsmentry_macros::{dot, dsl};

#[cfg(test)]
mod tests {
    use fsmentry_core::FSMGenerator;
    use syn::parse::Parser as _;

    #[test]
    fn trybuild() {
        let t = trybuild::TestCases::new();
        t.pass("trybuild/pass/**/*.rs");
        t.compile_fail("trybuild/fail/**/*.rs")
    }

    #[test]
    fn example() {
        let generator = FSMGenerator::parse_dsl
            .parse_str(include_str!("full.dsl"))
            .unwrap();
        let example = svg::attach(generator.codegen(), &generator);
        let expected = prettyplease::unparse(&example);
        print!("{}", expected);
        pretty_assertions::assert_str_eq!(expected, include_str!("example.rs"))
    }

    #[test]
    fn readme() {
        assert!(
            std::process::Command::new("cargo")
                .args(["rdme", "--check"])
                .output()
                .expect("couldn't run `cargo rdme`")
                .status
                .success(),
            "README.md is out of date - bless the new version by running `cargo rdme`"
        )
    }

    mod svg {
        include!("../macros/src/svg.rs"); // I'm not writing this 3 times...
    }
}
