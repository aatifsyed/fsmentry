//! A generator for state machines with the following features:
//! - An `entry` api to transition the state machine.
//! - Illegal states and transitions are unrepresentable.
//! - States can contain data.
//! - Custom `#[derive(..)]` support.
//! - Visual representation of the state machine in docs.
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
    #[test]
    fn trybuild() {
        let t = trybuild::TestCases::new();
        t.pass("trybuild/pass/**/*.rs");
        t.compile_fail("trybuild/fail/**/*.rs")
    }
}
