//! A code generator for state machines with an entry API.
//!
//! See the [`fsmentry` crate](https://docs.rs/fsmentry).

use fsmentry_core::FSMGenerator;
use quote::ToTokens as _;
use syn::parse_macro_input;

/// Generates a state machine from the following language:
/// ```
/// # use fsmentry_macros::dsl;
/// dsl! {
///     /// This is documentation for the state machine.
///     #[derive(Clone)] // these attributes will be passed to
///                      // MyStateMachine and the State enum
///     pub MyStateMachine {
///         /// This is a node declaration.
///         /// This documentation will be attached to the node.
///         ShavingYaks;
///
///         /// This node contains data.
///         SweepingHair: usize;
///
///         /// These are edge declarations
///         /// This documentation will be shared with each edge.
///         ShavingYaks -> SweepingHair -"this is edge-specific documentation"-> Resting;
///                             // implicit nodes will be created as appropriate ^
///     }
/// }
/// ```
#[proc_macro]
pub fn dsl(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let generator = parse_macro_input!(item with FSMGenerator::parse_dsl);
    let codegen = generator.codegen();
    #[cfg(feature = "svg")]
    let codegen = svg::attach(codegen, &generator);
    codegen.into_token_stream().into()
}

/// Generates a state machine from the [`DOT` graph description language](https://en.wikipedia.org/wiki/DOT_%28graph_description_language%29):
/// ```
/// # use fsmentry_macros::dot;
/// dot! {
///     digraph my_state_machine {
///         // declaring a node.
///         shaving_yaks;
///         
///         // declaring some edges, with implicit nodes.
///         shaving_yaks -> sweeping_hair -> resting;
///     }
/// }
/// ```
#[proc_macro]
pub fn dot(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let generator = parse_macro_input!(item with FSMGenerator::parse_dot);
    let codegen = generator.codegen();
    #[cfg(feature = "svg")]
    let codegen = svg::attach(codegen, &generator);
    codegen.into_token_stream().into()
}

#[cfg(feature = "svg")]
mod svg;
