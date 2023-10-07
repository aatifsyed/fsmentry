use fsmentry_core::FSMGenerator;
use quote::ToTokens as _;
use syn::parse_macro_input;

/// Generates a state machine from the following language:
/// ```rust,ignore
#[doc = include_str!("../../diagrams/doc.dsl")]
/// ```
#[proc_macro]
pub fn dsl(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item with FSMGenerator::parse_dsl);
    item.codegen().into_token_stream().into()
}

/// Generates a state machine from the [`DOT` graph description language](https://en.wikipedia.org/wiki/DOT_%28graph_description_language%29):
/// ```text
#[doc = include_str!("../../diagrams/doc.dot")]
/// ```
#[proc_macro]
pub fn dot(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item with FSMGenerator::parse_dot);
    item.codegen().into_token_stream().into()
}
