mod dsl;
mod util;

use heck::{ToSnakeCase as _, ToUpperCamelCase as _};
use proc_macro2::{Ident, Span};
use quote::quote;
use std::{collections::HashMap, iter};
use syn::{
    parse::ParseStream, parse_quote, punctuated::Punctuated, spanned::Spanned as _, token, Token,
};
use util::OuterDocString;

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
struct NodeId {
    inner: Ident,
}

impl From<Ident> for NodeId {
    fn from(inner: Ident) -> Self {
        Self { inner }
    }
}

impl NodeId {
    #[allow(non_snake_case)]
    pub fn UpperCamelCase(&self) -> Ident {
        ident(self.inner.to_string().to_upper_camel_case().as_str())
    }
    pub fn snake_case(&self) -> Ident {
        ident(self.inner.to_string().to_snake_case().as_str())
    }
    pub fn transition_fn(&self) -> Ident {
        self.snake_case()
    }
    pub fn variant(&self) -> Ident {
        self.UpperCamelCase()
    }
}

fn ident(s: impl AsRef<str>) -> Ident {
    Ident::new(s.as_ref(), Span::call_site())
}

#[derive(Debug)]
struct NodeData {
    /// Stored as a single tuple member in the state enum.
    ty: Option<syn::Type>,
    /// These are attached to each variant.
    docs: Vec<OuterDocString>,
}

/// Three main types are generated:
/// - The state machine struct.
///   This exposes the entry api, and persists state between transitions.
/// - The state enum.
///   This is the bare state, including any data that is stored in the state.
/// - The entry enum.
///   These contain the transition types, which progress the state machine.
///   Additional types are generated for each transition.
#[derive(Debug)]
pub struct FSMGenerator {
    /// These are passed through to the state enum and the state machine struct.
    attributes: Vec<syn::Attribute>,
    vis: syn::Visibility,
    ident: Ident,
    /// All nodes must be in this map.
    nodes: HashMap<NodeId, NodeData>,
    /// Directed L -> R.
    ///
    /// Documentation is passed through to the transition functions
    edges: HashMap<(NodeId, NodeId), Vec<OuterDocString>>,
}

impl FSMGenerator {
    fn state_machine_name(&self) -> &Ident {
        &self.ident
    }
    fn state_enum_name(&self) -> Ident {
        ident(format!("{}State", self.state_machine_name()))
    }
    fn entry_enum_name(&self) -> Ident {
        ident(format!("{}Entry", self.state_machine_name()))
    }
    fn transition_ty(&self, node_id: &NodeId) -> Ident {
        ident(format!(
            "{}{}Transition",
            node_id.UpperCamelCase(),
            self.state_machine_name()
        ))
    }
    fn getter_names(&self) -> (Ident, Ident) {
        fn names(root: &str) -> impl Iterator<Item = (Ident, Ident)> + '_ {
            (0..).map(move |n| {
                let mut get = String::from(root);
                let mut get_mut = format!("{}_mut", root);
                for _ in 0..n {
                    for s in [&mut get, &mut get_mut] {
                        s.push('_')
                    }
                }

                (ident(get), ident(get_mut))
            })
        }

