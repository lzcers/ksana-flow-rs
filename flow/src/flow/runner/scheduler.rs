use super::exec_context::{ExecutionContext, NodeState};
use crate::{
    Context,
    flow::graph::{AnyNode, Graph, Input, NodeId, Output, TriggerStrategy},
};
use serde_json::Value;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::RwLock;

type TaskPayload = (Vec<NodeId>, Input);

#[derive(Clone)]
pub struct StartSpec {
    pub node_id: NodeId,
    pub inputs: Input,
}

pub struct Scheduler {
    graph: Arc<Graph>,
    task_queue: VecDeque<TaskPayload>,
    runtime_nodes: HashMap<NodeId, Arc<RwLock<dyn AnyNode>>>,
    node_trigger_strategy: HashMap<NodeId, TriggerStrategy>,
}

impl Scheduler {
    pub fn new(graph: Arc<Graph>) -> Self {
        Self {
            graph,
            task_queue: VecDeque::new(),
            runtime_nodes: HashMap::new(),
            node_trigger_strategy: HashMap::new(),
        }
    }

    pub fn get_node_ids(&self) -> Vec<String> {
        self.graph.get_node_ids()
    }

    pub fn is_start_queue_empty(&self) -> bool {
        self.task_queue.is_empty()
    }

    pub fn enqueue_start(&mut self, node_id: &str, inputs: Input) {
        self.task_queue
            .push_back((vec![node_id.to_owned()], inputs));
    }

    pub fn pop_initial_starts(&mut self) -> Vec<StartSpec> {
        let mut starts = Vec::new();
        while let Some((node_ids, inputs)) = self.task_queue.pop_front() {
            for node_id in node_ids {
                starts.push(StartSpec {
                    node_id,
                    inputs: inputs.clone(),
                });
            }
        }
        starts
    }

    pub fn get_node(&self, node_id: &str) -> Option<Arc<RwLock<dyn AnyNode>>> {
        self.runtime_nodes.get(node_id).cloned()
    }

    pub async fn materialize_nodes(&mut self) -> Result<(), String> {
        self.runtime_nodes.clear();
        self.node_trigger_strategy.clear();

        for (node_id, factory) in self.graph.nodes.iter() {
            let node =
                factory().map_err(|e| format!("Failed to create node '{}': {}", node_id, e))?;
            let trigger_strategy = {
                let guard = node.read().await;
                guard.get_trigger_strategy()
            };
            self.node_trigger_strategy
                .insert(node_id.clone(), trigger_strategy);
            self.runtime_nodes.insert(node_id.clone(), node);
        }
        Ok(())
    }

    pub fn schedule_from_output(
        &self,
        from_node_id: &str,
        output: &Value,
        exec_ctx: &ExecutionContext,
        runtime_ctx: &Context,
    ) -> Result<Vec<StartSpec>, String> {
        let next_nodes = self.find_next_nodes(from_node_id, output, runtime_ctx)?;
        let mut starts = Vec::new();
        for next_node_id in next_nodes {
            if let Some(spec) = self.check_and_build_start(next_node_id, from_node_id, exec_ctx) {
                starts.push(spec);
            }
        }
        Ok(starts)
    }

    fn should_schedule(&self, node_id: &str, exec_ctx: &ExecutionContext) -> bool {
        match exec_ctx.get_state(node_id) {
            Some(NodeState::Running) | Some(NodeState::Completed) => false,
            _ => true,
        }
    }

    fn get_trigger_strategy(&self, node_id: &str) -> TriggerStrategy {
        self.node_trigger_strategy
            .get(node_id)
            .cloned()
            .unwrap_or_default()
    }

    fn find_next_nodes(
        &self,
        from_node_id: &str,
        output: &Value,
        runtime_ctx: &Context,
    ) -> Result<Vec<String>, String> {
        let mut next_nodes = vec![];
        let output = Output::new(Some(output.clone()));
        if let Some(edges) = self.graph.edges.get(from_node_id) {
            for edge in edges.iter() {
                if edge.check_condition(runtime_ctx, &output) {
                    next_nodes.push(edge.to().to_owned())
                }
            }
        }
        Ok(next_nodes)
    }

