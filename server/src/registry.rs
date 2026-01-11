use chrono::{Local, NaiveDateTime};
use flow::AnyNode;
use nodes::trade::{Backtester, ReactiveSourceNode, VOLMFINode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type NodeCreator = Box<dyn Fn(Value) -> Result<Arc<RwLock<dyn AnyNode>>, String> + Send + Sync>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub name: String,
    pub description: String,
    pub category: String,
    // Simple schema description: fields and their types
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub config: Value, // Example config or schema
}

pub struct NodeRegistry {
    creators: HashMap<String, NodeCreator>,
    metadata: HashMap<String, NodeMetadata>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            creators: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, meta: NodeMetadata, creator: F)
    where
        F: Fn(Value) -> Result<Arc<RwLock<dyn AnyNode>>, String> + Send + Sync + 'static,
    {
        self.creators.insert(meta.name.clone(), Box::new(creator));
        self.metadata.insert(meta.name.clone(), meta);
    }

    pub fn create_node(
        &self,
        name: &str,
        config: Value,
    ) -> Result<Arc<RwLock<dyn AnyNode>>, String> {
        if let Some(creator) = self.creators.get(name) {
            creator(config)
        } else {
            Err(format!("Node type '{}' not found", name))
        }
    }

    pub fn get_metadata(&self) -> Vec<NodeMetadata> {
        self.metadata.values().cloned().collect()
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();

    registry.register(
        NodeMetadata {
            name: "ReactiveSourceNode".to_string(),
            description: "Source node providing K-line data".to_string(),
            category: "Source".to_string(),
            inputs: vec![],
            outputs: vec!["ReactiveStream<K>".to_string()],
            config: json!({
                "code": "510300.SH",
                "start_time": "2023-01-01T00:00:00",
                "end_time": null
            }),
        },
        Box::new(|config: Value| {
            let code = config["code"].as_str().unwrap_or("510300.SH");
            let start_str = config["start_time"]
                .as_str()
                .unwrap_or("2023-01-01T00:00:00");
            let start_time = NaiveDateTime::parse_from_str(start_str, "%Y-%m-%dT%H:%M:%S")
                .map_err(|e| e.to_string())?
                .and_local_timezone(Local)
                .single()
                .ok_or("Invalid local time")?;

            let end_time = if let Some(s) = config["end_time"].as_str() {
                Some(
                    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                        .map_err(|e| e.to_string())?
                        .and_local_timezone(Local)
                        .single()
                        .ok_or("Invalid local time")?,
                )
            } else {
                None
            };

            let node =
                ReactiveSourceNode::new(code, start_time, end_time).map_err(|e| e.to_string())?;
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        }),
    );

    registry.register(
        NodeMetadata {
            name: "VOLMFINode".to_string(),
            description: "Volume Money Flow Index Strategy".to_string(),
            category: "Strategy".to_string(),
            inputs: vec!["K".to_string()],
            outputs: vec!["BacktesterInput".to_string()],
            config: json!({
                "ema_period": 8,
                "mfi_period": 8
            }),
        },
        Box::new(|config: Value| {
            let ema = config["ema_period"].as_u64().unwrap_or(8) as usize;
            let mfi = config["mfi_period"].as_u64().unwrap_or(8) as usize;
            let node = VOLMFINode::new(ema, mfi);
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        }),
    );

    registry.register(
        NodeMetadata {
            name: "Backtester".to_string(),
            description: "Backtesting Engine".to_string(),
            category: "Sink".to_string(),
            inputs: vec!["BacktesterInput".to_string()],
            outputs: vec![],
            config: json!({
                "initial_capital": 500000.0,
                "transaction_cost": 0.0002354
            }),
        },
        Box::new(|config: Value| {
            let capital = config["initial_capital"].as_f64().unwrap_or(500000.0);
            let cost = config["transaction_cost"].as_f64().unwrap_or(0.0002354);
            let node = Backtester::new(capital, cost);
            Ok(Arc::new(RwLock::new(node)) as Arc<RwLock<dyn AnyNode>>)
        }),
    );

    registry
}