        for (get, get_mut) in itertools::interleave(names("get"), names("get_data")) {
            if self.nodes.contains_key(&NodeId { inner: get.clone() }) {
                continue;
            }
            if self.nodes.contains_key(&NodeId {
                inner: get_mut.clone(),
            }) {
                continue;
            }
            return (get, get_mut);
        }
        unreachable!()
    }
    /// [`None`] if the node is a source
    fn incoming(&self, to: &NodeId) -> Option<Vec<&NodeId>> {
        let vec = self
            .edges
            .iter()
            .filter_map(move |((src, dst), _)| match dst == to {
                true => Some(src),
                false => None,
            })
            .collect::<Vec<_>>();
        match vec.is_empty() {
            true => None,
            false => Some(vec),
        }
    }
    fn reachability_docs(&self, node: &NodeId) -> Option<Vec<OuterDocString>> {
        let mut docs = vec![];
        let span = Span::call_site();
        if let Some(incoming) = self.incoming(node) {
            docs.push(OuterDocString::new(
                "This node is reachable from the following states:",
                span,
            ));
            for each in incoming {
                docs.push(OuterDocString::new(
                    format!("- [`{}::{}`]", self.state_enum_name(), each.variant()),
                    span,
                ))
            }
        }
        if let Some(outgoing) = self.outgoing(node) {
            if !docs.is_empty() {
                docs.push(OuterDocString::new("", span))
            }
            docs.push(OuterDocString::new(
                "This node can reach the following states:",
                span,
            ));
            for (each, _) in outgoing {
                docs.push(OuterDocString::new(
                    format!("- [`{}::{}`]", self.state_enum_name(), each.variant()),
                    span,
                ))
            }
        }
        match docs.is_empty() {
            true => None,
            false => Some(docs),
        }
    }
    /// [`None`] if the node is a sink
    fn outgoing<'a>(&'a self, from: &'a NodeId) -> Option<Vec<(&NodeId, &[OuterDocString])>> {
        let vec = self
            .edges
            .iter()
            .filter_map(move |((src, dst), docs)| match src == from {
                true => Some((dst, docs.as_slice())),
                false => None,
            })
            .collect::<Vec<_>>();
        match vec.is_empty() {
            true => None,
            false => Some(vec),
        }
    }
    /// Get a basic representation of this graph in dot
    pub fn dot(&self) -> syn_graphs::dot::Graph {
        use syn_graphs::dot::{
            kw, pun, EdgeDirectedness, EdgeTarget, Graph, GraphDirectedness, NodeId as DotNodeId,
            Stmt, StmtEdge, StmtList, StmtNode, ID,
        };
        fn conv_node_id(NodeId { inner }: NodeId) -> DotNodeId {
            DotNodeId {
                id: ID::AnyIdent(inner),
                port: None,
            }
        }

        let span = Span::call_site();
        let mut stmts = vec![];

        for node_id in self.nodes.keys() {
            stmts.push((
                Stmt::Node(StmtNode {
                    node_id: conv_node_id(node_id.clone()),
                    attrs: None,
                }),
                Some(Token![;](span)),
            ))
        }
        for (from, to) in self.edges.keys() {
            stmts.push((
                Stmt::Edge(StmtEdge {
                    from: EdgeTarget::NodeId(conv_node_id(from.clone())),
                    edges: vec![(
                        EdgeDirectedness::Directed(pun::DirectedEdge(span)),
                        EdgeTarget::NodeId(conv_node_id(to.clone())),
                    )],
                    attrs: None,
                }),
                Some(Token![;](span)),
            ))
        }

        Graph {
            strict: Some(kw::strict(span)),
            directedness: GraphDirectedness::Digraph(kw::digraph(span)),
            id: Some(ID::AnyIdent(self.ident.clone())),
            brace_token: token::Brace(span),
            stmt_list: StmtList { stmts },
        }
    }
    pub fn codegen(&self) -> syn::File {
        let vis = &self.vis;
        let state_machine_name = self.state_machine_name();
        let state_enum_name = self.state_enum_name();
        let entry_enum_name = self.entry_enum_name();

        let mut state_variants = Punctuated::<syn::Variant, Token![,]>::new();
        let mut entry_variants = Punctuated::<syn::Variant, Token![,]>::new();
        let mut entry_has_lifetime = false;
        let mut entry_construction = Vec::<syn::Arm>::new();
        let mut transition_tys = Vec::<syn::ItemStruct>::new();
        let mut transition_impls = Vec::<syn::ItemImpl>::new();
        for (
            node,
            NodeData {
                ty: node_ty,
                docs: node_docs,
            },
        ) in self.nodes.iter()
        {
            let node_variant_name = node.variant();
            let mut node_docs = node_docs.clone();
            if let Some(reachability_docs) = self.reachability_docs(node) {
                if !node_docs.is_empty() {
                    node_docs.push(OuterDocString::new("", Span::call_site()))
                }
                node_docs.extend(reachability_docs)
            }

            match (node_ty, self.outgoing(node)) {
                (None, None) => {
                    // This node has no data, and no transitions, so the entry and state enums are bare
                    state_variants.push(parse_quote!(#(#node_docs)* #node_variant_name));
                    entry_variants.push(parse_quote!(#(#node_docs)* #node_variant_name));
                    entry_construction.push(parse_quote!(#state_enum_name::#node_variant_name => #entry_enum_name::#node_variant_name,))
                }
                (Some(ty), None) => {
                    // This node has data, but no transitions, so the entry and state enums just contain a reference to the data
                    state_variants.push(parse_quote!(#(#node_docs)* #node_variant_name(#ty)));
                    entry_has_lifetime = true;
                    entry_variants
                        .push(parse_quote!(#(#node_docs)* #node_variant_name(&'a mut #ty)));
                    entry_construction.push(parse_quote!{
                        #state_enum_name::#node_variant_name(_) => {
                            // need to reborrow to get the data
                            match &mut self.state {
                                #state_enum_name::#node_variant_name(data) => #entry_enum_name::#node_variant_name(data),
                                _ => ::core::unreachable!("state cannot change underneath us while we hold a mutable reference")
                            }
                        }
                    });
                }
                (node_data_ty, Some(outgoing)) => {
                    // this node has transitions, so create a transition type
                    let transition_ty_name = self.transition_ty(node);
                    entry_has_lifetime = true;
                    transition_tys.push(parse_quote!(
                        #(#node_docs)*
                        #vis struct #transition_ty_name<'a> {
                            inner: &'a mut #state_enum_name,
                        }
                    ));
                    entry_variants.push(
                        parse_quote!(#(#node_docs)* #node_variant_name(#transition_ty_name<'a>)),
                    );
                    entry_construction.push(parse_quote!{
                        #state_enum_name::#node_variant_name{..} => #entry_enum_name::#node_variant_name(#transition_ty_name {
                            inner: &mut self.state,
                        }),
                    });
                    let msg = "this variant is only created when state is known to match, and we hold a mutable reference to state";
                    match node_data_ty {
                        Some(ty) => {
                            // this node has data, so store it in the state enum, and add getters for the transition type
                            state_variants
                                .push(parse_quote!(#(#node_docs)* #node_variant_name(#ty)));
                            let (get, get_mut) = self.getter_names();
                            transition_impls.push(parse_quote! {
                                impl #transition_ty_name<'_> {
                                    /// Get a reference to the data stored in this state
                                    pub fn #get(&self) -> & #ty {
                                        match &self.inner {
                                            #state_enum_name::#node_variant_name(data) => data,
                                            _ => ::core::unreachable!(#msg)
                                        }
                                    }
                                    /// Get a mutable reference to the data stored in this state
                                    pub fn #get_mut(&mut self) -> &mut #ty {
                                        match self.inner {
                                            #state_enum_name::#node_variant_name(data) => data,
                                            _ => ::core::unreachable!(#msg)
                                        }
                                    }
                                }
                            });
                        }
                        None => {
                            state_variants.push(parse_quote!(#(#node_docs)* #node_variant_name));
                        }
                    }
                    for (outgoing, transition_docs) in outgoing {
                        let transition_fn_name = outgoing.transition_fn();
                        let outgoing_variant_name = outgoing.variant();
                        let body: syn::ImplItemFn = match (node_data_ty, &self.nodes[outgoing].ty) {
                            // no data -> no data
                            (None, None) => parse_quote! {
                                pub fn #transition_fn_name(self) {
                                    let prev =
                                    ::core::mem::replace(self.inner, #state_enum_name::#outgoing_variant_name);
                                    ::core::debug_assert!(::core::matches!(prev, #state_enum_name::#node_variant_name));
                                }
                            },
                            // no data -> data
                            (None, Some(out)) => parse_quote! {
                                pub fn #transition_fn_name(self, next: #out) {
                                    let prev =
                                    ::core::mem::replace(self.inner, #state_enum_name::#outgoing_variant_name(next));
                                    ::core::debug_assert!(::core::matches!(prev, #state_enum_name::#node_variant_name));
                                }
                            },
                            // data -> no data
                            (Some(input), None) => parse_quote! {
                                pub fn #transition_fn_name(self) -> #input {
                                    let prev =
                                    ::core::mem::replace(self.inner, #state_enum_name::#outgoing_variant_name);
                                    match prev {
                                        #state_enum_name::#node_variant_name(data) => data,
                                        _ => ::core::unreachable!(#msg)
                                    }
                                }
                            },
                            // data -> data
                            (Some(input), Some(out)) => parse_quote! {
                                pub fn #transition_fn_name(self, next: #out) -> #input {
                                    let prev =
                                    ::core::mem::replace(self.inner, #state_enum_name::#outgoing_variant_name(next));
                                    match prev {
                                        #state_enum_name::#node_variant_name(data) => data,
                                        _ => ::core::unreachable!(#msg)
                                    }
                                }
                            },
                        };
                        transition_impls.push(parse_quote!(
                            impl #transition_ty_name<'_> {
                                #(#transition_docs)*
                                #body
                            }
                        ));
                    }
                }
            }
        }

        let attrs = &self.attributes;
        let state_machine_struct: syn::ItemStruct = parse_quote! {
            #(#attrs)*
            #vis struct #state_machine_name {
                state: #state_enum_name
            }
        };
        let state_machine_methods: syn::ItemImpl = parse_quote! {
            impl #state_machine_name {
                /// Create a new state machine
                pub fn new(initial: #state_enum_name) -> Self {
                    Self { state: initial }
                }
                /// Get a reference to the current state of the state machine
                pub fn state(&self) -> &#state_enum_name {
                    &self.state
                }
                /// Get a mutable reference to the current state of the state machine
                pub fn state_mut(&mut self) -> &mut #state_enum_name {
                    &mut self.state
                }
                /// Transition the state machine
                #[must_use = "The state must be inspected and transitioned through the returned enum"]
                pub fn entry(&mut self) -> #entry_enum_name {
                    match &mut self.state {
                        #(#entry_construction)*
                    }
                }
            }
        };
        let attrs = &self.attributes;
        let state_enum: syn::ItemEnum = parse_quote! {
            #(#attrs)*
            #vis enum #state_enum_name {
                #state_variants
            }
        };
        let entry_enum_lifetime_param = match entry_has_lifetime {
            false => None,
            true => Some(quote!(<'a>)),
        };
        let comment = format!("Created from [`{}::entry`].", state_machine_name);
        let entry_enum: syn::ItemEnum = parse_quote! {
            /// Access to the current state with valid transitions for the state machine.
            ///
            #[doc = #comment]
            #vis enum #entry_enum_name #entry_enum_lifetime_param {
                #entry_variants
            }
        };
        transition_impls.extend(transition_tys.iter().map(|strukt| {
            let ident = &strukt.ident;
            parse_quote! {
                impl ::core::fmt::Debug for #ident<'_> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        f.debug_struct(::core::stringify!(#ident)).finish_non_exhaustive()
                    }
                }
            }
        }));

        parse_quote! {
            #state_machine_struct
            #state_machine_methods
            #state_enum
            #entry_enum
            #(#transition_tys)*
            #(#transition_impls)*
        }
    }
}

macro_rules! bail_at {
    ($span:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        return Err(syn::Error::new($span, format!($fmt, $($arg,)*)))
    };
}

impl FSMGenerator {
    pub fn parse_dsl(input: ParseStream) -> syn::Result<Self> {
        use dsl::{DocumentedArrow, Dsl, Edge, Stmt, StmtEdges, StmtNode};
        use std::{
            cmp::Ordering::{Equal, Greater, Less},
            collections::hash_map::Entry::{Occupied, Vacant},
        };

        let Dsl {
            attrs,
            vis,
            name,
            brace_token: _,
            mut stmts,
        } = input.parse()?;

        let mut nodes = HashMap::new();
        let mut edges = HashMap::new();

        // Nodes first, so Node should be less than Edge
        stmts.sort_unstable_by(|left, right| match (left, right) {
            (Stmt::Edges(_), Stmt::Edges(_)) => Equal,
            (Stmt::Edges(_), Stmt::Node(_)) => Greater,
            (Stmt::Node(_), Stmt::Edges(_)) => Less,
            (Stmt::Node(_), Stmt::Node(_)) => Equal,
        });

        for stmt in stmts {
            match stmt {
                Stmt::Node(StmtNode {
                    attrs,
                    ident,
                    colon: _,
                    ty,
                    semi: _,
                }) => {
                    let span = ident.span();
                    match nodes.entry(ident.into()) {
                        Occupied(_) => bail_at!(span, "duplicate node definition"),
                        Vacant(v) => v.insert(NodeData { ty, docs: attrs }),
                    };
                }
                Stmt::Edges(StmtEdges {
                    attrs,
                    mut from,
                    edge,
                    to,
                    rest,
                    semi: _,
                }) => {
                    for ident in iter::once(&from)
                        .chain([&to])
                        .chain(rest.iter().map(|(_edge, ident)| ident))
                    {
                        nodes.entry(ident.clone().into()).or_insert(NodeData {
                            ty: None,
                            docs: vec![],
                        });
                    }
                    for (edge, to) in iter::once((edge, to)).chain(rest) {
                        match edges.entry((from.clone().into(), to.clone().into())) {
                            Occupied(_) => bail_at!(edge.span(), "duplicate edge definition"),
                            Vacant(v) => {
                                let mut attrs = attrs.clone();
                                if let Edge::Documented(DocumentedArrow { doc, .. }) = edge {
                                    if !attrs.is_empty() {
                                        // newline
                                        attrs.push(OuterDocString::new("", doc.span()))
                                    }
                                    attrs.push(OuterDocString::new(doc.value(), doc.span()))
                                }
                                v.insert(attrs);
                            }
                        }
                        from = to;
                    }
                }
            }
        }

        Ok(Self {
            attributes: attrs,
            vis,
            ident: name,
            nodes,
            edges,
        })
    }
}
