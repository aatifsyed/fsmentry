//! A code generator for state machines with an entry API.
//!
//! See the [`fsmentry` crate](https://docs.rs/fsmentry).

mod args;
mod dsl;
mod graph;

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    iter,
};

use args::*;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned as _,
    Arm, Attribute, Expr, Generics, Ident, ImplGenerics, ItemImpl, ItemStruct, Lifetime, Token,
    Type, TypeGenerics, Variant, Visibility, WhereClause,
};

use crate::dsl::*;
use crate::graph::*;

macro_rules! bail_at {
    ($span:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        return Err(syn::Error::new($span, format!($fmt, $($arg,)*)))
    };
}

/// Renderer for mermaid diagrams.
pub trait Renderer {
    /// Return [`None`] to skip rendering.
    fn render(&self, diagram: &str) -> Option<String>;
}

/// Skip rendering entirely.
impl Renderer for () {
    fn render(&self, _: &str) -> Option<String> {
        None
    }
}

/// Forward to the inner [`Renderer`], if present.
impl<T: Renderer> Renderer for Option<T> {
    fn render(&self, diagram: &str) -> Option<String> {
        self.as_ref().and_then(|it| it.render(diagram))
    }
}

/// Call the provided function.
impl<F: Fn(&str) -> Option<String>> Renderer for F {
    fn render(&self, diagram: &str) -> Option<String> {
        self(diagram)
    }
}

/// A [`Renderer`] which embeds a script to load `mermaidjs` into the docs.
pub struct Mermaid(
    /// The URL to import mermaid from.
    pub String,
);

impl Default for Mermaid {
    fn default() -> Self {
        Self(String::from(
            "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs",
        ))
    }
}

