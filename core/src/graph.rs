use core::fmt;
use std::collections::BTreeMap;

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;

use crate::dsl::DocAttr;

#[derive(Hash, PartialEq, Eq, Debug, Clone, PartialOrd, Ord)]
pub(crate) struct NodeId(pub Ident);
impl ToTokens for NodeId {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}
impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub(crate) struct NodeData {
    pub doc: Vec<DocAttr>,
    /// Stored as a single tuple member in the state enum.
    pub ty: Option<syn::Type>,
}
pub(crate) struct EdgeData {
    pub doc: Vec<DocAttr>,
    pub method_name: syn::Ident,
}

// Don't want to take a dependency on petgraph
pub(crate) struct Graph {
    /// All nodes referenced in `edges` are here.
    pub nodes: BTreeMap<NodeId, NodeData>,
    /// Directed L -> R.
    ///
    /// [`EdgeData::method_name`]s MUST be unique.
    pub edges: BTreeMap<(NodeId, NodeId), EdgeData>,
}

impl Graph {
    pub fn outgoing<'a>(&'a self, from: &NodeId) -> Vec<(&'a NodeId, &'a NodeData, &'a EdgeData)> {
        self.edges
            .iter()
            .filter_map(move |((it, to), edge_data)| {
                (it == from).then_some((to, &self.nodes[to], edge_data))
            })
            .collect()
    }
    pub fn incoming<'a>(&'a self, to: &NodeId) -> Vec<(&'a NodeId, &'a NodeData, &'a EdgeData)> {
        self.edges
            .iter()
            .filter_map(move |((from, it), edge_data)| {
                (it == to).then_some((from, &self.nodes[to], edge_data))
            })
            .collect()
    }
    pub fn nodes(&self) -> impl Iterator<Item = (&NodeId, &NodeData, Kind<'_>)> {
        self.nodes.iter().map(|(nid, data)| {
            let incoming = self.incoming(nid);
            let outgoing = self.outgoing(nid);
            (
                nid,
                data,
                match (incoming.is_empty(), outgoing.is_empty()) {
                    (true, true) => Kind::Isolate,
                    (true, false) => Kind::Source(outgoing),
                    (false, true) => Kind::Sink(incoming),
                    (false, false) => Kind::NonTerminal { incoming, outgoing },
                },
            )
        })
    }
}

pub(crate) enum Kind<'a> {
    /// `*`
    Isolate,
    /// `* -> ...`
    Source(Vec<(&'a NodeId, &'a NodeData, &'a EdgeData)>),
    /// `... -> *`
    Sink(Vec<(&'a NodeId, &'a NodeData, &'a EdgeData)>),
    /// `... -> * -> ...`
    NonTerminal {
        incoming: Vec<(&'a NodeId, &'a NodeData, &'a EdgeData)>,
        outgoing: Vec<(&'a NodeId, &'a NodeData, &'a EdgeData)>,
    },
}
