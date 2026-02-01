use crate::Context;

use super::{
    graph::{AnyNode, Edge, Graph},
    io::Output,
};

use serde::de::DeserializeOwned;

pub struct GraphBuilder {
    graph: Graph,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    pub fn add_node<N, F>(mut self, id: &str, creator: F) -> Self
    where
        N: AnyNode,
        F: Fn() -> N + Send + Sync + 'static,
    {
        self.graph.add_node::<N, F>(id, creator);
        self
    }

    pub fn add_edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.graph.add_edge(Edge {
            from: from.into(),
            to: to.into(),
            condition: None,
        });
        self
    }

    pub fn add_condition_edge<Out: DeserializeOwned>(
        mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        condition: impl Fn(&Context, &Out) -> bool + Send + Sync + 'static,
    ) -> Self {
        let condition = Box::new(move |ctx: &Context, output: &Output| {
            if let Some(out) = output.get_as::<Out>() {
                condition(ctx, &out)
            } else {
                false
            }
        });
        let edge = Edge {
            from: from.into(),
            to: to.into(),
            condition: Some(condition),
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
