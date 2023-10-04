use fsmentry_core::FSMGenerator;
use quote::ToTokens as _;
use syn::parse_macro_input;

#[proc_macro]
pub fn dot(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item with FSMGenerator::parse_dot);
    item.codegen().into_token_stream().into()
}

#[proc_macro]
pub fn dsl(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item with FSMGenerator::parse_dsl);
    item.codegen().into_token_stream().into()
}