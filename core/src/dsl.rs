use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    braced, bracketed, custom_keyword, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::{Pair, Punctuated},
    spanned::Spanned as _,
    token, Attribute, Generics, Ident, LitStr, Token, Type, Visibility,
};

pub(crate) struct Root {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    #[allow(unused)]
    pub r#enum: Token![enum],
    pub ident: Ident,
    pub generics: Generics,
    #[allow(unused)]
    pub brace: token::Brace,
    pub stmts: Punctuated<Statement, Token![,]>,
}

#[test]
fn state_enum() {
    let _: Root = syn::parse_quote! {
    pub enum State<'a, T>
    where
        T: Ord
    {
        PopulatedIsland(String),
        DesertIsland,

        Fountain(&'a mut T)
            /// Go over the water
            -fountain2bridge-> BeautifulBridge(Vec<u8>)
            /// Reuse the rocks
            -bridge2tombstone-> Tombstone(char),
        /// This fountain is so pretty!
        Fountain -> Plank ->
            /// This grave is simple, and beautiful in its simplicity.
            UnmarkedGrave,

        Stream -> BeautifulBridge,
        Stream -> Plank,
    }};
}
impl Parse for Root {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            attrs: Attribute::parse_outer(input)?,
            vis: input.parse()?,
            r#enum: input.parse()?,
            ident: input.parse()?,
            generics: {
                let mut it = input.parse::<Generics>()?;
                it.where_clause = input.parse()?;
                it
            },
            brace: braced!(content in input),
            stmts: {
                let mut stmts = Punctuated::new();
                while !content.is_empty() {
                    let fork = content.fork();
                    fork.parse::<Node>()?;
                    if fork.peek(Token![,]) || fork.is_empty() {
                        stmts.push(Statement::Node(content.parse()?));
                    } else {
                        let first = content.parse()?;
                        let mut rest = vec![];
                        while content.peek(Token![-]) || content.peek(Token![#]) {
                            let arrow = content.parse::<Arrow>()?;
                            let next = content.parse::<NodeGroup>()?;
                            if next.into_iter().len() > 1
                                && matches!(arrow.kind, ArrowKind::Named { .. })
                            {
                                let msg = "Named transitions (`-name->`) to node groups (`A & B`) are not supported, since it requires duplicate method names";
                                return Err(syn::Error::new(arrow.kind.span(), msg));
                            }
                            rest.push((arrow, next));
                        }
                        if rest.is_empty() {
                            return Err(content.error(
                                "Node groups (`A & B`) must be followed by transitions (`->`)",
                            ));
                        }
                        stmts.push(Statement::Transition { first, rest });
                    }
                    if !content.is_empty() {
                        stmts.push_punct(content.parse()?);
                    }
                }
                stmts
            },
        })
    }
}

pub(crate) enum Statement {
    Node(Node),
    Transition {
        first: NodeGroup,
        rest: Vec<(Arrow, NodeGroup)>,
    },
}

pub(crate) struct Node {
    pub doc: Vec<DocAttr>,
    pub name: Ident,
    pub ty: Option<(token::Paren, Type)>,
}
impl Parse for Node {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            doc: parse_docs(input)?,
            name: input.parse()?,
            ty: match input.peek(token::Paren) {
                true => {
                    let content;
                    Some((parenthesized!(content in input), content.parse()?))
                }
                false => None,
            },
        })
    }
}

pub(crate) struct NodeGroup(Punctuated<Node, Token![&]>);

impl Parse for NodeGroup {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(Punctuated::parse_separated_nonempty(input)?))
    }
}

impl<'a> IntoIterator for &'a NodeGroup {
    type Item = &'a Node;
    type IntoIter = <&'a Punctuated<Node, Token![&]> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

pub(crate) struct Arrow {
    pub doc: Vec<DocAttr>,
    pub kind: ArrowKind,
}
impl Parse for Arrow {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            doc: parse_docs(input)?,
            kind: input.parse()?,
        })
    }
}
impl ToTokens for Arrow {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { doc, kind } = self;
        docs_to_tokens(doc, tokens);
        kind.to_tokens(tokens);
    }
}

pub(crate) enum ArrowKind {
    Plain(Token![->]),
    Named {
        start: Token![-],
        ident: Ident,
        end: Token![->],
    },
}
impl Parse for ArrowKind {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![->]) {
            return Ok(Self::Plain(input.parse()?));
        }
        Ok(Self::Named {
            start: input.parse()?,
            ident: input.parse()?,
            end: input.parse()?,
        })
    }
}
impl ToTokens for ArrowKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            ArrowKind::Plain(it) => it.to_tokens(tokens),
            ArrowKind::Named { start, ident, end } => {
                start.to_tokens(tokens);
                ident.to_tokens(tokens);
                end.to_tokens(tokens);
            }
        }
    }
}

custom_keyword!(doc);

#[derive(Clone)]
pub(crate) struct DocAttr {
    pub pound: Token![#],
    pub bracket: token::Bracket,
    pub doc: doc,
    pub eq: Token![=],
    pub str: LitStr,
}
impl DocAttr {
    pub fn new(s: &str, span: Span) -> Self {
        Self {
            pound: Token![#](span),
            bracket: token::Bracket(span),
            doc: doc(span),
            eq: Token![=](span),
            str: LitStr::new(s, span),
        }
    }
    pub fn empty() -> Self {
        Self::new("", Span::call_site())
    }
}
impl Parse for DocAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            pound: input.parse()?,
            bracket: bracketed!(content in input),
            doc: content.parse()?,
            eq: content.parse()?,
            str: content.parse()?,
        })
    }
}
impl ToTokens for DocAttr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            pound,
            bracket,
            doc,
            eq,
            str,
        } = self;
        pound.to_tokens(tokens);
        bracket.surround(tokens, |tokens| {
            doc.to_tokens(tokens);
            eq.to_tokens(tokens);
            str.to_tokens(tokens);
        });
    }
}

fn parse_docs(input: ParseStream) -> syn::Result<Vec<DocAttr>> {
    let mut parsed = vec![];
    while input.peek(Token![#]) {
        parsed.push(input.parse()?);
    }
    Ok(parsed)
}
fn docs_to_tokens(docs: &[DocAttr], tokens: &mut TokenStream) {
    for doc in docs {
        doc.to_tokens(tokens);
    }
}

pub(crate) struct VisIdent {
    pub vis: Visibility,
    pub ident: Ident,
}
impl Parse for VisIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            vis: input.parse()?,
            ident: input.parse()?,
        })
    }
}
impl ToTokens for VisIdent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { vis, ident } = self;
        vis.to_tokens(tokens);
        ident.to_tokens(tokens);
    }
}

pub(crate) struct ModulePath {
    leading_colon: Option<Token![::]>,
    segments: Punctuated<Ident, Token![::]>,
}
impl Parse for ModulePath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let syn::Path {
            leading_colon,
            segments,
        } = syn::Path::parse_mod_style(input)?;
        Ok(Self {
            leading_colon,
            segments: segments
                .into_pairs()
                .map(|it| match it {
                    Pair::Punctuated(seg, sep) => Pair::Punctuated(seg.ident, sep),
                    Pair::End(seg) => Pair::End(seg.ident),
                })
                .collect(),
        })
    }
}
impl ToTokens for ModulePath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            leading_colon,
            segments,
        } = self;
        leading_colon.to_tokens(tokens);
        segments.to_tokens(tokens);
    }
}
