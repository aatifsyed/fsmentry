use derive_syn_parse::Parse;
use proc_macro2::Ident;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Attribute, LitStr, Token, Type,
};

pub mod pun {
    // `-->` cannot be custom punctuation because the first Minus token is Alone
    syn::custom_punctuation!(ShortArrow, ->);
}

#[test]
fn parse_dsl() {
    let dsl: Dsl = syn::parse_quote! {
        /// a node description
        Foo { bar: String };
        /// an edge
        Foo --> Bar;
        /// many edges
        Foo --> Bar -> Baz;
        /// another edge
        Foo -"with inline docs"-> Bar;
        Foo -"and"-> Bar -"a"-> Few;
    };
    dbg!(dsl);
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Dsl {
    pub stmts: Vec<Stmt>,
}

impl Parse for Dsl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut stmts = vec![];
        while !input.is_empty() {
            stmts.push(input.parse()?)
        }
        Ok(Self { stmts })
    }
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
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
    /// Attributes will be stolen from the first Node
    #[call(Attribute::parse_outer)]
    pub attrs: Vec<Attribute>,
    pub from: Node,
    pub edge: Edge,
    pub to: Node,
    #[call(Self::parse_rest)]
    pub rest: Vec<(Edge, Node)>,
    pub semi: Token![;],
}

impl StmtEdges {
    fn parse_rest(input: ParseStream) -> syn::Result<Vec<(Edge, Node)>> {
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
    pub node: Node,
    pub semi: Token![;],
}

#[derive(Parse)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Node {
    #[call(Attribute::parse_outer)]
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    #[peek_with(has_brace_or_paren_or_eq)]
    pub fields: Option<NodeFields>,
}

fn has_brace_or_paren_or_eq(input: ParseStream) -> bool {
    input.peek(token::Brace) || input.peek(token::Paren) || input.peek(Token![=])
}

#[derive(Parse)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum NodeFields {
    #[peek(Token![=], name = "=")]
    Alias(Alias),
    #[peek(token::Paren, name = "(")]
    TupleLike(TupleLike),
    #[peek(token::Brace, name = "{")]
    RecordLike(RecordLike),
}

#[derive(Parse)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Alias {
    pub eq: Token![=],
    pub ty: Type,
}

#[derive(Parse)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct TupleLike {
    #[paren]
    pub paren_token: token::Paren,
    #[inside(paren_token)]
    #[call(Punctuated::parse_terminated)]
    pub fields: Punctuated<Type, Token![,]>,
}

#[derive(Parse)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct RecordLike {
    #[brace]
    pub brace_token: token::Brace,
    #[inside(brace_token)]
    #[call(Punctuated::parse_terminated)]
    pub named_fields: Punctuated<NamedField, Token![,]>,
}

#[derive(Parse)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct NamedField {
    pub ident: Ident,
    pub colon_token: Token![:],
    pub ty: Type,
}

#[derive(Parse)]
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

#[derive(Parse)]
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
