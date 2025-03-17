#![allow(unused)]

use std::collections::BTreeMap;

use syn::{
    meta::ParseNestedMeta,
    parenthesized,
    parse::{Parse, ParseStream},
    Attribute, LitBool, Token,
};

/// Simple argument parser for `#[foo(bar = .., baz(..))]` arguments.
// This file is designed to be transplantable.
#[derive(Default)]
pub struct Parser<'a> {
    #[expect(clippy::type_complexity)]
    inner: BTreeMap<
        String,
        Either<
            Option<Box<dyn FnOnce(ParseStream<'_>) -> syn::Result<()> + 'a>>,
            Box<dyn FnMut(ParseStream<'_>) -> syn::Result<()> + 'a>,
        >,
    >,
}

impl<'a> Parser<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    /// Parse this argument at most once.
    pub fn once(
        mut self,
        key: impl Into<String>,
        f: impl FnOnce(ParseStream<'_>) -> syn::Result<()> + 'a,
    ) -> Self {
        let clobbered = self
            .inner
            .insert(key.into(), Either::Left(Some(Box::new(f))));
        assert!(clobbered.is_none());
        self
    }
    /// Parse this argument many times.
    pub fn many(
        mut self,
        key: impl Into<String>,
        f: impl FnMut(ParseStream<'_>) -> syn::Result<()> + 'a,
    ) -> Self {
        let clobbered = self.inner.insert(key.into(), Either::Right(Box::new(f)));
        assert!(clobbered.is_none());
        self
    }
    /// Use with [`Attribute::parse_nested_meta`].
    pub fn parse(&mut self, meta: ParseNestedMeta<'_>) -> syn::Result<()> {
        for (k, e) in &mut self.inner {
            if meta.path.is_ident(k) {
                return match e {
                    Either::Left(o) => match o.take() {
                        Some(f) => f(meta.input),
                        None => Err(meta.error("duplicate value for key")),
                    },
                    Either::Right(m) => m(meta.input),
                };
            }
        }
        Err(meta.error(format!("Expected one of {:?}", self.inner.keys())))
    }
    /// Filter out attributes with the given `ident`, parsing them as appropriate.
    pub fn extract(&mut self, ident: &str, attrs: &mut Vec<Attribute>) -> syn::Result<()> {
        let mut error = None;
        attrs.retain(|attr| {
            if attr.path().is_ident(ident) {
                if let Err(e) = attr.parse_nested_meta(|meta| self.parse(meta)) {
                    match &mut error {
                        None => error = Some(e),
                        Some(already) => already.combine(e),
                    }
                };
                false // we've parsed - filter it out
            } else {
                true // keep it
            }
        });
        match error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

enum Either<L, R> {
    Left(L),
    Right(R),
}

fn _on_value(
    input: ParseStream<'_>,
    f: impl FnOnce(ParseStream<'_>) -> syn::Result<()>,
) -> syn::Result<()> {
    match input.peek(Token![=]) {
        true => {
            input.parse::<Token![=]>()?;
            f(input)
        }
        false => {
            let content;
            parenthesized!(content in input);
            f(&content)
        }
    }
}

/// Calls `f` on the following portions of `input`:
/// ```text
/// foo = bar
///       ^^^
///  *or*
/// foo(bar)
///     ^^^
/// ```
pub fn on_value<'a>(
    mut f: impl FnMut(ParseStream<'_>) -> syn::Result<()> + 'a,
) -> impl FnMut(ParseStream<'_>) -> syn::Result<()> + 'a {
    move |input| {
        let f = &mut f;
        _on_value(input, f)
    }
}

/// Create a parser which assigns the given bool.
pub fn bool(dst: &mut bool) -> impl FnMut(ParseStream<'_>) -> syn::Result<()> + '_ {
    |input| {
        *dst = input.parse::<LitBool>()?.value;
        Ok(())
    }
}

/// Create a parser which assigns to the given item.
pub fn parse<T: Parse>(dst: &mut T) -> impl FnMut(ParseStream<'_>) -> syn::Result<()> + '_ {
    |input| {
        *dst = input.parse()?;
        Ok(())
    }
}
