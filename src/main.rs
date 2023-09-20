use std::collections::{HashMap, HashSet};

use heck::{ToSnakeCase as _, ToUpperCamelCase as _};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    parse::{ParseStream, Parser as _},
    spanned::Spanned,
    File,
};

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
    pub fn data(&self) -> Ident {
        ident(format!("{}Data", self.UpperCamelCase()).as_str())
    }
    pub fn transition_ty(&self) -> Ident {
        ident(format!("{}Transition", self.UpperCamelCase()).as_str())
    }
}

fn ident(s: impl AsRef<str>) -> Ident {
    Ident::new(s.as_ref(), Span::call_site())
}

#[derive(Debug)]
struct NodeData {}

#[derive(Debug)]
struct FSMGenerator {
    /// All nodes must be in this map
    nodes: HashMap<NodeId, Option<NodeData>>,
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
    fn data_accessor_name(&self) -> Ident {
        ident("data")
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
    fn codegen(&self) -> TokenStream {
        let state_machine_name = self.state_machine_name();
        let state_enum_name = self.state_enum_name();
        let entry_enum_name = self.entry_enum_name();
        let data_accessor_name = self.data_accessor_name();

        let mut entry_mut_ctor = TokenStream::new();
        let mut entry_ref_ctor = TokenStream::new();
        let mut entry_empty_ctor = TokenStream::new();
        let mut state_enum_variants = TokenStream::new();
        let mut data_structs = TokenStream::new();
        // structs and impls
        let mut transitions = TokenStream::new();
        let mut entry_enum_variants = TokenStream::new();

        for (node, node_data) in self.nodes.iter() {
            let node_variant_name = node.variant();
            let node_data_name = node.data();
            let node_transition_ty = node.transition_ty();
            let outgoing = self.outgoing(node);
            match node_data {
                Some(NodeData {}) => {
                    // TODO(aatifsyed): actually put something here
                    data_structs.extend(quote!(pub struct #node_data_name {}));
                    state_enum_variants.extend(quote!(#node_variant_name(#node_data_name),))
                }
                None => state_enum_variants.extend(quote!(#node_variant_name,)),
            }
            match (node_data, outgoing) {
                (Some(_), None) => {
                    entry_enum_variants.extend(quote!(#node_variant_name(&'a #node_data_name),));
                    entry_ref_ctor.extend(quote!{
                        if let #state_enum_name::#node_variant_name(data) = self.inner.as_ref().unwrap() {
                            return #entry_enum_name::#node_variant_name(data);
                        }
                    });
                }
                (None, None) => {
                    entry_enum_variants.extend(quote!(#node_variant_name,));
                    entry_empty_ctor.extend(quote! {
                        if let #state_enum_name::#node_variant_name = self.inner.as_ref().unwrap() {
                            return #entry_enum_name::#node_variant_name;
                        }
                    })
                }
                (node_data, Some(outgoing)) => {
                    entry_enum_variants
                        .extend(quote!(#node_variant_name(#node_transition_ty<'a>),));

                    transitions.extend(quote! {
                        pub struct #node_transition_ty<'a> {
                            inner: &'a mut Option<State>,
                        }
                    });

                    match node_data {
                        Some(NodeData {}) => {
                            transitions.extend(quote!{
                                impl #node_transition_ty<'_> {
                                    pub fn #data_accessor_name(&mut self) -> &#node_data_name {
                                        let Some(#state_enum_name::#node_variant_name(data)) = self.inner else {
                                            unreachable!()
                                        };
                                        data
                                    }
                                }
                            });
                            entry_mut_ctor.extend(quote! {
                                if let #state_enum_name::#node_variant_name(_) = self.inner.as_ref().unwrap() {
                                    return #entry_enum_name::#node_variant_name(#node_transition_ty {
                                        inner: &mut self.inner,
                                    });
                                }
                            });
                        }
                        None => {
                            entry_mut_ctor.extend(quote!{
                                if let #state_enum_name::#node_variant_name = self.inner.as_ref().unwrap() {
                                    return #entry_enum_name::#node_variant_name(#node_transition_ty {
                                        inner: &mut self.inner,
                                    });
                                }
                            })
                        },
                    }

                    for outgoing in outgoing {
                        let transition_fn_name = outgoing.transition_fn();
                        let outgoing_variant = outgoing.variant();
                        let outgoing_data_name = outgoing.data();
                        match (&self.nodes[node], &self.nodes[outgoing]) {
                            (None, None) => {
                                transitions.extend(quote!{
                                    impl #node_transition_ty<'_> {
                                        pub fn #transition_fn_name(self) {
                                            assert!(matches!(self.inner, Some(State::#node_variant_name)));
                                            *self.inner = Some(#state_enum_name::#outgoing_variant);
                                        }
                                    }
                                })
                            }
                            (None, Some(NodeData {})) => {
                                transitions.extend(quote!{
                                    impl #node_transition_ty<'_> {
                                        pub fn #transition_fn_name(self, next: #outgoing_data_name) {
                                            assert!(matches!(self.inner, Some(State::#node_variant_name)));
                                            *self.inner = Some(#state_enum_name::#outgoing_variant(next));
                                        }
                                    }
                                })
                            },
                            (Some(NodeData{}), None) => transitions.extend(quote!{
                                impl #node_transition_ty<'_> {
                                    pub fn #transition_fn_name(self) -> #node_data_name {
                                        let Some(#state_enum_name::#node_variant_name(prev)) = self.inner.take() else {
                                            unreachable!()
                                        };
                                        *self.inner = Some(#state_enum_name::#outgoing_variant);
                                        prev
                                    }
                                }
                            }),
                            (Some(NodeData{}), Some(NodeData{})) => transitions.extend(quote!{
                                impl #node_transition_ty<'_> {
                                    pub fn #transition_fn_name(self, next: #outgoing_data_name) -> #node_data_name {
                                        let Some(#state_enum_name::#node_variant_name(prev)) = self.inner.take() else {
                                            unreachable!()
                                        };
                                        *self.inner = Some(#state_enum_name::#outgoing_variant(next));
                                        prev
                                    }
                                }
                            }),
                        }
                    }
                }
            }
        }

        let entry_enum_lifetime_param = match entry_mut_ctor.is_empty() && entry_ref_ctor.is_empty()
        {
            true => None,
            false => Some(quote!(<'a>)),
        };

        quote! {
            pub struct #state_machine_name {
                /// Must always be [`Some`] when observable by a user
                inner: Option<#state_enum_name>,
            }

            impl #state_machine_name {
                pub fn new(initial: #state_enum_name) -> Self {
                    Self {
                        inner: Some(initial)
                    }
                }
                pub fn state(&self) -> &#state_enum_name {
                    self.inner.as_ref().unwrap()
                }

                pub fn entry(&mut self) -> #entry_enum_name {
                    // mut - must go first for borrow-checking
                    #entry_mut_ctor
                    #entry_ref_ctor
                    #entry_empty_ctor
                    unreachable!()
                }
            }

            pub enum State {
                #state_enum_variants
            }

            #data_structs

            #transitions

            pub enum #entry_enum_name #entry_enum_lifetime_param {
                #entry_enum_variants
            }
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
            Directedness, EdgeOp, Graph, NodeId as DotNodeId, NodeIdOrSubgraph, Statements, Stmt,
            StmtEdge, StmtNode, ID,
        };
        let Graph {
            direction,
            statements,
            ..
        } = input.parse::<Graph>()?;
        if !input.is_empty() {
            bail!(input.span(), "unexpected trailing input")
        }
        let Directedness::Digraph(_) = direction else {
            bail!(direction.span(), "must be a digraph")
        };

        let mut nodes = HashMap::new();
        let mut edges = HashSet::new();

        process_statements(&mut nodes, &mut edges, statements)?;

        return Ok(Self { nodes, edges });

        fn process_statements(
            nodes: &mut HashMap<NodeId, Option<NodeData>>,
            edges: &mut HashSet<(NodeId, NodeId)>,
            statements: Statements,
        ) -> syn::Result<()> {
            let Statements { list } = statements;
            for (statement, _) in list {
                let span = statement.span();
                match statement {
                    Stmt::Node(StmtNode {
                        node_id:
                            DotNodeId {
                                // TODO(aatifsyed): could support more things here
                                id: ID::AnyIdent(inner),
                                port: _,
                            },
                        attributes: _,
                    }) => {
                        nodes.insert(NodeId { inner }, None);
                    }
                    Stmt::Node(_) => {
                        bail!(span, "only nodes with bare idents are supported")
                    }
                    Stmt::Attr(_) | Stmt::Assign(_) | Stmt::Subgraph(_) => {
                        bail!(span, "only node and edge statements are supported")
                    }
                    Stmt::Edge(StmtEdge {
                        from,
                        ops,
                        attrs: _,
                    }) => {
                        let mut from = get_ident(from)?;
                        for (op, to) in ops {
                            let EdgeOp::Directed { .. } = op else {
                                bail!(op.span(), "only directed edges are supported")
                            };
                            let to = get_ident(to)?;
                            // TODO(aatifsyed): clobbering could happen here
                            nodes.insert(
                                NodeId {
                                    inner: from.clone(),
                                },
                                None,
                            );
                            nodes.insert(NodeId { inner: to.clone() }, None);
                            edges.insert((NodeId { inner: from }, NodeId { inner: to.clone() }));
                            from = to;
                        }
                    }
                }
            }
            return Ok(());

            fn get_ident(input: NodeIdOrSubgraph) -> syn::Result<Ident> {
                let span = input.span();
                match input {
                    NodeIdOrSubgraph::NodeId(DotNodeId {
                        id: ID::AnyIdent(id),
                        port: _,
                    }) => Ok(id),
                    _ => bail!(span, "only bare idents are supported here"),
                }
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let input = std::fs::read_to_string("/dev/stdin")?;
    let output = syn::parse2::<File>(FSMGenerator::parse_dot.parse_str(&input)?.codegen())?;
    println!("{}", prettyplease::unparse(&output));
    Ok(())
}
