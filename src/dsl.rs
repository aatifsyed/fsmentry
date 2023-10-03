use derive_syn_parse::Parse;
use proc_macro2::Ident;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    token, Attribute, LitStr, Token, Type, Visibility,
};

use crate::util::OuterDocString;

pub mod pun {
    // `-->` cannot be custom punctuation because the first Minus token is Alone
    syn::custom_punctuation!(ShortArrow, ->);
}

#[test]
fn parse_dsl() {
    let dsl: Dsl = syn::parse_quote! {
        /// These are state machine docs
        /// There will be a nice diagram here too
        ///
        /// The state machine struct and state enum will both implement Debug now
        #[derive(Debug)]
        pub TrafficLight {
            /// a node description
            Foo;
            /// This node has data associated with it
            Bar: String;

            /// an edge
            Foo -> Bar;

            /// many edges
            Foo --> Bar -> Baz; // the arrow length doesn't mean anything

            Foo -"with inline docs"-> Bar;

            /// This documentation is shared among the edges
            And -"some"-> Inline -"documentation"-> Too;
        }
    };
    dbg!(dsl);
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Dsl {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    pub brace_token: token::Brace,
    pub stmts: Vec<Stmt>,
}

impl Parse for Dsl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            name: input.parse()?,
            brace_token: braced!(content in input),
            stmts: {
                let mut stmts = vec![];
                while !content.is_empty() {
                    stmts.push(content.parse()?)
                }
                stmts
            },
        })
    }
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[allow(clippy::large_enum_variant)]
pub enum Stmt {
    Edges(StmtEdges),
    Node(StmtNode),
}

impl Parse for Stmt {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // bounded fork
        if input.fork().parse::<StmtNode>().is_ok() {
            return Ok(Self::Node(input.parse()?));
        }
        Ok(Self::Edges(input.parse()?))
    }
}

#[derive(Parse)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct StmtEdges {
    #[call(OuterDocString::parse_many)]
    pub attrs: Vec<OuterDocString>,
    pub from: Ident,
    pub edge: Edge,
    pub to: Ident,
    #[call(Self::parse_rest)]
    pub rest: Vec<(Edge, Ident)>,
    pub semi: Token![;],
}

impl StmtEdges {
    fn parse_rest(input: ParseStream) -> syn::Result<Vec<(Edge, Ident)>> {
        let mut rest = vec![];
        while !input.peek(Token![;]) {
            rest.push((input.parse()?, input.parse()?))
        }
        Ok(rest)
    }
}

#[derive(Parse)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct StmtNode {
    #[call(OuterDocString::parse_many)]
    pub attrs: Vec<OuterDocString>,
    pub ident: Ident,
    pub colon: Option<Token![:]>,
    #[parse_if(colon.is_some())]
    pub ty: Option<Type>,
    pub semi: Token![;],
}

#[derive(Parse, derive_quote_to_tokens::ToTokens)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum Edge {
    #[peek(pun::ShortArrow, name = "->")]
    Short(pun::ShortArrow),
    #[peek_with(minus_then_arrow, name = "-->")]
    Long(Token![-], pun::ShortArrow),
    #[peek(Token![-], name = r#"-"..."->"#)]
    Documented(DocumentedArrow),
}

fn minus_then_arrow(input: ParseStream) -> bool {
    input.peek(Token![-]) && input.peek2(pun::ShortArrow)
}

#[derive(Parse, derive_quote_to_tokens::ToTokens)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct DocumentedArrow {
    pub minus: Token![-],
    pub minus2: Option<Token![-]>,
    pub doc: LitStr,
    #[peek_with(not_short_arrow)]
    pub minus3: Option<Token![-]>,
    pub arrow: pun::ShortArrow,
}

fn not_short_arrow(input: ParseStream) -> bool {
    !input.peek(pun::ShortArrow)
}

#[test]
fn parse_arrow() {
    assert!(matches!(syn::parse_quote!(->), Edge::Short(_)));
    assert!(matches!(syn::parse_quote!(-->), Edge::Long(..)));
    assert!(matches!(syn::parse_quote!(-"hello"->), Edge::Documented(_)));
    assert!(matches!(syn::parse_quote!(--"ehlo"->), Edge::Documented(_)));
    assert!(matches!(syn::parse_quote!(-"ehlo"-->), Edge::Documented(_)));
    assert!(matches!(syn::parse_quote!(--"elo"-->), Edge::Documented(_)));
}
