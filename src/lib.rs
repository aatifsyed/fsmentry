mod dsl;

use heck::{ToSnakeCase as _, ToUpperCamelCase as _};
use itertools::Itertools as _;
use proc_macro2::{Ident, Span};
use quote::quote;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};
use syn::{parse::ParseStream, parse_quote, punctuated::Punctuated, spanned::Spanned as _, Token};
use syn_graphs::dot::{AttrAssign, Attrs, ID};

#[derive(Hash, PartialEq, Eq, Debug)]
struct NodeId {
    inner: Ident,
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
    pub fn transition_ty(&self) -> Ident {
        ident(format!("{}Transition", self.UpperCamelCase()).as_str())
    }
}

fn ident(s: impl AsRef<str>) -> Ident {
    Ident::new(s.as_ref(), Span::call_site())
}

#[derive(Debug, PartialEq)]
struct NodeData {
    ty: Option<syn::Type>,
}

#[derive(Debug)]
pub struct FSMGenerator {
    /// All nodes must be in this map
    nodes: HashMap<NodeId, NodeData>,
    /// Directed L -> R
    edges: HashSet<(NodeId, NodeId)>,
}

impl FSMGenerator {
    fn state_machine_name(&self) -> Ident {
        ident("StateMachine")
    }
    fn state_enum_name(&self) -> Ident {
        ident("State")
    }
    fn entry_enum_name(&self) -> Ident {
        ident("Entry")
    }
    #[allow(unused)] // for documentation
    /// [`None`] if the node is a source
    fn incoming(&self, to: &NodeId) -> Option<Vec<&NodeId>> {
        let vec = self
            .edges
            .iter()
            .filter_map(move |(src, dst)| match dst == to {
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
    fn outgoing<'a>(&'a self, from: &'a NodeId) -> Option<Vec<&NodeId>> {
        let vec = self
            .edges
            .iter()
            .filter_map(move |(src, dst)| match src == from {
                true => Some(dst),
                false => None,
            })
            .collect::<Vec<_>>();
        match vec.is_empty() {
            true => None,
            false => Some(vec),
        }
    }
    pub fn codegen(&self) -> syn::File {
        let state_machine_name = self.state_machine_name();
        let state_enum_name = self.state_enum_name();
        let entry_enum_name = self.entry_enum_name();

        let mut state_variants = Punctuated::<syn::Variant, Token![,]>::new();
        let mut entry_variants = Punctuated::<syn::Variant, Token![,]>::new();
        let mut entry_has_lifetime = false;
        let mut entry_construction = Vec::<syn::Arm>::new();
        let mut transition_tys = Vec::<syn::Ident>::new();
        let mut transition_impls = Vec::<syn::ItemImpl>::new();
        for (node, node_data) in self.nodes.iter() {
            let node_variant_name = node.variant();
            match (&node_data.ty, self.outgoing(node)) {
                (None, None) => {
                    state_variants.push(parse_quote!(#node_variant_name));
                    entry_variants.push(parse_quote!(#node_variant_name));
                    entry_construction.push(parse_quote!(#state_enum_name::#node_variant_name => #entry_enum_name::#node_variant_name,))
                }
                (Some(ty), None) => {
                    state_variants.push(parse_quote!(#node_variant_name(#ty)));
                    entry_has_lifetime = true;
                    entry_variants.push(parse_quote!(#node_variant_name(&'a mut #ty)));
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
                    let transition_ty_name = node.transition_ty();
                    entry_has_lifetime = true;
                    transition_tys.push(transition_ty_name.clone());
                    entry_variants.push(parse_quote!(#node_variant_name(#transition_ty_name<'a>)));
                    entry_construction.push(parse_quote!{
                        #state_enum_name::#node_variant_name{..} => #entry_enum_name::#node_variant_name(#transition_ty_name {
                            inner: &mut self.state,
                        }),
                    });
                    let msg = "this variant is only created when state is known to match, and we hold a mutable reference to state";
                    match node_data_ty {
                        Some(ty) => {
                            state_variants.push(parse_quote!(#node_variant_name(#ty)));
                            transition_impls.push(parse_quote! {
                                impl #transition_ty_name<'_> {
                                    pub fn get(&self) -> & #ty {
                                        match &self.inner {
                                            #state_enum_name::#node_variant_name(data) => data,
                                            _ => ::core::unreachable!(#msg)
                                        }
                                    }
                                    pub fn get_mut(&mut self) -> &mut #ty {
                                        match self.inner {
                                            #state_enum_name::#node_variant_name(data) => data,
                                            _ => ::core::unreachable!(#msg)
                                        }
                                    }
                                }
                            });
                        }
                        None => {
                            state_variants.push(parse_quote!(#node_variant_name));
                        }
                    }
                    for outgoing in outgoing {
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
                                        _ => unreachable!(#msg)
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
                                #body
                            }
                        ));
                    }
                }
            }
        }

        let state_machine: syn::ItemStruct = parse_quote! {
            pub struct #state_machine_name {
                state: #state_enum_name
            }
        };
        let state_machine_entry: syn::ItemImpl = parse_quote! {
            impl #state_machine_name {
                pub fn new(initial: #state_enum_name) -> Self {
                    Self { state: initial }
                }
                pub fn state(&self) -> &#state_enum_name {
                    &self.state
                }
                pub fn state_mut(&mut self) -> &mut #state_enum_name {
                    &mut self.state
                }
                pub fn entry(&mut self) -> #entry_enum_name {
                    match &mut self.state {
                        #(#entry_construction)*
                    }
                }
            }
        };
        let states: syn::ItemEnum = parse_quote! {
            pub enum #state_enum_name {
                #state_variants
            }
        };
        let entry_enum_lifetime_param = match entry_has_lifetime {
            false => None,
            true => Some(quote!(<'a>)),
        };
        let entry: syn::ItemEnum = parse_quote! {
            pub enum #entry_enum_name #entry_enum_lifetime_param {
                #entry_variants
            }
        };
        let transition_tys = transition_tys.into_iter().map(|ident| -> syn::ItemStruct {
            parse_quote!(
                pub struct #ident<'a> {
                    inner: &'a mut #state_enum_name,
                }
            )
        });

        parse_quote! {
            #state_machine
            #state_machine_entry
            #states
            #entry
            #(#transition_tys)*
            #(#transition_impls)*
        }
    }
}

impl FSMGenerator {
    pub fn parse_dot(input: ParseStream) -> syn::Result<Self> {
        macro_rules! bail {
            ($span:expr, $reason:literal) => {
                return Err(syn::Error::new($span, $reason))
            };
        }
        use syn_graphs::dot::{
            EdgeDirectedness, EdgeTarget, Graph, GraphDirectedness, NodeId as DotNodeId, Stmt,
            StmtEdge, StmtList, StmtNode,
        };
        let Graph {
            directedness,
            stmt_list,
            ..
        } = input.parse::<Graph>()?;
        if !input.is_empty() {
            bail!(input.span(), "unexpected trailing input")
        }
        let GraphDirectedness::Digraph(_) = directedness else {
            bail!(directedness.span(), "must be a digraph")
        };

        let mut nodes = HashMap::new();
        let mut edges = HashSet::new();

        process_stmt_list(&mut nodes, &mut edges, stmt_list)?;

        return Ok(Self { nodes, edges });

        fn process_stmt_list(
            all_nodes: &mut HashMap<NodeId, NodeData>,
            all_edges: &mut HashSet<(NodeId, NodeId)>,
            statements: StmtList,
        ) -> syn::Result<()> {
            let StmtList { stmts } = statements;
            for (stmt, _) in stmts {
                let span = stmt.span();
                match stmt {
                    Stmt::Node(StmtNode {
                        node_id:
                            DotNodeId {
                                // TODO(aatifsyed): could support more things here
                                id: ID::AnyIdent(inner),
                                port: _,
                            },
                        attrs,
                    }) => {
                        let ty = extract_ty(attrs)?;
                        all_nodes.insert(NodeId { inner }, NodeData { ty });
                    }
                    Stmt::Node(_) => {
                        bail!(span, "only nodes with bare idents are supported")
                    }
                    Stmt::Attr(_) | Stmt::Assign(_) | Stmt::Subgraph(_) => {
                        bail!(span, "only node and edge statements are supported")
                    }
                    Stmt::Edge(StmtEdge {
                        from,
                        edges,
                        attrs: _,
                    }) => {
                        let mut from = get_ident(from)?;
                        for (directedness, to) in edges {
                            let EdgeDirectedness::Directed { .. } = directedness else {
                                bail!(directedness.span(), "only directed edges are supported")
                            };
                            let to = get_ident(to)?;
                            for it in [&from, &to] {
                                all_nodes
                                    .entry(NodeId { inner: it.clone() })
                                    .or_insert(NodeData { ty: None });
                            }
                            all_edges
                                .insert((NodeId { inner: from }, NodeId { inner: to.clone() }));
                            from = to;
                        }
                    }
                }
            }
            return Ok(());

            fn get_ident(input: EdgeTarget) -> syn::Result<Ident> {
                let span = input.span();
                match input {
                    EdgeTarget::NodeId(DotNodeId {
                        id: ID::AnyIdent(id),
                        port: _,
                    }) => Ok(id),
                    _ => bail!(span, "only bare idents are supported here"),
                }
            }
        }
    }
}

fn extract_ty(attrs: Option<Attrs>) -> syn::Result<Option<syn::Type>> {
    let ty = attrs
        .and_then(|attrs| {
            attrs
                .lists
                .iter()
                .flat_map(|it| it.assigns.iter())
                .filter_map(
                    |AttrAssign {
                         left,
                         eq_token: _,
                         right,
                         trailing: _,
                     }| {
                        match left {
                            ID::AnyIdent(ident) if ident == "type" => Some(right),
                            ID::AnyLit(syn::Lit::Str(lit)) if lit.value() == "type" => Some(right),
                            _ => None,
                        }
                    },
                )
                .at_most_one()
                .map_err(|mut too_many| {
                    let final_straw = too_many.nth(2).unwrap();
                    syn::Error::new_spanned(final_straw, "`type` may only be specified once")
                })
                .and_then(|rhs| match rhs {
                    Some(id) => match id {
                        ID::AnyIdent(ident) => {
                            syn::parse_str::<syn::Type>(&ident.to_string()).map(Some)
                        }
                        ID::AnyLit(syn::Lit::Str(lit)) => syn::parse_str(&lit.value()).map(Some),
                        _ => Err(syn::Error::new_spanned(id, "unsupported type argument")),
                    },
                    None => Ok(None),
                })
                .transpose()
        })
        .transpose()?;
    Ok(ty)
}
