use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    parse_quote, token, Attribute, Expr, Lit, LitStr, Meta, MetaNameValue, Token,
};

mod kw {
    syn::custom_keyword!(doc);
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct OuterDocString {
    pub pound_token: Token![#],
    pub bracket_token: token::Bracket,
    pub path: kw::doc,
    pub eq_token: Token![=],
    pub doc: LitStr,
}

impl OuterDocString {
    pub fn new(value: impl AsRef<str>, span: Span) -> Self {
        Self {
            pound_token: Token![#](span),
            bracket_token: token::Bracket(span),
            path: kw::doc(span),
            eq_token: Token![=](span),
            doc: LitStr::new(value.as_ref(), span),
        }
    }
    pub fn parse_many(input: ParseStream) -> syn::Result<Vec<Self>> {
        let mut attrs = Vec::new();
        while input.peek(Token![#]) {
            attrs.push(input.call(Self::parse)?);
        }
        Ok(attrs)
    }
}

impl TryFrom<Attribute> for OuterDocString {
    type Error = syn::Error;

    fn try_from(value: Attribute) -> syn::Result<Self> {
        syn::parse2(value.into_token_stream())
    }
}

impl From<OuterDocString> for Attribute {
    fn from(value: OuterDocString) -> Self {
        let OuterDocString {
            pound_token,
            bracket_token,
            path,
            eq_token,
            doc,
        } = value;
        Self {
            pound_token,
            style: syn::AttrStyle::Outer,
            bracket_token,
            meta: Meta::NameValue(MetaNameValue {
                path: parse_quote!(#path),
                eq_token,
                value: Expr::Lit(syn::ExprLit {
                    attrs: vec![],
                    lit: Lit::Str(doc),
                }),
            }),
        }
    }
}

impl Parse for OuterDocString {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            pound_token: input.parse()?,
            bracket_token: bracketed!(content in input),
            path: content.parse()?,
            eq_token: content.parse()?,
            doc: content.parse()?,
        })
    }
}

impl ToTokens for OuterDocString {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            pound_token,
            bracket_token,
            path,
            eq_token,
            doc,
        } = self;
        pound_token.to_tokens(tokens);
        bracket_token.surround(tokens, |tokens| {
            path.to_tokens(tokens);
            eq_token.to_tokens(tokens);
            doc.to_tokens(tokens)
        })
    }
}