    fn check_and_build_start(
        &self,
        node_id: String,
        from_node_id: &str,
        exec_ctx: &ExecutionContext,
    ) -> Option<StartSpec> {
        if !self.should_schedule(&node_id, exec_ctx) {
            return None;
        }

        match self.get_trigger_strategy(&node_id) {
            TriggerStrategy::AnyUpstreamAvailable => {
                let mut inputs_map = HashMap::new();
                if let Some(output) = exec_ctx.get_output(from_node_id) {
                    inputs_map.insert(from_node_id.to_owned(), output);
                }
                Some(StartSpec {
                    node_id,
                    inputs: Input::new(inputs_map),
                })
            }
            TriggerStrategy::AllUpstreamReady => {
                let mut inputs_map = HashMap::new();
                let parents = self.graph.get_parents(&node_id);
                for parent_id in parents {
                    let state = exec_ctx.get_state(&parent_id);
                    match state {
                        Some(NodeState::Completed) => {
                            if let Some(output) = exec_ctx.get_output(&parent_id) {
                                inputs_map.insert(parent_id.clone(), output);
                            }
                        }
                        Some(NodeState::Skipped) => {}
                        _ => {
                            return None;
                        }
                    }
                }
                Some(StartSpec {
                    node_id,
                    inputs: Input::new(inputs_map),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::graph::Edge;
    use serde_json::json;

    #[test]
    fn schedule_any_upstream_available_includes_from_output() {
        let mut graph = Graph::new();
        graph.add_edge(Edge {
            from: "a".to_string(),
            to: "b".to_string(),
            condition: None,
        });

        let graph = Arc::new(graph);
        let mut node_trigger_strategy = HashMap::new();
        node_trigger_strategy.insert("b".to_string(), TriggerStrategy::AnyUpstreamAvailable);

        let scheduler = Scheduler {
            graph,
            task_queue: VecDeque::new(),
            runtime_nodes: HashMap::new(),
            node_trigger_strategy,
        };

        let exec_ctx = ExecutionContext::new();
        exec_ctx.set_output("a".to_string(), json!({"x": 1}));
        let runtime_ctx = Context::new();

        let starts = scheduler
            .schedule_from_output("a", &json!({"x": 1}), &exec_ctx, &runtime_ctx)
            .unwrap();
        assert_eq!(starts.len(), 1);
        assert_eq!(starts[0].node_id, "b");
        assert_eq!(starts[0].inputs.get_str("a"), Some(&json!({"x": 1})));
    }

    #[test]
    fn schedule_all_upstream_ready_waits_for_all_parents_and_collects_outputs() {
        let mut graph = Graph::new();
        graph.add_edge(Edge {
            from: "a".to_string(),
            to: "c".to_string(),
            condition: None,
        });
        graph.add_edge(Edge {
            from: "b".to_string(),
            to: "c".to_string(),
            condition: None,
        });

        let graph = Arc::new(graph);
        let scheduler = Scheduler {
            graph,
            task_queue: VecDeque::new(),
            runtime_nodes: HashMap::new(),
            node_trigger_strategy: HashMap::new(),
        };

        let exec_ctx = ExecutionContext::new();
        exec_ctx.set_state("a".to_string(), NodeState::Completed);
        exec_ctx.set_output("a".to_string(), json!({"a": 1}));
        exec_ctx.set_state("b".to_string(), NodeState::Pending);
        exec_ctx.set_output("b".to_string(), json!({"b": 1}));
        let runtime_ctx = Context::new();

        let starts = scheduler
            .schedule_from_output("a", &json!({"a": 1}), &exec_ctx, &runtime_ctx)
            .unwrap();
        assert_eq!(starts.len(), 0);

        exec_ctx.set_state("b".to_string(), NodeState::Completed);
        let starts = scheduler
            .schedule_from_output("a", &json!({"a": 1}), &exec_ctx, &runtime_ctx)
            .unwrap();
        assert_eq!(starts.len(), 1);
        assert_eq!(starts[0].node_id, "c");
        assert_eq!(starts[0].inputs.get_str("a"), Some(&json!({"a": 1})));
        assert_eq!(starts[0].inputs.get_str("b"), Some(&json!({"b": 1})));

        exec_ctx.set_state("c".to_string(), NodeState::Running);
        let starts = scheduler
            .schedule_from_output("a", &json!({"a": 1}), &exec_ctx, &runtime_ctx)
            .unwrap();
        assert_eq!(starts.len(), 0);
    }
}
