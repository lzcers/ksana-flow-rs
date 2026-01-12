use std::any::Any;

use crate::AnyNode;

use super::graph::{Context, Edge, Graph};

pub struct GraphBuilder {
    graph: Graph,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    pub fn add_node<N: AnyNode>(mut self, id: &str, node: N) -> Self {
        self.graph.add_node(id, node);
        self
    }

    pub fn add_edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.graph.add_edge(Edge::<()> {
            from: from.into(),
            to: to.into(),
            condition: None,
        });
        self
    }

    pub fn add_condition_edge<Out: Any>(
        mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        condition: impl Fn(&Context, &Out) -> bool + Send + 'static,
    ) -> Self {
        let edge = Edge {
            from: from.into(),
            to: to.into(),
            condition: Some(Box::new(condition)),
        };
        self.graph.add_edge(edge);
        self
    }

    pub fn build(self) -> Graph {
        self.graph
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
