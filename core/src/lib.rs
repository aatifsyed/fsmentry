/// A code generator for state machines.
///
/// See the `fsmentry` crate for more documentation.
mod dsl;
mod util;

use heck::{ToSnakeCase as _, ToUpperCamelCase as _};
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
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
    pub fn transition_fn(&self) -> Ident {
        self.inner.snake_case()
    }
    pub fn variant(&self) -> Ident {
        self.inner.UpperCamelCase()
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

/// Core code generator for state machines.
///
/// The generator is created with a graph definition in either:
/// - [The `DOT` graph description language](https://en.wikipedia.org/wiki/DOT_%28graph_description_language%29).
///   See [`Self::parse_dot`].
/// - A domain specific language.
///   See [`Self::parse_dsl`].
///
/// [`Self::codegen`] performs the actual generation.
#[derive(Debug)]
pub struct FSMGenerator {
    /// All are passed through to the state enum and the state machine struct.
    ///
    /// `#[doc]` attributes are passed through to the module
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
    /// Generates a state machine.
    ///
    /// The basic layout of the generated code is as follows:
    ///
    /// ```rust,ignore
    /// pub mod <name> {
    ///     // The actual state machine
    ///     pub struct <name> { .. }
    ///     // The possible states, including inner data
    ///     pub enum State { .. }
    ///     // The entry api, which gives you handles to transition the machine
    ///     pub enum Entry { .. }
    ///
    ///     // additional structs are generated to perform the actual state transitions
    /// }
    /// ```
    ///
    /// See the [module documentation](mod@self) for more.
    pub fn codegen(&self) -> syn::File {
        let state_machine_name = self.ident.UpperCamelCase();
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
                        pub struct #transition_ty_name<'a> {
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
            pub struct #state_machine_name {
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
            pub enum #state_enum_name {
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
            pub enum #entry_enum_name #entry_enum_lifetime_param {
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

        let vis = &self.vis;
        let module_name = self.ident.snake_case();
        let attrs = self
            .attributes
            .iter()
            .filter(|it| it.path().is_ident("doc"));

        parse_quote! {
            #(#attrs)*
            #vis mod #module_name {
                #state_machine_struct
                #state_machine_methods
                #state_enum
                #entry_enum
                #(#transition_tys)*
                #(#transition_impls)*
            }
        }
    }
    /// Get a basic representation of this graph in dot, suitable for documenting the state machine.
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
    fn state_enum_name(&self) -> Ident {
        ident("State")
    }
    fn entry_enum_name(&self) -> Ident {
        ident("Entry")
    }
    fn transition_ty(&self, node_id: &NodeId) -> Ident {
        ident(format!("{}", node_id.inner.UpperCamelCase()))
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
}

macro_rules! bail_at {
    ($span:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        return Err(syn::Error::new($span, format!($fmt, $($arg,)*)))
    };
}

impl FSMGenerator {
    #[doc = include_str!("../../common-docs/dsl.md")]
    pub fn parse_dsl(input: ParseStream) -> syn::Result<Self> {
        Self::try_from_dsl(input.parse()?)
    }

    #[doc = include_str!("../../common-docs/dot.md")]
    // Transpiles DOT to the DSL, and then calls [`Self::try_from_dsl`]
    pub fn parse_dot(input: ParseStream) -> syn::Result<Self> {
        use dsl::{
            pun, Edge as DslEdge, Stmt as DslStmt, StmtEdges as DslStmtEdges,
            StmtNode as DslStmtNode,
        };
        use syn_graphs::dot::{
            EdgeDirectedness, EdgeTarget, Graph, GraphDirectedness, NodeId as DotNodeId,
            Stmt as DotStmt, StmtEdge as DotStmtEdge, StmtNode as DotStmtNode, ID,
        };
        let Graph {
            strict: _,
            directedness,
            id,
            brace_token,
            stmt_list,
        } = input.parse::<Graph>()?;
        let GraphDirectedness::Digraph(_) = directedness else {
            bail_at!(directedness.span(), "must be `digraph`")
        };
        let Some(ID::AnyIdent(id)) = id else {
            bail_at!(directedness.span(), "graph must be named")
        };
        let mut stmts = vec![];
        let span = Span::call_site();
        for (stmt, _) in stmt_list.stmts {
            match stmt {
                DotStmt::Node(DotStmtNode {
                    node_id: DotNodeId { id, port },
                    attrs,
                }) => {
                    if let Some(attrs) = attrs {
                        bail_at!(attrs.span(), "attrs are not supported")
                    }
                    if let Some(port) = port {
                        bail_at!(port.span(), "ports are not supported")
                    }
                    let ID::AnyIdent(id) = id else {
                        bail_at!(id.span(), "unsupported id")
                    };
                    stmts.push(DslStmt::Node(DslStmtNode {
                        attrs: vec![],
                        ident: syn::parse2(id.into_token_stream())?,
                        colon: None,
                        ty: None,
                        semi: Token![;](span),
                    }))
                }
                DotStmt::Edge(DotStmtEdge { from, edges, attrs }) => {
                    if let Some(attrs) = attrs {
                        bail_at!(attrs.span(), "attrs are not supported")
                    };
                    let mut rest = edges
                        .into_iter()
                        .map(|(dir, to)| {
                            let EdgeDirectedness::Directed(_) = dir else {
                                bail_at!(dir.span(), "edge must be directed")
                            };
                            Ok((
                                DslEdge::Short(pun::ShortArrow(span)),
                                edge_target_to_ident(to)?,
                            ))
                        })
                        .collect::<syn::Result<Vec<_>>>()?;

                    let (edge, to) = rest.remove(0);

                    stmts.push(DslStmt::Edges(DslStmtEdges {
                        attrs: vec![],
                        from: edge_target_to_ident(from)?,
                        edge,
                        to,
                        rest,
                        semi: Token![;](span),
                    }))
                }
                it @ (DotStmt::Attr(_) | DotStmt::Assign(_) | DotStmt::Subgraph(_)) => {
                    bail_at!(it.span(), "unsupported statement")
                }
            }
        }
        return Self::try_from_dsl(crate::dsl::Dsl {
            attrs: vec![],
            vis: parse_quote!(pub),
            name: syn::parse2(id.into_token_stream())?,
            brace_token,
            stmts,
        });

        fn edge_target_to_ident(edge_target: EdgeTarget) -> syn::Result<Ident> {
            match edge_target {
                EdgeTarget::Subgraph(_) => {
                    bail_at!(edge_target.span(), "subgraphs are not supported")
                }
                EdgeTarget::NodeId(DotNodeId { id, port }) => {
                    if let Some(port) = port {
                        bail_at!(port.span(), "ports are not supported")
                    }
                    let ID::AnyIdent(id) = id else {
                        bail_at!(id.span(), "only idents are allowed here")
                    };
                    syn::parse2(id.into_token_stream())
                }
            }
        }
    }

    fn try_from_dsl(dsl: crate::dsl::Dsl) -> syn::Result<Self> {
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
        } = dsl;

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

trait IdentExt {
    fn get_ident(&self) -> &Ident;
    #[allow(non_snake_case)]
    fn UpperCamelCase(&self) -> Ident {
        Ident::new(
            &self.get_ident().to_string().to_upper_camel_case(),
            self.get_ident().span(),
        )
    }
    fn snake_case(&self) -> Ident {
        Ident::new(
            &self.get_ident().to_string().to_snake_case(),
            self.get_ident().span(),
        )
    }
}

impl IdentExt for Ident {
    fn get_ident(&self) -> &Ident {
        self
    }
}
