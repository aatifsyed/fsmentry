use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    braced, bracketed, custom_keyword, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token, Attribute, Generics, Ident, LitStr, Token, Type, Visibility,
};

pub struct Dsl<T = Punctuated<Statement, Token![,]>> {
    pub doc: Vec<DocAttr>,
    pub vis: Visibility,
    pub r#mod: Token![mod],
    pub name: Ident,
    pub brace: token::Brace,
    pub state: StateEnum<T>,
}

impl<T> Dsl<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Dsl<U> {
        let Self {
            doc,
            vis,
            r#mod,
            name,
            brace,
            state,
        } = self;
        Dsl {
            doc,
            vis,
            r#mod,
            name,
            brace,
            state: state.map(f),
        }
    }
}

impl Parse for Dsl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            doc: parse_docs(input)?,
            vis: input.parse()?,
            r#mod: input.parse()?,
            name: input.parse()?,
            brace: braced!(content in input),
            state: content.parse()?,
        })
    }
}

pub struct StateEnum<T = Punctuated<Statement, Token![,]>> {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub r#enum: Token![enum],
    pub name: Ident,
    pub generics: Generics,
    pub brace: token::Brace,
    pub dfn: T,
}

impl<T> StateEnum<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> StateEnum<U> {
        let Self {
            attrs,
            vis,
            r#enum,
            name,
            generics,
            brace,
            dfn,
        } = self;
        StateEnum {
            attrs,
            vis,
            r#enum,
            name,
            generics,
            brace,
            dfn: f(dfn),
        }
    }
}

impl Parse for StateEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            attrs: Attribute::parse_outer(input)?,
            vis: input.parse()?,
            r#enum: input.parse()?,
            name: input.parse()?,
            generics: {
                let mut it = input.parse::<Generics>()?;
                it.where_clause = input.parse()?;
                it
            },
            brace: braced!(content in input),
            dfn: Punctuated::parse_terminated(&content)?,
        })
    }
}
pub enum Statement {
    Node {
        doc: Vec<DocAttr>,
        node: Node,
    },
    Transition {
        doc: Vec<DocAttr>,
        first: Node,
        rest: Vec<(Arrow, Node)>,
    },
}

impl Parse for Statement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let doc = parse_docs(input)?;
        let node = input.parse()?;
        let mut rest = vec![];
        while input.peek(Token![-]) {
            rest.push((input.parse()?, input.parse()?))
        }
        Ok(match rest.is_empty() {
            true => Self::Node { doc, node },
            false => Self::Transition {
                doc,
                first: node,
                rest,
            },
        })
    }
}

pub struct Node {
    pub name: Ident,
    pub ty: Option<(token::Paren, Type)>,
}

impl Parse for Node {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
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
pub enum Arrow {
    Plain(Token![->]),
    Doc {
        start: Token![-],
        doc: LitStr,
        end: Token![->],
    },
}
impl Parse for Arrow {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![->]) {
            return Ok(Self::Plain(input.parse()?));
        }
        Ok(Self::Doc {
            start: input.parse()?,
            doc: input.parse()?,
            end: input.parse()?,
        })
    }
}

impl ToTokens for Arrow {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Arrow::Plain(rarrow) => rarrow.to_tokens(tokens),
            Arrow::Doc { start, doc, end } => {
                start.to_tokens(tokens);
                doc.to_tokens(tokens);
                end.to_tokens(tokens);
            }
        }
    }
}

custom_keyword!(doc);

#[derive(Debug, Clone)]
pub struct DocAttr {
    pub pound: Token![#],
    pub bracket: token::Bracket,
    pub doc: doc,
    pub eq: Token![=],
    pub str: LitStr,
}
impl DocAttr {
    pub fn new(lit: LitStr) -> Self {
        let span = lit.span();
        Self {
            pound: Token![#](span),
            bracket: token::Bracket(span),
            doc: doc(span),
            eq: Token![=](span),
            str: lit,
        }
    }
    pub fn empty() -> Self {
        Self::new(LitStr::new("", Span::call_site()))
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
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
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
