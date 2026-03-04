use serde::Serialize;

use crate::engine::ids::NodeId;
use crate::engine::node::EdgeSpec;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Graph {
    pub edges: Vec<EdgeSpec>,
}

impl Graph {
    pub fn new(mut edges: Vec<EdgeSpec>) -> Self {
        edges.sort_by_key(|edge| (edge.from.as_index(), edge.to.as_index()));
        Self { edges }
    }

    pub fn incoming(&self, target: NodeId) -> impl Iterator<Item = &EdgeSpec> {
        self.edges.iter().filter(move |edge| edge.to == target)
    }
}