impl Renderer for Mermaid {
    fn render(&self, diagram: &str) -> Option<String> {
        Some(format!(
            "\
<pre class=\"mermaid\">
{diagram}
</pre>
<script type=\"module\">
  import mermaid from \"{}\";
  var doc_theme = localStorage.getItem(\"rustdoc-theme\");
  if (doc_theme === \"dark\" || doc_theme === \"ayu\") mermaid.initialize({{theme: \"dark\"}});
</script>",
            self.0
        ))
    }
}

/// A [`Parse`]-able and [printable](ToTokens) representation of a state machine.
pub struct FsmEntry<MermaidR = ()> {
    state_attrs: Vec<Attribute>,
    state_vis: Visibility,
    state_ident: Ident,
    state_generics: Generics,

    r#unsafe: bool,
    path_to_core: ModulePath,

    entry_vis: Visibility,
    entry_ident: Ident,
    entry_lifetime: Lifetime,

    graph: Graph,

    render_mermaid: bool,
    mermaid_renderer: MermaidR,
}

impl<MermaidR> FsmEntry<MermaidR> {
    /// Change the mermaid renderer.
    pub fn map_mermaid<F, MermaidR2>(self, f: F) -> FsmEntry<MermaidR2>
    where
        F: FnOnce(MermaidR) -> MermaidR2,
    {
        let Self {
            state_attrs,
            state_vis,
            state_ident,
            state_generics,
            r#unsafe,
            path_to_core,
            entry_vis,
            entry_ident,
            entry_lifetime,
            graph,
            render_mermaid,
            mermaid_renderer,
        } = self;
        FsmEntry {
            state_attrs,
            state_vis,
            state_ident,
            state_generics,
            r#unsafe,
            path_to_core,
            entry_vis,
            entry_ident,
            entry_lifetime,
            graph,
            render_mermaid,
            mermaid_renderer: f(mermaid_renderer),
        }
    }
    fn nodes(&self) -> impl Iterator<Item = &Ident> {
        self.graph.nodes.keys().map(|NodeId(ident)| ident)
    }
    fn edges(&self) -> impl Iterator<Item = (&Ident, &Ident)> {
        self.graph.edges.keys().map(|(NodeId(f), NodeId(t))| (f, t))
    }
    pub fn dot(&self) -> String {
        let mut s = format!("digraph {}{{\n", self.state_ident);
        for draw in self.draw() {
            match draw {
                Draw::Edge(l, r) => s.write_fmt(format_args!("  {l} -> {r};\n")),
                Draw::Node(it) => s.write_fmt(format_args!("  {it};\n")),
            }
            .unwrap();
        }
        s.push_str("}\n");
        s
    }
    pub fn mermaid(&self) -> String {
        let mut s = String::from("graph LR\n");
        for draw in self.draw() {
            match draw {
                Draw::Edge(l, r) => s.write_fmt(format_args!("  {l} --> {r};\n")),
                Draw::Node(it) => s.write_fmt(format_args!("  {it};\n")),
            }
            .unwrap()
        }
        s
    }
    fn draw(&self) -> impl Iterator<Item = Draw<'_>> {
        let mut nodes = self.nodes().collect::<BTreeSet<_>>();
        let edges = self
            .edges()
            .map(|(l, r)| {
                nodes.remove(l);
                nodes.remove(r);
                Draw::Edge(l, r)
            })
            .collect::<Vec<_>>();
        edges.into_iter().chain(nodes.into_iter().map(Draw::Node))
    }
}
enum Draw<'a> {
    Edge(&'a Ident, &'a Ident),
    Node(&'a Ident),
}

impl<MermaidR: Renderer> ToTokens for FsmEntry<MermaidR> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            state_attrs,
            state_vis,
            state_ident,
            state_generics,
            r#unsafe,
            path_to_core,
            entry_vis,
            entry_ident,
            entry_lifetime,
            graph,
            mermaid_renderer,
            render_mermaid,
        } = self;
        let mut state_variants: Vec<Variant> = vec![];
        let mut entry_variants: Vec<Variant> = vec![];
        let mut entry_structs: Vec<ItemStruct> = vec![];
        let mut match_ctor: Vec<Arm> = vec![];
        let mut as_ref_as_mut: Vec<ItemImpl> = vec![];
        let mut transition: Vec<ItemImpl> = vec![];

        let replace: ModulePath = parse_quote!(#path_to_core::mem::replace);
        let panik: &Expr = &match r#unsafe {
            true => parse_quote!(unsafe { #path_to_core::hint::unreachable_unchecked() }),
            false => {
                parse_quote!(#path_to_core::panic!("entry struct was instantiated with a mismatched state"))
            }
        };

        let entry_generics = {
            let mut it = state_generics.clone();
            it.params.insert(0, parse_quote!(#entry_lifetime));
            it
        };
        let (state_impl_generics, state_type_generics, _) = state_generics.split_for_impl();
        let (entry_impl_generics, entry_type_generics, where_clause) =
            entry_generics.split_for_impl();

        for (node, NodeData { doc, ty }, ref kind) in graph.nodes() {
            state_variants.push(match ty {
                Some(ty) => parse_quote!(#(#doc)* #node(#ty)),
                None => parse_quote!(#(#doc)* #node),
            });
            match_ctor.push(match (ty, kind) {
                (Some(_), Kind::Isolate | Kind::Sink(_)) => {
                    parse_quote!(#state_ident::#node(it) => #entry_ident::#node(it))
                }
                (None, Kind::Isolate | Kind::Sink(_)) => {
                    parse_quote!(#state_ident::#node     => #entry_ident::#node)
                }
                (Some(_), Kind::NonTerminal { .. } | Kind::Source(_)) => {
                    parse_quote!(#state_ident::#node(_)  => #entry_ident::#node(#node(value)))
                }
                (None, Kind::NonTerminal { .. } | Kind::Source(_)) => {
                    parse_quote!(#state_ident::#node     => #entry_ident::#node(#node(value)))
                }
            });
            let reachability = reachability_docs(&node.0, state_ident, kind);
            entry_variants.push(match kind {
                Kind::Isolate | Kind::Sink(_) => match ty {
                    Some(ty) => parse_quote!(#(#reachability)* #node(&#entry_lifetime mut #ty)),
                    None => parse_quote!(#(#reachability)* #node),
                },
                Kind::Source(_) | Kind::NonTerminal { .. } => {
                    parse_quote!(#(#reachability)* #node(#node #entry_type_generics))
                }
            });
            if let Kind::Source(outgoing) | Kind::NonTerminal { outgoing, .. } = kind {
                let outer_doc = format!(" See [`{entry_ident}::{node}`]");
                let field_doc = format!(" MUST match [`{entry_ident}::{node}`]");
                entry_structs.push(parse_quote! {
                    #[doc = #outer_doc]
                    #entry_vis struct #node #entry_type_generics(
                        #[doc = #field_doc]
                        & #entry_lifetime mut #state_ident #state_type_generics
                    )
                    #where_clause;
                });
                for (dst, NodeData { ty: dst_ty, .. }, EdgeData { method_name, doc }) in outgoing {
                    let body = make_body(
                        state_ident,
                        node,
                        ty.as_ref(),
                        dst,
                        dst_ty.as_ref(),
                        method_name,
                        &replace,
                        panik,
                    );
                    let pointer = DocAttr::new(
                        &format!(" Transition to [`{state_ident}::{}`]", dst.0),
                        Span::call_site(),
                    );
                    let pointer = match doc.is_empty() {
                        true => vec![pointer],
                        false => vec![DocAttr::empty(), pointer],
                    };
                    transition.push(parse_quote! {
                        #[allow(clippy::needless_lifetimes)]
                        impl #entry_impl_generics #node #entry_type_generics
                        #where_clause
                        {
                            #(#doc)*
                            #(#pointer)*
                            #body
                        }
                    });
                }

                if let Some(ty) = ty {
                    as_ref_as_mut.extend(make_as_ref_mut(
                        &entry_impl_generics,
                        path_to_core,
                        ty,
                        state_ident,
                        &node.0,
                        &entry_type_generics,
                        where_clause,
                        panik,
                    ));
                }
            }
        }

        let mut entry_attrs: Vec<Attribute> = vec![{
            let doc = format!(" Progress through variants of [`{state_ident}`], created by its [`entry`]({state_ident}::entry) method.");
            parse_quote!(#[doc = #doc])
        }];

        if *render_mermaid {
            if let Some(rendered) = mermaid_renderer.render(&self.mermaid()) {
                if !entry_attrs.is_empty() {
                    entry_attrs.push(parse_quote!(#[doc = ""]));
                }
                entry_attrs.push(parse_quote!(#[doc = #rendered]));
            }
        }

        tokens.extend(quote! {
            #(#state_attrs)*
            #state_vis enum #state_ident #state_generics #where_clause {
                #(#state_variants),*
            }
            #(#entry_attrs)*
            #entry_vis enum #entry_ident #entry_generics #where_clause {
                #(#entry_variants),*
            }
            impl #entry_impl_generics
                #path_to_core::convert::From<& #entry_lifetime mut #state_ident #state_generics>
            for #entry_ident #entry_type_generics
            #where_clause {
                fn from(value: & #entry_lifetime mut #state_ident #state_generics) -> Self {
                    match value {
                        #(#match_ctor),*
                    }
                }
            }
            impl #state_impl_generics #state_ident #state_type_generics
            #where_clause {
                #[allow(clippy::needless_lifetimes)]
                #entry_vis fn entry<#entry_lifetime>(& #entry_lifetime mut self) -> #entry_ident #entry_type_generics {
                    self.into()
                }
            }
            #(#entry_structs)*
            #(#as_ref_as_mut)*
            #(#transition)*
        });
    }
}

impl Parse for FsmEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let Root {
            attrs: mut state_attrs,
            vis: state_vis,
            r#enum: _,
            ident: state_ident,
            generics: state_generics,
            brace: _,
            stmts,
        } = input.parse()?;

        let mut rename_methods = true;
        let mut entry = VisIdent {
            vis: state_vis.clone(),
            ident: Ident::new(&format!("{}Entry", state_ident), Span::call_site()),
        };
        let mut r#unsafe = false;
        let mut path_to_core: ModulePath = parse_quote!(::core);
        let mut render_mermaid = false;
        let mut parser = Parser::new()
            .once("rename_methods", on_value(bool(&mut rename_methods)))
            .once("entry", on_value(parse(&mut entry)))
            .once("unsafe", on_value(bool(&mut r#unsafe)))
            .once("path_to_core", on_value(parse(&mut path_to_core)))
            .once("mermaid", on_value(bool(&mut render_mermaid)));
        parser.extract("fsmentry", &mut state_attrs)?;
        drop(parser);
        let graph = stmts2graph(&stmts, rename_methods)?;
        if graph.edges.is_empty() {
            bail_at!(state_ident.span(), "must define at least one edge `A -> B`");
        }
        let VisIdent {
            vis: entry_vis,
            ident: entry_ident,
        } = entry;

        Ok(Self {
            state_attrs,
            state_vis,
            state_ident,
            state_generics,
            r#unsafe,
            path_to_core,
            entry_vis,
            entry_ident,
            entry_lifetime: parse_quote!('state),
            graph,
            mermaid_renderer: (),
            render_mermaid,
        })
    }
}

fn stmts2graph(
    stmts: &Punctuated<Statement, Token![,]>,
    rename_methods: bool,
) -> syn::Result<Graph> {
    use std::collections::btree_map::Entry::{Occupied, Vacant};

    let mut nodes = BTreeMap::<NodeId, NodeData>::new();
    let mut edges = BTreeMap::<(NodeId, NodeId), EdgeData>::new();

    // Define all the nodes upfront.
    // Note that transition definitions may include types, at any location.
    for Node { name, ty, doc } in stmts.iter().flat_map(|it| match it {
        Statement::Node(it) => Box::new(iter::once(it)) as Box<dyn Iterator<Item = _>>,
        Statement::Transition { first, rest, .. } => {
            Box::new(iter::once(first).chain(rest.iter().map(|(_, it)| it)))
        }
    }) {
        let ty = ty.as_ref().map(|(_, it)| it);
        match nodes.entry(NodeId(name.clone())) {
            Occupied(mut occ) => match (&occ.get().ty, ty) {
                (None, Some(_)) | (Some(_), None) | (None, None) => {
                    append_docs(&mut occ.get_mut().doc, doc)
                }
                // don't compile `syn` with `extra-traits`
                (Some(l), Some(r))
                    if l.to_token_stream().to_string() == r.to_token_stream().to_string() =>
                {
                    append_docs(&mut occ.get_mut().doc, doc)
                }
                (Some(_), Some(_)) => bail_at!(name.span(), "incompatible redefinition"),
            },
            Vacant(v) => {
                v.insert(NodeData {
                    ty: ty.cloned(),
                    doc: doc.clone(),
                });
            }
        };
    }

    for stmt in stmts {
        let Statement::Transition { first, rest } = stmt else {
            continue; // handled above
        };

        let mut from = first.name.clone();

        for (Arrow { doc, kind }, Node { name: to, .. }) in rest {
            match edges.entry((NodeId(from.clone()), NodeId(to.clone()))) {
                Occupied(_) => bail_at!(kind.span(), "duplicate edge definition"),
                Vacant(v) => {
                    v.insert(EdgeData {
                        doc: doc.clone(),
                        method_name: match kind {
                            ArrowKind::Plain(_) => match rename_methods {
                                true => snake_case(to),
                                false => to.clone(),
                            },
                            ArrowKind::Named { ident, .. } => ident.clone(),
                        },
                    });
                }
            }
            from = to.clone();
        }
    }

    Ok(Graph { nodes, edges })
}

fn reachability_docs(node_ident: &Ident, state_ident: &Ident, kind: &Kind<'_>) -> Vec<DocAttr> {
    let span = Span::call_site();
    let mut dst = vec![DocAttr::new(
        &format!(" Represents [`{state_ident}::{node_ident}`]"),
        span,
    )];
    if let Kind::Sink(incoming) | Kind::NonTerminal { incoming, .. } = kind {
        dst.extend([
            DocAttr::empty(),
            DocAttr::new(" This state is reachable from the following:", span),
        ]);
        dst.extend(incoming.iter().map(|(NodeId(other), _, EdgeData { method_name, .. })| {
            let s = format!(" - [`{other}`]({state_ident}::{other}) via [`{method_name}`]({other}::{method_name})");
            DocAttr::new(&s, Span::call_site())
        }));
    }
    if let Kind::Source(outgoing) | Kind::NonTerminal { outgoing, .. } = kind {
        dst.extend([
            DocAttr::empty(),
            DocAttr::new(" This state can transition to the following:", span),
        ]);
        dst.extend(outgoing.iter().map(|(NodeId(other), _, EdgeData { method_name, .. })| {
            let s = format!(" - [`{other}`]({state_ident}::{other}) via [`{method_name}`]({node_ident}::{method_name})");
            DocAttr::new(&s, Span::call_site())
        }));
    }
    dst
}

fn append_docs(dst: &mut Vec<DocAttr>, src: &[DocAttr]) {
    match (dst.is_empty(), src.is_empty()) {
        (true, true) => {}
        (true, false) => dst.extend_from_slice(src),
        (false, true) => {}
        (false, false) => {
            dst.push(DocAttr::empty());
            dst.extend_from_slice(src);
        }
    }
}

fn snake_case(ident: &Ident) -> Ident {
    let ident = ident.to_string();
    let mut snake = String::new();
    for (i, ch) in ident.char_indices() {
        if i > 0 && ch.is_uppercase() {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    match (syn::parse_str(&snake), {
        snake.insert_str(0, "r#");
        syn::parse_str(&snake)
    }) {
        (Ok(it), _) | (_, Ok(it)) => it,
        _ => panic!("bad ident {ident}"),
    }
}

#[allow(clippy::too_many_arguments)]
fn make_body(
    state_ident: &Ident,
    node: &NodeId,
    ty: Option<&Type>,
    dst: &NodeId,
    dst_ty: Option<&Type>,
    method_name: &Ident,
    replace: &ModulePath,
    panik: &Expr,
) -> TokenStream {
    match (ty, dst_ty) {
        (None, None) => quote! {
            pub fn #method_name(self) {
                match #replace(self.0, #state_ident::#dst) {
                    #state_ident::#node => {},
                    _ => #panik,
                }
            }
        },
        (None, Some(dst_ty)) => quote! {
            pub fn #method_name(self, next: #dst_ty) {
                match #replace(self.0, #state_ident::#dst(next)) {
                    #state_ident::#node => {},
                    _ => #panik,
                }
            }
        },
        (Some(ty), None) => quote! {
            pub fn #method_name(self) -> #ty {
                match #replace(self.0, #state_ident::#dst) {
                    #state_ident::#node(it) => it,
                    _ => #panik,
                }
            }
        },
        (Some(ty), Some(dst_ty)) => quote! {
            pub fn #method_name(self, next: #dst_ty) -> #ty {
                match #replace(self.0, #state_ident::#dst(next)) {
                    #state_ident::#node(it) => it,
                    _ => #panik,
                }
            }
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn make_as_ref_mut(
    entry_impl_generics: &ImplGenerics,
    path_to_core: &ModulePath,
    ty: &Type,
    state_ident: &Ident,
    node_ident: &Ident,
    entry_type_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>,
    panik: &Expr,
) -> [ItemImpl; 2] {
    let as_ref = parse_quote! {
        #[allow(clippy::needless_lifetimes)]
        impl #entry_impl_generics #path_to_core::convert::AsRef<#ty> for #node_ident #entry_type_generics
        #where_clause
        {
            fn as_ref(&self) -> &#ty {
                match &self.0 {
                    #state_ident::#node_ident(it) => it,
                    _ => #panik
                }
            }
        }
    };
    let as_mut = parse_quote! {
        #[allow(clippy::needless_lifetimes)]
        impl #entry_impl_generics #path_to_core::convert::AsMut<#ty> for #node_ident #entry_type_generics
        #where_clause
        {
            fn as_mut(&mut self) -> &mut #ty {
                match &mut self.0 {
                    #state_ident::#node_ident(it) => it,
                    _ => #panik
                }
            }
        }
    };
    [as_ref, as_mut]
}
